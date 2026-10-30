use super::*;
use vehicle::{VehiclePoint, spec};
fn active() -> Simulation {
    let mut s = Simulation::new(
        "lifecycle".into(),
        42,
        border_scenarios::load_pack(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios"),
        )
        .unwrap(),
    )
    .unwrap();
    for id in 1..=2 {
        s.join(format!("Officer {id}"), 0).unwrap();
    }
    s.world.peers[0].ready = true;
    s.start_shift().unwrap();
    s
}
fn at_point(s: &mut Simulation, peer: u64, id: u64, point: VehiclePoint) {
    let v = s
        .world
        .vehicles
        .iter()
        .find(|v| v.vehicle_id == id)
        .unwrap();
    s.world.peers[peer as usize - 1].position = spec(&v.kind).world_point(v.position, v.yaw, point);
}
fn run(s: &mut Simulation, seconds: u64) {
    for _ in 0..seconds * 20 {
        s.tick(s.world.server_time_ms + 50);
    }
}
#[test]
fn mixed_queues_preserve_bumper_gaps_and_each_lane_keeps_working() {
    let mut s = active();
    let mut saw_entry = [false; 2];
    for _ in 0..300 {
        s.tick(s.world.server_time_ms + 50);
        for lane in &s.world.lanes {
            saw_entry[lane.lane_id as usize - 1] |= lane.entry_barrier_open;
        }
        for lane in 1..=2 {
            let cars: Vec<_> = s
                .world
                .vehicles
                .iter()
                .filter(|v| v.lane_id == lane)
                .collect();
            assert!(cars.iter().any(|v| v.kind == "truck"));
            assert!(cars.iter().any(|v| v.kind == "sedan"));
            for pair in cars.windows(2) {
                let clearance = pair[1].distance_to_stop
                    - pair[0].distance_to_stop
                    - spec(&pair[0].kind).half_length
                    - spec(&pair[1].kind).half_length;
                assert!(
                    clearance >= vehicle::BUMPER_GAP - 0.05,
                    "bumper overlap {clearance}"
                );
            }
        }
    }
    assert_eq!(saw_entry, [true, true]);
    assert_eq!(s.world.vehicles[0].stage, VehicleStage::Stopped);
    assert_eq!(s.world.vehicles[1].stage, VehicleStage::Stopped);
    at_point(&mut s, 1, 1, VehiclePoint::Driver);
    s.decide_case(1, 1, Disposition::Admit).unwrap();
    s.release_case(1, 1).unwrap();
    run(&mut s, 20);
    assert_eq!(s.world.vehicles[2].stage, VehicleStage::Stopped);
    assert_eq!(
        s.world.vehicles[1].stage,
        VehicleStage::Stopped,
        "unprocessed lane must retain its own driver"
    );
    assert!(!s.world.lanes[0].barrier_open);
    assert!(!s.world.lanes[1].barrier_open);
}
#[test]
fn physical_inspection_checks_lane_stage_and_rotated_interaction_point() {
    let mut s = active();
    run(&mut s, 15);
    at_point(&mut s, 2, 2, VehiclePoint::Driver);
    assert!(s.public_case(2, 2, "offline").is_ok());
    assert!(
        s.inspect_point(2, 2, VehiclePoint::Cargo).is_err(),
        "no remote cargo checks"
    );
    at_point(&mut s, 1, 2, VehiclePoint::Cargo);
    assert!(
        s.inspect_point(1, 2, VehiclePoint::Cargo).is_err(),
        "other lane requires assistance"
    );
    at_point(&mut s, 2, 2, VehiclePoint::Cargo);
    s.inspect_point(2, 2, VehiclePoint::Cargo).unwrap();
    let n = s.world.latest_event_sequence;
    s.inspect_point(2, 2, VehiclePoint::Cargo).unwrap();
    assert_eq!(s.world.latest_event_sequence, n);
    assert!(s.world.vehicles[1].secondary_inspected);
    assert!(!s.world.vehicles[1].observed_evidence.is_empty());
    assert!(
        s.public_case(2, 2, "offline").is_err(),
        "cargo position cannot start an interview"
    );
    for point in [VehiclePoint::Passenger, VehiclePoint::Hood] {
        at_point(&mut s, 2, 2, point);
        s.inspect_point(2, 2, point).unwrap();
    }
    s.world.vehicles[1].yaw = 1.1;
    at_point(&mut s, 2, 2, VehiclePoint::Driver);
    assert!(s.public_case(2, 2, "offline").is_ok());
    assert!(
        s.inspect_point(2, 4, VehiclePoint::Cargo).is_err(),
        "queued vehicles cannot be inspected"
    );
}
#[test]
fn all_ten_mixed_vehicles_finish_both_departure_routes_without_stalling() {
    let mut s = active();
    let mut seen_exit_open = [false; 2];
    for _ in 0..6000 {
        s.tick(s.world.server_time_ms + 50);
        for l in &s.world.lanes {
            seen_exit_open[l.lane_id as usize - 1] |= l.barrier_open;
        }
        let stopped: Vec<_> = s
            .world
            .vehicles
            .iter()
            .filter(|v| v.stage == VehicleStage::Stopped)
            .map(|v| (v.vehicle_id, v.lane_id as u64))
            .collect();
        for (id, peer) in stopped {
            at_point(&mut s, peer, id, VehiclePoint::Driver);
            s.decide_case(
                peer,
                id,
                if id % 2 == 0 {
                    Disposition::Deny
                } else {
                    Disposition::Admit
                },
            )
            .unwrap();
            s.release_case(peer, id).unwrap();
            // Officers move clear of departure traffic after releasing the vehicle.
            s.world.peers[peer as usize - 1].position = map::SPAWNS[peer as usize - 1];
        }
    }
    assert_eq!(s.world.ledger.len(), 10);
    assert_eq!(seen_exit_open, [true, true]);
    for v in &s.world.vehicles {
        assert_eq!(
            v.stage,
            VehicleStage::Exited,
            "stalled {} at {:?}",
            v.vehicle_id,
            v.position
        );
        let end = *vehicle::departure_path(v.lane_id, v.vehicle_id % 2 != 0)
            .last()
            .unwrap();
        assert!((v.position[0] - end[0]).hypot(v.position[2] - end[1]) < 0.5);
    }
    assert!(
        s.world
            .lanes
            .iter()
            .all(|l| l.queue.is_empty() && !l.barrier_open && !l.entry_barrier_open)
    );
}
#[test]
fn truck_collision_and_yield_use_the_full_body_length() {
    let mut s = active();
    // Stand beyond the sedan's footprint but inside the pickup's front safety envelope.
    s.world.peers[1].position = [7., 1.7, 13.8];
    run(&mut s, 3);
    let truck = &s.world.vehicles[1];
    assert_eq!(truck.position[2], 18.);
    assert_eq!(truck.speed, 0.);
    s.world.peers[1].position = map::SPAWNS[1];
    run(&mut s, 15);
    assert_eq!(s.world.vehicles[1].stage, VehicleStage::Stopped);
}
