//! One reticle, one physical target. Dialogue never captures movement or opens a panel.
use super::*;
use bevy::render::render_resource::Face;
use border_net::{
    ClientCommand as Cmd, VehicleStage,
    combat::{self as c, *},
    vehicle::{self, VehiclePoint},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Talk,
    Door(Side),
    Panel(VehiclePoint),
    Compartment(border_scenarios::EvidenceLocation),
    Body(u8),
    Porter(u8),
    Ready,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub vehicle: u64,
    pub action: Action,
}
#[derive(Resource, Default)]
pub struct Focus {
    pub target: Option<Target>,
    material: Option<Handle<StandardMaterial>>,
    last_grab: f32,
}
#[derive(Component)]
pub struct Outline(Entity);
#[derive(Component)]
pub struct ServiceNpc(pub u8);
#[derive(Component)]
pub struct TrailerPiece(pub u64);
fn hit(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3, rotation: Quat) -> Option<f32> {
    let inv = rotation.inverse();
    c::ray_box(
        (inv * (origin - center)).to_array(),
        (inv * dir).to_array(),
        [0.; 3],
        half.to_array(),
        3.4,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn select(
    shift: Res<shift::Shift>,
    g: Res<Patrol>,
    camera: Query<&Transform, With<Player>>,
    mut focus: ResMut<Focus>,
) {
    focus.target = None;
    if g.overlay || g.map_open || !g.looking {
        return;
    }
    let (Some(w), Ok(t)) = (&shift.world, camera.single()) else {
        return;
    };
    let origin = t.translation;
    let dir = *t.forward();
    let mut choices = vec![];
    let mut offer = |target: Target, p: Vec3, half: Vec3, r: Quat| {
        if let Some(d) = hit(origin, dir, p, half, r) {
            choices.push((d, target));
        }
    };
    if w.phase == border_net::ShiftPhase::Lobby {
        offer(
            Target {
                vehicle: 0,
                action: Action::Ready,
            },
            Vec3::from_array(border_net::map::DOOR),
            Vec3::new(0.9, 1.5, 0.08),
            Quat::IDENTITY,
        );
    }
    for p in &w.porters {
        if p.phase == border_net::PorterPhase::Seated
            && shift.permits(p.lane_id)
            && w.vehicles.iter().any(|v| {
                v.lane_id == p.lane_id
                    && matches!(
                        v.stage,
                        VehicleStage::Stopped | VehicleStage::Disabled | VehicleStage::Resolved
                    )
                    && (v.physics.npc.health == 0
                        || matches!(v.physics.npc.mode, NpcMode::Outside | NpcMode::Fleeing))
            })
        {
            offer(
                Target {
                    vehicle: 0,
                    action: Action::Porter(p.lane_id),
                },
                Vec3::from_array(p.position) + Vec3::Y,
                Vec3::new(0.28, 0.55, 0.25),
                Quat::from_rotation_y(p.yaw),
            );
        }
    }
    for v in &w.vehicles {
        if v.physics.npc.health == 0 {
            for b in &v.physics.npc.ragdoll {
                let mut action = Action::Body(b.bone);
                if v.physics.npc.died_seated && v.physics.door_angles[0] < 0.8 {
                    action = Action::Door(Side::Driver);
                }
                let bone = border_net::rig::skeleton(false).bones[b.bone as usize];
                offer(
                    Target {
                        vehicle: v.vehicle_id,
                        action,
                    },
                    Vec3::from_array(b.position),
                    Vec3::new(
                        bone.radius + 0.06,
                        bone.length * 0.5 + 0.06,
                        bone.radius + 0.06,
                    ),
                    Quat::from_array(b.rotation),
                );
            }
        }
        if !matches!(
            v.stage,
            VehicleStage::Stopped | VehicleStage::Resolved | VehicleStage::Disabled
        ) {
            continue;
        }
        let rot = Quat::from_rotation_y(v.yaw);
        let point = |p: [f32; 3]| Vec3::from_array(c::world(v, p));
        if v.physics.npc.health > 0
            && v.stage == VehicleStage::Stopped
            && v.conversation_owner.is_none_or(|p| p == shift.peer_id)
            && shift.permits(v.lane_id)
        {
            let driver =
                vehicle::spec(&v.kind).world_point(v.position, v.yaw, VehiclePoint::Driver);
            if v.physics.npc.mode == NpcMode::Outside
                || (origin.x - driver[0]).hypot(origin.z - driver[2]) < 2.2
            {
                offer(
                    Target {
                        vehicle: v.vehicle_id,
                        action: Action::Talk,
                    },
                    Vec3::from_array(v.physics.npc.position) + Vec3::Y * 1.55,
                    Vec3::new(0.24, 0.28, 0.23),
                    Quat::from_rotation_y(v.physics.npc.yaw),
                );
            }
        }
        let layout = vehicle::door_layout(&v.kind);
        for side in [Side::Driver, Side::Passenger] {
            let angle = v.physics.door_angles[side_index(side)];
            let p = c::add(
                door_pivot(&v.kind, side),
                [0., (layout.belt + 0.55) * 0.5, layout.length * 0.5],
            );
            offer(
                Target {
                    vehicle: v.vehicle_id,
                    action: Action::Door(side),
                },
                point(door_point(&v.kind, side, angle, p)),
                Vec3::new(0.08, (layout.belt - 0.55) * 0.5, layout.length * 0.5),
                rot * Quat::from_rotation_y(if side == Side::Driver { -angle } else { angle }),
            );
        }
        if !shift.permits(v.lane_id) {
            continue;
        }
        for point_id in [VehiclePoint::Hood, VehiclePoint::Cargo] {
            let rear = point_id == VehiclePoint::Cargo;
            let lift = vehicle::cab_lift(&v.kind);
            let y = if vehicle::pickup(&v.kind) {
                if rear { 1.3 } else { 1.13 }
            } else {
                0.96 + lift
            };
            let z = if rear {
                vehicle::rear_glass_z(&v.kind)
            } else {
                -1.05
            };
            let len = if rear {
                (vehicle::body_half_length(&v.kind) - z).max(0.3)
            } else {
                vehicle::body_half_length(&v.kind) - 1.05
            };
            let open = v.open_panels.contains(&point_id);
            let rx = if open {
                if rear { -1.0 } else { 1.0 }
            } else {
                0.
            };
            let r = Quat::from_rotation_x(rx);
            let pivot = Vec3::new(0., y, z);
            let center = pivot + r * Vec3::new(0., 0., if rear { len * 0.5 } else { -len * 0.5 });
            offer(
                Target {
                    vehicle: v.vehicle_id,
                    action: Action::Panel(point_id),
                },
                point(center.to_array()),
                Vec3::new(vehicle::spec(&v.kind).half_width - 0.02, 0.07, len * 0.5),
                rot * r,
            );
        }
        if v.open_panels.contains(&VehiclePoint::Cargo) {
            let pickup = vehicle::pickup(&v.kind);
            offer(
                Target {
                    vehicle: v.vehicle_id,
                    action: Action::Compartment(border_scenarios::EvidenceLocation::Container),
                },
                point([
                    0.,
                    if pickup { 1.3 } else { 0.78 },
                    if pickup { 1.6 } else { 1.7 },
                ]),
                Vec3::new(0.42, 0.20, 0.32),
                rot,
            );
        }
        if v.physics.door_angles[1] > 0.7 {
            offer(
                Target {
                    vehicle: v.vehicle_id,
                    action: Action::Compartment(border_scenarios::EvidenceLocation::Glovebox),
                },
                point([
                    0.48,
                    1.01 + if vehicle::pickup(&v.kind) {
                        0.35
                    } else {
                        vehicle::cab_lift(&v.kind)
                    },
                    vehicle::spec(&v.kind).window_z - 0.385,
                ]),
                Vec3::new(0.22, 0.13, 0.06),
                rot,
            );
        }
        if v.kind == "utility_truck" {
            offer(
                Target {
                    vehicle: v.vehicle_id,
                    action: Action::Panel(VehiclePoint::Trailer),
                },
                point([0., 1.0, 5.4]),
                Vec3::new(0.9, 0.5, 0.12),
                rot,
            );
        }
    }
    choices.sort_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((distance, target)) = choices.first() {
        let obstructed = shift.collision.iter().any(|b| {
            c::ray_box(
                origin.to_array(),
                dir.to_array(),
                b.center,
                b.half,
                *distance,
            )
            .is_some_and(|d| d + 0.08 < *distance)
        });
        if !obstructed {
            focus.target = Some(*target);
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut g: ResMut<Patrol>,
    shift: Res<shift::Shift>,
    net: Res<Network>,
    mut focus: ResMut<Focus>,
    smoke: Option<Res<SmokeTest>>,
) {
    if smoke.as_ref().is_some_and(|s| !s.casual) {
        return;
    }
    let held = shift
        .world
        .as_ref()
        .and_then(|w| {
            w.vehicles
                .iter()
                .find(|v| v.physics.npc.body_holder == Some(shift.peer_id))
        })
        .map(|v| v.vehicle_id);
    if held.is_some() && (!keys.pressed(KeyCode::KeyE) || g.overlay || !g.looking) {
        shift.send(Cmd::ReleaseBody);
    }
    if g.overlay || !g.looking {
        return;
    }
    let Some(target) = focus.target else {
        return;
    };
    let id = target.vehicle;
    if let Action::Body(bone) = target.action {
        if keys.pressed(KeyCode::KeyE)
            && held.is_none()
            && time.elapsed_secs() - focus.last_grab > 0.25
        {
            focus.last_grab = time.elapsed_secs();
            shift.send(Cmd::GrabBody {
                vehicle_id: id,
                bone,
            });
        }
        return;
    }
    let automatic = smoke
        .as_ref()
        .is_some_and(|s| s.casual && s.step == 0 && s.robot_step >= 5)
        && g.case.is_none()
        && !g.pending
        && target.action == Action::Talk;
    if !keys.just_pressed(KeyCode::KeyE) && !automatic {
        return;
    }
    match target.action {
        Action::Talk => {
            if g.vehicle_id == Some(id) && g.case.is_some() {
                shift.send(Cmd::EndConversation { vehicle_id: id });
                g.reset_case();
                g.vehicle_id = None;
                g.looking = true;
            } else if !g.pending {
                if let Some(old) = g.vehicle_id {
                    shift.send(Cmd::EndConversation { vehicle_id: old });
                }
                g.reset_case();
                g.looking = true;
                g.vehicle_id = Some(id);
                g.pending = true;
                net.start(g.generation, id.to_string());
            }
        }
        Action::Door(side) => shift.send(Cmd::ToggleDoor {
            vehicle_id: id,
            side,
        }),
        Action::Panel(point) => shift.send(Cmd::TogglePanel {
            vehicle_id: id,
            point,
        }),
        Action::Compartment(location) => shift.send(Cmd::SearchCompartment {
            vehicle_id: id,
            location,
        }),
        Action::Porter(lane_id) => shift.send(Cmd::RequestPorter { lane_id }),
        Action::Body(_) | Action::Ready => {}
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn outlines(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut focus: ResMut<Focus>,
    meshes: Query<(Entity, &Mesh3d, Option<&ChildOf>), Without<Outline>>,
    parents: Query<&ChildOf>,
    details: Query<(Entity, &vehicle_physics::Detail)>,
    panels: Query<(Entity, &vehicles::Panel)>,
    porters: Query<(Entity, &ServiceNpc)>,
    trailers: Query<(Entity, &TrailerPiece)>,
    ready: Query<Entity, With<shift::ReadyDoor>>,
    mut outlines: Query<(&Outline, &mut Visibility)>,
) {
    if focus.material.is_none() {
        focus.material = Some(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..default()
        }));
    }
    let mut roots = std::collections::HashSet::new();
    if let Some(t) = focus.target {
        for (e, d) in &details {
            let match_kind = matches!(
                (t.action, d.kind),
                (Action::Talk, vehicle_physics::DetailKind::Npc)
                    | (Action::Body(_), vehicle_physics::DetailKind::Bone(_))
            ) || matches!((t.action,d.kind),(Action::Door(a),vehicle_physics::DetailKind::Door(b)) if a==b);
            if d.vehicle == t.vehicle && match_kind {
                roots.insert(e);
            }
        }
        for (e, p) in &panels {
            let id = 1 + p.lane as u64 + p.ordinal as u64 * 2;
            if id == t.vehicle
                && (matches!(t.action,Action::Panel(point) if point==p.point && p.compartment.is_none())
                    || matches!(t.action,Action::Compartment(l) if p.compartment==Some(l)))
            {
                roots.insert(e);
            }
        }
        for (e, p) in &porters {
            if t.action == Action::Porter(p.0) {
                roots.insert(e);
            }
        }
        for (e, p) in &trailers {
            if t.vehicle == p.0 && t.action == Action::Panel(VehiclePoint::Trailer) {
                roots.insert(e);
            }
        }
        if t.action == Action::Ready {
            roots.extend(ready.iter());
        }
    }
    let mut selected = std::collections::HashSet::new();
    for (e, _, _) in &meshes {
        let mut node = e;
        for _ in 0..12 {
            if roots.contains(&node) {
                selected.insert(e);
                break;
            }
            let Ok(parent) = parents.get(node) else {
                break;
            };
            node = parent.parent();
        }
    }
    let existing: std::collections::HashSet<_> = outlines.iter().map(|(o, _)| o.0).collect();
    for (o, mut visible) in &mut outlines {
        *visible = if selected.contains(&o.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for e in selected {
        if existing.contains(&e) {
            continue;
        }
        if let Ok((_, mesh, _)) = meshes.get(e) {
            let outline = commands
                .spawn((
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(focus.material.as_ref().unwrap().clone()),
                    Transform::from_scale(Vec3::splat(1.045)),
                    Outline(e),
                    bevy::light::NotShadowCaster,
                    Visibility::Inherited,
                ))
                .id();
            commands.entity(e).add_child(outline);
        }
    }
}
