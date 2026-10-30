//! Authoritative Rapier rigid bodies. Joint layout adapted from Dimforge's Apache-2.0
//! examples3d/stress_tests/ragdolls3.rs; see docs/THIRD_PARTY.md and license notices.
use border_net::{
    VehicleStage, VehicleView, WorldSnapshot,
    combat::{self as c, *},
};
use rapier3d::na::{Matrix3, Rotation3, UnitQuaternion};
use rapier3d::prelude::*;
use std::collections::{BTreeMap, HashSet};
struct Doll {
    parts: Vec<RigidBodyHandle>,
    belt: Option<ImpulseJointHandle>,
    grab: Option<(RigidBodyHandle, ImpulseJointHandle)>,
    pulling: bool,
}
struct Fragment {
    handle: RigidBodyHandle,
    id: u64,
    born: u64,
    size: [f32; 3],
}
pub struct PhysicalWorld {
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad: BroadPhaseBvh,
    narrow: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: ImpulseJointSet,
    multibody: MultibodyJointSet,
    ccd: CCDSolver,
    vehicles: BTreeMap<u64, RigidBodyHandle>,
    surfaces: BTreeMap<u64, (Vec<ColliderHandle>, Vec<f32>)>,
    dolls: BTreeMap<u64, Doll>,
    fragments: Vec<Fragment>,
    seen: HashSet<u64>,
    next_shard: u64,
    accumulator: f32,
}
fn vec(v: V3) -> Vector<Real> {
    vector![v[0], v[1], v[2]]
}
fn pose(position: V3, yaw: f32) -> Isometry<Real> {
    Isometry::new(vec(position), vector![0., yaw, 0.])
}
impl PhysicalWorld {
    pub fn new() -> Self {
        let mut s = Self {
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad: BroadPhaseBvh::new(),
            narrow: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: ImpulseJointSet::new(),
            multibody: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            vehicles: BTreeMap::new(),
            surfaces: BTreeMap::new(),
            dolls: BTreeMap::new(),
            fragments: vec![],
            seen: HashSet::new(),
            next_shard: 1,
            accumulator: 0.,
        };
        s.colliders.insert(
            ColliderBuilder::cuboid(1000., 0.2, 1000.)
                .translation(vector![0., -0.2, 0.])
                .friction(0.8)
                .build(),
        );
        for a in border_net::map::collision().solids {
            s.colliders.insert(
                ColliderBuilder::cuboid(a.half[0], a.half[1], a.half[2])
                    .translation(vec(a.center))
                    .build(),
            );
        }
        s
    }
    fn remove_body(&mut self, h: RigidBodyHandle) {
        self.bodies.remove(
            h,
            &mut self.islands,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multibody,
            true,
        );
    }
    pub fn start_pull(&mut self, id: u64) -> bool {
        let Some(doll) = self.dolls.get_mut(&id) else {
            return false;
        };
        if doll.pulling {
            return false;
        }
        doll.pulling = true;
        if let Some(belt) = doll.belt.take() {
            self.joints.remove(belt, true);
        }
        let chest = doll.parts[1];
        let mut position = *self.bodies[chest].position();
        position.translation.vector = self.bodies[chest]
            .position()
            .transform_point(&point![-0.18, 0.10, 0.])
            .coords;
        let anchor = self.bodies.insert(
            RigidBodyBuilder::kinematic_position_based()
                .pose(position)
                .build(),
        );
        // A bounded spring transmits the officer's pull. An infinitely strong
        // kinematic attachment can tear a constrained limb chain through a doorpost.
        let mut spring = SpringJoint::new(0.01, 7000., 250.);
        spring.set_local_anchor2(point![-0.18, 0.10, 0.]);
        spring.data.set_motor_max_force(JointAxis::LinX, 2500.);
        let joint = self.joints.insert(anchor, chest, spring, true);
        doll.grab = Some((anchor, joint));
        true
    }
    pub fn ray_dolls(&self, origin: V3, direction: V3, max: f32) -> Option<(f32, u64, u8)> {
        let ray = Ray::new(Point::from(vec(origin)), vec(direction));
        let mut best = None;
        for (&id, d) in &self.dolls {
            for (i, h) in d.parts.iter().enumerate() {
                for c in self.bodies[*h].colliders() {
                    let collider = &self.colliders[*c];
                    if let Some(distance) =
                        collider
                            .shape()
                            .cast_ray(collider.position(), &ray, max, true)
                        && best.is_none_or(|(d, _, _)| distance < d)
                    {
                        best = Some((distance, id, i as u8));
                    }
                }
            }
        }
        best
    }
    pub fn impulse(&mut self, id: u64, bone: u8, impulse: V3) {
        if let Some(d) = self.dolls.get(&id)
            && let Some(h) = d.parts.get(bone as usize)
        {
            self.bodies[*h].apply_impulse(vec(impulse), true);
        }
    }
    fn make_doll(&mut self, v: &VehicleView) {
        let seated = v.physics.npc.died_seated;
        let base = if seated {
            c::seated_position(v)
        } else {
            v.physics.npc.position
        };
        let yaw = v.physics.npc.yaw;
        let rig = border_net::rig::skeleton(seated);
        let mut parts = vec![];
        for bone in &rig.bones {
            let position = c::add(base, c::rotate(bone.center, yaw));
            let rotation = UnitQuaternion::from_axis_angle(&Vector::y_axis(), yaw)
                * UnitQuaternion::from_axis_angle(&Vector::x_axis(), bone.angle);
            let body = self.bodies.insert(
                RigidBodyBuilder::dynamic()
                    .pose(Isometry::from_parts(
                        Translation::from(vec(position)),
                        rotation,
                    ))
                    .linear_damping(0.45)
                    .angular_damping(1.8)
                    .ccd_enabled(true)
                    .additional_solver_iterations(16)
                    .build(),
            );
            self.colliders.insert_with_parent(
                ColliderBuilder::capsule_y((bone.length * 0.5 - bone.radius).max(0.), bone.radius)
                    .mass(bone.mass)
                    .friction(0.8)
                    .restitution(0.)
                    .build(),
                body,
                &mut self.bodies,
            );
            parts.push(body);
        }
        for link in &rig.links {
            let world_anchor = Point::from(vec(c::add(base, c::rotate(link.anchor, yaw))));
            let local_a = self.bodies[parts[link.parent]]
                .position()
                .inverse_transform_point(&world_anchor);
            let local_b = self.bodies[parts[link.child]]
                .position()
                .inverse_transform_point(&world_anchor);
            // Keep the anatomical reference frames: resetting these to the seated pose
            // makes the knees bend backwards and gives the spine shoulder-like freedom.
            let mut joint: GenericJoint = if link.hinge {
                RevoluteJointBuilder::new(Vector::x_axis())
                    .local_anchor1(local_a)
                    .local_anchor2(local_b)
                    .limits(link.limits[0])
                    .contacts_enabled(false)
                    .into()
            } else {
                SphericalJointBuilder::new()
                    .local_anchor1(local_a)
                    .local_anchor2(local_b)
                    .limits(JointAxis::AngX, link.limits[0])
                    .limits(JointAxis::AngY, link.limits[1])
                    .limits(JointAxis::AngZ, link.limits[2])
                    .contacts_enabled(false)
                    .into()
            };
            for axis in if link.hinge {
                vec![JointAxis::AngX]
            } else {
                vec![JointAxis::AngX, JointAxis::AngY, JointAxis::AngZ]
            } {
                joint.set_motor_position(axis, 0., link.stiffness, link.damping);
                joint.set_motor_max_force(
                    axis,
                    if link.parent == 0 && link.child == 1 {
                        90.
                    } else {
                        18.
                    },
                );
            }
            self.joints
                .insert(parts[link.parent], parts[link.child], joint, true);
        }
        let belt = if seated {
            self.vehicles.get(&v.vehicle_id).map(|&car| {
                self.joints.insert(
                    car,
                    parts[0],
                    SphericalJointBuilder::new()
                        .local_anchor1(Point::from(vec(c::add(
                            c::local(v, base),
                            rig.bones[0].center,
                        ))))
                        .contacts_enabled(false),
                    true,
                )
            })
        } else {
            None
        };
        // A small initial impulse allows gravity/constraints to produce a physical collapse, never a canned death pose.
        self.bodies[parts[1]].apply_impulse(
            vec(c::add(
                v.physics.npc.death_impulse,
                c::rotate([0.15, 0., -0.15], yaw),
            )),
            true,
        );
        self.dolls.insert(
            v.vehicle_id,
            Doll {
                parts,
                belt,
                grab: None,
                pulling: false,
            },
        );
    }
    fn update_environment(&mut self, world: &WorldSnapshot) {
        for v in world
            .vehicles
            .iter()
            .filter(|v| v.stage != VehicleStage::Exited)
        {
            let handle = *self.vehicles.entry(v.vehicle_id).or_insert_with(|| {
                self.bodies.insert(
                    RigidBodyBuilder::kinematic_position_based()
                        .pose(pose(v.position, v.yaw))
                        .build(),
                )
            });
            self.bodies[handle].set_next_kinematic_position(pose(v.position, v.yaw));
            let stamp: Vec<f32> = v
                .physics
                .door_angles
                .into_iter()
                .chain(
                    v.physics
                        .glass
                        .iter()
                        .flat_map(|g| [g.lowered, if g.shattered { 1. } else { 0. }]),
                )
                .collect();
            if self
                .surfaces
                .get(&v.vehicle_id)
                .is_some_and(|(_, old)| *old == stamp)
            {
                continue;
            }
            if let Some((old, _)) = self.surfaces.remove(&v.vehicle_id) {
                for h in old {
                    self.colliders
                        .remove(h, &mut self.islands, &mut self.bodies, true);
                }
            }
            self.surfaces.insert(v.vehicle_id, (vec![], stamp));

            let mut boxes = c::shell_boxes(v);
            boxes[0] = (
                [0., 0.30, 0.],
                [
                    border_net::vehicle::spec(&v.kind).half_width,
                    0.07,
                    border_net::vehicle::body_half_length(&v.kind),
                ],
            );
            boxes.extend([
                (
                    [
                        -0.48,
                        0.55 + if border_net::vehicle::pickup(&v.kind) {
                            0.35
                        } else {
                            border_net::vehicle::cab_lift(&v.kind)
                        },
                        border_net::vehicle::spec(&v.kind).window_z + 0.1,
                    ],
                    [0.26, 0.10, 0.33],
                ),
                (
                    [
                        -0.48,
                        1.03 + if border_net::vehicle::pickup(&v.kind) {
                            0.35
                        } else {
                            border_net::vehicle::cab_lift(&v.kind)
                        },
                        border_net::vehicle::spec(&v.kind).window_z + 0.28,
                    ],
                    [0.26, 0.28, 0.07],
                ),
            ]);
            for (p, half) in boxes {
                let h = self.colliders.insert_with_parent(
                    ColliderBuilder::cuboid(half[0], half[1], half[2])
                        .translation(vec(p))
                        .friction(0.75)
                        .build(),
                    handle,
                    &mut self.bodies,
                );
                self.surfaces.get_mut(&v.vehicle_id).unwrap().0.push(h);
            }
            for side in [Side::Driver, Side::Passenger] {
                let angle = v.physics.door_angles[side_index(side)]
                    * if side == Side::Driver { -1. } else { 1. };
                let layout = border_net::vehicle::door_layout(&v.kind);
                for (offset, half) in [
                    (
                        [0., (layout.belt + 0.55) * 0.5, layout.length * 0.5],
                        [0.035, (layout.belt - 0.55) * 0.5, layout.length * 0.5],
                    ),
                    (
                        [0., (layout.belt + layout.top) * 0.5, 0.02],
                        [0.0275, (layout.top - layout.belt) * 0.5, 0.0275],
                    ),
                    (
                        [0., (layout.belt + layout.top) * 0.5, layout.length - 0.02],
                        [0.0275, (layout.top - layout.belt) * 0.5, 0.0275],
                    ),
                    (
                        [0., layout.top, layout.length * 0.5],
                        [0.0275, 0.0275, layout.length * 0.5],
                    ),
                ] {
                    let center = c::door_point(
                        &v.kind,
                        side,
                        v.physics.door_angles[side_index(side)],
                        c::add(c::door_pivot(&v.kind, side), offset),
                    );
                    let h = self.colliders.insert_with_parent(
                        ColliderBuilder::cuboid(half[0], half[1], half[2])
                            .position(pose(center, angle))
                            .build(),
                        handle,
                        &mut self.bodies,
                    );
                    self.surfaces.get_mut(&v.vehicle_id).unwrap().0.push(h);
                }
            }

            for g in &v.physics.glass {
                if !g.shattered
                    && let Some(f) = c::pane_frame(v, g.pane)
                {
                    let u = vec(f.u).normalize();
                    let up = vec(f.v).normalize();
                    let n = vec(f.normal).normalize();
                    let rot = UnitQuaternion::from_rotation_matrix(
                        &Rotation3::from_matrix_unchecked(Matrix3::from_columns(&[u, up, n])),
                    );
                    let local = pose(v.position, v.yaw).inverse()
                        * Isometry::from_parts(Translation::from(vec(f.center)), rot);
                    let h = self.colliders.insert_with_parent(
                        ColliderBuilder::cuboid(f.half[0], f.half[1], 0.006)
                            .position(local)
                            .friction(0.4)
                            .build(),
                        handle,
                        &mut self.bodies,
                    );
                    self.surfaces.get_mut(&v.vehicle_id).unwrap().0.push(h);
                }
            }
        }
    }
    fn spawn_fragments(&mut self, world: &WorldSnapshot) {
        for hit in &world.impacts {
            if !hit.kind.starts_with("glass") || !self.seen.insert(hit.id) {
                continue;
            }
            let Some(v) = world
                .vehicles
                .iter()
                .find(|v| Some(v.vehicle_id) == hit.vehicle_id)
            else {
                continue;
            };
            let pane = v.physics.glass.iter().find(|g| {
                g.impacts.iter().any(|h| {
                    h.at_ms == hit.at_ms
                        && h.seed == hit.projectile.wrapping_mul(9176) + g.pane.index() as u64
                })
            });
            let Some(g) = pane else {
                continue;
            };
            let Some(f) = c::pane_frame(v, g.pane) else {
                continue;
            };
            let full = g.shattered;
            let count = if full { 48 } else { 8 };
            for j in 0..count {
                let seed = hit.id.wrapping_mul(977) + j as u64 * 71;
                let r = (seed % 101) as f32 / 100.;
                let uv = if full {
                    [-0.92 + (j % 8) as f32 * 0.26, -0.88 + (j / 8) as f32 * 0.34]
                } else {
                    let a = j as f32 * 0.785;
                    [a.cos() * 0.12, a.sin() * 0.12]
                };
                let position = if full {
                    c::add(
                        f.center,
                        c::add(
                            c::mul(f.u, uv[0] * f.half[0]),
                            c::mul(f.v, uv[1] * f.half[1]),
                        ),
                    )
                } else {
                    c::add(hit.position, c::add(c::mul(f.u, uv[0]), c::mul(f.v, uv[1])))
                };
                let direction = c::sub(hit.position, hit.origin);
                let direction = c::mul(direction, 1. / c::length(direction).max(0.001));
                let velocity = c::add(
                    c::mul(direction, 0.8 + r),
                    [
                        (r - 0.5) * 0.8,
                        r * 0.3,
                        ((seed % 19) as f32 / 19. - 0.5) * 0.8,
                    ],
                );
                let size = if full {
                    [f.half[0] * 0.20, f.half[1] * 0.28, 0.005]
                } else {
                    [0.035, 0.045, 0.004]
                };
                let h = self.bodies.insert(
                    RigidBodyBuilder::dynamic()
                        .translation(vec(position))
                        .linvel(vec(velocity))
                        .angvel(vector![r * 7., 2. - r * 4., 3.])
                        .linear_damping(0.15)
                        .angular_damping(0.25)
                        .ccd_enabled(true)
                        .build(),
                );
                self.colliders.insert_with_parent(
                    ColliderBuilder::cuboid(size[0] * 0.5, size[1] * 0.5, size[2] * 0.5)
                        .density(2300.)
                        .friction(0.7)
                        .restitution(0.12)
                        .collision_groups(InteractionGroups::new(Group::GROUP_2, Group::GROUP_1))
                        .build(),
                    h,
                    &mut self.bodies,
                );
                self.fragments.push(Fragment {
                    handle: h,
                    id: self.next_shard,
                    born: world.server_time_ms,
                    size,
                });
                self.next_shard += 1;
            }
        }
        while self.fragments.len() > 256 {
            let f = self.fragments.remove(0);
            self.remove_body(f.handle);
        }
    }
    #[cfg(test)]
    pub(crate) fn assert_rig_integrity(&self, context: &str) {
        for doll in self.dolls.values() {
            for (_, j) in self.joints.iter() {
                let Some(a) = doll.parts.iter().position(|h| *h == j.body1) else {
                    continue;
                };
                let Some(b) = doll.parts.iter().position(|h| *h == j.body2) else {
                    continue;
                };
                let f1 = self.bodies[j.body1].position() * j.data.local_frame1;
                let f2 = self.bodies[j.body2].position() * j.data.local_frame2;
                let gap = (f1.translation.vector - f2.translation.vector).norm();
                assert!(
                    gap < 0.035,
                    "{context}: joint {a}->{b} separated by {gap:.4}m"
                );
                let relative = f1.rotation.inverse() * f2.rotation;
                if a == 0 && b == 1 {
                    assert!(
                        relative.angle() < 0.45,
                        "spine folded: {} radians",
                        relative.angle()
                    );
                }
                if matches!((a, b), (3, 4) | (5, 6) | (7, 8) | (9, 10)) {
                    let (x, y, z) = relative.euler_angles();
                    assert!(
                        y.abs() < 0.08 && z.abs() < 0.08,
                        "hinge twisted {a}->{b}: {y}, {z}"
                    );
                    if a >= 7 {
                        assert!((-2.55..=0.10).contains(&x), "knee hyperextended: {x}");
                    } else {
                        assert!((-0.08..=2.55).contains(&x), "elbow hyperextended: {x}");
                    }
                }
            }
        }
    }
    pub fn advance(&mut self, world: &mut WorldSnapshot, dt: f32) {
        if world.phase == border_net::ShiftPhase::Lobby {
            return;
        }
        let exited: Vec<_> = self
            .vehicles
            .keys()
            .copied()
            .filter(|id| {
                !world
                    .vehicles
                    .iter()
                    .any(|v| v.vehicle_id == *id && v.stage != VehicleStage::Exited)
            })
            .collect();
        for id in exited {
            if let Some(h) = self.vehicles.remove(&id) {
                self.remove_body(h);
            }
            self.surfaces.remove(&id);
        }
        let active = !self.dolls.is_empty()
            || !self.fragments.is_empty()
            || world
                .vehicles
                .iter()
                .any(|v| v.physics.npc.health == 0 && v.stage != VehicleStage::Exited)
            || world
                .impacts
                .iter()
                .any(|h| h.kind.starts_with("glass") && !self.seen.contains(&h.id));
        if !active {
            world.shards.clear();
            return;
        }
        self.update_environment(world);
        self.spawn_fragments(world);
        for v in &world.vehicles {
            if v.physics.npc.health == 0
                && v.stage != VehicleStage::Exited
                && !self.dolls.contains_key(&v.vehicle_id)
            {
                self.make_doll(v);
            }
        }
        let removed: Vec<_> = self
            .dolls
            .keys()
            .copied()
            .filter(|id| {
                !world
                    .vehicles
                    .iter()
                    .any(|v| v.vehicle_id == *id && v.stage != VehicleStage::Exited)
            })
            .collect();
        for id in removed {
            if let Some(d) = self.dolls.remove(&id) {
                for h in d.parts {
                    self.remove_body(h);
                }
                if let Some((h, _)) = d.grab {
                    self.remove_body(h);
                }
            }
        }
        let mut completed = vec![];
        for (&id, doll) in &mut self.dolls {
            if let Some((anchor, _)) = doll.grab
                && let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == id)
            {
                let pelvis_local = c::local(
                    v,
                    [
                        self.bodies[doll.parts[0]].translation().x,
                        self.bodies[doll.parts[0]].translation().y,
                        self.bodies[doll.parts[0]].translation().z,
                    ],
                );
                let clearing_sill =
                    pelvis_local[0] > -border_net::vehicle::spec(&v.kind).half_width - 0.5;
                let target = c::world(
                    v,
                    [
                        -border_net::vehicle::spec(&v.kind).half_width - 1.7,
                        if clearing_sill {
                            1.25 + if border_net::vehicle::pickup(&v.kind) {
                                0.35
                            } else {
                                border_net::vehicle::cab_lift(&v.kind)
                            }
                        } else {
                            0.65
                        },
                        border_net::vehicle::spec(&v.kind).window_z - 0.10,
                    ],
                );
                let current = *self.bodies[anchor].translation();
                let delta = vec(target) - current;
                let chest = self.bodies[doll.parts[1]]
                    .position()
                    .transform_point(&point![-0.18, 0.10, 0.])
                    .coords;
                let pull_speed = if (current - chest).norm() < 0.6 {
                    1.0
                } else {
                    0.
                };
                let next = current
                    + delta.try_normalize(0.0001).unwrap_or_else(Vector::zeros)
                        * (dt * pull_speed).min(delta.norm());
                self.bodies[anchor].set_next_kinematic_translation(next);
                if delta.norm() < 0.08 {
                    completed.push(id);
                }
            }
        }
        self.accumulator = (self.accumulator + dt).min(1.);
        while self.accumulator >= 1. / 120. {
            let params = IntegrationParameters {
                dt: 1. / 120.,
                num_solver_iterations: 12,
                num_internal_pgs_iterations: 2,
                num_internal_stabilization_iterations: 2,
                ..Default::default()
            };
            self.pipeline.step(
                &vector![0., -9.81, 0.],
                &params,
                &mut self.islands,
                &mut self.broad,
                &mut self.narrow,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multibody,
                &mut self.ccd,
                &(),
                &(),
            );
            self.accumulator -= 1. / 120.;
        }
        for id in completed {
            if let Some(d) = self.dolls.get_mut(&id)
                && let Some((anchor, joint)) = d.grab.take()
            {
                self.joints.remove(joint, true);
                self.remove_body(anchor);
            }
            if let Some(v) = world.vehicles.iter_mut().find(|v| v.vehicle_id == id) {
                v.physics.npc.extracted = true;
                v.physics.npc.body_pull_active = false;
                v.physics.npc.died_seated = false;
            }
        }
        for v in &mut world.vehicles {
            if let Some(d) = self.dolls.get(&v.vehicle_id) {
                v.physics.npc.ragdoll = d
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, h)| {
                        let b = &self.bodies[*h];
                        let p = b.translation();
                        let q = b.rotation().quaternion();
                        BonePose {
                            bone: i as u8,
                            position: [p.x, p.y, p.z],
                            rotation: [q.i, q.j, q.k, q.w],
                        }
                    })
                    .collect();
            }
        }
        let expired: Vec<_> = self
            .fragments
            .iter()
            .filter(|f| world.server_time_ms.saturating_sub(f.born) > 8000)
            .map(|f| f.handle)
            .collect();
        self.fragments.retain(|f| !expired.contains(&f.handle));
        for h in expired {
            self.remove_body(h);
        }
        world.shards = self
            .fragments
            .iter()
            .map(|f| {
                let b = &self.bodies[f.handle];
                let p = b.translation();
                let q = b.rotation().quaternion();
                ShardPose {
                    id: f.id,
                    position: [p.x, p.y, p.z],
                    rotation: [q.i, q.j, q.k, q.w],
                    size: f.size,
                    born_ms: f.born,
                }
            })
            .collect();
    }
}
