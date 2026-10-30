//! Headless two-player host. Run from the repository root; see docs/HOST.md.
use anyhow::{Context, Result, ensure};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use border_net::*;
use border_simulation::Simulation;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;
struct Host {
    sim: Simulation,
    tokens: HashMap<String, u64>,
    join_code: String,
    operator: String,
    gateway: String,
    epoch: Instant,
    manual: bool,
    manual_now: u64,
    gateway_url: String,
    gateway_auth: String,
    pending: HashSet<u64>,
    mode: String,
}
type Shared = Arc<Mutex<Host>>;
struct PendingReply {
    state: Shared,
    vehicle: u64,
}
impl Drop for PendingReply {
    fn drop(&mut self) {
        if let Ok(mut host) = self.state.lock() {
            host.pending.remove(&self.vehicle);
        }
    }
}

type Api<T> = Result<Json<T>, (StatusCode, Json<Value>)>;
fn error(e: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error":e.to_string()})),
    )
}
fn bearer(h: &HeaderMap) -> &str {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
}
fn peer(h: &Host, headers: &HeaderMap) -> Result<u64> {
    h.tokens
        .get(bearer(headers))
        .copied()
        .context("invalid peer token")
}
fn now(h: &Host) -> u64 {
    if h.manual {
        h.manual_now
    } else {
        h.epoch.elapsed().as_millis() as u64
    }
}
async fn join(State(s): State<Shared>, Json(r): Json<JoinRequest>) -> Api<JoinResponse> {
    let mut h = s.lock().unwrap();
    ensure_api(
        r.protocol_version == PROTOCOL_VERSION,
        "unsupported protocol",
    )?;
    ensure_api(r.join_code == h.join_code, "invalid join code")?;
    let time = now(&h);
    let peer_id = h.sim.join(r.display_name, time).map_err(error)?;
    let token = Uuid::new_v4().to_string();
    h.tokens.insert(token.clone(), peer_id);
    Ok(Json(JoinResponse {
        protocol_version: PROTOCOL_VERSION,
        session_id: h.sim.snapshot().session_id,
        peer_id,
        peer_token: token,
        next_sequence: 1,
        snapshot: h.sim.snapshot(),
    }))
}
fn ensure_api(ok: bool, message: &str) -> Result<(), (StatusCode, Json<Value>)> {
    if ok { Ok(()) } else { Err(error(message)) }
}
async fn command(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<CommandEnvelope>,
) -> Api<CommandReceipt> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    ensure_api(
        !matches!(r.command, ClientCommand::ResetShift) || h.pending.is_empty(),
        "wait for pending AI replies before resetting",
    )?;
    let time = now(&h);
    h.sim.tick(time);
    Ok(Json(h.sim.command(id, r, time).map_err(error)?))
}
async fn heartbeat(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<SessionRequest>,
) -> Api<Value> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    h.sim
        .check_session(r.protocol_version, &r.session_id)
        .map_err(error)?;
    let time = now(&h);
    h.sim.heartbeat(id, time).map_err(error)?;
    Ok(Json(json!({"ok":true})))
}
async fn leave(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<SessionRequest>,
) -> Api<Value> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    h.sim
        .check_session(r.protocol_version, &r.session_id)
        .map_err(error)?;
    h.sim.leave(id);
    h.tokens.retain(|_, v| *v != id);
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct Cursor {
    after: Option<u64>,
}
async fn snapshot(State(s): State<Shared>, headers: HeaderMap) -> Api<WorldSnapshot> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    let time = now(&h);
    h.sim.heartbeat(id, time).map_err(error)?;
    Ok(Json(h.sim.snapshot()))
}
async fn events(
    State(s): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<Cursor>,
) -> Api<Replay> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    let time = now(&h);
    h.sim.heartbeat(id, time).map_err(error)?;
    Ok(Json(h.sim.replay(q.after.unwrap_or(0)).map_err(error)?))
}
async fn operate(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<HostEnvelope>,
) -> Api<WorldSnapshot> {
    let mut h = s.lock().unwrap();
    ensure_api(bearer(&headers) == h.operator, "invalid operator token")?;
    h.sim
        .check_session(r.protocol_version, &r.session_id)
        .map_err(error)?;
    if let HostCommand::AdvanceWorld { delta_ms } = r.command {
        ensure_api(
            h.manual,
            "world time is automatic; manual ticks are disabled",
        )?;
        ensure_api(delta_ms <= 1000, "tick too large")?;
        h.manual_now += delta_ms;
        let time = h.manual_now;
        h.sim.tick(time);
    } else {
        h.sim.operator(r.command).map_err(error)?;
    }
    Ok(Json(h.sim.snapshot()))
}
async fn input(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<InputFrame>,
) -> Api<WorldSnapshot> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    let time = now(&h);
    h.sim.tick(time);
    h.sim.input(id, r, time).map_err(error)?;
    Ok(Json(h.sim.snapshot()))
}
async fn start_case(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<border_caller::SessionRequest>,
) -> Api<border_caller::PublicCase> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    let mode = h.mode.clone();
    Ok(Json(
        h.sim
            .public_case(id, r.session_id.parse().map_err(error)?, &mode)
            .map_err(error)?,
    ))
}
async fn inspect_case(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<border_caller::SessionRequest>,
) -> Api<Vec<border_scenarios::Evidence>> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    Ok(Json(
        h.sim
            .inspect_case(id, r.session_id.parse().map_err(error)?)
            .map_err(error)?,
    ))
}
async fn decide_case(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<border_caller::DecisionRequest>,
) -> Api<border_caller::DecisionResponse> {
    let mut h = s.lock().unwrap();
    let id = peer(&h, &headers).map_err(error)?;
    let vehicle = r.session_id.parse().map_err(error)?;
    // Cached outcomes are safe to replay; unresolved cases still require physical access.
    if h.sim
        .snapshot()
        .vehicles
        .iter()
        .any(|v| v.vehicle_id == vehicle && v.outcome.is_none())
    {
        h.sim.authorize_case(id, vehicle).map_err(error)?;
    }
    let disposition = match r.decision {
        border_scenarios::Decision::Admit => Disposition::Admit,
        border_scenarios::Decision::Deny => Disposition::Deny,
        border_scenarios::Decision::Detain => Disposition::Detain,
    };
    Ok(Json(border_caller::DecisionResponse {
        reveal: h.sim.decide_case(id, vehicle, disposition).map_err(error)?,
    }))
}
async fn turn_case(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<border_caller::TurnRequest>,
) -> Api<border_caller::ActorReply> {
    let vehicle: u64 = r.session_id.parse().map_err(error)?;
    let question = r.utterance.clone();
    let (url, token, session, speaker, epoch, turn) = {
        let mut h = s.lock().unwrap();
        let id = peer(&h, &headers).map_err(error)?;
        h.sim.authorize_conversation(id, vehicle).map_err(error)?;

        ensure_api(
            !h.pending.contains(&vehicle),
            "another officer is awaiting this traveler's reply",
        )?;
        ensure_api(
            r.utterance.chars().count() <= 500 && !r.utterance.trim().is_empty(),
            "question must be 1–500 characters",
        )?;
        h.sim
            .register_utterance(id, vehicle, &r.utterance)
            .map_err(error)?;
        let context = h.sim.actor_context(vehicle).map_err(error)?;
        h.pending.insert(vehicle);
        (
            h.gateway_url.clone(),
            h.gateway_auth.clone(),
            h.sim.snapshot().session_id,
            id,
            context.conversation_epoch,
            context.expected_turn,
        )
    };
    let _pending = PendingReply {
        state: s.clone(),
        vehicle,
    };
    let result = async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(error)?;
        let response = client
            .post(format!("{}/host-turn", url.trim_end_matches('/')))
            .bearer_auth(token)
            .json(&HostTurnRequest {
                protocol_version: PROTOCOL_VERSION,
                session_id: session,
                vehicle_id: vehicle,
                utterance: r.utterance,
            })
            .send()
            .await
            .map_err(|_| {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"error":"AI gateway unavailable","retry_after_seconds":5})),
                )
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let body: Value = response.json().await.unwrap_or_else(
                |_| json!({"error":"AI gateway reply unavailable","retry_after_seconds":5}),
            );
            return Err((status, Json(body)));
        }
        response
            .json::<border_caller::ActorReply>()
            .await
            .map(Json)
            .map_err(|_| error("invalid AI gateway response"))
    }
    .await;
    match result {
        Ok(reply) => Ok(reply),
        Err(failure) => {
            border_caller::call_log::record(
                "gateway_failure",
                json!({"vehicle_id":vehicle,"officer":question,"status":failure.0.as_u16(),"message":failure.1.0["error"],"retry_after_seconds":failure.1.0["retry_after_seconds"]}),
            );
            let mut h = s.lock().unwrap();
            h.sim
                .delivery_failure(speaker, vehicle, epoch, turn, &question)
                .map_err(error)?;
            // Remember the pardon, but preserve the failure status for the client's
            // diagnostics and backoff. An HTTP 200 here concealed every outage.
            Err(failure)
        }
    }
}
async fn actor_context(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<ActorContextRequest>,
) -> Api<border_simulation::ActorContext> {
    let h = s.lock().unwrap();
    ensure_api(bearer(&headers) == h.gateway, "invalid gateway token")?;
    h.sim
        .check_session(r.protocol_version, &r.session_id)
        .map_err(error)?;
    Ok(Json(h.sim.actor_context(r.vehicle_id).map_err(error)?))
}
async fn actor(
    State(s): State<Shared>,
    headers: HeaderMap,
    Json(r): Json<ActorDelivery>,
) -> Api<Value> {
    let mut h = s.lock().unwrap();
    ensure_api(bearer(&headers) == h.gateway, "invalid gateway token")?;
    h.sim.actor(r).map_err(error)?;
    Ok(Json(json!({"ok":true})))
}
#[tokio::main]
async fn main() -> Result<()> {
    let operator = std::env::var("BORDER_HOST_OPERATOR_TOKEN")
        .context("set BORDER_HOST_OPERATOR_TOKEN (at least 24 characters)")?;
    let gateway = std::env::var("BORDER_HOST_GATEWAY_TOKEN")
        .context("set BORDER_HOST_GATEWAY_TOKEN (at least 24 characters)")?;
    let join_code = std::env::var("BORDER_HOST_JOIN_CODE")
        .context("set BORDER_HOST_JOIN_CODE (at least 8 characters)")?;
    ensure!(
        operator.len() >= 24 && gateway.len() >= 24 && join_code.len() >= 8,
        "host credentials too short"
    );
    ensure!(
        operator != gateway && operator != join_code && gateway != join_code,
        "use distinct host credentials"
    );
    let seed = std::env::var("BORDER_HOST_SEED")
        .unwrap_or_else(|_| "42".into())
        .parse()?;
    let root = std::env::var("BORDER_SCENARIOS").unwrap_or_else(|_| "assets/scenarios".into());
    let pack = border_scenarios::load_pack(std::path::Path::new(&root))?;
    let state = Arc::new(Mutex::new(Host {
        sim: Simulation::new(Uuid::new_v4().to_string(), seed, pack)?,
        tokens: HashMap::new(),
        join_code,
        operator,
        gateway,
        epoch: Instant::now(),
        manual: std::env::var("BORDER_HOST_MANUAL_CLOCK").as_deref() == Ok("1"),
        manual_now: 0,
        gateway_url: std::env::var("BORDER_GATEWAY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3847".into()),
        gateway_auth: std::env::var("BORDER_SESSION_TOKEN")
            .unwrap_or_else(|_| "local-development-only".into()),
        pending: HashSet::new(),
        mode: if std::env::var("OPENROUTER_API_KEY").is_ok() {
            "live AI / host gateway"
        } else {
            "offline rehearsal / host gateway"
        }
        .into(),
    }));
    let expiry = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
        loop {
            tick.tick().await;
            let mut h = expiry.lock().unwrap();
            let time = now(&h);
            h.sim.expire(time);
            if !h.manual {
                h.sim.tick(time);
            }
            let connected: Vec<_> = h
                .sim
                .snapshot()
                .peers
                .iter()
                .filter(|p| p.connected)
                .map(|p| p.peer_id)
                .collect();
            h.tokens.retain(|_, id| connected.contains(id));
        }
    });
    let app = Router::new()
        .route(
            "/health",
            get(|| async {
                Json(json!({"protocol_version":PROTOCOL_VERSION,"service":"border-host"}))
            }),
        )
        .route("/join", post(join))
        .route("/command", post(command))
        .route("/input", post(input))
        .route("/start", post(start_case))
        .route("/turn", post(turn_case))
        .route("/inspect", post(inspect_case))
        .route("/decide", post(decide_case))
        .route("/heartbeat", post(heartbeat))
        .route("/leave", post(leave))
        .route("/snapshot", get(snapshot))
        .route("/events", get(events))
        .route("/operator", post(operate))
        .route("/actor", post(actor))
        .route("/actor-context", post(actor_context))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state);
    let bind = std::env::var("BORDER_HOST_BIND").unwrap_or_else(|_| "127.0.0.1:8090".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!(
        "San Ignacio headless host listening at {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}
