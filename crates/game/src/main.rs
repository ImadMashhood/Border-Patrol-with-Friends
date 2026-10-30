mod network;
mod scene;
mod shift;
mod surfaces;
mod traffic;
mod ui;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
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
    vehicle_id: Option<u64>,
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
            vehicle_id: None,
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
    robot_step: usize,
    ready_capture: bool,
    live_capture: bool,
    robot: bool,
}
fn main() {
    if let Some(path) =
        std::env::args().find_map(|a| a.strip_prefix("--export-collision=").map(str::to_owned))
    {
        scene::export_collision(&path);
        return;
    }
    let seed = std::env::args()
        .find_map(|a| a.strip_prefix("--seed=").and_then(|s| s.parse().ok()))
        .unwrap_or(42);
    let ids: Vec<String> = (0..10).map(|i| i.to_string()).collect();
    let preview =
        std::env::args().any(|a| a.starts_with("--scene-view=") || a == "--scene-preview");
    let (shift_state, base, token) = if preview {
        (shift::Shift::preview(), String::new(), String::new())
    } else {
        let url =
            std::env::var("BORDER_HOST_URL").unwrap_or_else(|_| "http://127.0.0.1:8090".into());
        let code = std::env::var("BORDER_HOST_JOIN_CODE")
            .expect("Launch ./play.sh or set BORDER_HOST_JOIN_CODE for an existing host");
        let name = std::env::args()
            .find_map(|a| a.strip_prefix("--name=").map(str::to_owned))
            .unwrap_or_else(|| format!("Officer {}", std::process::id()));
        shift::Shift::connect(url, code, name).expect("Failed to join the shift host")
    };
    let seed = shift_state.world.as_ref().map(|w| w.seed).unwrap_or(seed);
    let (tx, rx) = mpsc::channel();
    let net = Network {
        base,
        token,
        tx,
        rx: Arc::new(Mutex::new(rx)),
    };
    let interview_smoke = std::env::args().any(|a| a == "--smoke-interview");
    let robot = interview_smoke || std::env::args().any(|a| a == "--smoke-shift");
    let smoke = robot || std::env::args().any(|a| a == "--smoke-test");
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.57, 0.72, 0.82)))
        .insert_resource(Patrol::new(ids, seed))
        .insert_resource(net)
        .insert_resource(shift_state)
        .init_resource::<traffic::Traffic>()
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
        .add_systems(
            Update,
            (
                shift::sync,
                controls,
                receive,
                traffic::progress,
                traffic::dust,
                voice,
            )
                .chain(),
        )
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
            robot_step: 0,
            ready_capture: false,
            live_capture: false,
            robot,
        })
        .add_systems(Update, smoke_test.after(voice));
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
    cars: Query<(&Transform, &traffic::Vehicle), Without<Player>>,
    smoke: Option<Res<SmokeTest>>,
    shift: Res<shift::Shift>,
    mut lights: Query<&mut Visibility, With<shift::Flashlight>>,
    mut gui: EguiContexts,
) {
    if let Some(smoke) = smoke {
        if shift.connected && !smoke.robot {
            *shift.controls.lock().unwrap() = shift::Controls::default();
        }
        if shift.connected
            && let Some(world) = &shift.world
            && let Some(me) = shift.me()
            && let Ok((mut t, mut p)) = player.single_mut()
        {
            let target = if world.phase == border_net::ShiftPhase::Lobby {
                Vec3::new(-12., 1.7, -3.5)
            } else {
                world
                    .vehicles
                    .iter()
                    .find(|v| {
                        world
                            .lanes
                            .iter()
                            .any(|l| l.lane_id == v.lane_id && l.owner == Some(me.peer_id))
                            && v.stage == border_net::VehicleStage::Stopped
                    })
                    .map(|v| Vec3::from_array(v.position) + Vec3::Y * 1.25)
                    .unwrap_or(Vec3::new(0., 1.25, 0.))
            };
            t.look_at(target, Vec3::Y);
            let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
            p.yaw = yaw;
            p.pitch = pitch;
        }
        g.looking = false;
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        g.overlay = false;
        g.looking = false;
        g.settings_open = false;
    }
    if !g.overlay {
        if keys.just_pressed(KeyCode::Digit1) {
            shift.send(border_net::ClientCommand::ClaimLane { lane_id: 1 });
        }
        if keys.just_pressed(KeyCode::Digit2) {
            shift.send(border_net::ClientCommand::ClaimLane { lane_id: 2 });
        }
        if keys.just_pressed(KeyCode::KeyG) {
            let lane = if shift.me().is_some_and(|p| p.assisting.is_some()) {
                None
            } else {
                shift
                    .world
                    .as_ref()
                    .and_then(|w| {
                        w.lanes
                            .iter()
                            .find(|l| l.owner.is_some() && l.owner != Some(shift.peer_id))
                    })
                    .map(|l| l.lane_id)
            };
            shift.send(border_net::ClientCommand::AssistLane { lane_id: lane });
        }
        if keys.just_pressed(KeyCode::KeyJ) {
            if shift
                .world
                .as_ref()
                .and_then(|w| w.lane_swap)
                .is_some_and(|p| p[1] == shift.peer_id)
            {
                shift.send(border_net::ClientCommand::AcceptLaneSwap);
            } else {
                shift.send(border_net::ClientCommand::RequestLaneSwap);
            }
        }
        if keys.just_pressed(KeyCode::KeyF) {
            for mut visibility in &mut lights {
                *visibility = if *visibility == Visibility::Hidden {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
    if keys.just_pressed(KeyCode::KeyM) && !g.overlay && !g.settings_open {
        g.map_open = !g.map_open;
    }
    if keys.just_pressed(KeyCode::Tab) {
        g.looking = !g.looking;
    }
    if mouse.just_pressed(MouseButton::Left)
        && !g.overlay
        && !gui.ctx_mut().is_ok_and(|ctx| ctx.is_pointer_over_area())
    {
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
        if shift.connected {
            let holding = shift
                .world
                .as_ref()
                .is_some_and(|w| w.phase == border_net::ShiftPhase::Lobby)
                && keys.pressed(KeyCode::KeyE)
                && border_net::map::door_distance(t.translation.to_array()) < 2.2;
            *shift.controls.lock().unwrap() = shift::Controls {
                movement: [(r * axis.x + f * axis.y).x, (r * axis.x + f * axis.y).z],
                yaw: p.yaw,
                jog: keys.pressed(KeyCode::ShiftLeft),
                ready: holding,
            };
        } else {
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
    } else if shift.connected {
        *shift.controls.lock().unwrap() = shift::Controls {
            yaw: p.yaw,
            ..default()
        };
    }
    let direction = *t.forward();
    let hit = cars
        .iter()
        .filter(|(_, v)| v.ordinal < 5 && shift.permits(v.lane + 1))
        .find(|(car, v)| {
            v.distance == 0.
                && ray_box(
                    t.translation,
                    direction,
                    car.translation + Vec3::new(-1.05, 0.2, -2.25),
                    car.translation + Vec3::new(1.05, 1.85, 2.25),
                )
                .is_some_and(|d| d <= 3.)
        });
    g.aiming = matches!(g.phase, Phase::Approach | Phase::Interview)
        && hit.is_some()
        && shift.world.as_ref().is_some_and(|w| {
            matches!(
                w.phase,
                border_net::ShiftPhase::Active | border_net::ShiftPhase::Grace
            )
        });
    if keys.just_pressed(KeyCode::KeyE) && g.aiming && !g.pending && (!g.overlay || g.looking) {
        g.overlay = true;
        g.looking = false;
        g.request_focus = true;
        if g.case.is_none() {
            g.pending = true;
            g.error.clear();
            let id = hit
                .map(|(_, v)| 1 + v.lane as u64 + v.ordinal as u64 * 2)
                .unwrap();
            g.vehicle_id = Some(id);
            net.start(g.generation, id.to_string());
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
fn receive(mut g: ResMut<Patrol>, net: Res<Network>, shift: Option<Res<shift::Shift>>) {
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
                if !shift.as_ref().is_some_and(|s| s.connected) {
                    g.history.push(Turn {
                        officer: q,
                        traveler: r.dialogue.clone(),
                        emotion: r.emotion.clone(),
                    });
                }
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
    shift: Res<shift::Shift>,
) {
    smoke.elapsed += time.delta_secs();
    if smoke.robot
        && shift.connected
        && let Some(world) = &shift.world
        && let Some(me) = shift.me()
    {
        let mut controls = shift::Controls {
            yaw: std::f32::consts::PI,
            ..default()
        };
        if world.phase == border_net::ShiftPhase::Lobby {
            let mut target = border_net::map::READY_TARGET;
            target[0] += if shift.peer_id % 2 == 1 { -0.55 } else { 0.55 };
            let dx = target[0] - me.position[0];
            let dz = target[2] - me.position[2];
            let length = dx.hypot(dz);
            if length > 0.13 {
                controls.movement = [
                    dx / length * (length / 0.45).min(1.),
                    dz / length * (length / 0.45).min(1.),
                ];
            } else {
                controls.ready = true;
            }
            if me.hold_ms > 1000 && !smoke.ready_capture {
                smoke.ready_capture = true;
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(format!(
                        "output/ready-peer-{}.png",
                        shift.peer_id
                    )));
            }
        } else {
            let lane = world
                .lanes
                .iter()
                .find(|l| l.owner == Some(me.peer_id))
                .map(|l| l.lane_id)
                .unwrap_or(1);
            let x = if lane == 1 { -1.6 } else { 5.4 };
            let route = [[-12., -2.], [-4., -2.], [-4., -8.], [x, -8.], [x, 0.]];
            if smoke.robot_step < route.len() {
                let target = route[smoke.robot_step];
                let dx = target[0] - me.position[0];
                let dz = target[1] - me.position[2];
                let length = dx.hypot(dz);
                if length < 0.18 {
                    smoke.robot_step += 1;
                } else {
                    controls.movement = [
                        dx / length * (length / 0.45).min(1.),
                        dz / length * (length / 0.45).min(1.),
                    ];
                }
            }
            if smoke.elapsed > 12. && !smoke.live_capture {
                smoke.live_capture = true;
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(format!(
                        "output/shift-peer-{}.png",
                        shift.peer_id
                    )));
            }
        }
        *shift.controls.lock().unwrap() = controls;
    }

    if smoke.interview {
        if smoke.step == 0
            && smoke.robot_step >= 5
            && let Some(world) = &shift.world
            && let Some(vehicle) = world
                .vehicles
                .iter()
                .find(|v| v.stage == border_net::VehicleStage::Stopped && shift.permits(v.lane_id))
        {
            smoke.step = 1;
            g.pending = true;
            g.overlay = true;
            g.vehicle_id = Some(vehicle.vehicle_id);
            net.start(g.generation, vehicle.vehicle_id.to_string());
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
        if smoke.elapsed > 17.
            && smoke.step == 2
            && !g.pending
            && let Some(case) = &g.case
        {
            net.inspect(g.generation, case.session_id.clone());
            g.pending = true;
            smoke.step = 3;
        }
        if smoke.elapsed > 21. && smoke.step == 3 && !g.pending {
            smoke.step = 4;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    "output/evidence.png",
                ));
        }
    }
    if smoke.elapsed > if smoke.interview { 16. } else { 6. } && !smoke.capture {
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
    if smoke.elapsed
        > if smoke.interview {
            35.
        } else if smoke.robot {
            20.
        } else {
            9.
        }
    {
        if let Some(world) = &shift.world {
            std::fs::create_dir_all("output").ok();
            std::fs::write(
                format!("output/shift-peer-{}.json", shift.peer_id),
                serde_json::to_string_pretty(world).unwrap(),
            )
            .ok();
        }
        if smoke.robot && !smoke.live_capture {
            error!("Ready door smoke did not complete");
            exit.write(bevy::app::AppExit::error());
            return;
        }
        if smoke.interview && (smoke.step < 4 || !g.secondary || !g.error.is_empty()) {
            error!(
                "Interview smoke failed: step={}, secondary={}, error={}",
                smoke.step, g.secondary, g.error
            );
            exit.write(bevy::app::AppExit::error());
        } else {
            exit.write(bevy::app::AppExit::Success);
        }
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
