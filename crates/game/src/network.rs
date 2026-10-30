use super::*;
use serde::{Serialize, de::DeserializeOwned};
#[derive(Resource)]
pub struct Network {
    pub base: String,
    pub spatial: Arc<Mutex<border_speech::SpatialVoice>>,
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
    fn request<T: Serialize + Send + 'static, R: DeserializeOwned + Send + 'static>(
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
                Err(message) => {
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
        let handle = border_speech::SpeechPlayback::default();
        let cancel = handle.token();
        let tx = self.tx.clone();
        let spatial = self.spatial.clone();
        std::thread::spawn(move || {
            let fallback = text == "Excuse me?";
            let error =
                border_speech::speak_spatial_cancellable(&text, &scenario_id, &cancel, &spatial)
                    .err()
                    .map(|e| e.to_string());
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
