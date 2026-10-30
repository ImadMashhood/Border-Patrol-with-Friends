use super::*;
use border_caller::PublicCase;
use border_scenarios::{Decision, Evidence};
impl Simulation {
    pub fn input(&mut self, id: u64, input: InputFrame, now: u64) -> Result<()> {
        self.check_session(input.protocol_version, &input.session_id)?;
        self.heartbeat(id, now)?;
        ensure!(
            input
                .movement
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1.)
                && input.yaw.is_finite(),
            "invalid movement"
        );
        let p = self.peers.get_mut(&id).unwrap();
        ensure!(input.frame > p.input_frame, "stale input frame");
        p.input_frame = input.frame;
        p.input = Some(input);
        p.input_at = now;
        Ok(())
    }
    fn qualified(&self) -> bool {
        let connected: Vec<_> = self.world.peers.iter().filter(|p| p.connected).collect();
        !connected.is_empty()
            && (connected
                .iter()
                .any(|p| Some(p.peer_id) == self.world.host_peer_id && p.ready && p.health > 0)
                || connected.iter().filter(|p| p.ready && p.health > 0).count()
                    > connected.len() / 2)
    }
    pub(super) fn start_shift(&mut self) -> Result<()> {
        ensure!(
            self.world.phase == ShiftPhase::Lobby,
            "shift already started"
        );
        ensure!(
            self.qualified(),
            "host or strict majority must complete the ready hold"
        );
        self.world.phase = ShiftPhase::Active;
        self.world.remaining_ms = SHIFT_DURATION_MS;
        self.world.shift_started_ms = Some(self.world.server_time_ms);
        self.world.door_open = true;
        self.world.queues_active = true;
        self.world.events_active = true;
        for p in &mut self.world.peers {
            if p.connected {
                p.health = 100;
            }
        }
        let mut assigned: Vec<u64> = self.world.lanes.iter().filter_map(|l| l.owner).collect();
        for lane in &mut self.world.lanes {
            if lane.owner.is_none() {
                lane.owner = self
                    .world
                    .peers
                    .iter()
                    .find(|p| p.connected && !assigned.contains(&p.peer_id))
                    .map(|p| p.peer_id);
                if let Some(id) = lane.owner {
                    assigned.push(id);
                }
            }
        }
        for i in 0..self.pack.len() {
            self.operator(HostCommand::SpawnNextVehicle {
                lane_id: (i % 2 + 1) as u8,
                kind: vehicle::scheduled_kind(i).into(),
            })?;
        }
        for v in &mut self.world.vehicles {
            let (p, yaw) = border_net::map::incoming(v.distance_to_stop, v.lane_id);
            v.position = p;
            v.yaw = yaw;
        }
        self.emit("shift_started",json!({"duration_ms":SHIFT_DURATION_MS,"queues_active":true,"security_event_hooks":true}));
        Ok(())
    }
    pub fn tick(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.world.server_time_ms);
        self.world.server_time_ms = now;
        let dt = (elapsed as f32 / 1000.).min(1.);
        let mut holds = vec![];
        for view in &mut self.world.peers {
            if !view.connected {
                continue;
            }
            let state = self.peers.get_mut(&view.peer_id).unwrap();
            let fresh = now.saturating_sub(state.input_at) <= 500;
            if let Some(input) = &state.input {
                if fresh && view.health > 0 {
                    let axis = input.movement;
                    let length = axis[0].hypot(axis[1]).max(1.);
                    view.yaw = input.yaw;
                    for axis_index in [0, 1] {
                        let mut next = view.position;
                        let coordinate = if axis_index == 0 { 0 } else { 2 };
                        next[coordinate] +=
                            axis[axis_index] / length * if input.jog { 5.8 } else { 3.2 } * dt;
                        let closed_door = border_net::map::Aabb {
                            center: map::DOOR,
                            half: [1.1, 1.5, 0.16],
                        };
                        if map::walkable(next[0], next[2])
                            && !self
                                .collision
                                .iter()
                                .any(|a| map::blocked(next[0], next[2], a))
                            && !self
                                .world
                                .vehicles
                                .iter()
                                .filter(|v| v.stage != VehicleStage::Exited)
                                .any(|v| {
                                    map::blocked(
                                        next[0],
                                        next[2],
                                        &vehicle::spec(&v.kind).aabb(v.position, v.yaw),
                                    )
                                })
                            && !self.world.lanes.iter().any(|lane| {
                                [
                                    (!lane.entry_barrier_open, vehicle::ENTRY_GATE_Z),
                                    (!lane.barrier_open, vehicle::EXIT_GATE_Z),
                                ]
                                .iter()
                                .any(|(closed, z)| {
                                    *closed
                                        && map::blocked(
                                            next[0],
                                            next[2],
                                            &map::Aabb {
                                                center: [(lane.lane_id - 1) as f32 * 7., 1.1, *z],
                                                half: [2.65, 0.1, 0.1],
                                            },
                                        )
                                })
                            })
                            && (self.world.door_open
                                || !map::blocked(next[0], next[2], &closed_door))
                        {
                            view.position = next;
                        }
                    }
                }
                if self.world.phase == ShiftPhase::Lobby && !view.ready {
                    if fresh
                        && input.ready_held
                        && map::door_distance(view.position) < 2.2
                        && view.health > 0
                    {
                        let start = *state.hold_started.get_or_insert(now);
                        view.hold_ms = now.saturating_sub(start).min(READY_HOLD_MS);
                        if view.hold_ms == READY_HOLD_MS {
                            view.ready = true;
                            holds.push(view.peer_id);
                        }
                    } else {
                        state.hold_started = None;
                        view.hold_ms = 0;
                    }
                }
            }
        }
        for id in holds {
            self.emit("officer_ready", json!({"peer_id":id}));
        }
        if self.world.phase == ShiftPhase::Lobby && self.qualified() {
            let _ = self.start_shift();
            return;
        }
        if self.world.phase == ShiftPhase::Active {
            self.world.remaining_ms = self.world.remaining_ms.saturating_sub(elapsed);
            if self.world.remaining_ms == 0 {
                self.world.phase = ShiftPhase::Grace;
                self.world.events_active = false;
                self.emit(
                    "shift_time_expired",
                    json!({"finish_current_vehicles":true}),
                );
            }
        }
        if self.world.queues_active {
            self.advance_vehicles(dt);
        }
        if elapsed > 0 {
            self.emit(
                "world_tick",
                json!({"server_time_ms":now,"remaining_ms":self.world.remaining_ms}),
            );
        }
        if self.world.events_active
            && self
                .world
                .shift_started_ms
                .is_some_and(|s| now.saturating_sub(s) >= self.next_notice_ms)
        {
            let kinds = [
                (
                    "perimeter_sensor",
                    "Perimeter sensor check queued for dispatch.",
                ),
                (
                    "queue_observation",
                    "Dispatch requests an extra look at the incoming queue.",
                ),
                ("radio_check", "Radio check: both lanes report status."),
            ];
            let index = (self.next_notice_ms as usize + self.world.seed as usize) % kinds.len();
            let (kind, message) = kinds[index];
            let id = self.world.notices.len() as u64 + 1;
            self.world.notices.push(ShiftNotice {
                id,
                at_ms: now,
                kind: kind.into(),
                message: message.into(),
            });
            self.next_notice_ms += 45_000 + self.world.seed.wrapping_mul(id + 1) % 30_000;
            self.emit("security_event_hook", json!({"id":id,"kind":kind}));
        }
    }
    fn advance_vehicles(&mut self, dt: f32) {
        let old = self.world.vehicles.clone();
        let departure_orders: HashMap<_, _> = self
            .cases
            .iter()
            .map(|(id, c)| (*id, c.departure_order))
            .collect();
        for v in &mut self.world.vehicles {
            match v.stage {
                VehicleStage::Queued => {
                    if self.world.phase != ShiftPhase::Active {
                        v.speed = 0.;
                        continue;
                    }
                    let lead = old.iter().rfind(|a| {
                        a.lane_id == v.lane_id
                            && a.vehicle_id < v.vehicle_id
                            && (matches!(
                                a.stage,
                                VehicleStage::Queued
                                    | VehicleStage::Stopped
                                    | VehicleStage::Resolved
                            ) || (matches!(
                                a.stage,
                                VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny
                            ) && (a.position[0] - v.position[0]).abs() < 3.
                                && a.position[2] > -10.))
                    });
                    let target = lead
                        .map(|a| {
                            let distance = if matches!(
                                a.stage,
                                VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny
                            ) {
                                a.position[2]
                            } else {
                                a.distance_to_stop
                            };
                            (distance
                                + vehicle::spec(&a.kind).half_length
                                + vehicle::spec(&v.kind).half_length
                                + vehicle::BUMPER_GAP)
                                .max(0.)
                        })
                        .unwrap_or(0.);
                    let desired = ((v.distance_to_stop - target).max(0.) * 3.6)
                        .sqrt()
                        .min(10.);
                    v.speed = (v.speed + (desired - v.speed).clamp(-4. * dt, 2. * dt)).max(0.);
                    let next_distance = (v.distance_to_stop - v.speed * dt).max(target);
                    let (next, _) = map::incoming(next_distance, v.lane_id);
                    let dx = next[0] - v.position[0];
                    let dz = next[2] - v.position[2];
                    let length = dx.hypot(dz);
                    let blocked = length > 0.0001
                        && self.world.peers.iter().filter(|p| p.connected).any(|p| {
                            let x = p.position[0] - v.position[0];
                            let z = p.position[2] - v.position[2];
                            let ahead = (x * dx + z * dz) / length;
                            let side = (x * dz - z * dx).abs() / length;
                            ahead > 0.
                                && ahead < vehicle::spec(&v.kind).half_length + 1.2
                                && side < vehicle::spec(&v.kind).half_width + 0.35
                        });
                    if blocked {
                        v.speed = 0.;
                    } else {
                        v.distance_to_stop = next_distance;
                    }

                    if v.distance_to_stop - target < 0.025 {
                        v.distance_to_stop = target;
                        v.speed = 0.;
                        if target == 0. {
                            v.stage = VehicleStage::Stopped;
                        }
                    }
                    let (p, yaw) = map::incoming(v.distance_to_stop, v.lane_id);
                    v.position = p;
                    v.yaw = yaw;
                    v.position_mm = ((18. - v.distance_to_stop).max(0.) / 18. * 10_000.) as u32;
                }
                VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny => {
                    let case = self.cases.get_mut(&v.vehicle_id).unwrap();
                    let admitted = v.stage == VehicleStage::DepartingAdmit;
                    let path = vehicle::departure_path(v.lane_id, admitted);
                    let target = path[case.waypoint.min(path.len() - 1)];
                    let dx = target[0] - v.position[0];
                    let dz = target[1] - v.position[2];
                    let remaining = dx.hypot(dz);
                    let blocked = remaining > 0.01
                        && self.world.peers.iter().filter(|p| p.connected).any(|p| {
                            let x = p.position[0] - v.position[0];
                            let z = p.position[2] - v.position[2];
                            let ahead = (x * dx + z * dz) / remaining;
                            let side = (x * dz - z * dx).abs() / remaining;
                            ahead > 0.
                                && ahead < vehicle::spec(&v.kind).half_length + 1.2
                                && side < vehicle::spec(&v.kind).half_width + 0.35
                        });
                    // Only the intersecting return merge is reserved; straight exits remain independent.
                    let merge_wait = v.position[2] > -7.
                        && old.iter().any(|a| {
                            a.vehicle_id != v.vehicle_id
                                && matches!(
                                    a.stage,
                                    VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny
                                )
                                && (v.stage == VehicleStage::DepartingDeny
                                    || a.stage == VehicleStage::DepartingDeny)
                                && (departure_orders[&a.vehicle_id], a.vehicle_id)
                                    < (departure_orders[&v.vehicle_id], v.vehicle_id)
                                && a.position[0] > -15.
                                && a.position[2] > -23.
                        });
                    v.speed = if blocked || merge_wait {
                        0.
                    } else {
                        (v.speed + dt * 2.).min(12.)
                    };
                    if remaining > 0.01 {
                        let step = (v.speed * dt).min(remaining);
                        v.position[0] += dx / remaining * step;
                        v.position[2] += dz / remaining * step;
                        v.yaw = (-dx).atan2(-dz);
                    }
                    if remaining < 0.4 {
                        if case.waypoint + 1 < path.len() {
                            case.waypoint += 1;
                        } else {
                            v.stage = VehicleStage::Exited;
                        }
                    }
                    if (v.position[2] < -19. || v.position[0] < -13.)
                        && let Some(l) =
                            self.world.lanes.iter_mut().find(|l| l.lane_id == v.lane_id)
                    {
                        l.queue.retain(|id| *id != v.vehicle_id);
                        l.barrier_open = false;
                    }
                }
                _ => {}
            }
        }
        for lane in &mut self.world.lanes {
            lane.entry_barrier_open = self.world.vehicles.iter().any(|v| {
                v.lane_id == lane.lane_id
                    && v.stage == VehicleStage::Queued
                    && (v.distance_to_stop - vehicle::ENTRY_GATE_Z).abs()
                        < vehicle::spec(&v.kind).half_length + 2.
            });
            lane.barrier_open = self.world.vehicles.iter().any(|v| {
                v.lane_id == lane.lane_id
                    && matches!(
                        v.stage,
                        VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny
                    )
                    && v.position[2] + vehicle::spec(&v.kind).half_length
                        > vehicle::EXIT_GATE_Z - 1.
            });
        }
    }

    pub fn authorize_case(&self, peer: u64, id: u64) -> Result<usize> {
        let i = self.owns_vehicle(peer, id)?;
        self.require_point(peer, i, vehicle::VehiclePoint::Driver)?;
        Ok(i)
    }
    fn require_point(&self, peer: u64, i: usize, point: vehicle::VehiclePoint) -> Result<()> {
        let p = self
            .world
            .peers
            .iter()
            .find(|p| p.peer_id == peer && p.connected && p.health > 0)
            .ok_or_else(|| anyhow::anyhow!("officer unavailable"))?;
        let v = &self.world.vehicles[i];
        let target = vehicle::spec(&v.kind).world_point(v.position, v.yaw, point);
        ensure!(
            (p.position[0] - target[0]).hypot(p.position[2] - target[2])
                <= vehicle::INTERACTION_RANGE,
            "approach the {} first",
            point.label(&v.kind)
        );
        Ok(())
    }
    pub fn inspect_point(
        &mut self,
        peer: u64,
        id: u64,
        point: vehicle::VehiclePoint,
    ) -> Result<()> {
        ensure!(
            point != vehicle::VehiclePoint::Driver,
            "use the driver window to talk"
        );
        let i = self.owns_vehicle(peer, id)?;
        self.require_point(peer, i, point)?;
        if !self.world.vehicles[i].inspected_points.contains(&point) {
            self.world.vehicles[i].inspected_points.push(point);
            if point == vehicle::VehiclePoint::Cargo {
                self.reveal_cargo(i, id);
            }
            self.emit(
                "vehicle_point_inspected",
                json!({"vehicle_id":id,"point":point}),
            );
        }
        Ok(())
    }
    fn reveal_cargo(&mut self, i: usize, id: u64) -> Vec<Evidence> {
        let c = self.cases.get_mut(&id).unwrap();
        c.secondary = true;
        let evidence = c.scenario.visible_evidence(true);
        self.world.vehicles[i].observed_evidence = evidence
            .iter()
            .map(|e| EvidenceView {
                id: e.id.clone(),
                description: e.description.clone(),
                reveal_condition: e.reveal_condition.clone(),
            })
            .collect();
        self.world.vehicles[i].secondary_inspected = true;
        evidence
    }
    pub fn public_case(&mut self, peer: u64, id: u64, mode: &str) -> Result<PublicCase> {
        self.authorize_case(peer, id)?;
        let c = &self.cases[&id];
        Ok(PublicCase {
            session_id: id.to_string(),
            scenario_id: c.scenario.scenario_id.clone(),
            name: c.scenario.character.name.clone(),
            occupation: c.scenario.character.occupation.clone(),
            opening_line: c.scenario.opening_line.clone(),
            evidence: c.scenario.visible_evidence(c.secondary),
            mode: mode.into(),
        })
    }
    pub fn inspect_case(&mut self, peer: u64, id: u64) -> Result<Vec<Evidence>> {
        let i = self.authorize_case(peer, id)?;
        let evidence = self.reveal_cargo(i, id);
        self.emit("evidence_inspected", json!({"vehicle_id":id}));
        Ok(evidence)
    }
    pub fn decide_case(
        &mut self,
        peer: u64,
        id: u64,
        disposition: Disposition,
    ) -> Result<border_scenarios::Reveal> {
        if let Some(v) = self.world.vehicles.iter().find(|v| v.vehicle_id == id)
            && let Some(outcome) = &v.outcome
        {
            ensure!(
                matches!(
                    (disposition, outcome.decision),
                    (Disposition::Admit, Decision::Admit) | (Disposition::Deny, Decision::Deny)
                ),
                "disposition already committed"
            );
            return Ok(outcome.clone());
        }
        let i = self.authorize_case(peer, id)?;
        let c = &self.cases[&id];
        let decision = if disposition == Disposition::Admit {
            Decision::Admit
        } else {
            Decision::Deny
        };
        let outcome = c.scenario.resolve(decision, c.secondary);
        let correct = outcome.correct;
        let amount = match (disposition, correct) {
            (Disposition::Admit, true) => 100,
            (Disposition::Admit, false) => -400,
            (Disposition::Deny, true) => 200,
            (Disposition::Detain, true) => 250,
            _ => -150,
        };
        self.world.money += amount;
        self.world.ledger.push(LedgerEntry {
            vehicle_id: id,
            disposition,
            correct,
            amount,
            reason: if correct {
                "correct disposition"
            } else {
                "incorrect disposition"
            }
            .into(),
        });
        self.world.vehicles[i].outcome = Some(outcome.clone());
        self.world.vehicles[i].stage = VehicleStage::Resolved;
        self.emit("case_resolved", json!({"vehicle_id":id,"amount":amount}));
        Ok(outcome)
    }
    pub(super) fn release_case(&mut self, peer: u64, id: u64) -> Result<()> {
        let v = self
            .world
            .vehicles
            .iter_mut()
            .find(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown vehicle"))?;
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == v.lane_id && l.owner == Some(peer))
                || self
                    .world
                    .peers
                    .iter()
                    .any(|p| p.peer_id == peer && p.assisting == Some(v.lane_id)),
            "lane not assigned"
        );
        if matches!(
            v.stage,
            VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny | VehicleStage::Exited
        ) {
            return Ok(());
        }
        ensure!(v.stage == VehicleStage::Resolved, "case unresolved");
        self.cases.get_mut(&id).unwrap().departure_order = self.world.latest_event_sequence + 1;
        v.stage = if v.outcome.as_ref().unwrap().decision == Decision::Admit {
            VehicleStage::DepartingAdmit
        } else {
            VehicleStage::DepartingDeny
        };
        self.world
            .lanes
            .iter_mut()
            .find(|l| l.lane_id == v.lane_id)
            .unwrap()
            .barrier_open = true;
        self.emit("vehicle_released", json!({"vehicle_id":id}));
        Ok(())
    }
    pub(super) fn reset_shift(&mut self, peer: u64) -> Result<()> {
        ensure!(
            self.world.host_peer_id == Some(peer),
            "only the host can start another shift"
        );
        ensure!(
            self.world.phase == ShiftPhase::Grace || self.world.ledger.len() == self.pack.len(),
            "shift still active"
        );
        self.epoch += 1;
        self.world.phase = ShiftPhase::Lobby;
        self.world.remaining_ms = SHIFT_DURATION_MS;
        self.world.shift_started_ms = None;
        self.world.door_open = false;
        self.world.queues_active = false;
        self.world.events_active = false;
        self.world.vehicles.clear();
        self.world.ledger.clear();
        self.world.notices.clear();
        self.world.lane_swap = None;
        self.cases.clear();
        self.cursor = 0;
        self.next_notice_ms = 45_000 + self.world.seed % 30_000;
        for (i, p) in self
            .world
            .peers
            .iter_mut()
            .filter(|p| p.connected)
            .enumerate()
        {
            p.position = map::SPAWNS[i % 2];
            p.health = 100;
            p.ready = false;
            p.hold_ms = 0;
            p.assisting = None;
            let state = self.peers.get_mut(&p.peer_id).unwrap();
            state.hold_started = None;
            state.input = None;
        }
        for l in &mut self.world.lanes {
            l.queue.clear();
            l.barrier_open = false;
            l.entry_barrier_open = false;
        }
        self.emit("shift_reset", json!({"door_open":false}));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lobby() -> Simulation {
        let mut s = Simulation::new(
            "ready-test".into(),
            42,
            border_scenarios::load_pack(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios"),
            )
            .unwrap(),
        )
        .unwrap();
        s.join("Host".into(), 0).unwrap();
        s.join("Guest".into(), 0).unwrap();
        s
    }
    fn pulse(s: &mut Simulation, id: u64, now: u64, held: bool, movement: [f32; 2]) {
        let frame = s.peers[&id]
            .input
            .as_ref()
            .map(|i| i.frame + 1)
            .unwrap_or(1);
        s.input(
            id,
            InputFrame {
                protocol_version: PROTOCOL_VERSION,
                session_id: "ready-test".into(),
                frame,
                movement,
                yaw: std::f32::consts::PI,
                jog: false,
                ready_held: held,
            },
            now,
        )
        .unwrap();
        s.tick(now);
    }
    #[test]
    fn hold_requires_presence_duration_and_continuous_fresh_input() {
        let mut s = lobby();
        for now in (100..=3500).step_by(100) {
            pulse(&mut s, 1, now, true, [0., 0.]);
        }
        assert_eq!(s.world.phase, ShiftPhase::Lobby);
        assert_eq!(s.world.peers[0].hold_ms, 0);
        s.world.peers[0].position = map::READY_TARGET;
        for now in (3600..=5000).step_by(100) {
            pulse(&mut s, 1, now, true, [0., 0.]);
        }
        assert!(s.world.peers[0].hold_ms < READY_HOLD_MS);
        pulse(&mut s, 1, 5100, false, [0., 0.]);
        assert_eq!(s.world.peers[0].hold_ms, 0);
        pulse(&mut s, 1, 5200, true, [0., 0.]);
        s.tick(5900);
        assert_eq!(s.world.peers[0].hold_ms, 0);
        assert_eq!(s.world.phase, ShiftPhase::Lobby);
        assert_eq!(s.world.remaining_ms, SHIFT_DURATION_MS);
        assert!(s.world.vehicles.is_empty());
    }
    #[test]
    fn guest_vote_alone_cannot_start_but_host_hold_can() {
        let mut s = lobby();
        s.world.peers[1].position = map::READY_TARGET;
        for now in (0..=3100).step_by(100) {
            pulse(&mut s, 2, now, true, [0., 0.]);
        }
        assert!(s.world.peers[1].ready);
        assert_eq!(s.world.phase, ShiftPhase::Lobby);
        s.world.peers[1].ready = false;
        s.peers.get_mut(&2).unwrap().hold_started = None;
        s.world.peers[0].position = map::READY_TARGET;
        for now in (3200..=6200).step_by(100) {
            pulse(&mut s, 1, now, true, [0., 0.]);
        }
        assert_eq!(s.world.phase, ShiftPhase::Active);
        assert!(s.world.door_open && s.world.queues_active && s.world.events_active);
        assert_eq!(s.world.vehicles.len(), 10);
        assert_eq!(s.world.peers.iter().filter(|p| p.ready).count(), 1);
        assert_eq!(s.world.remaining_ms, SHIFT_DURATION_MS);
        assert_eq!(s.world.lanes.len(), 2);
    }
    #[test]
    fn closed_door_blocks_walking_and_clock_is_exactly_host_elapsed_time() {
        let mut s = lobby();
        s.world.peers[0].position = map::READY_TARGET;
        for now in (0..=1800).step_by(100) {
            pulse(&mut s, 1, now, false, [0., 1.]);
        }
        assert!(s.world.peers[0].position[2] < -3.8);
        s.world.peers[0].position = map::READY_TARGET;
        for now in (1900..=4900).step_by(100) {
            pulse(&mut s, 1, now, true, [0., 0.]);
        }
        let start = s.world.shift_started_ms.unwrap();
        pulse(&mut s, 1, start + 100, false, [0., 1.]);
        s.tick(start + 1100);
        assert_eq!(s.world.remaining_ms, SHIFT_DURATION_MS - 1100);
        s.tick(start + SHIFT_DURATION_MS);
        assert_eq!(s.world.remaining_ms, 0);
        assert_eq!(s.world.phase, ShiftPhase::Grace);
        assert!(!s.world.events_active);
        s.tick(start + SHIFT_DURATION_MS + 5000);
        assert_eq!(s.world.remaining_ms, 0);
    }
    #[test]
    fn security_hooks_start_with_shift_and_reset_returns_equipped_officers_to_office() {
        let mut s = lobby();
        s.tick(120_000);
        assert!(s.world.notices.is_empty());
        s.world.peers[0].position = map::READY_TARGET;
        for now in (120_100..=123_100).step_by(100) {
            pulse(&mut s, 1, now, true, [0., 0.]);
        }
        s.tick(190_000);
        assert!(!s.world.notices.is_empty());
        s.tick(800_000);
        assert!(s.reset_shift(2).is_err());
        s.reset_shift(1).unwrap();
        assert_eq!(s.world.phase, ShiftPhase::Lobby);
        assert!(!s.world.door_open);
        for (i, p) in s.world.peers.iter().enumerate() {
            assert_eq!(p.position, map::SPAWNS[i]);
            assert_eq!(p.health, 100);
            assert_eq!(p.equipment.len(), 3);
        }
        assert!(s.world.vehicles.is_empty());
    }
    #[test]
    fn lane_swaps_require_other_officers_acceptance() {
        let mut s = lobby();
        s.world.lanes[0].owner = Some(1);
        s.world.lanes[1].owner = Some(2);
        let command = |s: &mut Simulation, id, command| {
            let n = s.peers[&id].next_sequence;
            s.command(
                id,
                CommandEnvelope {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: "ready-test".into(),
                    request_id: format!("{id}-{n}"),
                    sequence: n,
                    expected_revision: None,
                    command,
                },
                0,
            )
        };
        command(&mut s, 1, ClientCommand::RequestLaneSwap).unwrap();
        assert_eq!(s.world.lanes[0].owner, Some(1));
        assert!(command(&mut s, 1, ClientCommand::AcceptLaneSwap).is_err());
        command(&mut s, 2, ClientCommand::AcceptLaneSwap).unwrap();
        assert_eq!(s.world.lanes[0].owner, Some(2));
        assert_eq!(s.world.lanes[1].owner, Some(1));
        command(&mut s, 1, ClientCommand::AssistLane { lane_id: Some(1) }).unwrap();
        assert_eq!(s.world.peers[0].assisting, Some(1));
    }
}
