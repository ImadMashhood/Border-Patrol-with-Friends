//! Thin client for the authoritative host. Movement is intent; snapshots own positions and time.
use super::*;
use border_net::*;
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Clone, Default)]
pub struct Controls {
    pub movement: [f32; 2],
    pub yaw: f32,
    pub jog: bool,
    pub ready: bool,
}
#[derive(Resource)]
pub struct Shift {
    pub world: Option<WorldSnapshot>,
    pub peer_id: u64,
    pub error: String,
    pub interaction: Option<(u64, vehicle::VehiclePoint)>,
    pub inspection: Option<(u64, vehicle::VehiclePoint)>,
    pub connected: bool,
    pub prediction: prediction::Prediction,
    pub collision: Vec<map::Aabb>,
    pub controls: Arc<Mutex<Controls>>,
    pub commands: mpsc::Sender<ClientCommand>,
    updates: Mutex<mpsc::Receiver<Result<WorldSnapshot, String>>>,
    alive: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Shift {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Shift {
    pub fn preview() -> Self {
        let (commands, _) = mpsc::channel();
        let (_, updates) = mpsc::channel();
        Self {
            world: None,
            peer_id: 0,
            error: String::new(),
            interaction: None,
            inspection: None,
            connected: false,
            prediction: prediction::Prediction::default(),
            collision: map::collision().solids,
            controls: Arc::new(Mutex::new(Controls::default())),
            commands,
            updates: Mutex::new(updates),
            alive: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }
    pub fn connect(
        url: String,
        code: String,
        name: String,
    ) -> Result<(Self, String, String), String> {
        let url = url.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client
            .post(format!("{url}/join"))
            .json(&JoinRequest {
                protocol_version: PROTOCOL_VERSION,
                join_code: code,
                display_name: name,
            })
            .send()
            .map_err(|_| {
                "Cannot reach shift host. Launch ./play.sh or check the join address.".to_string()
            })?;
        if !response.status().is_success() {
            return Err("Host rejected the join: check the invite, available player slots and protocol version.".into());
        }
        let joined: JoinResponse = response.json().map_err(|_| "Invalid host join response")?;
        let token = joined.peer_token.clone();
        let peer_id = joined.peer_id;
        let (commands, requests) = mpsc::channel::<ClientCommand>();
        let (tx, updates) = mpsc::channel();
        let controls = Arc::new(Mutex::new(Controls::default()));
        let input = controls.clone();
        let alive = Arc::new(AtomicBool::new(true));
        let running = alive.clone();
        let base = url.clone();
        let auth = token.clone();
        let session = joined.session_id.clone();
        let worker = std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .expect("host HTTP client");
            let mut sequence = joined.next_sequence;
            let mut frame = 0;
            let mut pending: Option<CommandEnvelope> = None;
            while running.load(Ordering::Relaxed) {
                let started = std::time::Instant::now();
                for _ in 0..8 {
                    if pending.is_none() {
                        if let Ok(command) = requests.try_recv() {
                            pending = Some(CommandEnvelope {
                                protocol_version: PROTOCOL_VERSION,
                                session_id: session.clone(),
                                request_id: format!("{peer_id}-{sequence}"),
                                sequence,
                                expected_revision: None,
                                command,
                            });
                        } else {
                            break;
                        }
                    }
                    let envelope = pending.as_ref().unwrap();
                    let send = || {
                        client
                            .post(format!("{base}/command"))
                            .bearer_auth(&auth)
                            .json(envelope)
                            .send()
                    };
                    match send().or_else(|_| send()) {
                        Ok(response) if response.status().is_success() => {
                            sequence += 1;
                            pending = None;
                            let _ = tx.send(Err(String::new()));
                        }
                        Ok(response) => {
                            let body: serde_json::Value = response.json().unwrap_or_default();
                            let _ = tx.send(Err(body["error"]
                                .as_str()
                                .unwrap_or("Host rejected the command")
                                .into()));
                            pending = None;
                        }
                        Err(_) => {
                            let _ = tx.send(Err(
                                "Connection interrupted; retrying the same command safely".into(),
                            ));
                            break;
                        }
                    }
                }
                frame += 1;
                let state = input.lock().unwrap().clone();
                let result = client
                    .post(format!("{base}/input"))
                    .bearer_auth(&auth)
                    .json(&InputFrame {
                        protocol_version: PROTOCOL_VERSION,
                        session_id: session.clone(),
                        frame,
                        movement: state.movement,
                        yaw: state.yaw,
                        jog: state.jog,
                        ready_held: state.ready,
                    })
                    .send();
                match result {
                    Ok(response) if response.status().is_success() => {
                        let value = response
                            .json::<WorldSnapshot>()
                            .map_err(|_| "Invalid host state".into());
                        if let Ok(world) = &value
                            && let Some(peer) = world.peers.iter().find(|p| p.peer_id == peer_id)
                            && peer.next_command_sequence > sequence
                        {
                            pending = None;
                            sequence = peer.next_command_sequence;
                        }
                        let _ = tx.send(value);
                    }
                    _ => {
                        let _=tx.send(Err("Host connection lost — movement is frozen until communication recovers".into()));
                    }
                }
                let delay = Duration::from_millis(50).saturating_sub(started.elapsed());
                std::thread::sleep(delay);
            }
            let _ = client
                .post(format!("{base}/leave"))
                .bearer_auth(auth)
                .json(&SessionRequest {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: session,
                })
                .send();
        });
        Ok((
            Self {
                world: Some(joined.snapshot),
                peer_id,
                error: String::new(),
                interaction: None,
                inspection: None,
                connected: true,
                prediction: prediction::Prediction::default(),
                collision: map::collision().solids,
                controls,
                commands,
                updates: Mutex::new(updates),
                alive,
                worker: Some(worker),
            },
            url,
            token,
        ))
    }
    pub fn send(&self, command: ClientCommand) {
        let _ = self.commands.send(command);
    }
    pub fn me(&self) -> Option<&PeerView> {
        self.world
            .as_ref()?
            .peers
            .iter()
            .find(|p| p.peer_id == self.peer_id)
    }
    pub fn permits(&self, lane: u8) -> bool {
        self.world.as_ref().is_some_and(|w| {
            w.lanes
                .iter()
                .any(|l| l.lane_id == lane && l.owner == Some(self.peer_id))
                || self.me().is_some_and(|p| p.assisting == Some(lane))
        })
    }
}
#[derive(Component)]
pub struct ReadyDoor;
#[derive(Component)]
pub struct RemoteOfficer(pub u64);
#[derive(Component)]
pub struct Flashlight;
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync(
    mut commands: Commands,
    mut shift: ResMut<Shift>,
    mut g: ResMut<Patrol>,
    smoke: Option<Res<SmokeTest>>,
    mut players: Query<
        (&mut Transform, &mut Player),
        (
            Without<traffic::Vehicle>,
            Without<ReadyDoor>,
            Without<RemoteOfficer>,
        ),
    >,
    mut doors: Query<
        &mut Transform,
        (
            With<ReadyDoor>,
            Without<Player>,
            Without<traffic::Vehicle>,
            Without<RemoteOfficer>,
        ),
    >,
    mut cars: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut traffic::Vehicle,
            &mut Solid,
        ),
        (Without<Player>, Without<ReadyDoor>, Without<RemoteOfficer>),
    >,
    mut barriers: Query<
        (
            &mut Transform,
            &traffic::LaneBarrier,
            Option<&vehicles::EntryGate>,
        ),
        (
            With<Barrier>,
            Without<Player>,
            Without<ReadyDoor>,
            Without<traffic::Vehicle>,
            Without<RemoteOfficer>,
        ),
    >,
    mut officers: Query<
        (Entity, &mut Transform, &RemoteOfficer),
        (
            Without<Player>,
            Without<ReadyDoor>,
            Without<traffic::Vehicle>,
            Without<Barrier>,
        ),
    >,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !shift.connected {
        return;
    }
    let updates: Vec<_> = shift.updates.lock().unwrap().try_iter().collect();
    let previous = shift.world.as_ref().map(|w| w.phase.clone());
    let mut received = false;
    for update in updates {
        match update {
            Ok(world) => {
                received = true;
                shift.world = Some(world);
                if shift.error.starts_with("Host connection") {
                    shift.error.clear();
                }
            }
            Err(message) => shift.error = message,
        }
    }
    if previous.as_ref().is_some_and(|p| *p != ShiftPhase::Lobby)
        && shift
            .world
            .as_ref()
            .is_some_and(|w| w.phase == ShiftPhase::Lobby)
    {
        shift.inspection = None;
        shift.interaction = None;
    }
    let reset = previous.as_ref().is_some_and(|p| *p != ShiftPhase::Lobby)
        && shift
            .world
            .as_ref()
            .is_some_and(|w| w.phase == ShiftPhase::Lobby);
    if smoke.is_none() {
        if (received || !shift.prediction.initialized)
            && let Some(me) = shift.me()
        {
            let position = Vec3::from_array(me.position);
            let controls = shift.controls.lock().unwrap().clone();
            let speed = if controls.jog { 5.8 } else { 3.2 };
            shift.prediction.reconcile(
                position,
                Vec3::new(controls.movement[0], 0., controls.movement[1]) * speed,
                time.elapsed_secs(),
                reset,
            );
        }
        shift.prediction.correct(time.delta_secs().min(0.05));
    }
    let Some(world) = &shift.world else {
        return;
    };
    let Some(me) = shift.me() else {
        return;
    };
    if let Ok((mut t, mut player)) = players.single_mut() {
        let target = Vec3::from_array(me.position);
        let reset =
            previous.is_some_and(|p| p != ShiftPhase::Lobby) && world.phase == ShiftPhase::Lobby;
        if smoke.is_none() {
            t.translation = shift.prediction.position;
        } else if reset || t.translation.distance(target) > 3. {
            t.translation = target;
        } else {
            t.translation = t
                .translation
                .lerp(target, (time.delta_secs() * 18.).min(1.));
        }
        if reset {
            player.yaw = std::f32::consts::PI;
            player.pitch = 0.;
            g.reset_case();
            g.score = 0;
            g.vehicle_id = None;
        }
    }
    for mut door in &mut doors {
        door.rotation = door.rotation.slerp(
            Quat::from_rotation_y(if world.door_open { -1.5 } else { 0. }),
            (time.delta_secs() * 5.).min(1.),
        );
    }
    for (mut t, mut visibility, mut car, mut solid) in &mut cars {
        let id = 1 + car.lane as u64 + car.ordinal as u64 * 2;
        if let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == id) {
            *visibility = if v.stage == VehicleStage::Exited {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            let target = Vec3::from_array(v.position);
            t.translation = if t.translation.distance(target) > 4. {
                target
            } else {
                t.translation
                    .lerp(target, (time.delta_secs() * 20.).min(1.))
            };
            t.rotation = t.rotation.slerp(
                Quat::from_rotation_y(v.yaw),
                (time.delta_secs() * 20.).min(1.),
            );
            solid.half = Vec3::from_array(vehicle::spec(&v.kind).aabb(v.position, v.yaw).half);
            solid.half.y = vehicle::spec(&v.kind).height;
            car.distance = v.distance_to_stop;
            car.speed = v.speed;
        } else {
            *visibility = Visibility::Hidden;
            car.speed = 0.;
        }
    }
    for (mut t, lane, entry) in &mut barriers {
        let open = world.lanes.iter().any(|l| {
            l.lane_id == lane.0 + 1
                && if entry.is_some() {
                    l.entry_barrier_open
                } else {
                    l.barrier_open
                }
        });
        t.rotation = t.rotation.slerp(
            Quat::from_rotation_z(if open { -1.35 } else { 0. }),
            (time.delta_secs() * 5.).min(1.),
        );
    }
    for p in world
        .peers
        .iter()
        .filter(|p| p.connected && p.peer_id != shift.peer_id)
    {
        if let Some((_, mut t, _)) = officers.iter_mut().find(|(_, _, o)| o.0 == p.peer_id) {
            t.translation = t.translation.lerp(
                Vec3::new(p.position[0], 0., p.position[2]),
                (time.delta_secs() * 18.).min(1.),
            );
            t.rotation = t.rotation.slerp(
                Quat::from_rotation_y(p.yaw),
                (time.delta_secs() * 18.).min(1.),
            );
        } else {
            let root = commands
                .spawn((
                    Transform::from_xyz(p.position[0], 0., p.position[2]),
                    Visibility::default(),
                    RemoteOfficer(p.peer_id),
                ))
                .id();
            for (size, pos, color) in [
                (
                    Vec3::new(0.5, 0.8, 0.3),
                    Vec3::new(0., 1.1, 0.),
                    Color::srgb(0.17, 0.29, 0.23),
                ),
                (
                    Vec3::new(0.32, 0.35, 0.3),
                    Vec3::new(0., 1.67, 0.),
                    Color::srgb(0.68, 0.48, 0.34),
                ),
                (
                    Vec3::new(0.48, 0.65, 0.28),
                    Vec3::new(0., 0.4, 0.),
                    Color::srgb(0.17, 0.20, 0.17),
                ),
            ] {
                let part = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(size))),
                        MeshMaterial3d(materials.add(color)),
                        Transform::from_translation(pos),
                    ))
                    .id();
                commands.entity(root).add_child(part);
            }
        }
    }
    for (entity, _, officer) in &officers {
        if !world
            .peers
            .iter()
            .any(|p| p.peer_id == officer.0 && p.connected)
        {
            commands.entity(entity).despawn();
        }
    }
    g.position = world.ledger.len();
    g.score = world.ledger.iter().filter(|e| e.correct).count();
    if let Some(id) = g.vehicle_id {
        if let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == id) {
            if matches!(
                v.stage,
                VehicleStage::DepartingAdmit
                    | VehicleStage::DepartingDeny
                    | VehicleStage::Exited
                    | VehicleStage::Disabled
            ) || !shift.permits(v.lane_id)
            {
                g.reset_case();
                g.vehicle_id = None;
            } else if let Some(outcome) = &v.outcome
                && g.phase != Phase::Departing
            {
                g.reveal = Some(outcome.clone());
                g.phase = Phase::Reveal;
            } else if g.case.is_some() {
                if v.secondary_inspected {
                    g.secondary = true;
                    g.evidence = v
                        .observed_evidence
                        .iter()
                        .map(|e| Evidence {
                            id: e.id.clone(),
                            description: e.description.clone(),
                            reveal_condition: e.reveal_condition.clone(),
                        })
                        .collect();
                }
                g.history = v.history.clone();
                if let Some(last) = g.history.last() {
                    g.subtitle = last.traveler.clone();
                }
            }
        } else {
            g.reset_case();
            g.vehicle_id = None;
        }
    }
    if (world.phase == ShiftPhase::Grace
        || world.ledger.len() == 10 && world.lanes.iter().all(|l| l.queue.is_empty()))
        && g.case.is_none()
        && !world
            .vehicles
            .iter()
            .any(|v| matches!(v.stage, VehicleStage::Stopped | VehicleStage::Resolved))
    {
        g.phase = Phase::Complete;
        g.overlay = true;
        g.looking = false;
    }
}

pub fn draw(ctx: &bevy_egui::egui::Context, shift: &mut Shift) {
    use bevy_egui::egui;
    let Some(world) = &shift.world else {
        return;
    };
    let Some(me) = shift.me() else {
        return;
    };
    let mut close_inspection = false;
    if let Some((id, point)) = shift.inspection
        && let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == id)
    {
        egui::Window::new("VEHICLE INSPECTION").anchor(egui::Align2::CENTER_CENTER,[0.,0.]).default_width(400.).show(ctx, |ui| {
            ui.heading(format!("Lane {} · {} #{}",v.lane_id,v.kind,id));
            ui.label(point.label(&v.kind));
            if v.stage != VehicleStage::Stopped { ui.label("Vehicle released; inspection closed."); }
            else if v.inspected_points.contains(&point) {
                match point {
                    vehicle::VehiclePoint::Cargo => { ui.label("Cargo compartment opened. Recorded findings:"); for e in &v.observed_evidence { ui.label(&e.description); } }
                    vehicle::VehiclePoint::Hood => { ui.label("Hood opened. Visual access to the engine bay recorded; this check does not establish cargo or legal status."); }
                    _ => { ui.label("Passenger door opened. Cabin access recorded; continue questioning the driver and checking the cargo."); }
                }
            } else { ui.label("Waiting for host inspection confirmation…"); }
            if !shift.error.is_empty() { ui.colored_label(egui::Color32::LIGHT_RED,&shift.error); }
            close_inspection = ui.button("Close [Esc]").clicked();
        });
    }
    egui::Window::new("DUTY ROSTER")
        .default_pos([20., 105.])
        .default_width(250.)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!(
                "{}  ·  {} HP{}",
                me.display_name,
                me.health,
                if world.host_peer_id == Some(me.peer_id) {
                    "  ·  HOST"
                } else {
                    ""
                }
            ));
            ui.label(format!(
                "Equipped: {}",
                me.equipment
                    .iter()
                    .map(|item| match item.as_str() {
                        "pistol" => "Pistol",
                        "flashlight" => "Flashlight",
                        "radio" => "Radio",
                        "document_clipboard" => "Clipboard",
                        other => other,
                    })
                    .collect::<Vec<_>>()
                    .join(" · ")
            ));
            ui.label("F: flashlight   1 / 2: claim lane");
            for lane in &world.lanes {
                let owner = world
                    .peers
                    .iter()
                    .find(|p| Some(p.peer_id) == lane.owner)
                    .map(|p| p.display_name.as_str())
                    .unwrap_or("Unassigned");
                ui.horizontal(|ui| {
                    ui.label(format!("Lane {} · {}", lane.lane_id, owner));
                    if lane.owner.is_none() && ui.small_button("Claim").clicked() {
                        shift.send(ClientCommand::ClaimLane {
                            lane_id: lane.lane_id,
                        });
                    }
                });
                if let Some(v) = world.vehicles.iter().find(|v| {
                    v.lane_id == lane.lane_id
                        && matches!(
                            v.stage,
                            VehicleStage::Stopped | VehicleStage::Resolved | VehicleStage::Queued
                        )
                }) {
                    ui.small(format!(
                            "{} #{} · {:?} · {} waiting",
                            v.kind,
                            v.vehicle_id,
                            v.stage,
                            world
                                .vehicles
                                .iter()
                                .filter(|a| a.lane_id == lane.lane_id
                                    && a.stage == VehicleStage::Queued)
                                .count()
                        ));
                    ui.small(format!(
                        "Entry: {}  ·  Exit: {}",
                        if lane.entry_barrier_open {
                            "open"
                        } else {
                            "closed"
                        },
                        if lane.barrier_open { "open" } else { "closed" }
                    ));
                }
            }
            if let Some(other) = world
                .lanes
                .iter()
                .find(|l| l.owner.is_some() && l.owner != Some(me.peer_id))
                && ui
                    .button(if me.assisting == Some(other.lane_id) {
                        "Stop assisting"
                    } else {
                        "Assist other lane [G]"
                    })
                    .clicked()
            {
                shift.send(ClientCommand::AssistLane {
                    lane_id: if me.assisting.is_some() {
                        None
                    } else {
                        Some(other.lane_id)
                    },
                });
            }
            if world.lane_swap.is_some_and(|p| p[1] == me.peer_id) {
                if ui.button("Accept lane swap [J]").clicked() {
                    shift.send(ClientCommand::AcceptLaneSwap);
                }
            } else if world.peers.iter().filter(|p| p.connected).count() > 1
                && ui.button("Request lane swap [J]").clicked()
            {
                shift.send(ClientCommand::RequestLaneSwap);
            }
            if let Some((id, _)) = weapon::nearby(shift, Vec3::from_array(me.position))
                && let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == id)
                && v.physics.npc.health == 0
            {
                if v.physics.npc.body_pull_active {
                    ui.label("Pulling body clear of the seat…");
                } else if v.physics.npc.died_seated
                    && !v.physics.npc.extracted
                    && ui.button("Pull body out [Y]").clicked()
                {
                    shift.send(ClientCommand::PullBody { vehicle_id: id });
                }
            }
            ui.small("Tab releases the mouse for roster controls.");
            if world.phase == ShiftPhase::Grace
                && world.host_peer_id == Some(me.peer_id)
                && ui.button("End shift / return to office").clicked()
            {
                shift.send(ClientCommand::ResetShift);
            }

            if let Some(notice) = world.notices.last() {
                ui.separator();
                ui.label(format!("RADIO · {}", notice.message));
            }
            if !shift.error.is_empty() {
                ui.colored_label(egui::Color32::LIGHT_RED, &shift.error);
            }
        });
    if world.phase == ShiftPhase::Lobby {
        egui::Window::new("BEFORE THE SHIFT")
            .anchor(egui::Align2::CENTER_BOTTOM, [0., -80.])
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Start at the office exit");
                ui.label("Click to look, walk to the door, then hold E for 2 seconds.");
                ui.label("The host can start the shift, or a strict majority can register ready.");
                ui.label(format!(
                    "Ready: {} / {} officers",
                    world
                        .peers
                        .iter()
                        .filter(|p| p.ready && p.connected)
                        .count(),
                    world.peers.iter().filter(|p| p.connected).count()
                ));
                if border_net::map::door_distance(me.position) < 2.2 {
                    ui.add(
                        egui::ProgressBar::new(me.hold_ms as f32 / READY_HOLD_MS as f32)
                            .desired_width(340.)
                            .text(if me.ready {
                                "READY — WAITING FOR HOST / MAJORITY".into()
                            } else {
                                format!("HOLD E  ·  {:.1} / 2.0 s", me.hold_ms as f32 / 1000.)
                            }),
                    );
                }
                if me.ready && ui.small_button("Withdraw ready vote").clicked() {
                    shift.send(ClientCommand::SetReady { ready: false });
                }
            });
    }
    if close_inspection {
        shift.inspection = None;
    }
}
