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
            let mut messages = vec![
                json!({"role":"system","content":SYSTEM_PROMPT}),
                json!({"role":"user","content":prompt(s,secondary,h,q).to_string()}),
            ];
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
                    "max_tokens":1024,"temperature":0.5,"response_format":{"type":"json_object"}}))
                    .send()
                    .await?;
                // Never return provider bodies/credentials to the client or logs.
                anyhow::ensure!(
                    response.status().is_success(),
                    "Provider HTTP error {}",
                    response.status()
                );
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
                match validate_turn_reply(s, secondary, h, raw) {
                    Ok(r) => return Ok(r),
                    Err(e) if attempt == 0 => {
                        messages.push(json!({"role":"assistant","content":raw}));
                        messages.push(json!({"role":"user","content":format!("Contract validation failed: {e}. Repair the JSON only. Do not change scenario facts.")}));
                    }
                    Err(_) => anyhow::bail!(
                        "Provider returned invalid dialogue after repair. Retry the question."
                    ),
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
        .map_err(|e| error(StatusCode::BAD_GATEWAY, e))?;
    live.history.push(Turn {
        officer: req.utterance,
        traveler: reply.dialogue.clone(),
        emotion: reply.emotion.clone(),
    });
    if live.history.len() > 30 {
        live.history.remove(0);
    }
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
}
