mod network;
mod scene;
mod ui;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use border_caller::{PublicCase, Turn};
use border_scenarios::{Evidence, Reveal};
use network::{NetEvent, Network};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Approach,
    Interview,
    Reveal,
    Departing,
    Complete,
}
#[derive(Resource)]
struct Patrol {
    ids: Vec<String>,
    position: usize,
    score: usize,
    seed: u64,
    phase: Phase,
    case: Option<PublicCase>,
    history: Vec<Turn>,
    evidence: Vec<Evidence>,
    input: String,
    subtitle: String,
    emotion: String,
    error: String,
    pending: bool,
    overlay: bool,
    looking: bool,
    aiming: bool,
    secondary: bool,
    evidence_open: bool,
    reveal: Option<Reveal>,
    departure: f32,
    generation: u64,
    muted: bool,
    mic_device: String,
    mic: Option<border_speech::OpenMicrophone>,
    mic_enabled: bool,
    mic_failed: bool,
    mic_threshold: f32,
    mic_active: bool,
    mic_error: String,
    speaking: bool,
    speech_pending: bool,
    request_focus: bool,
    settings_open: bool,
    map_open: bool,
}
impl Patrol {
    fn new(ids: Vec<String>, seed: u64) -> Self {
        Self {
            ids,
            position: 0,
            score: 0,
            seed,
            phase: Phase::Approach,
            case: None,
            history: vec![],
            evidence: vec![],
            input: String::new(),
            subtitle: String::new(),
            emotion: "cooperative".into(),
            error: String::new(),
            pending: false,
            overlay: false,
            looking: false,
            aiming: false,
            secondary: false,
            evidence_open: false,
            reveal: None,
            departure: 0.,
            generation: 0,
            muted: false,
            mic_device: "default".into(),
            mic: None,
            mic_enabled: true,
            mic_failed: false,
            mic_threshold: 0.015,
            mic_active: false,
            mic_error: String::new(),
            speaking: false,
            speech_pending: false,
            request_focus: false,
            settings_open: false,
            map_open: std::env::args().any(|a| a == "--show-map"),
        }
    }
    fn reset_case(&mut self) {
        self.case = None;
        self.history.clear();
        self.evidence.clear();
        self.input.clear();
        self.subtitle.clear();
        self.error.clear();
        self.pending = false;
        self.overlay = false;
        self.secondary = false;
        self.evidence_open = false;
        self.reveal = None;
        self.departure = 0.;
        self.generation += 1;
        self.mic = None;
        self.mic_active = false;
        self.speech_pending = false;
        self.speaking = false;
        self.phase = Phase::Approach;
    }
}
#[derive(Component)]
struct Player {
    yaw: f32,
    pitch: f32,
}
#[derive(Component)]
struct Car;
#[derive(Component)]
struct Barrier;
#[derive(Component)]
struct Solid {
    half: Vec3,
}
#[derive(Resource)]
struct SmokeTest {
    elapsed: f32,
    capture: bool,
    interview: bool,
    step: u8,
}
fn main() {
    let seed = std::env::args()
        .find_map(|a| a.strip_prefix("--seed=").and_then(|s| s.parse().ok()))
        .unwrap_or(42);
    let path = std::env::var("BORDER_SCENARIOS").unwrap_or_else(|_| "assets/scenarios".into());
    let pack = border_scenarios::load_pack(std::path::Path::new(&path))
        .expect("Scenario pack invalid; run from the repository root");
    let ids = border_scenarios::seeded_order(pack.len(), seed)
        .iter()
        .map(|i| pack[*i].scenario_id.clone())
        .collect();
    let (tx, rx) = mpsc::channel();
    let net = Network {
        base: std::env::var("BORDER_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3847".into()),
        token: std::env::var("BORDER_SESSION_TOKEN")
            .unwrap_or_else(|_| "local-development-only".into()),
        tx,
        rx: Arc::new(Mutex::new(rx)),
    };
    let interview_smoke = std::env::args().any(|a| a == "--smoke-interview");
    let smoke = interview_smoke || std::env::args().any(|a| a == "--smoke-test");
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.73, 0.79, 0.79)))
        .insert_resource(Patrol::new(ids, seed))
        .insert_resource(net)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Border Patrol with Friends • San Ignacio".into(),
                resolution: if std::env::args().any(|a| a == "--reference-hardware") {
                    (1920, 1080).into()
                } else {
                    (1440, 900).into()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, scene::setup)
        .add_systems(Update, (controls, receive, progress, voice).chain())
        .add_systems(Update, scene::atmosphere)
        .add_systems(EguiPrimaryContextPass, (ui::landmarks, ui::draw).chain());
    if smoke {
        // Rendering smoke checks never record the room or play spoken audio.
        let mut patrol = app.world_mut().resource_mut::<Patrol>();
        patrol.mic_enabled = false;
        patrol.muted = true;
        app.insert_resource(SmokeTest {
            elapsed: 0.,
            capture: false,
            interview: interview_smoke,
            step: 0,
        })
        .add_systems(Update, smoke_test);
    }
    app.run();
}
#[allow(clippy::too_many_arguments)] // Bevy system inputs are independently borrowed resources.
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    mut g: ResMut<Patrol>,
    net: Res<Network>,
    mut player: Query<(&mut Transform, &mut Player)>,
    solids: Query<(&Transform, &Solid), Without<Player>>,
    mut cursor: Single<&mut CursorOptions>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        g.overlay = false;
        g.looking = false;
        g.settings_open = false;
    }
    if keys.just_pressed(KeyCode::KeyM) && !g.overlay && !g.settings_open {
        g.map_open = !g.map_open;
    }
    if keys.just_pressed(KeyCode::Tab) {
        g.looking = !g.looking;
    }
    if mouse.just_pressed(MouseButton::Left) && !g.overlay {
        g.looking = true;
    }
    cursor.visible = !g.looking;
    cursor.grab_mode = if g.looking {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    let Ok((mut t, mut p)) = player.single_mut() else {
        return;
    };
    if g.looking {
        p.yaw -= motion.delta.x * 0.0025;
        p.pitch = (p.pitch - motion.delta.y * 0.0025).clamp(-1.4, 1.4);
        t.rotation = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.);
        let f = Vec3::new(-p.yaw.sin(), 0., -p.yaw.cos());
        let r = Vec3::new(p.yaw.cos(), 0., -p.yaw.sin());
        let axis = Vec2::new(
            (keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32) as f32,
            (keys.pressed(KeyCode::KeyW) as i32 - keys.pressed(KeyCode::KeyS) as i32) as f32,
        )
        .normalize_or_zero();
        let delta = (r * axis.x + f * axis.y)
            * time.delta_secs().min(0.05)
            * if keys.pressed(KeyCode::ShiftLeft) {
                5.8
            } else {
                3.2
            };
        for movement in [Vec3::new(delta.x, 0., 0.), Vec3::new(0., 0., delta.z)] {
            let candidate = t.translation + movement;
            let blocked = solids.iter().any(|(s, b)| {
                (candidate.x - s.translation.x).abs() < b.half.x + 0.22
                    && (candidate.z - s.translation.z).abs() < b.half.z + 0.22
            });
            if !blocked && scene::walkable(candidate) {
                t.translation = candidate;
            }
        }
    }
    let direction = *t.forward();
    g.aiming = matches!(g.phase, Phase::Approach | Phase::Interview)
        && ray_box(
            t.translation,
            direction,
            Vec3::new(-1.05, 0.2, -2.25),
            Vec3::new(1.05, 1.85, 2.25),
        )
        .is_some_and(|d| d <= 3.);
    if keys.just_pressed(KeyCode::KeyE) && g.aiming && !g.pending && (!g.overlay || g.looking) {
        g.overlay = true;
        g.looking = false;
        g.request_focus = true;
        if g.case.is_none() {
            g.pending = true;
            g.error.clear();
            net.start(g.generation, g.ids[g.position].clone());
        } else {
            g.phase = Phase::Interview;
        }
    }
}
fn ray_box(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut near: f32 = 0.;
    let mut far: f32 = f32::INFINITY;
    for i in 0..3 {
        if dir[i].abs() < 1e-6 {
            if origin[i] < min[i] || origin[i] > max[i] {
                return None;
            }
        } else {
            let a = (min[i] - origin[i]) / dir[i];
            let b = (max[i] - origin[i]) / dir[i];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}
fn receive(mut g: ResMut<Patrol>, net: Res<Network>) {
    let events: Vec<_> = net.rx.lock().unwrap().try_iter().collect();
    for (generation, event) in events {
        if generation != g.generation {
            continue;
        }
        match event {
            NetEvent::Started(case) => {
                g.subtitle = case.opening_line.clone();
                if !g.muted {
                    g.speaking = true;
                    net.speak(
                        g.generation,
                        case.opening_line.clone(),
                        case.scenario_id.clone(),
                    );
                }
                g.evidence = case.evidence.clone();
                g.case = Some(case);
                g.phase = Phase::Interview;
                g.pending = false;
                g.request_focus = true;
            }
            NetEvent::Reply(q, r) => {
                g.subtitle = r.dialogue.clone();
                g.emotion = format!("{:?}", r.emotion).to_lowercase();
                g.history.push(Turn {
                    officer: q,
                    traveler: r.dialogue.clone(),
                    emotion: r.emotion.clone(),
                });
                g.pending = false;
                g.request_focus = true;
                if !g.muted {
                    g.speaking = true;
                    net.speak(
                        g.generation,
                        r.dialogue,
                        g.case.as_ref().unwrap().scenario_id.clone(),
                    );
                }
            }
            NetEvent::Evidence(e) => {
                g.evidence = e;
                g.secondary = true;
                g.evidence_open = true;
                g.pending = false;
            }
            NetEvent::Decision(r) => {
                if r.correct {
                    g.score += 1;
                }
                g.reveal = Some(r);
                g.phase = Phase::Reveal;
                g.pending = false;
                g.overlay = true;
                g.looking = false;
                g.subtitle.clear();
            }
            NetEvent::Error(e) => {
                g.error = e;
                g.pending = false;
                g.speech_pending = false;
            }
            NetEvent::Microphone(event) => match event {
                border_speech::VoiceEvent::Listening => {
                    g.speech_pending = false;
                }
                border_speech::VoiceEvent::Activity(active) => {
                    g.mic_active = active;
                }
                border_speech::VoiceEvent::Transcribing => {
                    g.speech_pending = true;
                    g.mic_active = false;
                }
                border_speech::VoiceEvent::Transcript(q) => {
                    g.speech_pending = false;
                    if g.phase == Phase::Interview
                        && !g.pending
                        && !g.speaking
                        && g.mic_enabled
                        && let Some(case) = &g.case
                    {
                        net.turn(g.generation, case.session_id.clone(), q.clone());
                        g.subtitle = format!("You: {q}");
                        g.pending = true;
                        g.error.clear();
                    }
                }
                border_speech::VoiceEvent::Error(e) => {
                    g.mic_error = e;
                    g.mic_failed = true;
                    g.mic = None;
                    g.mic_active = false;
                    g.speech_pending = false;
                }
            },
            NetEvent::SpeechFinished(error) => {
                g.speaking = false;
                if let Some(e) = error {
                    g.error = e;
                }
            }
        }
    }
}
fn progress(
    time: Res<Time>,
    mut g: ResMut<Patrol>,
    mut cars: Query<&mut Transform, (With<Car>, Without<Barrier>)>,
    mut barriers: Query<&mut Transform, (With<Barrier>, Without<Car>)>,
) {
    if g.phase != Phase::Departing {
        return;
    }
    g.departure += time.delta_secs();
    let admitted = g
        .reveal
        .as_ref()
        .is_some_and(|r| r.decision == border_scenarios::Decision::Admit);
    for mut t in &mut barriers {
        t.rotation = Quat::from_rotation_z(if admitted {
            g.departure.min(1.) * -1.35
        } else {
            0.
        });
    }
    for mut t in &mut cars {
        t.translation.z =
            if admitted { -1. } else { 1. } * (g.departure - 0.8).max(0.).powi(2) * 5.;
    }
    if g.departure > 2.8 {
        g.position += 1;
        if g.position == g.ids.len() {
            g.phase = Phase::Complete;
            g.overlay = true;
            g.looking = false;
        } else {
            g.reset_case();
            g.looking = true;
        }
        for mut t in &mut cars {
            t.translation.z = 0.;
        }
        for mut t in &mut barriers {
            t.rotation = Quat::IDENTITY;
        }
    }
}
fn voice(mut g: ResMut<Patrol>, net: Res<Network>) {
    if g.phase != Phase::Interview || g.case.is_none() || !g.mic_enabled {
        g.mic = None;
        g.mic_active = false;
        return;
    }
    if g.mic.is_none() && !g.mic_failed {
        match net.microphone(g.generation, &g.mic_device, g.mic_threshold) {
            Ok(mic) => {
                g.mic = Some(mic);
                g.mic_error.clear();
            }
            Err(e) => {
                g.mic_error = e;
                g.mic_failed = true;
            }
        }
    }
    if let Some(mic) = &g.mic {
        mic.set_paused(g.pending || g.speaking || g.speech_pending);
    }
}

fn smoke_test(
    time: Res<Time>,
    mut smoke: ResMut<SmokeTest>,
    mut commands: Commands,
    mut exit: MessageWriter<bevy::app::AppExit>,
    mut g: ResMut<Patrol>,
    net: Res<Network>,
) {
    smoke.elapsed += time.delta_secs();
    if smoke.interview {
        if smoke.elapsed > 1. && smoke.step == 0 {
            smoke.step = 1;
            g.pending = true;
            g.overlay = true;
            net.start(g.generation, g.ids[g.position].clone());
        }
        if smoke.elapsed > 2.
            && smoke.step == 1
            && !g.pending
            && let Some(case) = &g.case
        {
            net.turn(
                g.generation,
                case.session_id.clone(),
                "What goods are you carrying?".into(),
            );
            g.pending = true;
            smoke.step = 2;
        }
        if smoke.elapsed > 7.
            && smoke.step == 2
            && !g.pending
            && let Some(case) = &g.case
        {
            net.inspect(g.generation, case.session_id.clone());
            g.pending = true;
            smoke.step = 3;
        }
        if smoke.elapsed > 9. && smoke.step == 3 && !g.pending {
            smoke.step = 4;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    "output/evidence.png",
                ));
        }
    }
    if smoke.elapsed > 6. && !smoke.capture {
        smoke.capture = true;
        std::fs::create_dir_all("output").ok();
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                if let Some(view) = std::env::args()
                    .find_map(|a| a.strip_prefix("--scene-view=").map(str::to_owned))
                {
                    format!("output/scene-{view}.png")
                } else if smoke.interview {
                    "output/interview.png".to_owned()
                } else {
                    "output/checkpoint.png".to_owned()
                },
            ));
    }
    if smoke.elapsed > if smoke.interview { 12. } else { 9. } {
        exit.write(bevy::app::AppExit::Success);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detected_speech_sends_a_live_turn_without_a_keypress_or_send_click() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                if let Ok((stream, _)) = listener.accept() {
                    break stream;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "Automatic question never reached the server"
                );
                std::thread::sleep(Duration::from_millis(5));
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = vec![];
            let body = loop {
                let mut bytes = [0; 2048];
                let n = stream.read(&mut bytes).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&bytes[..n]);
                if let Some(split) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..split]);
                    let length: usize = header
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(str::trim)
                                .and_then(|v| v.parse().ok())
                        })
                        .unwrap();
                    if request.len() >= split + 4 + length {
                        break request[split + 4..split + 4 + length].to_vec();
                    }
                }
            };
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["utterance"], "What is the purpose of your visit?");
            assert_eq!(body["session_id"], "voice-session");
            let reply = r#"{"dialogue":"Visiting the wetlands.","emotion":"cooperative","action":"none","fact_ids":["purpose"],"end_conversation":false}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply).unwrap();
        });
        let (tx, rx) = mpsc::channel();
        tx.send((
            0,
            NetEvent::Microphone(border_speech::VoiceEvent::Transcript(
                "What is the purpose of your visit?".into(),
            )),
        ))
        .unwrap();
        let network = Network {
            base: format!("http://{address}"),
            token: "test-token".into(),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };
        let mut patrol = Patrol::new(vec!["01_mara".into()], 42);
        patrol.phase = Phase::Interview;
        patrol.muted = true;
        patrol.case = Some(PublicCase {
            session_id: "voice-session".into(),
            scenario_id: "01_mara".into(),
            name: "Mara".into(),
            occupation: "Photographer".into(),
            opening_line: "Hello".into(),
            evidence: vec![],
            mode: "mock live service".into(),
        });
        let mut app = App::new();
        app.insert_resource(patrol)
            .insert_resource(network)
            .add_systems(Update, receive);
        app.update();
        assert!(app.world().resource::<Patrol>().pending);
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while app.world().resource::<Patrol>().history.is_empty() {
            assert!(
                std::time::Instant::now() < deadline,
                "AI reply was not received"
            );
            std::thread::sleep(Duration::from_millis(5));
            app.update();
        }
        assert!(!app.world().resource::<Patrol>().pending);
        server.join().unwrap();
    }
    #[test]
    fn interaction_requires_car_intersection_and_range() {
        let min = Vec3::splat(-1.);
        let max = Vec3::splat(1.);
        assert_eq!(
            ray_box(Vec3::new(0., 0., 3.), Vec3::NEG_Z, min, max),
            Some(2.)
        );
        assert_eq!(ray_box(Vec3::new(4., 0., 3.), Vec3::NEG_Z, min, max), None);
        assert!(ray_box(Vec3::new(0., 0., 6.), Vec3::NEG_Z, min, max).unwrap() > 3.);
        assert_eq!(ray_box(Vec3::new(0., 0., 3.), Vec3::Z, min, max), None);
    }
}
