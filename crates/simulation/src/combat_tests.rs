use super::*;
use border_net::combat::*;
fn active() -> Simulation {
    let mut s = Simulation::new(
        "combat".into(),
        42,
        border_scenarios::load_pack(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios"),
        )
        .unwrap(),
    )
    .unwrap();
    s.join("Host".into(), 0).unwrap();
    s.join("Guest".into(), 0).unwrap();
    s.world.peers[0].ready = true;
    s.start_shift().unwrap();
    run(&mut s, 15.);
    s.world.peers[0].weapon.drawn = true;
    s
}
fn run(s: &mut Simulation, seconds: f32) {
    for _ in 0..(seconds * 100.) as usize {
        s.tick(s.world.server_time_ms + 10);
    }
}
fn pose(s: &mut Simulation, p: V3) {
    s.world.peers[0].position = p;
}
#[test]
fn projectile_has_travel_time_and_cannot_inject_damage_or_origin() {
    let mut s = active();
    pose(&mut s, [-0.48, 1.47, -4.]);
    s.fire(1, [0., 0., 1.]).unwrap();
    assert_eq!(s.world.vehicles[0].physics.npc.health, 100);
    assert_eq!(s.world.projectiles.len(), 1);
    run(&mut s, 0.01);
    assert_eq!(s.world.vehicles[0].physics.npc.health, 100);
    run(&mut s, 0.1);
    assert!(s.world.vehicles[0].physics.npc.health < 100);
    for raw in [
        r#"{"type":"fire","direction":[0,0,1],"damage":1000}"#,
        r#"{"type":"fire","direction":[0,0,1],"origin":[0,0,0]}"#,
        r#"{"type":"fire","direction":[0,0,1],"target_id":1}"#,
    ] {
        assert!(serde_json::from_str::<ClientCommand>(raw).is_err());
    }
}
#[test]
fn windshield_hole_penetrates_but_bodywork_and_world_walls_stop_shots() {
    let mut s = active();
    pose(&mut s, [-0.48, 1.47, -4.]);
    s.fire(1, [0., 0., 1.]).unwrap();
    run(&mut s, 0.25);
    let first = &s.world.vehicles[0].physics;
    assert_eq!(first.glass[Pane::Windshield.index()].impacts.len(), 1);
    assert!(!first.glass[0].shattered);
    assert!(first.npc.health > 0 && first.npc.health < 100);
    s.fire(1, [0., 0., 1.]).unwrap();
    run(&mut s, 0.25);
    let second = &s.world.vehicles[0].physics;
    assert_eq!(second.glass[0].impacts.len(), 1, "same hole must transmit");
    assert_eq!(second.npc.health, 0);
    assert!(second.npc.died_seated);
    assert_eq!(s.world.vehicles[0].stage, VehicleStage::Disabled);
    pose(&mut s, [0.45, 1.45, -4.]);
    s.fire(1, [0., 0., 1.]).unwrap();
    run(&mut s, 0.25);
    assert!(
        s.world.vehicles[0].physics.glass[0].shattered,
        "second distinct windshield strike removes the pane"
    );
    let mut s = active();
    pose(&mut s, [-3., 0.7, -0.25]);
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.1);
    assert_eq!(s.world.vehicles[0].physics.npc.health, 100);
    assert!(s.world.impacts.iter().any(|i| i.kind == "metal"));
    pose(&mut s, [-12., 1.5, -7.]);
    run(&mut s, 0.25);
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.2);
    assert!(s.world.impacts.iter().any(|i| i.kind == "world"));
}
#[test]
fn moving_side_window_and_open_door_use_actual_apertures() {
    let mut s = active();
    pose(&mut s, [1.6, 1.47, -0.25]);
    s.toggle_window(1, 1, Pane::PassengerFront).unwrap();
    run(&mut s, 0.8);
    assert!(pane_frame(&s.world.vehicles[0], Pane::PassengerFront).is_none());
    s.fire(1, [-1., 0., 0.]).unwrap();
    run(&mut s, 0.1);
    assert!(s.world.vehicles[0].physics.npc.health < 100);
    assert!(
        s.world.vehicles[0].physics.glass[Pane::PassengerFront.index()]
            .impacts
            .is_empty()
    );
    s.toggle_window(1, 1, Pane::PassengerFront).unwrap();
    run(&mut s, 0.8);
    let before = pane_frame(&s.world.vehicles[0], Pane::PassengerFront)
        .unwrap()
        .center;
    s.toggle_door(1, 1, Side::Passenger).unwrap();
    run(&mut s, 0.7);
    let after = pane_frame(&s.world.vehicles[0], Pane::PassengerFront)
        .unwrap()
        .center;
    assert!(length(sub(after, before)) > 0.4);
    assert!(s.world.vehicles[0].physics.door_angles[1] > 1.);
}
#[test]
fn side_glass_shatters_and_repeated_shots_kill_an_occupant() {
    let mut s = active();
    pose(&mut s, [2., 1.47, -0.25]);
    s.fire(1, [-1., 0., 0.]).unwrap();
    run(&mut s, 0.25);
    assert!(s.world.vehicles[0].physics.glass[Pane::PassengerFront.index()].shattered);
    assert!(s.world.vehicles[0].physics.npc.health < 100);
    s.fire(1, [-1., 0., 0.]).unwrap();
    run(&mut s, 0.25);
    assert_eq!(s.world.vehicles[0].physics.npc.mode, NpcMode::Dead);
    assert!(s.public_case(1, 1, "offline").is_err());
    assert!(s.actor_context(1).is_err());
}
#[test]
fn npc_can_exit_return_and_be_hit_outside_without_driving_an_empty_car() {
    let mut s = active();
    pose(&mut s, [-1.6, 1.7, -0.25]);
    s.order_exit(1, 1).unwrap();
    run(&mut s, 3.);
    assert_eq!(s.world.vehicles[0].physics.npc.mode, NpcMode::Outside);
    assert!(s.world.vehicles[0].physics.npc.position[0] < -2.);
    s.decide_case(1, 1, Disposition::Admit).unwrap();
    assert!(s.release_case(1, 1).is_err());
    s.order_enter(1, 1).unwrap();
    run(&mut s, 2.);
    assert_eq!(s.world.vehicles[0].physics.npc.mode, NpcMode::Seated);
    s.release_case(1, 1).unwrap();
    let mut s = active();
    pose(&mut s, [-1.6, 1.7, -0.25]);
    s.order_exit(1, 1).unwrap();
    run(&mut s, 3.);
    let target = s.world.vehicles[0].physics.npc.position;
    pose(&mut s, [target[0] - 2., target[1] + 1.62, target[2]]);
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.1);
    assert_eq!(s.world.vehicles[0].physics.npc.health, 0);
    assert!(!s.world.vehicles[0].physics.npc.died_seated);
}
#[test]
fn dead_driver_holds_queue_until_recovery_and_reset_restores_everything() {
    let mut s = active();
    pose(&mut s, [-1.6, 1.47, -0.25]);
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.1);
    assert_eq!(s.world.vehicles[0].physics.npc.health, 0);
    run(&mut s, 5.);
    assert_eq!(s.world.vehicles[2].stage, VehicleStage::Queued);
    assert!(s.tow_vehicle(2, 1).is_err());
    s.tow_vehicle(1, 1).unwrap();
    run(&mut s, 4.);
    assert_eq!(s.world.vehicles[0].stage, VehicleStage::Disabled);
    run(&mut s, 16.);
    assert_eq!(s.world.vehicles[0].stage, VehicleStage::Exited);
    assert_eq!(s.world.vehicles[2].stage, VehicleStage::Stopped);
    s.world.phase = ShiftPhase::Grace;
    s.reset_shift(1).unwrap();
    assert!(s.world.projectiles.is_empty() && s.world.impacts.is_empty());
    assert_eq!(s.world.peers[0].weapon.magazine, MAGAZINE);
    assert!(!s.world.peers[0].weapon.drawn);
}
#[test]
fn fire_rate_ammunition_reload_and_friendly_damage_are_host_owned() {
    let mut s = active();
    pose(&mut s, [-4., 1.7, -2.]);
    assert!(s.fire(1, [0., 0., 10.]).is_err());
    assert!(s.fire(1, [f32::NAN, 0., 1.]).is_err());
    s.fire(1, [0., 1., 0.]).unwrap();
    assert!(s.fire(1, [0., 1., 0.]).is_err());
    assert_eq!(s.world.peers[0].weapon.magazine, 11);
    s.reload(1).unwrap();
    assert!(s.fire(1, [0., 1., 0.]).is_err());
    run(&mut s, 1.6);
    assert_eq!(s.world.peers[0].weapon.magazine, 12);
    assert_eq!(s.world.peers[0].weapon.reserve, 59);
    s.world.peers[1].position = [-2., 1.7, -2.];
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.1);
    assert!(s.world.peers[1].health < 100);
    assert_eq!(s.world.peers[0].health, 100);
    s.world.phase = ShiftPhase::Lobby;
    assert!(s.fire(1, [0., 1., 0.]).is_err());
    s.world.phase = ShiftPhase::Grace;
    s.world.peers[0].health = 0;
    let sequence = s.peers[&1].next_sequence;
    s.command(
        1,
        CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            session_id: "combat".into(),
            request_id: "incapacitated-reset".into(),
            sequence,
            expected_revision: None,
            command: ClientCommand::ResetShift,
        },
        s.world.server_time_ms,
    )
    .unwrap();
    assert_eq!(s.world.peers[0].health, 100);
}

#[test]
fn ragdoll_uses_jointed_bodies_in_the_cabin_and_can_be_physically_pulled_out() {
    let mut s = active();
    pose(&mut s, [-1.6, 1.47, -0.25]);
    s.fire(1, [1., 0., 0.]).unwrap();
    run(&mut s, 0.3);
    let first = s.world.vehicles[0].physics.npc.ragdoll.clone();
    assert_eq!(first.len(), 11);
    run(&mut s, 0.6);
    let second = &s.world.vehicles[0].physics.npc.ragdoll;
    assert!(
        length(sub(first[2].position, second[2].position)) > 0.01,
        "head must fall under constraints"
    );
    assert!(
        length(sub(second[0].position, [-0.48, 0.88, -0.25])) < 0.3,
        "seat belt should hold pelvis in the cab"
    );
    pose(&mut s, [-1.6, 1.7, -0.25]);
    s.pull_body(1, 1).unwrap();
    assert!(s.pull_body(1, 1).is_err());
    run(&mut s, 8.);
    let n = &s.world.vehicles[0].physics.npc;
    assert!(
        n.extracted,
        "pull active={}, bones={:?}",
        n.body_pull_active, n.ragdoll
    );
    assert!(!n.died_seated);
    assert!(
        n.ragdoll[1].position[0] < -1.4,
        "chest did not leave vehicle: {:?}",
        n.ragdoll[1].position
    );
    for b in &n.ragdoll {
        assert!(b.position.iter().all(|x| x.is_finite()));
        assert!(b.position[1] > -0.3, "body fell through ground");
        assert!(
            length(sub(b.position, n.ragdoll[0].position)) < 1.9,
            "joint stretched apart"
        );
    }
}
#[test]
fn shattered_glass_is_rigid_body_debris_that_falls_collides_and_expires() {
    let mut s = active();
    pose(&mut s, [2., 1.47, -0.25]);
    s.fire(1, [-1., 0., 0.]).unwrap();
    run(&mut s, 0.08);
    assert!(s.world.shards.len() >= 24);
    let initial: f32 =
        s.world.shards.iter().map(|x| x.position[1]).sum::<f32>() / s.world.shards.len() as f32;
    run(&mut s, 0.8);
    let later: f32 =
        s.world.shards.iter().map(|x| x.position[1]).sum::<f32>() / s.world.shards.len() as f32;
    assert!(later < initial - 0.1, "glass did not fall");
    assert!(s.world.shards.iter().all(|s| s.position[1] > -0.12));
    run(&mut s, 9.);
    assert!(s.world.shards.is_empty());
}

#[test]
fn anatomical_constraints_survive_seated_standing_impacts_and_extraction() {
    for kind in border_net::vehicle::KINDS {
        for seated in [true, false] {
            for yaw in [0., 1.2] {
                let mut s = active();
                let v = &mut s.world.vehicles[0];
                v.kind = kind.into();
                v.yaw = yaw;
                v.stage = VehicleStage::Disabled;
                v.physics.npc.health = 0;
                v.physics.npc.mode = NpcMode::Dead;
                v.physics.npc.died_seated = seated;
                v.physics.npc.yaw = yaw;
                v.physics.npc.position = [-3., 0., 0.];
                v.physics.npc.death_impulse = [4., 0.5, -3.];
                v.physics.door_angles = [1.35, 0.];
                for g in &mut v.physics.glass {
                    g.shattered = true;
                }
                for step in 0..600 {
                    if step == 120 && seated {
                        assert!(s.physics.start_pull(1));
                    }
                    if step == 300 {
                        s.physics.impulse(1, 1, [5., 1., -4.]);
                    }
                    s.physics.advance(&mut s.world, 1. / 60.);
                    s.physics.assert_rig_integrity(&format!(
                        "kind={kind}, seated={seated}, yaw={yaw}, step={step}"
                    ));
                }
                if seated {
                    assert!(
                        s.world.vehicles[0].physics.npc.extracted,
                        "extraction stalled: {kind}, yaw={yaw}"
                    );
                }
            }
        }
    }
}
