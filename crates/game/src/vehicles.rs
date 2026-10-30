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
    pub compartment: Option<border_scenarios::EvidenceLocation>,
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
    kind: &'static str,
) {
    let body = commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            Body {
                lane,
                ordinal,
                kind,
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
        Vec3::new(2.08, 0.58, 0.14),
        Vec3::new(0., 1.06, 0.03),
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
                compartment: None,
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
    kind: &str,
) {
    for (point, pivot, size, offset) in [
        (
            VehiclePoint::Cargo,
            Vec3::new(
                0.,
                0.96 + vehicle::cab_lift(kind),
                vehicle::rear_glass_z(kind),
            ),
            Vec3::new(
                2. * (vehicle::spec(kind).half_width - 0.03),
                0.07,
                (vehicle::spec(kind).half_length - vehicle::rear_glass_z(kind)).max(0.3),
            ),
            Vec3::new(
                0.,
                0.,
                (vehicle::spec(kind).half_length - vehicle::rear_glass_z(kind)).max(0.3) * 0.5,
            ),
        ),
        (
            VehiclePoint::Hood,
            Vec3::new(0., 0.96 + vehicle::cab_lift(kind), -1.05),
            Vec3::new(
                2. * (vehicle::spec(kind).half_width - 0.03),
                0.07,
                vehicle::spec(kind).half_length - 1.05,
            ),
            Vec3::new(0., 0., -(vehicle::spec(kind).half_length - 1.05) * 0.5),
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
                    && if let Some(location) = p.compartment {
                        v.searched_compartments.contains(&location)
                    } else {
                        v.open_panels.contains(&p.point)
                    }
            })
        });
        let target = if open {
            match p.point {
                VehiclePoint::Cargo => {
                    Quat::from_rotation_x(if p.truck_tailgate { 1.2 } else { -1.0 })
                }
                VehiclePoint::Hood => Quat::from_rotation_x(1.0),
                _ if p.compartment.is_some() => Quat::from_rotation_x(1.1),
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
            if point == VehiclePoint::Trailer && v.kind != "utility_truck" {
                return None;
            }
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

/// Shared close-up fittings plus silhouette-specific bodywork; all geometry is authored here.
#[allow(clippy::too_many_arguments)]
pub fn dressing(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    lane: u8,
    ordinal: usize,
    kind: &str,
    paint: Handle<StandardMaterial>,
) {
    let mut spec = vehicle::spec(kind);
    spec.half_length = vehicle::body_half_length(kind);
    let lift = vehicle::cab_lift(kind);
    let pickup = vehicle::pickup(kind);
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.06, 0.065),
        perceptual_roughness: 0.75,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.58, 0.60),
        metallic: 0.85,
        perceptual_roughness: 0.26,
        ..default()
    });
    let plate = materials.add(Color::srgb(0.89, 0.88, 0.76));
    let light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.86, 0.62),
        emissive: LinearRgba::rgb(0.1, 0.08, 0.04),
        ..default()
    });
    for (location, point, pivot, size, offset) in [
        (
            border_scenarios::EvidenceLocation::Glovebox,
            VehiclePoint::Passenger,
            Vec3::new(
                0.48,
                0.91 + if pickup { 0.35 } else { lift },
                spec.window_z - 0.385,
            ),
            Vec3::new(0.40, 0.20, 0.035),
            Vec3::new(0., 0.10, 0.),
        ),
        (
            border_scenarios::EvidenceLocation::Container,
            VehiclePoint::Cargo,
            Vec3::new(
                0.,
                if pickup { 1.46 } else { 0.91 },
                if pickup { 1.3 } else { 1.4 },
            ),
            Vec3::new(0.80, 0.04, 0.60),
            Vec3::new(0., 0., 0.30),
        ),
    ] {
        let hinge = commands
            .spawn((
                Transform::from_translation(pivot),
                Visibility::default(),
                Panel {
                    lane,
                    ordinal,
                    point,
                    base: Quat::IDENTITY,
                    truck_tailgate: false,
                    compartment: Some(location),
                },
            ))
            .id();
        let lid = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(dark.clone()),
                Transform::from_translation(offset),
            ))
            .id();
        commands.entity(hinge).add_child(lid);
        commands.entity(root).add_child(hinge);
    }
    if kind == "utility_truck" {
        for x in [-0.95_f32, 0.95] {
            let wheel = commands
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.35, 0.18).mesh().resolution(18))),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(x, 0.35, 4.5)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ))
                .id();
            commands.entity(root).add_child(wheel);
        }
    }
    let mut part = |size: Vec3, pos: Vec3, mat: Handle<StandardMaterial>| {
        let e = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(mat),
                Transform::from_translation(pos),
            ))
            .id();
        if kind == "utility_truck" && pos.z > 3.4 {
            commands.entity(e).insert(interactions::TrailerPiece(
                1 + lane as u64 + ordinal as u64 * 2,
            ));
        }
        commands.entity(root).add_child(e);
    };
    for (center, half) in vehicle::assembly_boxes(kind) {
        part(
            Vec3::from_array(half) * 2.,
            Vec3::from_array(center),
            paint.clone(),
        );
    }
    // A hollow, visibly attached cargo box; its lid is the searched container hinge.
    let crate_y = if pickup { 1.28 } else { 0.73 };
    let crate_z = if pickup { 1.6 } else { 1.7 };
    part(
        Vec3::new(0.80, 0.04, 0.60),
        Vec3::new(0., crate_y - 0.16, crate_z),
        dark.clone(),
    );
    for x in [-0.38, 0.38] {
        part(
            Vec3::new(0.04, 0.34, 0.60),
            Vec3::new(x, crate_y, crate_z),
            dark.clone(),
        );
    }
    for z in [-0.28, 0.28] {
        part(
            Vec3::new(0.80, 0.34, 0.04),
            Vec3::new(0., crate_y, crate_z + z),
            dark.clone(),
        );
    }
    part(
        Vec3::new(0.60, 0.04, 0.42),
        Vec3::new(0., crate_y - 0.1, crate_z),
        plate.clone(),
    );
    if kind == "utility_truck" {
        part(
            Vec3::new(1.3, 0.6, 1.3),
            Vec3::new(0., 1.12, 4.5),
            dark.clone(),
        );
        part(
            Vec3::new(0.40, 0.02, 0.25),
            Vec3::new(0., 1.43, 4.5),
            plate.clone(),
        );
    }
    // Plates, grille slats, hood seams, wipers, sills and fuel cover are visible at officer distance.
    for z in [-spec.half_length - 0.035, spec.half_length + 0.035] {
        part(
            Vec3::new(0.49, 0.16, 0.018),
            Vec3::new(0., 0.58, z),
            plate.clone(),
        );
        for i in 0..7 {
            let h = if (i + ordinal + lane as usize).is_multiple_of(3) {
                0.07
            } else {
                0.095
            };
            part(
                Vec3::new(0.025, h, 0.006),
                Vec3::new(-0.16 + i as f32 * 0.05, 0.58, z + z.signum() * 0.012),
                dark.clone(),
            );
        }
    }
    for i in 0..9 {
        part(
            Vec3::new(0.085, 0.16, 0.04),
            Vec3::new(
                -0.62 + i as f32 * 0.155,
                0.83 + lift,
                -spec.half_length - 0.02,
            ),
            metal.clone(),
        );
    }
    for x in [-0.5, 0.5] {
        part(
            Vec3::new(0.55, 0.025, 0.018),
            Vec3::new(x, 1.02 + lift, if pickup { -2.13 } else { -1.07 }),
            dark.clone(),
        );
    }
    for x in [-spec.half_width, spec.half_width] {
        part(
            Vec3::new(0.025, 0.08, spec.half_length * 1.6),
            Vec3::new(x, 0.55, 0.),
            dark.clone(),
        );
        part(
            Vec3::new(0.012, 0.21, 0.018),
            Vec3::new(x * 1.001, 0.75, 0.28),
            dark.clone(),
        );
        part(
            Vec3::new(0.012, 0.18, 0.20),
            Vec3::new(x * 1.005, 0.84, spec.half_length - 0.65),
            paint.clone(),
        );
    }
    if matches!(kind, "suv" | "van") {
        let rear = vehicle::rear_glass_z(kind);
        let width = spec.half_width - 0.15;
        part(
            Vec3::new(width * 2., 0.12, rear + 1.1),
            Vec3::new(0., 1.65 + lift, (rear - 1.1) * 0.5),
            paint.clone(),
        );
        for x in [-width, width] {
            part(
                Vec3::new(0.12, 0.30 + lift, rear - 0.2),
                Vec3::new(x, 0.80 + lift * 0.5, (rear + 0.2) * 0.5),
                paint.clone(),
            );
            part(
                Vec3::new(0.09, 0.72, 0.09),
                Vec3::new(x, 1.28 + lift, rear),
                paint.clone(),
            );
            part(
                Vec3::new(0.09, 0.07, rear - 0.25),
                Vec3::new(x, 1.61 + lift, (rear + 0.25) * 0.5),
                paint.clone(),
            );
        }
        for z in [0.55, rear - 0.35] {
            part(
                Vec3::new(width * 1.95, 0.08, 0.09),
                Vec3::new(0., 1.78 + lift, z),
                dark.clone(),
            );
        }
        part(
            Vec3::new(width * 1.96, 0.65, 0.08),
            Vec3::new(0., 0.92, rear + 0.03),
            paint.clone(),
        );
        part(
            Vec3::new(0.45, 0.055, 0.025),
            Vec3::new(0., 1.1, rear + 0.09),
            metal.clone(),
        );
        if kind == "van" {
            for x in [-0.58, 0., 0.58] {
                part(
                    Vec3::new(0.035, 0.018, 3.5),
                    Vec3::new(x, 2.27, 0.7),
                    metal.clone(),
                );
            }
            part(
                Vec3::new(0.025, 0.8, 0.035),
                Vec3::new(0., 1., spec.half_length),
                dark.clone(),
            );
        }
    }
    if kind == "hatchback" {
        part(
            Vec3::new(1.65, 0.07, 0.3),
            Vec3::new(0., 1.57, 1.3),
            paint.clone(),
        );
        part(
            Vec3::new(0.45, 0.025, 0.025),
            Vec3::new(0., 1.25, 1.22),
            dark.clone(),
        );
    }
    if kind == "utility_truck" {
        part(
            Vec3::new(1.95, 0.6, 0.55),
            Vec3::new(0., 1.38, 0.65),
            metal.clone(),
        );
        for x in [-0.9, 0.9] {
            for z in [0.5, 2.7] {
                part(
                    Vec3::new(0.07, 1.25, 0.07),
                    Vec3::new(x, 1.95, z),
                    metal.clone(),
                );
            }
            part(
                Vec3::new(0.07, 0.07, 2.4),
                Vec3::new(x, 2.6, 1.6),
                metal.clone(),
            );
        }
        for z in [0.5, 1.6, 2.7] {
            part(
                Vec3::new(1.85, 0.06, 0.06),
                Vec3::new(0., 2.6, z),
                metal.clone(),
            );
        }
        part(
            Vec3::new(0.65, 0.25, 0.4),
            Vec3::new(0., 1.9, 2.6),
            dark.clone(),
        );
    }
    let seat_lift = if pickup { 0.35 } else { lift };
    for x in [-0.48, 0.48] {
        part(
            Vec3::new(0.52, 0.20, 0.66),
            Vec3::new(x, 0.55 + seat_lift, spec.window_z + 0.1),
            dark.clone(),
        );
        part(
            Vec3::new(0.52, 0.56, 0.14),
            Vec3::new(x, 1.03 + seat_lift, spec.window_z + 0.28),
            dark.clone(),
        );
        part(
            Vec3::new(0.28, 0.20, 0.14),
            Vec3::new(x, 1.37 + seat_lift, spec.window_z + 0.28),
            dark.clone(),
        );
        for z in [-0.1, 0.25] {
            part(
                Vec3::new(0.38, 0.22 + seat_lift, 0.07),
                Vec3::new(x, 0.41 + seat_lift * 0.5, spec.window_z + z),
                metal.clone(),
            );
        }
    }
    part(
        Vec3::new(0.06, 0.06, 0.22),
        Vec3::new(-0.48, 1.04 + seat_lift, spec.window_z - 0.49),
        dark.clone(),
    );
    // Interior dash, glove box and radio cluster are separate from the damageable driver.
    part(
        Vec3::new(1.7, 0.22, 0.3),
        Vec3::new(0., 0.96 + seat_lift, spec.window_z - 0.58),
        dark.clone(),
    );
    part(
        Vec3::new(0.34, 0.10, 0.035),
        Vec3::new(0.2, 1.04 + seat_lift, spec.window_z - 0.4),
        metal.clone(),
    );
    part(
        Vec3::new(0.27, 0.07, 0.02),
        Vec3::new(0.2, 1.04 + seat_lift, spec.window_z - 0.375),
        dark.clone(),
    );
    for x in [-0.7, 0.7] {
        part(
            Vec3::new(0.18, 0.07, 0.025),
            Vec3::new(x, 1.03 + seat_lift, spec.window_z - 0.4),
            metal.clone(),
        );
    }
    for i in 0..14 {
        let a = i as f32 * std::f32::consts::TAU / 14.;
        part(
            Vec3::new(0.055, 0.035, 0.035),
            Vec3::new(
                -0.48 + a.cos() * 0.15,
                1.04 + seat_lift + a.sin() * 0.13,
                spec.window_z - 0.4,
            ),
            dark.clone(),
        );
    }
    for x in [-0.8, 0.8] {
        part(
            Vec3::new(0.28, 0.07, 0.06),
            Vec3::new(x, 0.68, -spec.half_length - 0.04),
            light.clone(),
        );
    }
}
