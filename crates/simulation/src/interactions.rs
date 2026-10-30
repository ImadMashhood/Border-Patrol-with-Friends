use super::*;
use border_net::combat::{self as c, NpcMode, Pane};
impl Simulation {
    pub fn authorize_conversation(&self, peer: u64, id: u64) -> Result<usize> {
        let i = self
            .world
            .vehicles
            .iter()
            .position(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown traveler"))?;
        let v = &self.world.vehicles[i];
        ensure!(
            v.conversation_owner == Some(peer),
            "only the initiating officer can speak to this traveler"
        );
        let p = self
            .world
            .peers
            .iter()
            .find(|p| p.peer_id == peer && p.connected && p.health > 0)
            .ok_or_else(|| anyhow::anyhow!("officer unavailable"))?;
        ensure!(
            v.stage == VehicleStage::Stopped && v.physics.npc.health > 0,
            "traveler unavailable"
        );
        let target = if matches!(v.physics.npc.mode, NpcMode::Outside | NpcMode::Fleeing) {
            v.physics.npc.position
        } else {
            v.position
        };
        ensure!(
            (p.position[0] - target[0]).hypot(p.position[2] - target[2]) < 8.,
            "conversation out of range"
        );
        Ok(i)
    }
    pub fn end_conversation(&mut self, peer: u64, id: u64) -> Result<()> {
        let v = self
            .world
            .vehicles
            .iter_mut()
            .find(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown traveler"))?;
        ensure!(v.conversation_owner == Some(peer), "not your conversation");
        v.conversation_owner = None;
        self.cases.get_mut(&id).unwrap().conversation_epoch += 1;
        Ok(())
    }
    pub(crate) fn physical_context(&self, v: &VehicleView) -> border_scenarios::PhysicalContext {
        let near: Vec<_> = self
            .world
            .peers
            .iter()
            .filter(|p| {
                p.connected
                    && p.health > 0
                    && (p.position[0] - v.physics.npc.position[0])
                        .hypot(p.position[2] - v.physics.npc.position[2])
                        < 8.
            })
            .collect();
        let aimed = near.iter().any(|p| aiming_at(p, v));
        border_scenarios::PhysicalContext {
            officer_peer: v.conversation_owner,
            weapon_drawn: near.iter().any(|p| p.weapon.drawn),
            weapon_aimed: aimed,
            injured: v.physics.npc.health < 100,
            violence_nearby: self.world.impacts.iter().any(|i| {
                self.world.server_time_ms.saturating_sub(i.at_ms) < 8000
                    && c::length(c::sub(i.position, v.physics.npc.position)) < 8.
            }),
        }
    }
    pub fn register_utterance(&mut self, peer: u64, id: u64, q: &str) -> Result<()> {
        let i = self.authorize_conversation(peer, id)?;
        let severity = border_caller::threat_severity(q);
        let physical = self.physical_context(&self.world.vehicles[i]);
        let case = self.cases.get_mut(&id).unwrap();
        let key = (case.history.len(), q.to_string());
        let officer_aimed = self
            .world
            .peers
            .iter()
            .any(|p| p.peer_id == peer && aiming_at(p, &self.world.vehicles[i]));
        if (severity >= 65 || officer_aimed) && case.misconduct_turn.as_ref() != Some(&key) {
            case.misconduct_turn = Some(key);
            self.world.money -= 100;
            self.world.notices.push(ShiftNotice {id:self.world.latest_event_sequence+1,at_ms:self.world.server_time_ms,kind:"misconduct_complaint".into(),message:format!("Officer {peer}: threatening a traveler. Complaint recorded; shared misconduct penalty 100.")});
            self.emit("officer_misconduct",json!({"peer_id":peer,"vehicle_id":id,"penalty":100,"reason":if officer_aimed { "aimed weapon" } else { "credible verbal threat" }}));
        }
        // The actor may describe resistance. The host alone can start an actual escape.
        let mut scenario = self.cases[&id].scenario.clone();
        scenario.encounter = physical;
        let psychology = border_caller::dialogue_state(&scenario, &self.cases[&id].history, q);
        if psychology.fear > 80
            && scenario.character.psychology.aggression > 65
            && scenario.character.psychology.boldness > 65
        {
            self.cases.get_mut(&id).unwrap().flee_requested = true;
            let v = &mut self.world.vehicles[i];
            v.physics.door_open[0] = true;
            if v.physics.npc.mode == NpcMode::Seated {
                v.physics.npc.mode = NpcMode::Exiting;
                v.physics.npc.exit_progress = 0.;
            } else if v.physics.npc.mode == NpcMode::Outside {
                v.physics.npc.mode = NpcMode::Fleeing;
            }
        }
        Ok(())
    }
    pub fn delivery_failure(
        &mut self,
        peer: u64,
        id: u64,
        epoch: u64,
        turn: u64,
        q: &str,
    ) -> Result<border_caller::ActorReply> {
        let i = self.authorize_conversation(peer, id)?;
        let physical = self.physical_context(&self.world.vehicles[i]);
        let case = self.cases.get_mut(&id).unwrap();
        ensure!(
            case.conversation_epoch == epoch && case.history.len() as u64 == turn,
            "stale delivery failure"
        );
        let reply = border_caller::ActorReply {
            dialogue: "Excuse me?".into(),
            emotion: border_caller::Emotion::Confused,
            action: "none".into(),
            fact_ids: vec![],
            end_conversation: false,
        };
        case.history.push(border_caller::Turn {
            officer: q.into(),
            traveler: reply.dialogue.clone(),
            emotion: reply.emotion.clone(),
            physical,
        });
        self.world.vehicles[i].history = case.history.clone();
        self.emit(
            "dialogue_delivery_failure",
            json!({"vehicle_id":id,"fallback":"Excuse me?"}),
        );
        Ok(reply)
    }
    pub(crate) fn grab_body(&mut self, peer: u64, id: u64, bone: u8) -> Result<()> {
        self.release_bodies(peer);
        let i = self
            .world
            .vehicles
            .iter()
            .position(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown body"))?;
        let v = &self.world.vehicles[i];
        ensure!(
            v.physics.npc.health == 0 && v.physics.npc.body_holder.is_none(),
            "body unavailable"
        );
        let p = self
            .world
            .peers
            .iter()
            .find(|p| p.peer_id == peer && p.connected && p.health > 0)
            .ok_or_else(|| anyhow::anyhow!("officer unavailable"))?;
        let b = v
            .physics
            .npc
            .ragdoll
            .iter()
            .find(|b| b.bone == bone)
            .ok_or_else(|| anyhow::anyhow!("invalid body part"))?;
        ensure!(
            c::length(c::sub(p.position, b.position)) < 3.2,
            "approach the body"
        );
        ensure!(self.physics.start_grab(id, peer, bone), "body already held");
        self.world.vehicles[i].physics.npc.body_holder = Some(peer);
        self.world.vehicles[i].physics.npc.body_pull_active = true;
        self.world.vehicles[i].physics.door_open[0] = true;
        self.cases.get_mut(&id).unwrap().grab_started_ms = self.world.server_time_ms;
        Ok(())
    }
    pub(crate) fn release_bodies(&mut self, peer: u64) {
        let ids: Vec<_> = self
            .world
            .vehicles
            .iter()
            .filter(|v| v.physics.npc.body_holder == Some(peer))
            .map(|v| v.vehicle_id)
            .collect();
        for id in ids {
            self.physics.release_grab(id);
            let v = self
                .world
                .vehicles
                .iter_mut()
                .find(|v| v.vehicle_id == id)
                .unwrap();
            v.physics.npc.body_holder = None;
            v.physics.npc.body_pull_active = false;
        }
    }
    pub(crate) fn tick_interactions(&mut self, _dt: f32) {
        let mut end = vec![];
        let mut drop = vec![];
        for v in &mut self.world.vehicles {
            if let Some(peer) = v.conversation_owner {
                let active = self.world.peers.iter().any(|p| {
                    p.peer_id == peer
                        && p.connected
                        && p.health > 0
                        && (p.position[0] - v.physics.npc.position[0])
                            .hypot(p.position[2] - v.physics.npc.position[2])
                            < 8.
                });
                if !active || v.physics.npc.health == 0 || v.stage != VehicleStage::Stopped {
                    end.push((peer, v.vehicle_id));
                }
            }
            if let Some(peer) = v.physics.npc.body_holder {
                let alive = self
                    .world
                    .peers
                    .iter()
                    .any(|p| p.peer_id == peer && p.connected && p.health > 0);
                let fresh = self.peers.get(&peer).is_some_and(|p| {
                    self.world.server_time_ms.saturating_sub(p.input_at) < 500
                        && p.input.as_ref().is_some_and(|i| i.grab_held)
                });
                if !alive
                    || (!fresh
                        && self
                            .world
                            .server_time_ms
                            .saturating_sub(self.cases[&v.vehicle_id].grab_started_ms)
                            > 200)
                {
                    drop.push(peer);
                }
            }
            if v.physics.npc.health > 0
                && v.stage == VehicleStage::Stopped
                && v.physics.npc.mode == NpcMode::Seated
                && self.world.server_time_ms >= v.physics.manual_window_until
            {
                let window = vehicle::spec(&v.kind).world_point(
                    v.position,
                    v.yaw,
                    vehicle::VehiclePoint::Driver,
                );
                let approached = self.world.peers.iter().any(|p| {
                    p.connected
                        && p.health > 0
                        && (p.position[0] - window[0]).hypot(p.position[2] - window[2]) < 3.2
                });
                v.physics.glass[Pane::DriverFront.index()].target_down =
                    approached || v.conversation_owner.is_some();
            }
        }
        for (p, id) in end {
            let _ = self.end_conversation(p, id);
        }
        for p in drop {
            self.release_bodies(p);
        }
    }
    pub(crate) fn request_porter(&mut self, peer: u64, lane: u8) -> Result<()> {
        let p = self
            .world
            .peers
            .iter()
            .find(|p| p.peer_id == peer && p.connected && p.health > 0)
            .ok_or_else(|| anyhow::anyhow!("officer unavailable"))?;
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == lane && l.owner == Some(peer))
                || p.assisting == Some(lane),
            "claim or assist this lane"
        );
        let v = self
            .world
            .vehicles
            .iter()
            .find(|v| {
                v.lane_id == lane
                    && matches!(
                        v.stage,
                        VehicleStage::Stopped | VehicleStage::Disabled | VehicleStage::Resolved
                    )
                    && (v.physics.npc.health == 0
                        || matches!(v.physics.npc.mode, NpcMode::Outside | NpcMode::Fleeing))
            })
            .ok_or_else(|| anyhow::anyhow!("no driverless vehicle to clear"))?;
        let chair = porter_chair(lane);
        let driver =
            vehicle::spec(&v.kind).world_point(v.position, v.yaw, vehicle::VehiclePoint::Driver);
        ensure!(
            (p.position[0] - chair[0]).hypot(p.position[2] - chair[2]) < 3.
                || (p.position[0] - driver[0]).hypot(p.position[2] - driver[2]) < 3.5,
            "approach the porter or driver door"
        );
        let worker = self
            .world
            .porters
            .iter_mut()
            .find(|p| p.lane_id == lane)
            .unwrap();
        ensure!(
            worker.phase == PorterPhase::Seated,
            "porter already working"
        );
        worker.vehicle_id = Some(v.vehicle_id);
        worker.phase = PorterPhase::Approaching;
        worker.progress = 0.;
        Ok(())
    }
    pub(crate) fn tick_porters(&mut self, dt: f32) {
        for k in 0..self.world.porters.len() {
            let mut worker = self.world.porters[k].clone();
            let chair = porter_chair(worker.lane_id);
            if let Some(id) = worker.vehicle_id
                && let Some(i) = self.world.vehicles.iter().position(|v| v.vehicle_id == id)
            {
                let v = self.world.vehicles[i].clone();
                let door = vehicle::spec(&v.kind).world_point(
                    v.position,
                    v.yaw,
                    vehicle::VehiclePoint::Driver,
                );
                match worker.phase {
                    PorterPhase::Approaching => {
                        let target = if worker.progress < 1. {
                            [door[0], 0., -vehicle::body_half_length(&v.kind) - 0.9]
                        } else {
                            c::add(door, c::rotate([-0.85, -door[1], -1.2], v.yaw))
                        };
                        if self.walk_actor(
                            navigation::Walker::Porter(worker.lane_id),
                            &mut worker.position,
                            &mut worker.yaw,
                            target,
                            3.5,
                            dt,
                        ) {
                            if worker.progress < 1. {
                                worker.progress = 1.;
                            } else {
                                worker.phase = PorterPhase::RemovingBody;
                                worker.progress = 0.;
                            }
                        }
                    }
                    PorterPhase::RemovingBody => {
                        if v.physics.npc.body_holder.is_none() {
                            if v.physics.npc.health == 0
                                && v.physics.npc.died_seated
                                && !v.physics.npc.extracted
                            {
                                let _ = self.physics.start_pull(id);
                                self.world.vehicles[i].physics.door_open[0] = true;
                            } else {
                                worker.phase = PorterPhase::Entering;
                                worker.progress = 0.;
                                self.world.vehicles[i].physics.door_open[0] = true;
                            }
                        }
                    }
                    PorterPhase::Entering => {
                        if v.physics.npc.health > 0
                            && !matches!(v.physics.npc.mode, NpcMode::Outside | NpcMode::Fleeing)
                        {
                            worker.phase = PorterPhase::Returning;
                            worker.progress = 0.;
                            self.world.porters[k] = worker;
                            continue;
                        }
                        worker.progress += dt;
                        if worker.progress > 1.4 {
                            worker.phase = PorterPhase::Driving;
                            worker.progress = 0.;
                            let v = &mut self.world.vehicles[i];
                            v.stage = VehicleStage::Recovering;
                            v.physics.door_open = [false; 2];
                            for glass in &mut v.physics.glass {
                                glass.target_down = false;
                            }
                            v.conversation_owner = None;
                        }
                    }
                    PorterPhase::Driving => {
                        let parking_x = -35. - ((id - 1) % 5) as f32 * 5.5;
                        let parking_z = -23. - ((id - 1) / 5) as f32 * 8.;
                        let paths = [
                            [v.position[0], -10.],
                            [-4., -16.],
                            [parking_x, -16.],
                            [parking_x, parking_z],
                        ];
                        let target = paths[(worker.progress as usize).min(3)];
                        let dx = target[0] - v.position[0];
                        let dz = target[1] - v.position[2];
                        let len = dx.hypot(dz);
                        let v = &mut self.world.vehicles[i];
                        v.speed = (v.speed + dt * 2.).min(6.);
                        let blocked = self.world.peers.iter().any(|p| {
                            let px = p.position[0] - v.position[0];
                            let pz = p.position[2] - v.position[2];
                            let length = len.max(0.01);
                            let ahead = (dx * px + dz * pz) / length;
                            let side = (px * dz - pz * dx).abs() / length;
                            p.connected
                                && ahead > 0.
                                && ahead < vehicle::spec(&v.kind).half_length + 1.
                                && side < vehicle::spec(&v.kind).half_width + 0.35
                        });
                        if !blocked && len > 0.1 {
                            let step = (v.speed * dt).min(len) / len;
                            v.position[0] += dx * step;
                            v.position[2] += dz * step;
                            v.yaw = (-dx).atan2(-dz);
                        } else if blocked {
                            v.speed = 0.;
                        }
                        worker.position = v.position;
                        worker.yaw = v.yaw;
                        if len < 0.25 {
                            worker.progress += 1.;
                            if worker.progress >= 4. {
                                v.stage = VehicleStage::Detained;
                                v.speed = 0.;
                                self.world
                                    .lanes
                                    .iter_mut()
                                    .for_each(|l| l.queue.retain(|old| *old != id));
                                worker.phase = PorterPhase::Returning;
                                worker.progress = 0.;
                                worker.position[0] -= 1.6;
                            }
                        }
                    }
                    PorterPhase::Returning => {
                        let path = [[-35., 0., -18.], [-4., 0., -18.], [-4., 0., -7.], chair];
                        let target = path[(worker.progress as usize).min(3)];
                        if self.walk_actor(
                            navigation::Walker::Porter(worker.lane_id),
                            &mut worker.position,
                            &mut worker.yaw,
                            target,
                            3.5,
                            dt,
                        ) {
                            worker.progress += 1.;
                            if worker.progress >= 4. {
                                worker.phase = PorterPhase::Seated;
                                worker.position = chair;
                                worker.yaw = 0.;
                                self.walkers
                                    .remove(&navigation::Walker::Porter(worker.lane_id));
                                worker.vehicle_id = None;
                                worker.progress = 0.;
                            }
                        }
                    }
                    PorterPhase::Seated => {}
                }
            }
            self.world.porters[k] = worker;
        }
    }
}

fn aiming_at(p: &PeerView, v: &VehicleView) -> bool {
    let delta = c::sub(c::add(v.physics.npc.position, [0., 1.5, 0.]), p.position);
    let distance = c::length(delta).max(0.01);
    let direction = [
        -p.yaw.sin() * p.pitch.cos(),
        p.pitch.sin(),
        -p.yaw.cos() * p.pitch.cos(),
    ];
    p.connected
        && p.health > 0
        && p.weapon.drawn
        && distance < 8.
        && c::dot(direction, delta) / distance > 0.94
}
