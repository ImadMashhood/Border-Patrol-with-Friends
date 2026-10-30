use super::*;
use serde::{Serialize, de::DeserializeOwned};
#[derive(Resource)]
pub struct Network {
    pub base: String,
    pub spatial: Arc<Mutex<border_speech::SpatialVoice>>,
    pub token: String,
    pub retry_until: Arc<Mutex<std::time::Instant>>,
    pub tx: mpsc::Sender<(u64, NetEvent)>,
    pub rx: Arc<Mutex<mpsc::Receiver<(u64, NetEvent)>>>,
}
pub enum NetEvent {
    Started(PublicCase),
    Reply(String, border_caller::ActorReply),
    Evidence(Vec<Evidence>),
    Decision(Reveal),
    Error(String),
    TurnFailed {
        message: String,
        question: String,
    },
    Microphone(border_speech::VoiceEvent),
    SpeechFinished {
        error: Option<String>,
        fallback: bool,
    },
}
impl Network {
    fn request<T: Serialize + Send + 'static, R: DeserializeOwned + Serialize + Send + 'static>(
        &self,
        generation: u64,
        path: &str,
        body: T,
        map: impl FnOnce(R) -> NetEvent + Send + 'static,
        retry_question: Option<String>,
    ) {
        let url = format!("{}{path}", self.base);
        let token = self.token.clone();
        let tx = self.tx.clone();
        let retry_until = self.retry_until.clone();
        let is_turn = path == "/turn";
        let request_id = border_caller::call_log::next_id();
        let fields = serde_json::to_value(&body).unwrap_or_default();
        border_caller::call_log::record(
            "request",
            serde_json::json!({"request_id":request_id,"generation":generation,"path":path,"session_id":fields["session_id"],"officer":fields["utterance"]}),
        );
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let result = (|| -> Result<R, String> {
                if is_turn && *retry_until.lock().unwrap() > std::time::Instant::now() {
                    return Err("AI provider cooldown is active; wait before asking again.".into());
                }
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(27))
                    .build()
                    .map_err(|e| e.to_string())?;
                let r=client.post(url).bearer_auth(token).json(&body).send().map_err(|_|"Cannot reach interview service. Launch with ./play.sh, then press E to retry.".to_string())?;
                if !r.status().is_success() {
                    let status = r.status();
                    let body: serde_json::Value = r.json().unwrap_or_default();
                    if is_turn {
                        let seconds = body["retry_after_seconds"]
                            .as_u64()
                            .unwrap_or(if status.as_u16() == 429 { 30 } else { 5 })
                            .clamp(1, 120);
                        *retry_until.lock().unwrap() =
                            std::time::Instant::now() + Duration::from_secs(seconds);
                    }
                    border_caller::call_log::record(
                        "http_error",
                        serde_json::json!({"request_id":request_id,"status":status.as_u16(),"message":body["error"],"retry_after_seconds":body["retry_after_seconds"]}),
                    );
                    return Err(body["error"]
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| format!("Service error {status}")));
                }
                r.json()
                    .map_err(|_| "Invalid response from interview service".into())
            })();
            let event = match result {
                Ok(v) => {
                    let reply = serde_json::to_value(&v).unwrap_or_default();
                    border_caller::call_log::record(
                        "reply",
                        serde_json::json!({"request_id":request_id,"generation":generation,"latency_ms":started.elapsed().as_millis(),"traveler":reply["dialogue"],"opening":reply["opening_line"],"emotion":reply["emotion"]}),
                    );
                    map(v)
                }
                Err(message) => {
                    if is_turn && *retry_until.lock().unwrap() <= std::time::Instant::now() {
                        *retry_until.lock().unwrap() =
                            std::time::Instant::now() + Duration::from_secs(5);
                    }
                    border_caller::call_log::record(
                        "request_failed",
                        serde_json::json!({"request_id":request_id,"generation":generation,"latency_ms":started.elapsed().as_millis(),"message":message}),
                    );
                    if let Some(question) = retry_question {
                        NetEvent::TurnFailed { message, question }
                    } else {
                        NetEvent::Error(message)
                    }
                }
            };
            let _ = tx.send((generation, event));
        });
    }
    pub fn ready_for_turn(&self) -> bool {
        *self.retry_until.lock().unwrap() <= std::time::Instant::now()
    }
    pub fn retry_seconds(&self) -> u64 {
        self.retry_until
            .lock()
            .unwrap()
            .saturating_duration_since(std::time::Instant::now())
            .as_secs()
            .saturating_add(1)
    }
    pub fn start(&self, g: u64, id: String) {
        self.request(
            g,
            "/start",
            border_caller::SessionRequest { session_id: id },
            NetEvent::Started,
            None,
        );
    }
    pub fn turn(&self, g: u64, id: String, q: String) {
        let question = q.clone();
        let retry_question = Some(q.clone());
        self.request(
            g,
            "/turn",
            border_caller::TurnRequest {
                session_id: id,
                utterance: q,
            },
            move |r| NetEvent::Reply(question, r),
            retry_question,
        );
    }
    pub fn inspect(&self, g: u64, id: String) {
        self.request(
            g,
            "/inspect",
            border_caller::SessionRequest { session_id: id },
            NetEvent::Evidence,
            None,
        );
    }
    pub fn decide(&self, g: u64, id: String, d: border_scenarios::Decision) {
        self.request(
            g,
            "/decide",
            border_caller::DecisionRequest {
                session_id: id,
                decision: d,
            },
            |r: border_caller::DecisionResponse| NetEvent::Decision(r.reveal),
            None,
        );
    }
    pub fn microphone(
        &self,
        g: u64,
        device: &str,
        threshold: f32,
    ) -> Result<border_speech::OpenMicrophone, String> {
        let tx = self.tx.clone();
        border_speech::OpenMicrophone::start(device, threshold, move |event| {
            let _ = tx.send((g, NetEvent::Microphone(event)));
        })
        .map_err(|e| e.to_string())
    }
    pub fn speak(
        &self,
        g: u64,
        text: String,
        scenario_id: String,
    ) -> border_speech::SpeechPlayback {
        self.speak_emotional(g, text, scenario_id, "cooperative".into())
    }
    pub fn speak_emotional(
        &self,
        g: u64,
        text: String,
        scenario_id: String,
        emotion: String,
    ) -> border_speech::SpeechPlayback {
        let handle = border_speech::SpeechPlayback::default();
        let cancel = handle.token();
        let tx = self.tx.clone();
        let spatial = self.spatial.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let fallback = text == "Excuse me?";
            border_caller::call_log::record(
                "speech_started",
                serde_json::json!({"generation":g,"scenario_id":scenario_id,"text":text,"emotion":emotion,"fallback":fallback}),
            );
            let error = border_speech::speak_spatial_expressive(
                &text,
                &scenario_id,
                &emotion,
                &cancel,
                &spatial,
            )
            .err()
            .map(|e| e.to_string());
            border_caller::call_log::record(
                "speech_finished",
                serde_json::json!({"generation":g,"elapsed_ms":started.elapsed().as_millis(),"error":error,"fallback":fallback}),
            );
            let _ = tx.send((g, NetEvent::SpeechFinished { error, fallback }));
        });
        handle
    }
}

/// Follow the displayed head bone after transform propagation, including vehicle/exit motion.
pub fn spatial_voice(
    net: Res<Network>,
    g: Res<Patrol>,
    camera: Query<&GlobalTransform, With<Player>>,
    heads: Query<(&vehicle_physics::BodyPart, &ChildOf, &GlobalTransform)>,
    people: Query<&vehicle_physics::Detail>,
) {
    let mut voice = net.spatial.lock().unwrap();
    voice.active = false;
    let Ok(listener) = camera.single() else {
        return;
    };
    voice.listener = listener.translation().to_array();
    voice.right = listener.right().to_array();
    if let Some(id) = g.vehicle_id {
        for (part, parent, face) in &heads {
            if part.bone == 2
                && people.get(parent.parent()).is_ok_and(|p| {
                    p.vehicle == id && matches!(p.kind, vehicle_physics::DetailKind::Npc)
                })
            {
                voice.source = (face.translation() + face.forward() * 0.16).to_array();
                voice.active = true;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn rate_limit_preserves_diagnostics_and_blocks_a_second_provider_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).unwrap() > 0);
            let body = r#"{"error":"Test provider quota reached","retry_after_seconds":30}"#;
            write!(stream,"HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let (tx, rx) = mpsc::channel();
        let net = Network {
            base,
            token: "test-secret-do-not-log".into(),
            spatial: Default::default(),
            retry_until: Arc::new(Mutex::new(std::time::Instant::now())),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };
        net.turn(8, "vehicle-one".into(), "What is your name?".into());
        let (_, event) = net
            .rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        match event {
            NetEvent::TurnFailed { message, question } => {
                assert_eq!(message, "Test provider quota reached");
                assert_eq!(question, "What is your name?");
            }
            _ => panic!("Expected visible failure"),
        }
        assert!(!net.ready_for_turn());
        net.turn(8, "vehicle-one".into(), "Can you hear me?".into());
        let (_, event) = net
            .rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        assert!(matches!(event,NetEvent::TurnFailed{message,..} if message.contains("cooldown")));
        server.join().unwrap();
        let directory = std::env::var_os("BORDER_CALL_LOG_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "output/call-logs".into());
        let logs =
            std::fs::read_to_string(directory.join(format!("calls-{}.jsonl", std::process::id())))
                .unwrap();
        assert!(logs.contains("Test provider quota reached"));
        assert!(!logs.contains("test-secret-do-not-log"));
        for line in logs.lines() {
            serde_json::from_str::<serde_json::Value>(line).unwrap();
        }
    }
}
