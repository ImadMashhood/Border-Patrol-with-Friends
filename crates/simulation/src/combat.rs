//! Host-owned swept projectiles, articulated vehicle access and physical traveler state.
use super::*;
use border_net::combat::*;
#[derive(Clone, Copy)]
enum Hit {
    World,
    Body(usize),
    Glass(usize, Pane, [f32; 2]),
    Npc(usize, bool),
    Officer(usize),
    Ragdoll(u64, u8),
}
impl Simulation {
    pub(super) fn fire(&mut self, peer: u64, direction: V3) -> Result<()> {
        ensure!(
            self.world.phase != ShiftPhase::Lobby,
            "weapons are holstered in the office lobby"
        );
        ensure!(
            direction.iter().all(|v| v.is_finite()) && (length(direction) - 1.).abs() < 0.03,
            "invalid firing direction"
        );
        ensure!(
            self.world.projectiles.len() < 128,
            "too many active projectiles"
        );
        let p = self
            .world
            .peers
            .iter_mut()
            .find(|p| p.peer_id == peer)
            .unwrap();
        let w = &mut p.weapon;
        let now = self.world.server_time_ms;
        ensure!(
            w.drawn && w.reload_until.is_none(),
            "draw the pistol and finish reloading first"
        );
        ensure!(w.magazine > 0, "magazine empty — press R to reload");
        ensure!(
            w.last_fired_ms
                .is_none_or(|t| now.saturating_sub(t) >= FIRE_INTERVAL_MS),
            "weapon cycling"
        );
        let direction = mul(direction, 1. / length(direction));
        w.magazine -= 1;
        w.last_fired_ms = Some(now);
        let origin = p.position;
        self.world.projectiles.push(Projectile {
            id: self.next_projectile,
            shooter: peer,
            position: origin,
            velocity: mul(direction, PROJECTILE_SPEED),
            energy: 1.,
            born_ms: now,
            origin,
        });
        self.next_projectile += 1;
        self.emit(
            "weapon_fired",
            json!({"peer_id":peer,"projectile":self.next_projectile-1}),
        );
        Ok(())
    }
    pub(super) fn reload(&mut self, peer: u64) -> Result<()> {
        let p = self
            .world
            .peers
            .iter_mut()
            .find(|p| p.peer_id == peer)
            .unwrap();
        let w = &mut p.weapon;
        ensure!(
            w.drawn && w.magazine < MAGAZINE && w.reserve > 0 && w.reload_until.is_none(),
            "cannot reload now"
        );
        w.reload_until = Some(self.world.server_time_ms + RELOAD_MS);
        Ok(())
    }
    fn physical_vehicle(&self, peer: u64, id: u64, side: Side) -> Result<usize> {
        ensure!(self.world.phase != ShiftPhase::Lobby, "shift inactive");
        let i = self
            .world
            .vehicles
            .iter()
            .position(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown vehicle"))?;
        let v = &self.world.vehicles[i];
        ensure!(
            matches!(
                v.stage,
                VehicleStage::Stopped | VehicleStage::Resolved | VehicleStage::Disabled
            ),
            "vehicle must be stationary"
        );
        let p = self
            .world
            .peers
            .iter()
            .find(|p| p.peer_id == peer && p.connected && p.health > 0)
            .unwrap();
        let point = vehicle::spec(&v.kind).world_point(
            v.position,
            v.yaw,
            if side == Side::Driver {
                vehicle::VehiclePoint::Driver
            } else {
                vehicle::VehiclePoint::Passenger
            },
        );
        ensure!(
            (p.position[0] - point[0]).hypot(p.position[2] - point[2]) <= 2.8,
            "approach this vehicle door first"
        );
        Ok(i)
    }
    pub(super) fn toggle_door(&mut self, peer: u64, id: u64, side: Side) -> Result<()> {
        let i = self.physical_vehicle(peer, id, side)?;
        let p = &mut self.world.vehicles[i].physics;
        ensure!(
            !matches!(p.npc.mode, NpcMode::Exiting | NpcMode::Entering),
            "traveler is using the doorway"
        );
        let index = side_index(side);
        p.door_open[index] = !p.door_open[index];
        self.emit("vehicle_door_toggled", json!({"vehicle_id":id,"side":side}));
        Ok(())
    }
    pub(super) fn toggle_window(&mut self, peer: u64, id: u64, pane: Pane) -> Result<()> {
        let side = pane
            .side()
            .ok_or_else(|| anyhow::anyhow!("windshields do not roll down"))?;
        let i = self.physical_vehicle(peer, id, side)?;
        let state = &mut self.world.vehicles[i].physics.glass[pane.index()];
        ensure!(!state.shattered, "window glass has been destroyed");
        state.target_down = !state.target_down;
        self.emit(
            "vehicle_window_toggled",
            json!({"vehicle_id":id,"pane":pane}),
        );
        Ok(())
    }
    pub(super) fn order_exit(&mut self, peer: u64, id: u64) -> Result<()> {
        let i = self.physical_vehicle(peer, id, Side::Driver)?;
        self.owns_vehicle(peer, id)?;
        let v = &mut self.world.vehicles[i];
        ensure!(
            v.physics.npc.health > 0 && v.physics.npc.mode == NpcMode::Seated,
            "traveler cannot exit now"
        );
        v.physics.door_open[0] = true;
        v.physics.npc.mode = NpcMode::Exiting;
        v.speed = 0.;
        self.emit("traveler_exiting", json!({"vehicle_id":id}));
        Ok(())
    }
    pub(super) fn order_enter(&mut self, peer: u64, id: u64) -> Result<()> {
        let i = self.physical_vehicle(peer, id, Side::Driver)?;
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == self.world.vehicles[i].lane_id && l.owner == Some(peer))
                || self
                    .world
                    .peers
                    .iter()
                    .any(|p| p.peer_id == peer
                        && p.assisting == Some(self.world.vehicles[i].lane_id)),
            "claim or assist this lane first"
        );
        let v = &mut self.world.vehicles[i];
        ensure!(
            v.physics.npc.health > 0 && v.physics.npc.mode == NpcMode::Outside,
            "traveler cannot return now"
        );
        v.physics.door_open[0] = true;
        v.physics.npc.mode = NpcMode::Entering;
        self.emit("traveler_returning", json!({"vehicle_id":id}));
        Ok(())
    }
    pub(super) fn pull_body(&mut self, peer: u64, id: u64) -> Result<()> {
        let i = self.physical_vehicle(peer, id, Side::Driver)?;
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == self.world.vehicles[i].lane_id && l.owner == Some(peer))
                || self
                    .world
                    .peers
                    .iter()
                    .any(|p| p.peer_id == peer
                        && p.assisting == Some(self.world.vehicles[i].lane_id)),
            "claim or assist this lane first"
        );
        let v = &mut self.world.vehicles[i];
        ensure!(
            v.physics.npc.health == 0 && v.physics.npc.died_seated && !v.physics.npc.extracted,
            "no seated body to remove"
        );
        v.physics.door_open[0] = true;
        v.physics.glass[Pane::DriverFront.index()].target_down = true;
        ensure!(
            self.physics.start_pull(id),
            "body is already being pulled out"
        );
        self.world.vehicles[i].physics.npc.body_pull_active = true;
        self.emit(
            "body_extraction_started",
            json!({"vehicle_id":id,"peer_id":peer}),
        );
        Ok(())
    }
    pub(super) fn tow_vehicle(&mut self, peer: u64, id: u64) -> Result<()> {
        let i = self.physical_vehicle(peer, id, Side::Driver)?;
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == self.world.vehicles[i].lane_id && l.owner == Some(peer)),
            "only the lane owner can request recovery"
        );
        let v = &mut self.world.vehicles[i];
        ensure!(
            v.physics.npc.health == 0 && v.physics.tow_until.is_none(),
            "recovery is for disabled vehicles"
        );
        v.physics.tow_until = Some(self.world.server_time_ms + 5000);
        self.emit("vehicle_recovery_requested", json!({"vehicle_id":id}));
        Ok(())
    }
    pub(super) fn tick_combat(&mut self, dt: f32) {
        let now = self.world.server_time_ms;
        for p in &mut self.world.peers {
            if p.weapon.reload_until.is_some_and(|t| now >= t) {
                let n = (MAGAZINE - p.weapon.magazine).min(p.weapon.reserve as u8);
                p.weapon.magazine += n;
                p.weapon.reserve -= n as u16;
                p.weapon.reload_until = None;
            }
        }
        for v in &mut self.world.vehicles {
            for g in &mut v.physics.glass {
                let target = if g.target_down { 1. } else { 0. };
                g.lowered += (target - g.lowered).clamp(-dt * 1.5, dt * 1.5);
            }
            for i in 0..2 {
                let target = if v.physics.door_open[i] { 1.15 } else { 0. };
                v.physics.door_angles[i] +=
                    (target - v.physics.door_angles[i]).clamp(-dt * 2., dt * 2.);
            }
            match v.physics.npc.mode {
                NpcMode::Seated => {
                    v.physics.npc.position = seated_position(v);
                    v.physics.npc.yaw = v.yaw;
                }
                NpcMode::Exiting | NpcMode::Entering => {
                    let entering = v.physics.npc.mode == NpcMode::Entering;
                    if v.physics.door_angles[0] > 0.8 {
                        v.physics.npc.exit_progress = (v.physics.npc.exit_progress
                            + if entering { -dt / 1.7 } else { dt / 1.7 })
                        .clamp(0., 1.);
                    }

                    let t = v.physics.npc.exit_progress;
                    let start = seated_position(v);
                    let end = world(
                        v,
                        [
                            -vehicle::spec(&v.kind).half_width - 1.25,
                            0.,
                            vehicle::spec(&v.kind).window_z + 0.75,
                        ],
                    );
                    v.physics.npc.position = add(mul(start, 1. - t), mul(end, t));
                    v.physics.npc.yaw = v.yaw - 1.57;
                    if entering && t <= 0. {
                        v.physics.npc.mode = NpcMode::Seated;
                        v.physics.door_open[0] = false;
                    } else if !entering && t >= 1. {
                        v.physics.npc.mode = NpcMode::Outside;
                    }
                }
                _ => {}
            }
            if v.physics.tow_until.is_some_and(|t| now >= t) {
                v.stage = VehicleStage::Exited;
                v.physics.tow_until = None;
                for lane in &mut self.world.lanes {
                    lane.queue.retain(|id| *id != v.vehicle_id);
                }
            }
        }
        self.world
            .impacts
            .retain(|i| now.saturating_sub(i.at_ms) < 3000);
        let mut shots = std::mem::take(&mut self.world.projectiles);
        for mut shot in shots.drain(..) {
            if now.saturating_sub(shot.born_ms) > 4000 {
                continue;
            }
            let steps = (dt / 0.01).ceil().max(1.) as usize;
            let sub_dt = dt / steps as f32;
            let mut alive = true;
            for _ in 0..steps {
                if !alive {
                    break;
                }
                // A single step can penetrate more than one pane before reaching the occupant/body.
                let mut travel = length(shot.velocity) * sub_dt;
                shot.velocity[1] -= 2.0 * sub_dt;
                let direction = mul(shot.velocity, 1. / length(shot.velocity));
                for _ in 0..6 {
                    if travel <= 0.0001 || !alive {
                        break;
                    }
                    if let Some((distance, hit, normal)) =
                        self.projectile_hit(&shot, direction, travel)
                    {
                        shot.position = add(shot.position, mul(direction, distance));
                        let (kind, vehicle_id) = match hit {
                            Hit::Glass(i, pane, uv) => {
                                let v = &mut self.world.vehicles[i];
                                let g = &mut v.physics.glass[pane.index()];
                                g.impacts.push(GlassImpact {
                                    uv,
                                    at_ms: now,
                                    seed: shot.id.wrapping_mul(9176) + pane.index() as u64,
                                });
                                if g.impacts.len() >= 2 || pane.side().is_some() {
                                    g.shattered = true;
                                }
                                shot.energy *= 0.7;
                                (
                                    if g.shattered {
                                        "glass_shatter"
                                    } else {
                                        "glass_crack"
                                    },
                                    Some(v.vehicle_id),
                                )
                            }
                            Hit::Npc(i, head) => {
                                let v = &mut self.world.vehicles[i];
                                let damage = if head { 110. } else { 55. } * shot.energy;
                                v.physics.npc.health =
                                    v.physics.npc.health.saturating_sub(damage.round() as u8);
                                if v.physics.npc.health == 0 {
                                    v.physics.npc.death_impulse = mul(direction, shot.energy * 2.);
                                    v.physics.npc.died_seated =
                                        v.physics.npc.mode == NpcMode::Seated;
                                    v.physics.npc.mode = NpcMode::Dead;
                                    v.physics.disabled_at = Some(now);
                                    v.stage = VehicleStage::Disabled;
                                    v.speed = 0.;
                                }
                                alive = false;
                                ("npc_hit", Some(v.vehicle_id))
                            }
                            Hit::Ragdoll(id, bone) => {
                                self.physics
                                    .impulse(id, bone, mul(direction, shot.energy * 3.));
                                alive = false;
                                ("ragdoll_hit", Some(id))
                            }
                            Hit::Officer(i) => {
                                self.world.peers[i].health = self.world.peers[i]
                                    .health
                                    .saturating_sub((45. * shot.energy) as u8);
                                alive = false;
                                ("officer_hit", None)
                            }
                            Hit::Body(i) => {
                                alive = false;
                                ("metal", Some(self.world.vehicles[i].vehicle_id))
                            }
                            Hit::World => {
                                alive = false;
                                ("world", None)
                            }
                        };
                        self.world.impacts.push(Impact {
                            id: self.next_impact,
                            projectile: shot.id,
                            position: shot.position,
                            normal,
                            kind: kind.into(),
                            vehicle_id,
                            at_ms: now,
                            origin: shot.origin,
                        });
                        self.next_impact += 1;
                        if self.world.impacts.len() > 128 {
                            self.world.impacts.remove(0);
                        }
                        if matches!(hit, Hit::Npc(..)) {
                            self.emit(
                                "traveler_damaged",
                                json!({"vehicle_id":vehicle_id,"shooter":shot.shooter}),
                            );
                        }
                        travel -= (distance + 0.04).min(travel);
                        shot.position = add(shot.position, mul(direction, 0.04));
                    } else {
                        shot.position = add(shot.position, mul(direction, travel));
                        break;
                    }
                }
            }
            if alive {
                self.world.projectiles.push(shot);
            }
        }
    }
    fn projectile_hit(&self, shot: &Projectile, direction: V3, max: f32) -> Option<(f32, Hit, V3)> {
        let mut best: Option<(f32, Hit, V3)> = None;
        let mut offer = |d: f32, hit: Hit, normal: V3| {
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, hit, normal));
            }
        };
        if direction[1] < 0. && shot.position[1] >= 0. {
            let d = -shot.position[1] / direction[1];
            if d <= max {
                offer(d, Hit::World, [0., 1., 0.]);
            }
        }
        for a in &self.collision {
            if let Some(d) = ray_box(shot.position, direction, a.center, a.half, max) {
                offer(d, Hit::World, mul(direction, -1.));
            }
        }
        if let Some((distance, id, bone)) = self.physics.ray_dolls(shot.position, direction, max) {
            offer(distance, Hit::Ragdoll(id, bone), mul(direction, -1.));
        }
        for (i, p) in self
            .world
            .peers
            .iter()
            .enumerate()
            .filter(|(_, p)| p.connected && p.health > 0 && p.peer_id != shot.shooter)
        {
            if let Some(d) = ray_box(
                shot.position,
                direction,
                [p.position[0], 0.95, p.position[2]],
                [0.28, 0.85, 0.28],
                max,
            ) {
                offer(d, Hit::Officer(i), mul(direction, -1.));
            }
        }
        for (i, v) in self
            .world
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| v.stage != VehicleStage::Exited)
        {
            for g in &v.physics.glass {
                if !g.shattered
                    && let Some(frame) = pane_frame(v, g.pane)
                    && let Some((d, uv)) = pane_hit(shot.position, direction, max, frame)
                {
                    // Existing windshield holes transmit later shots instead of an invisible whole-pane collider.
                    let through = glass_hole(uv, &g.impacts);
                    if !through {
                        offer(d, Hit::Glass(i, g.pane, uv), frame.normal);
                    }
                }
            }
            let origin = local(v, shot.position);
            let dir = rotate(direction, -v.yaw);
            for (center, half) in shell_boxes(v) {
                if let Some(d) = ray_box(origin, dir, center, half, max) {
                    offer(d, Hit::Body(i), mul(direction, -1.));
                }
            }
            for side in [Side::Driver, Side::Passenger] {
                let angle = v.physics.door_angles[side_index(side)]
                    * if side == Side::Driver { -1. } else { 1. };
                let pivot = door_pivot(&v.kind, side);
                let origin = add(pivot, rotate(sub(origin, pivot), -angle));
                let dir = rotate(dir, -angle);
                let len = 1.15;
                let center = add(
                    pivot,
                    [
                        0.,
                        if border_net::vehicle::pickup(&v.kind) {
                            1.05
                        } else {
                            0.75 + vehicle::cab_lift(&v.kind)
                        },
                        len * 0.5,
                    ],
                );
                if let Some(d) = ray_box(origin, dir, center, [0.045, 0.25, len * 0.5], max) {
                    offer(d, Hit::Body(i), mul(direction, -1.));
                }
                let belt = if border_net::vehicle::pickup(&v.kind) {
                    1.3
                } else {
                    0.96
                };
                let top = if border_net::vehicle::pickup(&v.kind) {
                    2.1
                } else {
                    1.61
                };
                for (offset, half) in [
                    (
                        [0., (belt + top) * 0.5, 0.02],
                        [0.035, (top - belt) * 0.5, 0.035],
                    ),
                    (
                        [0., (belt + top) * 0.5, len - 0.02],
                        [0.035, (top - belt) * 0.5, 0.035],
                    ),
                    ([0., top, len * 0.5], [0.035, 0.035, len * 0.5]),
                ] {
                    if let Some(d) = ray_box(origin, dir, add(pivot, offset), half, max) {
                        offer(d, Hit::Body(i), mul(direction, -1.));
                    }
                }
            }
            if v.physics.npc.health > 0 {
                let seated = v.physics.npc.mode == NpcMode::Seated;
                let base = v.physics.npc.position;
                for (head, offset, half) in [
                    (
                        true,
                        [0., if seated { 1.47 } else { 1.62 }, 0.],
                        [0.17, 0.19, 0.17],
                    ),
                    (
                        false,
                        [0., if seated { 1.02 } else { 1.03 }, 0.],
                        [0.25, 0.40, 0.22],
                    ),
                ] {
                    if let Some(d) = ray_box(shot.position, direction, add(base, offset), half, max)
                    {
                        offer(d, Hit::Npc(i, head), mul(direction, -1.));
                    }
                }
            }
        }
        best
    }
}
