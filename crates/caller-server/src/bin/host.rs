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
    collections::HashMap,
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
}
type Shared = Arc<Mutex<Host>>;
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
    h.epoch.elapsed().as_millis() as u64
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
    let time = now(&h);
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
    h.sim.operator(r.command).map_err(error)?;
    Ok(Json(h.sim.snapshot()))
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
    let pack = border_scenarios::load_pack(std::path::Path::new("assets/scenarios"))?;
    let state = Arc::new(Mutex::new(Host {
        sim: Simulation::new(Uuid::new_v4().to_string(), seed, pack)?,
        tokens: HashMap::new(),
        join_code,
        operator,
        gateway,
        epoch: Instant::now(),
    }));
    let expiry = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tick.tick().await;
            let mut h = expiry.lock().unwrap();
            let time = now(&h);
            h.sim.expire(time);
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
