//! Reusable pickup body and articulated inspection panels for both physical vehicle types.
use super::*;
use border_net::{
    VehicleStage,
    vehicle::{self, VehiclePoint},
};
#[derive(Component)]
pub struct EntryGate;
#[derive(Component)]
pub struct Body {
    pub lane: u8,
    pub ordinal: usize,
    pub kind: &'static str,
}
#[derive(Component)]
pub struct Panel {
    pub lane: u8,
    pub ordinal: usize,
    pub point: VehiclePoint,
    pub base: Quat,
    pub truck_tailgate: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn truck(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    lane: u8,
    ordinal: usize,
    paint: Handle<StandardMaterial>,
) {
    let body = commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            Body {
                lane,
                ordinal,
                kind: "truck",
            },
        ))
        .id();
    commands.entity(root).add_child(body);
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.51, 0.52),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let rubber = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.045, 0.05),
        perceptual_roughness: 0.85,
        ..default()
    });
    let liner = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.20, 0.20),
        perceptual_roughness: 0.8,
        ..default()
    });
    let lamp = materials.add(Color::srgb(0.96, 0.89, 0.67));
    let tail = materials.add(Color::srgb(0.75, 0.12, 0.06));
    let mut block = |size: Vec3, pos: Vec3, mat: Handle<StandardMaterial>| {
        let e = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(mat),
                Transform::from_translation(pos),
            ))
            .id();
        commands.entity(body).add_child(e);
    };
    // Long chassis, raised cab, separate hood and a real open pickup bed.
    block(
        Vec3::new(2.24, 0.30, 6.5),
        Vec3::new(0., 0.65, 0.),
        paint.clone(),
    );
    block(
        Vec3::new(2.05, 0.18, 2.25),
        Vec3::new(0., 2.18, -1.05),
        paint.clone(),
    );
    for x in [-1.04, 1.04] {
        for z in [-2.08, -0.98, 0.02] {
            block(
                Vec3::new(0.10, 1.08, 0.10),
                Vec3::new(x, 1.68, z),
                paint.clone(),
            );
        }
        block(
            Vec3::new(0.32, 0.18, 0.34),
            Vec3::new(x * 1.17, 1.7, -1.92),
            rubber.clone(),
        );
        block(
            Vec3::new(0.18, 0.13, 0.025),
            Vec3::new(x * 1.17, 1.7, -1.74),
            metal.clone(),
        );
        block(
            Vec3::new(0.28, 0.12, 1.95),
            Vec3::new(x * 1.05, 0.58, -1.03),
            metal.clone(),
        );
        block(
            Vec3::new(0.12, 0.50, 2.97),
            Vec3::new(x, 1.3, 1.63),
            paint.clone(),
        );
        block(
            Vec3::new(0.15, 0.07, 2.97),
            Vec3::new(x, 1.58, 1.63),
            liner.clone(),
        );
        block(
            Vec3::new(0.025, 0.05, 0.22),
            Vec3::new(x * 1.08, 1.35, -0.7),
            metal.clone(),
        );
    }
    block(
        Vec3::new(1.96, 0.12, 2.9),
        Vec3::new(0., 1.04, 1.62),
        liner.clone(),
    );
    for x in [-0.84, -0.56, -0.28, 0., 0.28, 0.56, 0.84] {
        block(
            Vec3::new(0.035, 0.035, 2.84),
            Vec3::new(x, 1.12, 1.62),
            metal.clone(),
        );
    }
    block(
        Vec3::new(1.95, 0.5, 0.14),
        Vec3::new(0., 1.32, 0.15),
        paint.clone(),
    );
    block(
        Vec3::new(1.9, 0.24, 0.10),
        Vec3::new(0., 0.99, -3.28),
        rubber.clone(),
    );
    for x in [-0.7, -0.35, 0., 0.35, 0.7] {
        block(
            Vec3::new(0.18, 0.05, 0.025),
            Vec3::new(x, 1.0, -3.34),
            metal.clone(),
        );
    }
    for z in [-3.31, 3.31] {
        block(
            Vec3::new(2.22, 0.20, 0.16),
            Vec3::new(0., 0.65, z),
            metal.clone(),
        );
    }
    for x in [-0.87, 0.87] {
        block(
            Vec3::new(0.39, 0.25, 0.08),
            Vec3::new(x, 1.13, -3.3),
            lamp.clone(),
        );
        block(
            Vec3::new(0.17, 0.38, 0.08),
            Vec3::new(x, 1.3, 3.29),
            tail.clone(),
        );
    }
    block(
        Vec3::new(0.55, 0.18, 0.03),
        Vec3::new(0., 0.75, 3.41),
        lamp.clone(),
    );
    // Visible seated driver, steering wheel and bench.
    block(
        Vec3::new(1.8, 0.5, 0.45),
        Vec3::new(0., 1.25, -0.55),
        rubber.clone(),
    );
    for x in [-1.12_f32, 1.12] {
        for z in [-2.26, 2.18] {
            for (radius, width, offset, mat) in [
                (0.50, 0.28, 0., rubber.clone()),
                (0.29, 0.035, x.signum() * 0.16, metal.clone()),
            ] {
                let e = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(radius, width).mesh().resolution(20))),
                        MeshMaterial3d(mat),
                        Transform::from_xyz(x + offset, 0.5, z)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    ))
                    .id();
                commands.entity(body).add_child(e);
            }
        }
    }
    for (point, pivot, size, offset) in [
        (
            VehiclePoint::Cargo,
            Vec3::new(0., 1.05, 3.17),
            Vec3::new(2.08, 0.52, 0.13),
            Vec3::new(0., 0.26, 0.),
        ),
        (
            VehiclePoint::Hood,
            Vec3::new(0., 1.13, -2.12),
            Vec3::new(2.08, 0.10, 1.05),
            Vec3::new(0., 0., -0.52),
        ),
    ] {
        panel(
            commands,
            meshes,
            body,
            paint.clone(),
            lane,
            ordinal,
            point,
            pivot,
            size,
            offset,
        );
    }
}
#[allow(clippy::too_many_arguments)]
fn panel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    body: Entity,
    mat: Handle<StandardMaterial>,
    lane: u8,
    ordinal: usize,
    point: VehiclePoint,
    pivot: Vec3,
    size: Vec3,
    offset: Vec3,
) {
    let hinge = commands
        .spawn((
            Transform::from_translation(pivot),
            Visibility::default(),
            Panel {
                lane,
                ordinal,
                point,
                base: Quat::IDENTITY,
                truck_tailgate: point == VehiclePoint::Cargo && size.z < 0.2,
            },
        ))
        .id();
    let mesh = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(mat),
            Transform::from_translation(offset),
        ))
        .id();
    commands.entity(hinge).add_child(mesh);
    commands.entity(body).add_child(hinge);
}
pub fn sedan_panels(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    body: Entity,
    paint: Handle<StandardMaterial>,
    lane: u8,
    ordinal: usize,
) {
    for (point, pivot, size, offset) in [
        (
            VehiclePoint::Cargo,
            Vec3::new(0., 0.96, 1.13),
            Vec3::new(1.9, 0.07, 1.1),
            Vec3::new(0., 0., 0.55),
        ),
        (
            VehiclePoint::Hood,
            Vec3::new(0., 0.96, -1.05),
            Vec3::new(1.9, 0.07, 1.1),
            Vec3::new(0., 0., -0.55),
        ),
    ] {
        panel(
            commands,
            meshes,
            body,
            paint.clone(),
            lane,
            ordinal,
            point,
            pivot,
            size,
            offset,
        );
    }
}
pub fn animate(
    shift: Res<shift::Shift>,
    time: Res<Time>,
    mut bodies: Query<(&Body, &mut Visibility)>,
    mut panels: Query<(&Panel, &mut Transform)>,
) {
    for (b, mut visibility) in &mut bodies {
        let id = 1 + b.lane as u64 + b.ordinal as u64 * 2;
        let kind = shift
            .world
            .as_ref()
            .and_then(|w| w.vehicles.iter().find(|v| v.vehicle_id == id))
            .map(|v| v.kind.as_str())
            .unwrap_or(vehicle::scheduled_kind(id as usize - 1));
        *visibility = if b.kind == kind {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (p, mut transform) in &mut panels {
        let id = 1 + p.lane as u64 + p.ordinal as u64 * 2;
        let open = shift.world.as_ref().is_some_and(|w| {
            w.vehicles.iter().any(|v| {
                v.vehicle_id == id
                    && v.stage == VehicleStage::Stopped
                    && v.inspected_points.contains(&p.point)
            })
        });
        let target = if open {
            match p.point {
                VehiclePoint::Cargo => {
                    Quat::from_rotation_x(if p.truck_tailgate { 1.2 } else { -1.0 })
                }
                VehiclePoint::Hood => Quat::from_rotation_x(1.0),
                _ => Quat::from_rotation_y(1.0),
            }
        } else {
            p.base
        };
        transform.rotation = transform
            .rotation
            .slerp(target, (time.delta_secs() * 5.).min(1.));
    }
}

/// The same target selection is used by normal E interaction and the rendered acceptance walk.
pub fn interaction(
    world: &border_net::WorldSnapshot,
    peer: u64,
    origin: Vec3,
    direction: Vec3,
) -> Option<(u64, VehiclePoint)> {
    let permits = |lane| {
        world
            .lanes
            .iter()
            .any(|l| l.lane_id == lane && l.owner == Some(peer))
            || world
                .peers
                .iter()
                .any(|p| p.peer_id == peer && p.assisting == Some(lane))
    };
    world
        .vehicles
        .iter()
        .filter(|v| v.stage == VehicleStage::Stopped && permits(v.lane_id))
        .flat_map(|v| VehiclePoint::ALL.into_iter().map(move |point| (v, point)))
        .filter_map(|(v, point)| {
            let spec = vehicle::spec(&v.kind);
            if point == VehiclePoint::Driver && v.physics.npc.health == 0 {
                return None;
            }
            let outside = point == VehiclePoint::Driver
                && v.physics.npc.mode == border_net::combat::NpcMode::Outside;
            let stand = if outside {
                Vec3::from_array(v.physics.npc.position)
            } else {
                Vec3::from_array(spec.world_point(v.position, v.yaw, point))
            };
            let target = if outside {
                stand + Vec3::Y * 1.5
            } else {
                Vec3::from_array(spec.aim_point(v.position, v.yaw, point))
            };
            let distance = Vec2::new(origin.x - stand.x, origin.z - stand.z).length();
            (distance <= vehicle::INTERACTION_RANGE
                && (target - origin).normalize_or_zero().dot(direction) > 0.55)
                .then_some((v.vehicle_id, point, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(id, point, _)| (id, point))
}
