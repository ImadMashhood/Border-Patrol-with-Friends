mod animation;
mod equipment;
mod hud;
mod interactions;
mod network;
mod porter;
mod prediction;
mod scene;
mod shift;
mod surfaces;
mod traffic;
mod ui;
mod vehicle_physics;
mod vehicles;
mod weapon;
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
    conversation_confirmed: bool,
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
    playback: Option<border_speech::SpeechPlayback>,
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
            conversation_confirmed: false,
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
            playback: None,
            speech_pending: false,
            request_focus: false,
            settings_open: false,
            map_open: std::env::args().any(|a| a == "--show-map"),
            vehicle_id: None,
        }
    }
    fn reset_case(&mut self) {
        self.case = None;
        self.conversation_confirmed = false;
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
        self.playback = None;
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
    interface: bool,
    repairs: bool,
    repair_stage: u8,
    aim_target: Option<Vec3>,
    ui_step: u8,
    casual: bool,
    saw_outline: bool,
    walked_in_conversation: bool,
    grip_repeated: bool,
    porter_seen: bool,
    elapsed: f32,
    capture: bool,
    interview: bool,
    lifecycle: bool,
    combat: bool,
    physics: bool,
    ads_capture: bool,
    max_shards: usize,
    fleet_capture: bool,
    last_combat_fire: f32,
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
        retry_until: Arc::new(Mutex::new(std::time::Instant::now())),
        spatial: Default::default(),
        base,
        token,
        tx,
        rx: Arc::new(Mutex::new(rx)),
    };
    let physics_smoke = std::env::args().any(|a| a == "--smoke-physics");
    let combat_smoke = physics_smoke || std::env::args().any(|a| a == "--smoke-combat");
    let repairs = std::env::args().any(|a| a == "--smoke-repairs");
    let interface = std::env::args().any(|a| a == "--smoke-ui");
    let casual = repairs || interface || std::env::args().any(|a| a == "--smoke-casual");
    let lifecycle = casual || std::env::args().any(|a| a == "--smoke-lifecycle");
    let interview_smoke = lifecycle || std::env::args().any(|a| a == "--smoke-interview");
    let robot = combat_smoke || interview_smoke || std::env::args().any(|a| a == "--smoke-shift");
    let smoke = robot || std::env::args().any(|a| a == "--smoke-test");
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.57, 0.72, 0.82)))
        .insert_resource(Patrol::new(ids, seed))
        .insert_resource(net)
        .insert_resource(shift_state)
        .init_resource::<traffic::Traffic>()
        .init_resource::<weapon::WeaponUi>()
        .init_resource::<equipment::Equipment>()
        .init_resource::<animation::Actors>()
        .init_resource::<hud::Hud>()
        .init_resource::<interactions::Focus>()
        .init_resource::<vehicle_physics::Effects>()
        .init_resource::<vehicle_physics::ShardAssets>()
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
                vehicles::animate,
                equipment::input,
                controls,
                interactions::select,
                interactions::input,
                weapon::controls,
                receive,
                traffic::progress,
                traffic::dust,
                voice,
                hud::update,
            )
                .chain(),
        )
        .add_systems(Update, scene::atmosphere)
        .add_systems(
            PostUpdate,
            network::spatial_voice.after(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            Update,
            (
                vehicle_physics::sync,
                vehicle_physics::effects,
                vehicle_physics::physical_shards,
                weapon::render,
                porter::render,
                equipment::render,
                animation::officers,
                animation::faces,
                interactions::outlines,
            )
                .chain()
                .after(voice),
        )
        .add_systems(EguiPrimaryContextPass, (ui::landmarks, ui::draw).chain());
    if smoke {
        // Rendering smoke checks never record the room or play spoken audio.
        let mut patrol = app.world_mut().resource_mut::<Patrol>();
        patrol.mic_enabled = false;
        patrol.muted = true;
        if casual {
            patrol.looking = true;
        }
        app.insert_resource(SmokeTest {
            interface,
            repairs,
            repair_stage: 0,
            aim_target: None,
            ui_step: 0,
            casual,
            saw_outline: false,
            walked_in_conversation: false,
            grip_repeated: false,
            porter_seen: false,
            elapsed: 0.,
            capture: false,
            interview: interview_smoke,
            lifecycle,
            combat: combat_smoke,
            physics: physics_smoke,
            ads_capture: false,
            max_shards: 0,
            fleet_capture: false,
            last_combat_fire: 0.,
            step: 0,
            robot_step: 0,
            ready_capture: false,
            live_capture: false,
            robot,
        })
        .add_systems(
            Update,
            smoke_test.after(voice).before(vehicle_physics::sync),
        );
    }
    if !app.run().is_success() {
        std::process::exit(1);
    }
}
#[allow(clippy::too_many_arguments)] // Bevy system inputs are independently borrowed resources.
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    mut g: ResMut<Patrol>,
    equipment: Res<equipment::Equipment>,
    mut player: Query<(&mut Transform, &mut Player)>,
    solids: Query<(&Transform, &Solid), Without<Player>>,
    mut cursor: Single<&mut CursorOptions>,
    cars: Query<(&Transform, &traffic::Vehicle), Without<Player>>,
    smoke: Option<Res<SmokeTest>>,
    mut shift: ResMut<shift::Shift>,
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
                            && matches!(
                                v.stage,
                                border_net::VehicleStage::Stopped
                                    | border_net::VehicleStage::Disabled
                            )
                    })
                    .map(|v| {
                        if smoke.physics
                            && smoke.step >= 16
                            && let Some(worker) = world.porters.iter().find(|p| p.lane_id == 1)
                        {
                            return Vec3::from_array(worker.position) + Vec3::Y * 1.2;
                        }
                        if smoke.combat {
                            if smoke.physics
                                && smoke.step >= 11
                                && let Some(chest) =
                                    v.physics.npc.ragdoll.iter().find(|p| p.bone == 1)
                            {
                                return Vec3::from_array(chest.position);
                            }
                            if shift.peer_id == 1
                                && smoke.step >= 7
                                && let Some(frame) = border_net::combat::pane_frame(
                                    v,
                                    border_net::combat::Pane::Windshield,
                                )
                            {
                                let target = if smoke.step >= 9 {
                                    border_net::combat::add(
                                        frame.center,
                                        border_net::combat::mul(frame.u, -0.5),
                                    )
                                } else {
                                    frame.center
                                };
                                return Vec3::from_array(target);
                            }
                            return Vec3::from_array(if smoke.step <= 2 {
                                border_net::combat::pane_frame(
                                    v,
                                    border_net::combat::Pane::DriverFront,
                                )
                                .map(|f| f.center)
                                .unwrap_or_else(|| {
                                    border_net::vehicle::spec(&v.kind).aim_point(
                                        v.position,
                                        v.yaw,
                                        border_net::vehicle::VehiclePoint::Driver,
                                    )
                                })
                            } else {
                                border_net::combat::add(
                                    v.physics.npc.position,
                                    [
                                        0.,
                                        if v.physics.npc.mode == border_net::combat::NpcMode::Seated
                                        {
                                            1.2
                                        } else {
                                            1.62
                                        },
                                        0.,
                                    ],
                                )
                            });
                        }
                        if smoke.casual && smoke.step < 4 {
                            return Vec3::from_array(v.physics.npc.position) + Vec3::Y * 1.55;
                        }
                        Vec3::from_array(border_net::vehicle::spec(&v.kind).aim_point(
                            v.position,
                            v.yaw,
                            if smoke.lifecycle && smoke.step >= 4 {
                                border_net::vehicle::VehiclePoint::Cargo
                            } else {
                                border_net::vehicle::VehiclePoint::Driver
                            },
                        ))
                    })
                    .unwrap_or(Vec3::new(0., 1.25, 0.))
            };
            t.look_at(smoke.aim_target.unwrap_or(target), Vec3::Y);
            let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
            p.yaw = yaw;
            p.pitch = pitch;
        }
        g.looking = smoke.casual;
        cursor.visible = !smoke.casual;
        cursor.grab_mode = if smoke.casual {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let resume = g.overlay;
        g.overlay = false;
        shift.inspection = None;
        g.looking = resume;
        g.settings_open = false;
    }
    if !g.overlay && !equipment.wheel {
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
    if (mouse.just_pressed(MouseButton::Left)
        || mouse.just_pressed(MouseButton::Right) && shift.me().is_some_and(|p| p.weapon.drawn))
        && !g.overlay
        && !equipment.wheel
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
        let sensitivity =
            if mouse.pressed(MouseButton::Right) && shift.me().is_some_and(|p| p.weapon.drawn) {
                0.0014
            } else {
                0.0025
            };
        p.yaw -= motion.delta.x * sensitivity;
        p.pitch = (p.pitch - motion.delta.y * sensitivity).clamp(-1.4, 1.4);
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
                grab_held: keys.pressed(KeyCode::KeyE),
                pitch: p.pitch,
            };
            if time.elapsed_secs() - shift.prediction.received_at < 0.25
                && shift.me().is_some_and(|p| p.health > 0)
                && let Some(world) = &shift.world
            {
                let moved = border_net::movement::step(
                    t.translation.to_array(),
                    [(r * axis.x + f * axis.y).x, (r * axis.x + f * axis.y).z],
                    keys.pressed(KeyCode::ShiftLeft),
                    time.delta_secs().min(0.05),
                    &shift.collision,
                    &world.vehicles,
                    &world.lanes,
                    world.door_open,
                );
                shift.prediction.position = Vec3::from_array(moved);
                t.translation = shift.prediction.position;
            }
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
    if shift.connected {
        let hit = shift.world.as_ref().and_then(|world| {
            vehicles::interaction(world, shift.peer_id, t.translation, direction)
        });
        shift.interaction = hit;
        g.aiming = hit.is_some();
    } else {
        // Scene previews never open provider sessions.
        let _ = cars;
        g.aiming = false;
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
    if g.muted {
        g.playback = None;
    }
    let events: Vec<_> = net.rx.lock().unwrap().try_iter().collect();
    for (generation, event) in events {
        if generation != g.generation {
            continue;
        }
        match event {
            NetEvent::Started(case) => {
                g.subtitle = case.opening_line.clone();
                let resumed = shift
                    .as_ref()
                    .and_then(|s| s.world.as_ref())
                    .is_some_and(|w| {
                        w.vehicles
                            .iter()
                            .any(|v| Some(v.vehicle_id) == g.vehicle_id && !v.history.is_empty())
                    });
                if !g.muted && !resumed {
                    g.speaking = true;
                    g.playback = Some(net.speak(
                        g.generation,
                        case.opening_line.clone(),
                        case.scenario_id.clone(),
                    ));
                }
                g.evidence = case.evidence.clone();
                g.case = Some(case);
                g.phase = Phase::Interview;
                g.pending = false;
                g.request_focus = false;
                g.overlay = false;
                g.looking = true;
            }
            NetEvent::Reply(q, r) => {
                g.error.clear();
                g.subtitle = r.dialogue.clone();
                g.emotion = format!("{:?}", r.emotion).to_lowercase();
                if !shift.as_ref().is_some_and(|s| s.connected) {
                    g.history.push(Turn {
                        officer: q,
                        traveler: r.dialogue.clone(),
                        emotion: r.emotion.clone(),
                        physical: Default::default(),
                    });
                }
                g.pending = false;
                g.request_focus = true;
                if !g.muted {
                    g.speaking = true;
                    g.playback = Some(net.speak_emotional(
                        g.generation,
                        r.dialogue,
                        g.case.as_ref().unwrap().scenario_id.clone(),
                        g.emotion.clone(),
                    ));
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
                g.playback = None;
                g.mic = None;
                g.pending = false;
                g.overlay = true;
                g.looking = false;
                g.subtitle.clear();
            }
            NetEvent::TurnFailed { message, question } => {
                border_caller::call_log::record(
                    "turn_failed",
                    serde_json::json!({"generation":generation,"officer":question,"message":message}),
                );
                g.error = message;
                g.input = question;
                g.pending = false;
                g.speech_pending = false;
                if let Some(scenario) = g.case.as_ref().map(|c| c.scenario_id.clone()) {
                    if !g.muted {
                        g.speaking = true;
                        g.playback = Some(net.speak(g.generation, "Excuse me?".into(), scenario));
                    }
                    g.subtitle = "Excuse me?".into();
                }
            }
            NetEvent::Error(e) => {
                if !g.overlay && g.vehicle_id.is_some() && g.case.is_none() {
                    let available = shift
                        .as_ref()
                        .and_then(|s| s.world.as_ref().map(|w| (s.peer_id, w)))
                        .is_none_or(|(peer, w)| {
                            w.vehicles.iter().any(|v| {
                                Some(v.vehicle_id) == g.vehicle_id
                                    && v.physics.npc.health > 0
                                    && v.conversation_owner.is_none_or(|owner| owner == peer)
                            })
                        });
                    if available && !g.muted {
                        g.speaking = true;
                        g.playback =
                            Some(net.speak(g.generation, "Excuse me?".into(), String::new()));
                    }
                } else {
                    g.error = e;
                }
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
                    border_caller::call_log::record(
                        "transcript",
                        serde_json::json!({"generation":generation,"text":q,"accepted":g.phase==Phase::Interview && !g.pending && !g.speaking && g.mic_enabled && net.ready_for_turn()}),
                    );
                    g.speech_pending = false;
                    if g.phase == Phase::Interview
                        && !g.pending
                        && !g.speaking
                        && g.mic_enabled
                        && net.ready_for_turn()
                        && let Some(case) = &g.case
                    {
                        net.turn(g.generation, case.session_id.clone(), q.clone());
                        g.subtitle = format!("You: {q}");
                        g.pending = true;
                        g.error.clear();
                    }
                }
                border_speech::VoiceEvent::Error(e) => {
                    if g.phase == Phase::Interview && !g.muted && !g.speaking {
                        g.speaking = true;
                        g.playback =
                            Some(net.speak(g.generation, "Excuse me?".into(), String::new()));
                    }
                    border_caller::call_log::record(
                        "microphone_error",
                        serde_json::json!({"generation":generation,"message":e}),
                    );
                    g.mic_error = e;
                    g.mic_failed = true;
                    g.mic = None;
                    g.mic_active = false;
                    g.speech_pending = false;
                }
            },
            NetEvent::SpeechFinished { error, fallback } => {
                g.speaking = false;
                if let Some(e) = error {
                    g.error = e;
                    if !fallback && !g.muted && g.phase == Phase::Interview {
                        g.speaking = true;
                        g.playback =
                            Some(net.speak(g.generation, "Excuse me?".into(), String::new()));
                    }
                }
            }
        }
    }
}
fn voice(mut g: ResMut<Patrol>, net: Res<Network>, shift: Res<shift::Shift>) {
    let in_range = !shift.connected
        || shift.world.as_ref().is_some_and(|w| {
            w.vehicles.iter().any(|v| {
                Some(v.vehicle_id) == g.vehicle_id
                    && v.stage == border_net::VehicleStage::Stopped
                    && v.conversation_owner == Some(shift.peer_id)
                    && shift.me().is_some_and(|p| {
                        let window = border_net::vehicle::spec(&v.kind).world_point(
                            v.position,
                            v.yaw,
                            border_net::vehicle::VehiclePoint::Driver,
                        );
                        let target = if matches!(
                            v.physics.npc.mode,
                            border_net::combat::NpcMode::Outside
                                | border_net::combat::NpcMode::Fleeing
                        ) {
                            v.physics.npc.position
                        } else {
                            window
                        };
                        (p.position[0] - target[0]).hypot(p.position[2] - target[2]) <= 8.0
                    })
            })
        });
    if shift.connected && shift.me().is_none_or(|p| p.health == 0)
        || !in_range
        || g.phase != Phase::Interview
        || g.case.is_none()
        || !g.mic_enabled
    {
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
        mic.set_paused(g.pending || g.speaking || g.speech_pending || !net.ready_for_turn());
    }
}

#[allow(clippy::too_many_arguments)] // The rendered walk checks camera targeting as well as host state.
fn smoke_test(
    time: Res<Time>,
    mut smoke: ResMut<SmokeTest>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut camera: Query<&mut Transform, With<Player>>,
    mut commands: Commands,
    mut exit: MessageWriter<bevy::app::AppExit>,
    mut g: ResMut<Patrol>,
    net: Res<Network>,
    mut shift: ResMut<shift::Shift>,
    focus: Res<interactions::Focus>,
    mut equipment: ResMut<equipment::Equipment>,
    hud: Res<hud::Hud>,
    panels: Query<(&vehicles::Panel, &GlobalTransform), Without<Player>>,
) {
    smoke.saw_outline |= focus.target.is_some();
    if smoke.casual && g.case.is_some() && !g.overlay && smoke.robot_step >= 6 {
        smoke.walked_in_conversation = true;
    }
    smoke.elapsed += time.delta_secs();
    if let Some(world) = &shift.world {
        smoke.max_shards = smoke.max_shards.max(world.shards.len());
    }
    if smoke.physics {
        if smoke.elapsed > 18. && smoke.elapsed < 19.5 {
            mouse.press(MouseButton::Right);
            g.looking = true;
        } else {
            mouse.release(MouseButton::Right);
        }
        if smoke.elapsed > 19. && !smoke.ads_capture {
            smoke.ads_capture = true;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(format!(
                    "output/physics-aim-peer-{}.png",
                    shift.peer_id
                )));
        }
    }

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
            let cargo_z = world
                .vehicles
                .iter()
                .find(|v| v.lane_id == lane && v.stage == border_net::VehicleStage::Stopped)
                .map(|v| {
                    border_net::vehicle::spec(&v.kind)
                        .local_point(border_net::vehicle::VehiclePoint::Cargo)[2]
                })
                .unwrap_or(4.);
            let mut route = vec![
                [-12., -2.],
                [-4., -2.],
                [-4., -4.2],
                [x, -4.2],
                [x, -1.2],
                [x, cargo_z],
                [(lane - 1) as f32 * 7., cargo_z],
            ];
            if smoke.interface {
                route.extend([[x, cargo_z], [x, -1.2]]);
            }
            if smoke.combat {
                route.truncate(5);
                route.extend([
                    [-3.3, -1.2],
                    [-3.3, -4.2],
                    [0., -4.2],
                    [-3.3, -4.2],
                    [-3.3, -0.25],
                    [-1.6, -0.25],
                ]);
                if smoke.physics {
                    route.extend([[-3.2, -0.25], [-4.5, 1.0]]);
                }
            }
            if smoke.physics && shift.peer_id == 1 && matches!(smoke.step, 12 | 15) {
                controls.grab_held = true;
                controls.yaw = camera.single().unwrap().rotation.to_euler(EulerRot::YXZ).0;
                controls.pitch = camera.single().unwrap().rotation.to_euler(EulerRot::YXZ).1;
            }
            let allowed = if smoke.interface && smoke.elapsed > 36. {
                9
            } else if smoke.physics && shift.peer_id == 1 && smoke.step >= 12 {
                13
            } else if smoke.physics && shift.peer_id == 1 && smoke.step >= 11 {
                11
            } else if smoke.combat && shift.peer_id == 1 && smoke.step >= 7 {
                8
            } else if smoke.lifecycle && smoke.step >= 4 {
                7
            } else {
                5
            };
            if smoke.robot_step < allowed {
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
            if smoke.robot_step >= 5 && !smoke.live_capture {
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

    if smoke.combat
        && smoke.robot_step >= 5
        && let Some(world) = &shift.world
        && let Some(v) = world.vehicles.iter().find(|v| {
            world
                .lanes
                .iter()
                .any(|l| l.lane_id == v.lane_id && l.owner == Some(shift.peer_id))
        })
    {
        use border_net::{
            ClientCommand as Cmd,
            combat::{NpcMode, Pane},
        };
        let id = v.vehicle_id;
        let shoot = || {
            let t = camera.single().unwrap();
            shift.send(Cmd::Fire {
                direction: t.forward().to_array(),
            });
        };
        let capture = |commands: &mut Commands, name: &str| {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(format!(
                    "output/combat-{name}-peer-{}.png",
                    shift.peer_id
                )));
        };
        match smoke.step {
            0 if v.stage == border_net::VehicleStage::Stopped
                && smoke.elapsed > 15.
                && v.physics.glass[Pane::DriverFront.index()].target_down =>
            {
                shift.send(Cmd::SetWeaponDrawn { drawn: true });
                shift.send(Cmd::ToggleWindow {
                    vehicle_id: id,
                    pane: Pane::DriverFront,
                });
                smoke.step = 1;
            }
            1 if smoke.elapsed > 16.
                && v.physics.glass[Pane::DriverFront.index()].lowered < 0.03
                && shift.me().is_some_and(|p| p.weapon.drawn) =>
            {
                shoot();
                smoke.step = 2;
            }
            2 if v.physics.glass[Pane::DriverFront.index()].shattered => {
                capture(&mut commands, "glass");
                smoke.step = 3;
            }
            3 if smoke.elapsed > 20. => {
                if shift.peer_id == 1 {
                    shoot();
                } else {
                    shift.send(Cmd::OrderExit { vehicle_id: id });
                }
                smoke.step = 4;
            }
            4 if shift.peer_id == 1
                && v.physics.npc.health > 0
                && smoke.elapsed > 21.
                && smoke.elapsed - smoke.last_combat_fire > 0.4
                && shift.me().is_some_and(|p| {
                    p.weapon
                        .last_fired_ms
                        .is_none_or(|t| world.server_time_ms.saturating_sub(t) > 350)
                }) =>
            {
                shoot();
                smoke.last_combat_fire = smoke.elapsed;
            }
            4 if shift.peer_id == 1 && v.physics.npc.health == 0 => {
                capture(&mut commands, "seated-death");
                smoke.step = 7;
            }
            4 if shift.peer_id != 1 && v.physics.npc.mode == NpcMode::Outside => {
                capture(&mut commands, "outside");
                smoke.step = 5;
            }
            5 if smoke.elapsed > 26. => {
                shoot();
                smoke.step = 6;
            }
            6 if v.physics.npc.health == 0 => {
                capture(&mut commands, "outside-death");
                smoke.step = 7;
            }
            7 if shift.peer_id == 1 && smoke.robot_step >= 8 => {
                shoot();
                smoke.step = 8;
            }
            8 if shift.peer_id == 1
                && !v.physics.glass[Pane::Windshield.index()].impacts.is_empty() =>
            {
                capture(&mut commands, "windshield-cracks");
                smoke.step = 9;
            }
            9 if shift.peer_id == 1 && smoke.elapsed > 31. => {
                shoot();
                smoke.step = 10;
            }
            10 if shift.peer_id == 1 && v.physics.glass[Pane::Windshield.index()].shattered => {
                capture(&mut commands, "windshield-shattered");
                smoke.step = 11;
            }
            11 if smoke.physics && shift.peer_id == 1 && smoke.robot_step >= 11 => {
                shift.send(Cmd::GrabBody {
                    vehicle_id: id,
                    bone: 1,
                });
                smoke.step = 12;
            }
            12 if smoke.physics && v.physics.npc.extracted && smoke.elapsed > 42. => {
                capture(&mut commands, "extracted-ragdoll");
                shift.send(Cmd::ReleaseBody);
                smoke.step = 13;
            }
            13 if smoke.physics && smoke.elapsed > 43. && v.physics.npc.body_holder.is_none() => {
                shift.send(Cmd::GrabBody {
                    vehicle_id: id,
                    bone: 1,
                });
                smoke.step = 15;
            }
            15 if smoke.physics
                && smoke.elapsed > 45.
                && v.physics.npc.body_holder == Some(shift.peer_id) =>
            {
                smoke.grip_repeated = true;
                shift.send(Cmd::ReleaseBody);
                shift.send(Cmd::RequestPorter { lane_id: v.lane_id });
                smoke.step = 16;
            }
            16 if smoke.physics
                && smoke.elapsed > 46.
                && world.porters.iter().any(|p| {
                    p.lane_id == v.lane_id && p.phase != border_net::PorterPhase::Seated
                }) =>
            {
                smoke.porter_seen = true;
                capture(&mut commands, "porter-running");
                smoke.step = 17;
            }
            _ => {}
        }
    }
    if smoke.lifecycle
        && smoke.step == 4
        && smoke.robot_step >= 7
        && let Some(v) = shift.world.as_ref().and_then(|w| {
            w.vehicles
                .iter()
                .find(|v| v.stage == border_net::VehicleStage::Stopped && shift.permits(v.lane_id))
        })
    {
        let id = v.vehicle_id;
        let camera = camera.single().unwrap();
        if vehicles::interaction(
            shift.world.as_ref().unwrap(),
            shift.peer_id,
            camera.translation,
            *camera.forward(),
        ) != Some((id, border_net::vehicle::VehiclePoint::Cargo))
        {
            return;
        }
        shift.send(border_net::ClientCommand::InspectVehiclePoint {
            vehicle_id: id,
            point: border_net::vehicle::VehiclePoint::Cargo,
        });
        shift.send(border_net::ClientCommand::SearchCompartment {
            vehicle_id: id,
            location: border_scenarios::EvidenceLocation::Container,
        });
        shift.inspection = Some((id, border_net::vehicle::VehiclePoint::Cargo));
        g.overlay = false;
        smoke.step = 5;
    }
    if smoke.lifecycle
        && smoke.step == 5
        && smoke.elapsed > 31.
        && shift.world.as_ref().is_some_and(|w| {
            w.vehicles.iter().any(|v| {
                shift.permits(v.lane_id)
                    && v.inspected_points
                        .contains(&border_net::vehicle::VehiclePoint::Cargo)
                    && v.searched_compartments
                        .contains(&border_scenarios::EvidenceLocation::Container)
            })
        })
    {
        smoke.step = 6;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(format!(
                "output/cargo-peer-{}.png",
                shift.peer_id
            )));
    }
    if smoke.casual && smoke.step == 0 && g.case.is_some() && !g.pending {
        assert!(
            g.looking && !g.overlay,
            "casual dialogue must not capture movement or open UI"
        );
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(format!(
                "output/casual-peer-{}.png",
                shift.peer_id
            )));
        smoke.step = 1;
    }
    if smoke.interview {
        if !smoke.casual
            && smoke.step == 0
            && smoke.robot_step >= 5
            && let Some(world) = &shift.world
            && let Some(vehicle) = world
                .vehicles
                .iter()
                .find(|v| v.stage == border_net::VehicleStage::Stopped && shift.permits(v.lane_id))
        {
            let camera = camera.single().unwrap();
            if vehicles::interaction(world, shift.peer_id, camera.translation, *camera.forward())
                != Some((
                    vehicle.vehicle_id,
                    border_net::vehicle::VehiclePoint::Driver,
                ))
            {
                return;
            }
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
            if let Some(id) = g.vehicle_id {
                shift.send(border_net::ClientCommand::RequestSecondary { vehicle_id: id });
                g.evidence_open = true;
            } else {
                net.inspect(g.generation, case.session_id.clone());
                g.pending = true;
            }
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
    if smoke.physics
        && smoke.elapsed > 48.5
        && let Ok(mut t) = camera.single_mut()
    {
        t.translation = Vec3::new(18., 7., 25.);
        t.look_at(Vec3::new(3., 1., 16.), Vec3::Y);
        if !smoke.fleet_capture {
            smoke.fleet_capture = true;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    "output/physics-fleet.png",
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
    if smoke.repairs
        && smoke.step >= 6
        && let Some(w) = &shift.world
        && let Some(v) = w.vehicles.iter().find(|v| v.vehicle_id == shift.peer_id)
    {
        use border_net::vehicle::VehiclePoint;
        use border_scenarios::EvidenceLocation;
        use interactions::Action;
        let container = smoke.repair_stage < 4;
        if let Some((p, t)) = panels.iter().find(|(p, _)| {
            1 + p.lane as u64 + p.ordinal as u64 * 2 == v.vehicle_id
                && p.point == VehiclePoint::Cargo
                && p.compartment
                    == if container {
                        Some(EvidenceLocation::Container)
                    } else {
                        None
                    }
        }) {
            smoke.aim_target = Some(t.transform_point(p.offset));
        }
        let desired = if container {
            Action::Compartment(EvidenceLocation::Container)
        } else {
            Action::Panel(VehiclePoint::Cargo)
        };
        let focused = focus
            .target
            .is_some_and(|t| t.vehicle == v.vehicle_id && t.action == desired);
        let opened = if container {
            v.open_compartments.contains(&EvidenceLocation::Container)
        } else {
            v.open_panels.contains(&VehiclePoint::Cargo)
        };
        match smoke.repair_stage {
            0 => smoke.repair_stage = 1,
            1 | 4 if focused && opened => {
                interactions::activate(focus.target.unwrap(), &mut g, &shift, &net);
                smoke.repair_stage += 1;
            }
            2 | 5 if focused && !opened => {
                let name = if container {
                    "container-closed"
                } else {
                    "cargo-closed"
                };
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(format!(
                        "output/repairs-{name}-peer-{}.png",
                        shift.peer_id
                    )));
                interactions::activate(focus.target.unwrap(), &mut g, &shift, &net);
                smoke.repair_stage += 1;
            }
            3 if opened => smoke.repair_stage = 4,
            6 if opened => {
                assert!(
                    v.searched_compartments
                        .contains(&EvidenceLocation::Container)
                );
                assert!(
                    v.observed_evidence
                        .iter()
                        .any(|f| f.location == EvidenceLocation::Cargo)
                );
                assert!(hud.drew_finding);
                smoke.repair_stage = 7;
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(format!(
                        "output/repairs-cargo-reopened-peer-{}.png",
                        shift.peer_id
                    )));
            }
            _ => {}
        }
    }
    if smoke.interface {
        let now = smoke.elapsed;
        let shot = match smoke.ui_step {
            0 if !g.history.is_empty() && hud.visible_captions(time.elapsed_secs()) => {
                smoke.ui_step = 1;
                Some("hud")
            }
            1 if now > 32. => {
                equipment.wheel = true;
                equipment.hover = equipment::Item::Tablet;
                g.looking = false;
                smoke.ui_step = 2;
                None
            }
            2 if now > 32.7 => {
                smoke.ui_step = 3;
                Some("wheel")
            }
            3 if now > 33. => {
                equipment.equip(equipment::Item::Tablet, &mut g, &shift);
                equipment.page = 0;
                smoke.ui_step = 4;
                None
            }
            4 if now > 34. => {
                smoke.ui_step = 5;
                Some("tablet")
            }
            5 if now > 35. => {
                equipment.page = 1;
                smoke.ui_step = 6;
                None
            }
            6 if now > 35.7 => {
                smoke.ui_step = 7;
                Some("transcript")
            }
            7 if now > 40. && smoke.robot_step >= 9 => {
                let id = shift.peer_id;
                let disposition = if id == 1 {
                    border_net::Disposition::Admit
                } else {
                    border_net::Disposition::Deny
                };
                assert!(
                    hud::submit_decision(
                        &mut equipment,
                        &shift,
                        id,
                        disposition,
                        time.elapsed_secs()
                    ),
                    "tablet decision controller rejected nearby assigned driver"
                );
                smoke.ui_step = 8;
                None
            }
            8 if now > 41.
                && shift.world.as_ref().is_some_and(|w| {
                    w.vehicles
                        .iter()
                        .any(|v| v.vehicle_id == shift.peer_id && v.outcome.is_some())
                }) =>
            {
                smoke.ui_step = 9;
                Some("decision")
            }
            _ => None,
        };
        if let Some(name) = shot {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(format!(
                    "output/ui-{name}-peer-{}.png",
                    shift.peer_id
                )));
        }
    }
    if smoke.elapsed
        > if smoke.interface || smoke.repairs {
            43.
        } else if smoke.physics {
            50.
        } else if smoke.interview || smoke.combat {
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
        if smoke.repairs {
            assert_eq!(
                smoke.repair_stage, 7,
                "cargo interaction repair smoke failed; focus={:?}",
                focus.target
            );
            std::fs::write(
                format!("output/repairs-metrics-peer-{}.json", shift.peer_id),
                serde_json::json!({"stage":smoke.repair_stage,"finding":hud.drew_finding})
                    .to_string(),
            )
            .unwrap();
        }
        if smoke.interface {
            assert!(
                smoke.ui_step == 9
                    && hud.drew_hud
                    && hud.drew_hint
                    && hud.drew_captions
                    && hud.drew_wheel
                    && hud.drew_tablet
            );
            std::fs::write(format!("output/ui-metrics-peer-{}.json",shift.peer_id),serde_json::json!({"hud":hud.drew_hud,"hint":hud.drew_hint,"captions":hud.drew_captions,"wheel":hud.drew_wheel,"tablet":hud.drew_tablet,"decision_confirmed":smoke.ui_step==9,"voice_source":net.spatial.lock().unwrap().source}).to_string()).unwrap();
            g.overlay = false;
            g.looking = true;
        }
        if smoke.casual {
            std::fs::write(format!("output/casual-metrics-peer-{}.json",shift.peer_id),serde_json::json!({"outline":smoke.saw_outline,"walked_in_conversation":smoke.walked_in_conversation,"overlay":g.overlay,"looking":g.looking}).to_string()).unwrap();
        }
        if smoke.physics {
            std::fs::write(format!("output/physics-metrics-peer-{}.json",shift.peer_id),serde_json::to_string_pretty(&serde_json::json!({"max_shards":smoke.max_shards,"ads_capture":smoke.ads_capture,"grip_repeated":smoke.grip_repeated,"porter_seen":smoke.porter_seen})).unwrap()).unwrap();
        }
        if smoke.robot && !smoke.live_capture {
            error!("Ready door smoke did not complete");
            exit.write(bevy::app::AppExit::error());
            return;
        }
        if smoke.combat
            && smoke.step
                < if shift.peer_id == 1 {
                    if smoke.physics { 13 } else { 11 }
                } else {
                    7
                }
        {
            error!(
                "Combat smoke incomplete at step {}: {}",
                smoke.step, shift.error
            );
            exit.write(bevy::app::AppExit::error());
            return;
        }
        if smoke.interview
            && (smoke.step < if smoke.lifecycle { 6 } else { 4 }
                || (smoke.lifecycle && !g.secondary)
                || !g.error.is_empty())
        {
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
            retry_until: Arc::new(Mutex::new(std::time::Instant::now())),
            spatial: Default::default(),
            base: format!("http://{address}"),
            token: "test-token".into(),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };
        let mut patrol = Patrol::new(vec!["01_mara".into()], 42);
        patrol.phase = Phase::Interview;
        patrol.muted = true;
        patrol.case = Some(PublicCase {
            declared_claims: vec![],
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
    #[test]
    fn failed_turn_keeps_the_question_and_does_not_invent_dialogue() {
        let (tx, rx) = mpsc::channel();
        tx.send((
            0,
            NetEvent::TurnFailed {
                message: "Provider rate limit; retry".into(),
                question: "What is your name?".into(),
            },
        ))
        .unwrap();
        let network = Network {
            retry_until: Arc::new(Mutex::new(std::time::Instant::now())),
            spatial: Default::default(),
            base: String::new(),
            token: String::new(),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };
        let mut patrol = Patrol::new(vec![], 42);
        patrol.pending = true;
        let mut app = App::new();
        app.insert_resource(patrol)
            .insert_resource(network)
            .add_systems(Update, receive);
        app.update();
        let p = app.world().resource::<Patrol>();
        assert_eq!(p.input, "What is your name?");
        assert!(p.history.is_empty());
        assert!(!p.pending);
    }
}
