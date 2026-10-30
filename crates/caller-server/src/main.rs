use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use border_caller::*;
use border_scenarios::*;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    pack: Arc<Vec<Scenario>>,
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<Session>>>>>,
    provider: Arc<dyn Provider>,
    token: String,
}
struct Session {
    scenario: usize,
    history: Vec<Turn>,
    secondary: bool,
    committed: bool,
    attempts: u32,
    created: Instant,
}
// Provider future is object safe and permits mocked adapters without the renderer.
type ProviderFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<ActorReply>> + Send + 'a>>;
trait Provider: Send + Sync {
    fn mode(&self) -> &'static str;
    fn reply<'a>(
        &'a self,
        s: &'a Scenario,
        secondary: bool,
        history: &'a [Turn],
        q: &'a str,
    ) -> ProviderFuture<'a>;
}
struct Offline;
impl Provider for Offline {
    fn mode(&self) -> &'static str {
        "offline rehearsal"
    }
    fn reply<'a>(
        &'a self,
        s: &'a Scenario,
        secondary: bool,
        h: &'a [Turn],
        q: &'a str,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let r = offline_reply(s, secondary, h, q);
            validate_reply(s, secondary, &serde_json::to_string(&r)?)
        })
    }
}
#[derive(Debug)]
struct ProviderHttp(StatusCode);
impl std::fmt::Display for ProviderHttp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "upstream status {}", self.0)
    }
}
impl std::error::Error for ProviderHttp {}
fn provider_error(e: anyhow::Error) -> (StatusCode, Json<Value>) {
    if let Some(ProviderHttp(code)) = e.downcast_ref::<ProviderHttp>() {
        return match *code {
            StatusCode::TOO_MANY_REQUESTS => error(
                StatusCode::TOO_MANY_REQUESTS,
                "AI provider is rate-limited. Wait a minute, then retry your question. The conversation is preserved.",
            ),
            StatusCode::UNAUTHORIZED | StatusCode::PAYMENT_REQUIRED => error(
                StatusCode::SERVICE_UNAVAILABLE,
                "AI provider credentials or credits are unavailable. The host must check the provider account.",
            ),
            _ => error(
                StatusCode::BAD_GATEWAY,
                "AI provider is temporarily unavailable. Retry your question.",
            ),
        };
    }
    error(
        StatusCode::BAD_GATEWAY,
        "AI response could not be delivered. Retry your question; no traveler reply was invented.",
    )
}
struct OpenRouter {
    client: reqwest::Client,
    key: String,
    model: String,
    attempts: AtomicU64,
    max_attempts: u64,
    input_rate: f64,
    output_rate: f64,
}
impl Provider for OpenRouter {
    fn mode(&self) -> &'static str {
        "live AI / OpenRouter"
    }
    fn reply<'a>(
        &'a self,
        s: &'a Scenario,
        secondary: bool,
        h: &'a [Turn],
        q: &'a str,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let started = Instant::now();
            let mut messages = conversation_messages(s, secondary, h, q);
            for attempt in 0..2 {
                anyhow::ensure!(
                    self.attempts.fetch_add(1, Ordering::Relaxed) < self.max_attempts,
                    "Process provider request budget exhausted"
                );
                let response = self
                    .client
                    .post("https://openrouter.ai/api/v1/chat/completions")
                    .bearer_auth(&self.key)
                    .json(&json!({"model":self.model,"messages":messages,
                    "max_tokens":1024,"temperature":0.5,"response_format":response_format(s,secondary),"provider":{"require_parameters":true}}))
                    .send()
                    .await?;
                // Never return provider bodies/credentials to the client or logs.
                if !response.status().is_success() {
                    return Err(ProviderHttp(response.status()).into());
                }
                let body: Value = response.json().await?;
                let input = body["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
                let output = body["usage"]["completion_tokens"].as_u64().unwrap_or(0);
                let estimate = (input as f64 * self.input_rate + output as f64 * self.output_rate)
                    / 1_000_000.;
                eprintln!(
                    "provider latency_ms={} input_tokens={input} output_tokens={output} estimated_usd={estimate:.6} repair={attempt}",
                    started.elapsed().as_millis()
                );
                let raw = body["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Provider missing dialogue"))?;
                match validate_turn_reply(s, secondary, h, q, raw) {
                    Ok(r) => return Ok(r),
                    Err(e) if attempt == 0 => {
                        eprintln!("provider output rejected before repair: {e}");
                        messages = repair_messages(s, secondary, h, q);
                    }
                    Err(e) => {
                        eprintln!("provider output rejected after repair: {e}");
                        anyhow::bail!(
                            "Provider returned invalid dialogue after repair. Retry the question."
                        )
                    }
                }
            }
            unreachable!()
        })
    }
}
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<Value>)>;
fn error(code: StatusCode, message: impl ToString) -> (StatusCode, Json<Value>) {
    (code, Json(json!({"error":message.to_string()})))
}
fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
    if headers.get("authorization").and_then(|v| v.to_str().ok())
        != Some(&format!("Bearer {}", state.token))
    {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "Invalid local session token",
        ));
    }
    Ok(())
}
async fn session(
    state: &AppState,
    id: &str,
) -> Result<Arc<Mutex<Session>>, (StatusCode, Json<Value>)> {
    state.sessions.lock().await.get(id).cloned().ok_or_else(|| {
        error(
            StatusCode::NOT_FOUND,
            "Session expired; reopen the interview",
        )
    })
}
/// Host URL and gateway credential come from server configuration, never player input.
async fn host_turn(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<border_net::HostTurnRequest>,
) -> ApiResult<ActorReply> {
    authorize(&s, &headers)?;
    if req.utterance.trim().is_empty() || req.utterance.chars().count() > 500 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Question must be 1–500 characters",
        ));
    }
    let url = std::env::var("BORDER_HOST_URL").map_err(|_| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Host bridge not configured",
        )
    })?;
    let token = std::env::var("BORDER_HOST_GATEWAY_TOKEN").map_err(|_| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Host bridge not configured",
        )
    })?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Host client unavailable"))?;
    let context = client
        .post(format!("{}/actor-context", url.trim_end_matches('/')))
        .bearer_auth(&token)
        .json(&border_net::ActorContextRequest {
            protocol_version: req.protocol_version,
            session_id: req.session_id.clone(),
            vehicle_id: req.vehicle_id,
        })
        .send()
        .await
        .map_err(|_| error(StatusCode::BAD_GATEWAY, "Host unavailable"))?;
    if !context.status().is_success() {
        return Err(error(
            StatusCode::CONFLICT,
            "Host rejected conversation context",
        ));
    }
    let context: border_simulation::ActorContext = context
        .json()
        .await
        .map_err(|_| error(StatusCode::BAD_GATEWAY, "Invalid host context"))?;
    let reply = tokio::time::timeout(
        Duration::from_secs(25),
        s.provider.reply(
            &context.scenario,
            context.secondary,
            &context.history,
            &req.utterance,
        ),
    )
    .await
    .map_err(|_| error(StatusCode::GATEWAY_TIMEOUT, "Provider timed out"))?
    .map_err(provider_error)?;
    let raw = serde_json::to_string(&reply)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid reply"))?;
    validate_turn_reply(
        &context.scenario,
        context.secondary,
        &context.history,
        &req.utterance,
        &raw,
    )
    .map_err(|_| {
        error(
            StatusCode::BAD_GATEWAY,
            "Provider returned invalid dialogue",
        )
    })?;
    let response = client
        .post(format!("{}/actor", url.trim_end_matches('/')))
        .bearer_auth(&token)
        .json(&border_net::ActorDelivery {
            case_epoch: context.case_epoch,
            expected_turn: context.expected_turn,
            protocol_version: req.protocol_version,
            session_id: req.session_id,
            vehicle_id: req.vehicle_id,
            officer_utterance: req.utterance,
            raw_reply: raw,
        })
        .send()
        .await
        .map_err(|_| error(StatusCode::BAD_GATEWAY, "Host unavailable"))?;
    if !response.status().is_success() {
        return Err(error(
            StatusCode::CONFLICT,
            "Host rejected stale or invalid dialogue",
        ));
    }
    Ok(Json(reply))
}
async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"ok":true,"service":"border-caller","protocol":1,"mode":s.provider.mode()}))
}
async fn start(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StartRequest>,
) -> ApiResult<PublicCase> {
    authorize(&s, &headers)?;
    let index = s
        .pack
        .iter()
        .position(|x| x.scenario_id == req.scenario_id)
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "Unknown case"))?;
    let case = &s.pack[index];
    let id = uuid::Uuid::new_v4().to_string();
    let mut sessions = s.sessions.lock().await;
    // Expired sessions are safe to evict; memory and session starts are bounded.
    sessions.retain(|_, v| {
        v.try_lock()
            .map(|v| v.created.elapsed() < Duration::from_secs(7200))
            .unwrap_or(true)
    });
    if sessions.len() >= 100 {
        return Err(error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many sessions; restart the local service",
        ));
    }
    sessions.insert(
        id.clone(),
        Arc::new(Mutex::new(Session {
            scenario: index,
            history: vec![],
            secondary: false,
            committed: false,
            attempts: 0,
            created: Instant::now(),
        })),
    );
    Ok(Json(PublicCase {
        declared_claims: case.public_claims.clone(),
        session_id: id,
        scenario_id: case.scenario_id.clone(),
        name: case.character.name.clone(),
        occupation: case.character.occupation.clone(),
        opening_line: case.opening_line.clone(),
        evidence: case.visible_evidence(false),
        mode: s.provider.mode().into(),
    }))
}
async fn turn(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TurnRequest>,
) -> ApiResult<ActorReply> {
    authorize(&s, &headers)?;
    if req.utterance.trim().is_empty() || req.utterance.chars().count() > 500 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Questions must contain 1–500 characters",
        ));
    }
    let handle = session(&s, &req.session_id).await?;
    let mut live = handle
        .try_lock()
        .map_err(|_| error(StatusCode::CONFLICT, "A reply is already pending"))?;
    if live.committed {
        return Err(error(StatusCode::CONFLICT, "Decision already committed"));
    }
    if live.attempts >= 40 {
        return Err(error(
            StatusCode::TOO_MANY_REQUESTS,
            "Case turn budget reached. Make a decision.",
        ));
    }
    live.attempts += 1;
    let result = tokio::time::timeout(
        Duration::from_secs(24),
        s.provider.reply(
            &s.pack[live.scenario],
            live.secondary,
            &live.history,
            &req.utterance,
        ),
    )
    .await;
    let reply = result
        .map_err(|_| {
            error(
                StatusCode::GATEWAY_TIMEOUT,
                "AI timed out. You can retry or make a decision.",
            )
        })?
        .map_err(provider_error)?;
    live.history.push(Turn {
        officer: req.utterance,
        traveler: reply.dialogue.clone(),
        emotion: reply.emotion.clone(),
    });
    Ok(Json(reply))
}
async fn inspect(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SessionRequest>,
) -> ApiResult<Vec<Evidence>> {
    authorize(&s, &headers)?;
    let h = session(&s, &req.session_id).await?;
    let mut live = h
        .try_lock()
        .map_err(|_| error(StatusCode::CONFLICT, "Wait for the current reply"))?;
    if live.committed {
        return Err(error(StatusCode::CONFLICT, "Decision already committed"));
    }
    live.secondary = true;
    Ok(Json(s.pack[live.scenario].visible_evidence(true)))
}
async fn decide(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DecisionRequest>,
) -> ApiResult<DecisionResponse> {
    authorize(&s, &headers)?;
    let h = session(&s, &req.session_id).await?;
    let mut live = h
        .try_lock()
        .map_err(|_| error(StatusCode::CONFLICT, "Wait for the current reply"))?;
    if live.committed {
        return Err(error(StatusCode::CONFLICT, "Decision already committed"));
    }
    live.committed = true;
    Ok(Json(DecisionResponse {
        reveal: s.pack[live.scenario].resolve(req.decision, live.secondary),
    }))
}
fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/start", post(start))
        .route("/turn", post(turn))
        .route("/host-turn", post(host_turn))
        .route("/inspect", post(inspect))
        .route("/decide", post(decide))
        .layer(DefaultBodyLimit::max(8192))
        .with_state(state)
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = std::env::var("BORDER_SCENARIOS").unwrap_or_else(|_| "assets/scenarios".into());
    let pack = load_pack(std::path::Path::new(&root))?;
    let token =
        std::env::var("BORDER_SESSION_TOKEN").unwrap_or_else(|_| "local-development-only".into());
    let provider: Arc<dyn Provider> = if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
        let model = std::env::var("OPENROUTER_MODEL")
            .map_err(|_| anyhow::anyhow!("Set OPENROUTER_MODEL to an available model ID"))?;
        Arc::new(OpenRouter {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            key,
            model,
            attempts: AtomicU64::new(0),
            max_attempts: std::env::var("BORDER_MAX_PROVIDER_REQUESTS")
                .unwrap_or_else(|_| "120".into())
                .parse()?,
            input_rate: std::env::var("MODEL_INPUT_USD_PER_MILLION")
                .unwrap_or_else(|_| "0".into())
                .parse()?,
            output_rate: std::env::var("MODEL_OUTPUT_USD_PER_MILLION")
                .unwrap_or_else(|_| "0".into())
                .parse()?,
        })
    } else {
        Arc::new(Offline)
    };
    let address = std::env::var("BORDER_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3847".into());
    let addr: std::net::SocketAddr = address.parse()?;
    anyhow::ensure!(
        addr.ip().is_loopback(),
        "This development service only supports loopback binding"
    );
    eprintln!("Border caller listening on {addr} — {}", provider.mode());
    let state = AppState {
        pack: Arc::new(pack),
        sessions: Arc::new(Mutex::new(HashMap::new())),
        provider,
        token,
    };
    axum::serve(tokio::net::TcpListener::bind(addr).await?, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Slow;
    impl Provider for Slow {
        fn mode(&self) -> &'static str {
            "slow mock"
        }
        fn reply<'a>(
            &'a self,
            _: &'a Scenario,
            _: bool,
            _: &'a [Turn],
            _: &'a str,
        ) -> ProviderFuture<'a> {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                anyhow::bail!("delayed")
            })
        }
    }
    #[tokio::test]
    async fn slow_provider_yields_to_other_tasks_and_can_be_cancelled() {
        let p = load_pack(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios"
        )))
        .unwrap();
        let now = Instant::now();
        let slow = tokio::time::timeout(
            Duration::from_millis(30),
            Slow.reply(&p[0], false, &[], "hello"),
        );
        let responsive = async {
            tokio::time::sleep(Duration::from_millis(1)).await;
            42
        };
        let (slow, fast) = tokio::join!(slow, responsive);
        assert!(slow.is_err());
        assert_eq!(fast, 42);
        assert!(now.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn provider_failures_are_actionable_and_never_expose_provider_bodies() {
        let (status, Json(body)) =
            provider_error(ProviderHttp(StatusCode::TOO_MANY_REQUESTS).into());
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("conversation is preserved")
        );
        let (_, Json(body)) =
            provider_error(anyhow::anyhow!("private upstream body and credentials"));
        assert!(!body.to_string().contains("private upstream"));
    }
}
