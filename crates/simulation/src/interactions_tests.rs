use super::*;
use border_net::combat::{NpcMode, Pane};
fn active() -> Simulation {
    let pack = border_scenarios::load_pack(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/scenarios"
    )))
    .unwrap();
    let mut s = Simulation::new("casual".into(), 42, pack).unwrap();
    s.join("A".into(), 0).unwrap();
    s.join("B".into(), 0).unwrap();
    s.world.peers[0].ready = true;
    s.start_shift().unwrap();
    run(&mut s, 15.);
    s
}
fn run(s: &mut Simulation, seconds: f32) {
    for _ in 0..(seconds * 20.) as usize {
        s.tick(s.world.server_time_ms + 50);
    }
}
fn driver(s: &mut Simulation, peer: u64) {
    let v = &s.world.vehicles[0];
    s.world.peers[peer as usize - 1].position =
        vehicle::spec(&v.kind).world_point(v.position, v.yaw, vehicle::VehiclePoint::Driver);
}
fn corpse(s: &mut Simulation, seated: bool) {
    let v = &mut s.world.vehicles[0];
    v.stage = VehicleStage::Disabled;
    v.physics.npc.health = 0;
    v.physics.npc.mode = NpcMode::Dead;
    v.physics.npc.died_seated = seated;
    v.physics.npc.position = [-3., 0., 0.];
    run(s, 0.1);
}
#[test]
fn exclusive_conversations_allow_walking_and_reject_old_owner_deliveries() {
    let mut s = active();
    driver(&mut s, 1);
    driver(&mut s, 2);
    s.world.peers[1].assisting = Some(1);
    s.public_case(1, 1, "offline").unwrap();
    let old = s.actor_context(1).unwrap();
    assert!(s.public_case(2, 1, "offline").is_err());
    assert!(s.authorize_conversation(2, 1).is_err());
    s.world.peers[0].position = [0., 1.7, 3.5];
    assert!(s.authorize_conversation(1, 1).is_ok());
    assert!(s.authorize_case(1, 1).is_err());
    s.end_conversation(1, 1).unwrap();
    s.public_case(2, 1, "offline").unwrap();
    assert!(s.actor(ActorDelivery{conversation_epoch:old.conversation_epoch,case_epoch:old.case_epoch,expected_turn:0,protocol_version:PROTOCOL_VERSION,session_id:"casual".into(),vehicle_id:1,officer_utterance:"hello".into(),raw_reply:r#"{"dialogue":"Hello","emotion":"cooperative","action":"none","fact_ids":[],"end_conversation":false}"#.into()}).is_err());
    s.world.peers[1].position = [-15., 1.7, -20.];
    run(&mut s, 0.1);
    assert_eq!(s.world.vehicles[0].conversation_owner, None);
}
#[test]
fn arrivals_keep_windows_up_until_an_officer_approaches() {
    let mut s = active();
    assert_eq!(
        s.world.vehicles[0].physics.glass[Pane::DriverFront.index()].lowered,
        0.
    );
    driver(&mut s, 1);
    run(&mut s, 1.);
    assert!(s.world.vehicles[0].physics.glass[Pane::DriverFront.index()].lowered > 0.95);
    s.world.peers[0].position = [-15., 1.7, -20.];
    run(&mut s, 1.);
    assert!(s.world.vehicles[0].physics.glass[Pane::DriverFront.index()].lowered < 0.05);
}
#[test]
fn delivery_failure_is_a_remembered_pardon_without_new_world_facts() {
    let mut s = active();
    driver(&mut s, 1);
    s.public_case(1, 1, "live").unwrap();
    let context = s.actor_context(1).unwrap();
    let before = serde_json::to_value(&s.cases[&1].scenario.truth).unwrap();
    s.register_utterance(1, 1, "I will kill you").unwrap();
    assert_eq!(s.world.money, -100);
    let r = s
        .delivery_failure(1, 1, context.conversation_epoch, 0, "I will kill you")
        .unwrap();
    assert_eq!(r.dialogue, "Excuse me?");
    assert_eq!(s.world.vehicles[0].history.len(), 1);
    assert_eq!(
        serde_json::to_value(&s.cases[&1].scenario.truth).unwrap(),
        before
    );
    assert!(
        s.delivery_failure(1, 1, context.conversation_epoch, 0, "same retry")
            .is_err()
    );
}
#[test]
fn body_grips_survive_thirty_seconds_and_can_be_released_and_repeated() {
    let mut s = active();
    corpse(&mut s, false);
    s.world.peers[0].position = [-4., 1.7, 0.];
    for round in 0..2 {
        s.grab_body(1, 1, 1).unwrap();
        assert!(s.grab_body(2, 1, 1).is_err());
        for frame in 1..=600 {
            let now = s.world.server_time_ms;
            s.input(
                1,
                InputFrame {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: "casual".into(),
                    frame: round * 1000 + frame,
                    movement: [0., 0.],
                    yaw: 1.57,
                    jog: false,
                    ready_held: false,
                    grab_held: true,
                    pitch: 0.,
                },
                now,
            )
            .unwrap();
            s.tick(now + 50);
            s.physics.assert_rig_integrity("held body");
        }
        assert_eq!(s.world.vehicles[0].physics.npc.body_holder, Some(1));
        s.release_bodies(1);
        assert_eq!(s.world.vehicles[0].physics.npc.body_holder, None);
        let chest = s.world.vehicles[0].physics.npc.ragdoll[1].position;
        s.world.peers[0].position = [chest[0] + 1., 1.7, chest[2]];
    }
}
#[test]
fn porter_removes_a_seated_body_drives_the_vehicle_and_returns_to_his_chair() {
    let mut s = active();
    corpse(&mut s, true);
    driver(&mut s, 1);
    s.request_porter(1, 1).unwrap();
    let mut driving = false;
    for _ in 0..2400 {
        s.heartbeat(1, s.world.server_time_ms).unwrap();
        s.tick(s.world.server_time_ms + 50);
        driving |= s.world.porters[0].phase == PorterPhase::Driving;
    }
    assert!(driving);
    assert_eq!(s.world.vehicles[0].stage, VehicleStage::Detained);
    assert!(!s.world.lanes[0].queue.contains(&1));
    assert_eq!(s.world.porters[0].phase, PorterPhase::Seated);
    assert_eq!(s.world.porters[0].yaw, 0.);
    assert_eq!(s.world.porters[0].position, porter_chair(1));
    assert!(s.world.vehicles[0].physics.npc.extracted);
    assert_eq!(s.world.vehicles[0].physics.npc.ragdoll.len(), 11);
}

#[test]
fn weapon_context_uses_pitch_and_does_not_charge_the_wrong_officer() {
    let mut s = active();
    driver(&mut s, 1);
    driver(&mut s, 2);
    s.public_case(1, 1, "live").unwrap();
    let target = border_net::combat::add(s.world.vehicles[0].physics.npc.position, [0., 1.5, 0.]);
    let p = &mut s.world.peers[1];
    let d = border_net::combat::sub(target, p.position);
    p.yaw = (-d[0]).atan2(-d[2]);
    p.pitch = d[1].atan2(d[0].hypot(d[2]));
    p.weapon.drawn = true;
    assert!(s.physical_context(&s.world.vehicles[0]).weapon_aimed);
    s.register_utterance(1, 1, "Hello").unwrap();
    assert_eq!(s.world.money, 0);
    s.world.peers[1].pitch = 1.4;
    assert!(!s.physical_context(&s.world.vehicles[0]).weapon_aimed);
    assert!(s.physical_context(&s.world.vehicles[0]).weapon_drawn);
}

#[test]
fn an_abandoned_body_grip_releases_when_input_goes_stale() {
    let mut s = active();
    corpse(&mut s, false);
    s.world.peers[0].position = [-4., 1.7, 0.];
    s.grab_body(1, 1, 1).unwrap();
    run(&mut s, 1.);
    assert_eq!(s.world.vehicles[0].physics.npc.body_holder, None);
}

#[test]
fn porter_enters_without_dragging_an_already_outside_body() {
    let mut s = active();
    corpse(&mut s, false);
    driver(&mut s, 1);
    s.request_porter(1, 1).unwrap();
    for _ in 0..400 {
        s.heartbeat(1, s.world.server_time_ms).unwrap();
        s.tick(s.world.server_time_ms + 50);
        assert!(!s.world.vehicles[0].physics.npc.body_pull_active);
        if s.world.porters[0].phase == PorterPhase::Driving {
            return;
        }
    }
    panic!("porter never entered the empty driver's seat");
}

#[test]
fn repeated_threats_can_make_a_bold_aggressive_traveler_physically_flee() {
    let mut s = active();
    let personality = s
        .cases
        .values()
        .find(|c| c.scenario.scenario_id == "06_dorian")
        .unwrap()
        .scenario
        .character
        .clone();
    s.cases.get_mut(&1).unwrap().scenario.character = personality;
    driver(&mut s, 1);
    s.public_case(1, 1, "live").unwrap();
    for _ in 0..3 {
        let context = s.actor_context(1).unwrap();
        s.register_utterance(1, 1, "I will kill you").unwrap();
        s.delivery_failure(
            1,
            1,
            context.conversation_epoch,
            context.expected_turn,
            "I will kill you",
        )
        .unwrap();
    }
    run(&mut s, 4.);
    assert_eq!(s.world.vehicles[0].physics.npc.mode, NpcMode::Fleeing);
    let before = s.world.vehicles[0].physics.npc.position;
    run(&mut s, 1.);
    assert!(
        border_net::combat::length(border_net::combat::sub(
            s.world.vehicles[0].physics.npc.position,
            before
        )) > 3.
    );
    assert_eq!(s.world.money, -300);
}

#[test]
fn cargo_and_container_close_reopen_and_keep_recorded_findings_on_every_vehicle() {
    for kind in vehicle::KINDS {
        let mut s = active();
        s.world.vehicles[0].kind = kind.into();
        let v = &s.world.vehicles[0];
        s.world.peers[0].position =
            vehicle::spec(kind).world_point(v.position, v.yaw, vehicle::VehiclePoint::Cargo);
        let command = |s: &mut Simulation, cmd| {
            let sequence = s.peers[&1].next_sequence;
            s.command(
                1,
                CommandEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: "casual".into(),
                    request_id: sequence.to_string(),
                    sequence,
                    expected_revision: None,
                    command: cmd,
                },
                s.world.server_time_ms,
            )
            .unwrap();
        };
        let toggle = ClientCommand::TogglePanel {
            vehicle_id: 1,
            point: vehicle::VehiclePoint::Cargo,
        };
        command(&mut s, toggle.clone());
        let count = s.world.vehicles[0].observed_evidence.len();
        assert!(count > 0);
        s.toggle_compartment(1, 1, border_scenarios::EvidenceLocation::Container)
            .unwrap();
        assert!(
            s.world.vehicles[0]
                .open_compartments
                .contains(&border_scenarios::EvidenceLocation::Container)
        );
        let found = s.world.vehicles[0].observed_evidence.len();
        command(&mut s, toggle.clone());
        assert!(
            !s.world.vehicles[0]
                .open_panels
                .contains(&vehicle::VehiclePoint::Cargo)
        );
        assert!(
            !s.world.vehicles[0]
                .open_compartments
                .contains(&border_scenarios::EvidenceLocation::Container)
        );
        assert_eq!(s.world.vehicles[0].observed_evidence.len(), found);
        command(&mut s, toggle);
        s.toggle_compartment(1, 1, border_scenarios::EvidenceLocation::Container)
            .unwrap();
        s.toggle_compartment(1, 1, border_scenarios::EvidenceLocation::Container)
            .unwrap();
        assert!(
            !s.world.vehicles[0]
                .open_compartments
                .contains(&border_scenarios::EvidenceLocation::Container)
        );
        assert_eq!(s.world.vehicles[0].observed_evidence.len(), found);
    }
}

#[test]
fn walking_agents_replan_around_a_moving_rotated_vehicle_without_crossing_it() {
    let mut s = active();
    s.world.vehicles.truncate(1);
    s.world.peers.clear();
    s.world.porters.clear();
    s.collision.clear();
    for l in &mut s.world.lanes {
        l.entry_barrier_open = true;
        l.barrier_open = true;
    }
    s.world.vehicles[0].position = [0.; 3];
    s.world.vehicles[0].yaw = 0.7;
    let mut p = [-5., 0., 0.];
    let mut yaw = 0.;
    let goal = [5., 0., 0.];
    let mut arrived = false;
    for frame in 0..600 {
        if frame == 10 {
            s.world.vehicles[0].position = [1.5, 0., 0.];
        }
        let before = p;
        arrived = s.walk_actor(
            navigation::Walker::Traveler(99),
            &mut p,
            &mut yaw,
            goal,
            3.5,
            0.05,
        );
        assert!(border_net::combat::length(border_net::combat::sub(p, before)) <= 0.176);
        let v = &s.world.vehicles[0];
        let local = border_net::combat::local(v, p);
        let spec = vehicle::spec(&v.kind);
        let dx = (local[0].abs() - spec.half_width).max(0.);
        let dz = (local[2].abs() - spec.half_length).max(0.);
        assert!(dx.hypot(dz) >= 0.269, "agent crossed vehicle at {p:?}");
        if arrived {
            break;
        }
        s.world.server_time_ms += 50;
    }
    assert!(arrived, "agent did not find a route around the car: {p:?}");
}
