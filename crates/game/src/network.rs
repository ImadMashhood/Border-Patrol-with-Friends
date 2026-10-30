use super::*;
use serde::{Serialize, de::DeserializeOwned};
#[derive(Resource)]
pub struct Network {
    pub base: String,
    pub token: String,
    pub tx: mpsc::Sender<(u64, NetEvent)>,
    pub rx: Arc<Mutex<mpsc::Receiver<(u64, NetEvent)>>>,
}
pub enum NetEvent {
    Started(PublicCase),
    Reply(String, border_caller::ActorReply),
    Evidence(Vec<Evidence>),
    Decision(Reveal),
    Error(String),
    Microphone(border_speech::VoiceEvent),
    SpeechFinished(Option<String>),
}
impl Network {
    fn request<T: Serialize + Send + 'static, R: DeserializeOwned + Send + 'static>(
        &self,
        generation: u64,
        path: &str,
        body: T,
        map: impl FnOnce(R) -> NetEvent + Send + 'static,
    ) {
        let url = format!("{}{path}", self.base);
        let token = self.token.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<R, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(27))
                    .build()
                    .map_err(|e| e.to_string())?;
                let r=client.post(url).bearer_auth(token).json(&body).send().map_err(|_|"Cannot reach interview service. Launch with ./play.sh, then press E to retry.".to_string())?;
                if !r.status().is_success() {
                    let status = r.status();
                    let body: serde_json::Value = r.json().unwrap_or_default();
                    return Err(body["error"]
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| format!("Service error {status}")));
                }
                r.json()
                    .map_err(|_| "Invalid response from interview service".into())
            })();
            let event = match result {
                Ok(v) => map(v),
                Err(e) => NetEvent::Error(e),
            };
            let _ = tx.send((generation, event));
        });
    }
    pub fn start(&self, g: u64, id: String) {
        self.request(
            g,
            "/start",
            border_caller::StartRequest { scenario_id: id },
            NetEvent::Started,
        );
    }
    pub fn turn(&self, g: u64, id: String, q: String) {
        let question = q.clone();
        self.request(
            g,
            "/turn",
            border_caller::TurnRequest {
                session_id: id,
                utterance: q,
            },
            move |r| NetEvent::Reply(question, r),
        );
    }
    pub fn inspect(&self, g: u64, id: String) {
        self.request(
            g,
            "/inspect",
            border_caller::SessionRequest { session_id: id },
            NetEvent::Evidence,
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
    pub fn speak(&self, g: u64, text: String, scenario_id: String) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let error = border_speech::speak(&text, &scenario_id)
                .err()
                .map(|e| e.to_string());
            let _ = tx.send((g, NetEvent::SpeechFinished(error)));
        });
    }
}
