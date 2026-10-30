//! Snapshot-driven articulated doors, fractured glazing, seated/exiting travelers and debris.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};
use border_net::vehicle;
use border_net::{
    VehicleStage,
    combat::{self as c, *},
};
#[derive(Component)]
pub struct Detail {
    pub vehicle: u64,
    pub kind: DetailKind,
}
#[derive(Clone, Copy)]
pub enum DetailKind {
    Door(Side),
    Glass(Pane),
    Seal(Pane),
    Npc,
    Bone(u8),
}
#[derive(Component)]
pub struct GlassVisual {
    revision: usize,
    cracks: Handle<Mesh>,
}
#[derive(Component)]
pub struct BodyPart {
    bone: usize,
}
#[derive(Component)]
pub struct Debris {
    velocity: Vec3,
    spin: Vec3,
    ttl: f32,
}
#[derive(Resource, Default)]
pub struct Effects {
    seen: std::collections::HashSet<u64>,
    epoch: Option<u64>,
}
fn mat(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.65,
        ..default()
    })
}
fn block(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    parent: Entity,
    size: Vec3,
    pos: Vec3,
    material: Handle<StandardMaterial>,
) -> Entity {
    let e = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
            Transform::from_translation(pos),
        ))
        .id();
    commands.entity(parent).add_child(e);
    e
}
fn door(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: u64,
    side: Side,
    kind: &str,
) -> Entity {
    let root = commands
        .spawn((
            Transform::default(),
            Visibility::Hidden,
            Detail {
                vehicle: id,
                kind: DetailKind::Door(side),
            },
        ))
        .id();
    let sign = if side == Side::Driver { -1. } else { 1. };
    let layout = vehicle::door_layout(kind);
    let len = layout.length;
    let paint = materials.add(StandardMaterial {
        base_color: match (((id - 1) / 2) + (id - 1) % 2) % 5 {
            0 => Color::srgb(0.22, 0.41, 0.39),
            1 => Color::srgb(0.69, 0.42, 0.28),
            2 => Color::srgb(0.81, 0.76, 0.60),
            3 => Color::srgb(0.36, 0.4, 0.44),
            _ => Color::srgb(0.55, 0.25, 0.18),
        },
        metallic: 0.42,
        perceptual_roughness: 0.27,
        clearcoat: 1.,
        ..default()
    });
    let liner = mat(materials, Color::srgb(0.11, 0.12, 0.12));
    let chrome = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.62, 0.64),
        metallic: 0.85,
        ..default()
    });
    let belt = layout.belt;
    let top = layout.top;
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.07, belt - 0.55, len),
        Vec3::new(0., (belt + 0.55) * 0.5, len / 2.),
        paint.clone(),
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.025, 0.37, len - 0.1),
        Vec3::new(-sign * 0.05, belt - 0.25, len / 2.),
        liner.clone(),
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.1, 0.08, 0.45),
        Vec3::new(-sign * 0.08, belt - 0.2, len * 0.5),
        liner.clone(),
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.035, 0.035, 0.22),
        Vec3::new(sign * 0.05, belt - 0.08, len * 0.8),
        chrome.clone(),
    );
    for z in [0.02, len - 0.02] {
        block(
            commands,
            meshes,
            root,
            Vec3::new(0.055, top - belt, 0.055),
            Vec3::new(0., (belt + top) * 0.5, z),
            paint.clone(),
        );
    }
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.055, 0.055, len),
        Vec3::new(0., top, len / 2.),
        paint,
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.06, 0.025, len),
        Vec3::new(0., belt, len / 2.),
        liner.clone(),
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.18, 0.055, 0.07),
        Vec3::new(sign * 0.09, belt + 0.13, 0.12),
        liner.clone(),
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.17, 0.13, 0.25),
        Vec3::new(sign * 0.20, belt + 0.13, 0.12),
        liner,
    );
    block(
        commands,
        meshes,
        root,
        Vec3::new(0.15, 0.10, 0.015),
        Vec3::new(sign * 0.20, belt + 0.13, 0.25),
        chrome,
    );
    root
}
fn bone_mesh(meshes: &mut Assets<Mesh>, bone: usize) -> Handle<Mesh> {
    let b = border_net::rig::skeleton(false).bones[bone];
    match bone {
        0..=2 => {
            let size = match bone {
                0 => Vec3::new(0.21, 0.145, 0.16),
                1 => Vec3::new(0.235, 0.25, 0.17),
                _ => Vec3::new(0.15, 0.17, 0.145),
            };
            let mut mesh = Sphere::new(1.).mesh().uv(20, 16);
            mesh.scale_by(size);
            meshes.add(mesh)
        }
        // Rounded sleeves/skin overlap at the joint instead of ending short of it.
        _ => meshes.add(Capsule3d::new(b.radius, b.length - b.radius)),
    }
}
fn npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: u64,
) {
    let root = commands
        .spawn((
            Transform::default(),
            Visibility::Hidden,
            Detail {
                vehicle: id,
                kind: DetailKind::Npc,
            },
        ))
        .id();
    let skin = mat(
        materials,
        match id % 3 {
            0 => Color::srgb(0.43, 0.29, 0.20),
            1 => Color::srgb(0.75, 0.55, 0.40),
            _ => Color::srgb(0.62, 0.42, 0.28),
        },
    );
    let shirt = mat(
        materials,
        match id % 4 {
            0 => Color::srgb(0.23, 0.3, 0.38),
            1 => Color::srgb(0.5, 0.32, 0.2),
            2 => Color::srgb(0.34, 0.4, 0.27),
            _ => Color::srgb(0.48, 0.26, 0.23),
        },
    );
    let pants = mat(materials, Color::srgb(0.12, 0.15, 0.18));
    let dark = mat(materials, Color::srgb(0.08, 0.07, 0.055));
    for bone in 0..border_net::rig::BONE_COUNT {
        let material = if bone == 2 || matches!(bone, 4 | 6) {
            skin.clone()
        } else if bone == 0 || bone >= 7 {
            pants.clone()
        } else {
            shirt.clone()
        };
        let e = commands
            .spawn((
                Mesh3d(bone_mesh(meshes, bone)),
                MeshMaterial3d(material),
                Transform::default(),
                BodyPart { bone },
            ))
            .id();
        commands.entity(root).add_child(e);
        extremities(commands, meshes, e, bone, skin.clone(), dark.clone());
    }
}
fn extremities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    bone: usize,
    skin: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
) {
    if bone == 2 {
        block(
            commands,
            meshes,
            root,
            Vec3::new(0.25, 0.05, 0.18),
            Vec3::new(0., 0.15, 0.02),
            dark.clone(),
        );
        for x in [-0.065, 0.065] {
            block(
                commands,
                meshes,
                root,
                Vec3::new(0.035, 0.018, 0.015),
                Vec3::new(x, 0.03, -0.14),
                dark.clone(),
            );
        }
        block(
            commands,
            meshes,
            root,
            Vec3::new(0.045, 0.06, 0.04),
            Vec3::new(0., -0.015, -0.145),
            skin,
        );
    } else if matches!(bone, 4 | 6) {
        block(
            commands,
            meshes,
            root,
            Vec3::new(0.10, 0.12, 0.08),
            Vec3::new(0., -0.17, 0.),
            skin,
        );
    } else if matches!(bone, 8 | 10) {
        block(
            commands,
            meshes,
            root,
            Vec3::new(0.15, 0.10, 0.26),
            Vec3::new(0., -0.17, -0.065),
            dark,
        );
    }
}
fn triangle_mesh(triangles: &[[[f32; 2]; 3]], z: f32) -> Mesh {
    let points: Vec<[f32; 3]> = triangles
        .iter()
        .flat_map(|t| t.iter().map(|p| [p[0], p[1], z]))
        .collect();
    let n = points.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; n])
    .with_inserted_indices(Indices::U32((0..n as u32).collect()))
}
fn grid() -> Vec<[[f32; 2]; 3]> {
    let mut t = vec![];
    for x in 0..12 {
        for y in 0..8 {
            let a = [-1. + x as f32 / 6., -1. + y as f32 / 4.];
            let b = [a[0] + 1. / 6., a[1]];
            let c = [a[0] + 1. / 6., a[1] + 0.25];
            let d = [a[0], a[1] + 0.25];
            t.push([a, b, c]);
            t.push([a, c, d]);
        }
    }
    t
}
fn crack_mesh(impacts: &[GlassImpact]) -> Mesh {
    let mut t = vec![];
    let mut line = |a: [f32; 2], b: [f32; 2], width: f32| {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let l = dx.hypot(dy).max(0.001);
        let n = [-dy / l * width, dx / l * width];
        let p = [
            [a[0] + n[0], a[1] + n[1]],
            [a[0] - n[0], a[1] - n[1]],
            [b[0] + n[0], b[1] + n[1]],
            [b[0] - n[0], b[1] - n[1]],
        ];
        t.push([p[0], p[1], p[2]]);
        t.push([p[2], p[1], p[3]]);
    };
    for h in impacts {
        for i in 0..14 {
            let angle = i as f32 * std::f32::consts::TAU / 14. + (h.seed % 11) as f32 * 0.031;
            let direction = [angle.cos(), angle.sin()];
            let mut a = h.uv;
            for j in 1..=5 {
                let r = j as f32 * 0.17;
                let bend = if j % 2 == 0 { 0.025 } else { -0.025 };
                let b = [
                    (h.uv[0] + direction[0] * r + direction[1] * bend).clamp(-1., 1.),
                    (h.uv[1] + direction[1] * r - direction[0] * bend).clamp(-1., 1.),
                ];
                line(a, b, 0.004 / (j as f32).sqrt());
                if j % 2 == 0 {
                    line(
                        b,
                        [
                            (b[0] + angle.sin() * 0.12).clamp(-1., 1.),
                            (b[1] - angle.cos() * 0.12).clamp(-1., 1.),
                        ],
                        0.002,
                    );
                }
                a = b;
            }
        }
        for i in 0..18 {
            let a = i as f32 * std::f32::consts::TAU / 18.;
            let b = (i + 1) as f32 * std::f32::consts::TAU / 18.;
            line(
                [h.uv[0] + a.cos() * 0.19, h.uv[1] + a.sin() * 0.19],
                [h.uv[0] + b.cos() * 0.19, h.uv[1] + b.sin() * 0.19],
                0.003,
            );
        }
    }
    triangle_mesh(&t, 0.003)
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync(
    shift: Res<shift::Shift>,
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut details: Query<
        (
            Entity,
            &Detail,
            &mut Transform,
            &mut Visibility,
            Option<&mut GlassVisual>,
            Option<&Mesh3d>,
        ),
        Without<BodyPart>,
    >,
    mut parts: Query<(&BodyPart, &ChildOf, &mut Transform), Without<Detail>>,
    roots: Query<(Entity, &Transform, &traffic::Vehicle), (Without<Detail>, Without<BodyPart>)>,
) {
    let Some(world) = &shift.world else {
        return;
    };
    let poses: std::collections::HashMap<_, _> = roots
        .iter()
        .map(|(e, t, v)| (1 + v.lane as u64 + v.ordinal as u64 * 2, (e, *t)))
        .collect();
    for v in &world.vehicles {
        if v.stage == VehicleStage::Exited {
            continue;
        }
        if !details.iter().any(|(_, d, ..)| d.vehicle == v.vehicle_id) {
            for side in [Side::Driver, Side::Passenger] {
                let door_root = door(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    v.vehicle_id,
                    side,
                    &v.kind,
                );
                if let Some((parent, _)) = poses.get(&v.vehicle_id) {
                    commands.entity(*parent).add_child(door_root);
                }
            }
            npc(&mut commands, &mut meshes, &mut materials, v.vehicle_id);
            for pane in Pane::ALL {
                if !matches!(pane, Pane::DriverFront | Pane::PassengerFront) {
                    let seal = mat(&mut materials, Color::srgb(0.045, 0.05, 0.05));
                    let frame = commands
                        .spawn((
                            Transform::default(),
                            Visibility::Hidden,
                            Detail {
                                vehicle: v.vehicle_id,
                                kind: DetailKind::Seal(pane),
                            },
                        ))
                        .id();
                    if let Some((parent, _)) = poses.get(&v.vehicle_id) {
                        commands.entity(*parent).add_child(frame);
                    }
                    for x in [-1., 1.] {
                        block(
                            &mut commands,
                            &mut meshes,
                            frame,
                            Vec3::new(0.07, 2.08, 0.045),
                            Vec3::new(x, 0., 0.),
                            seal.clone(),
                        );
                    }
                    for y in [-1., 1.] {
                        block(
                            &mut commands,
                            &mut meshes,
                            frame,
                            Vec3::new(2.08, 0.09, 0.045),
                            Vec3::new(0., y, 0.),
                            seal.clone(),
                        );
                    }
                }
                let glass = materials.add(StandardMaterial {
                    base_color: Color::srgba(0.52, 0.76, 0.8, 0.25),
                    alpha_mode: AlphaMode::Blend,
                    metallic: 0.15,
                    perceptual_roughness: 0.10,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                });
                let cracks = meshes.add(crack_mesh(&[]));
                let cracks_material = materials.add(StandardMaterial {
                    base_color: Color::srgba(0.90, 0.96, 1., 0.90),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                });
                let root = commands
                    .spawn((
                        Mesh3d(meshes.add(triangle_mesh(&grid(), 0.))),
                        MeshMaterial3d(glass),
                        Transform::default(),
                        Visibility::Hidden,
                        Detail {
                            vehicle: v.vehicle_id,
                            kind: DetailKind::Glass(pane),
                        },
                        GlassVisual {
                            revision: usize::MAX,
                            cracks: cracks.clone(),
                        },
                    ))
                    .id();
                if let Some((parent, _)) = poses.get(&v.vehicle_id) {
                    commands.entity(*parent).add_child(root);
                }
                let e = commands
                    .spawn((
                        Mesh3d(cracks),
                        MeshMaterial3d(cracks_material),
                        Transform::default(),
                    ))
                    .id();
                commands.entity(root).add_child(e);
            }
        }
    }
    for v in &world.vehicles {
        if v.stage == VehicleStage::Exited
            || v.physics.npc.ragdoll.is_empty()
            || details.iter().any(|(_, d, ..)| {
                d.vehicle == v.vehicle_id && matches!(d.kind, DetailKind::Bone(0))
            })
        {
            continue;
        }
        let shirt = materials.add(match v.vehicle_id % 4 {
            0 => Color::srgb(0.23, 0.3, 0.38),
            1 => Color::srgb(0.5, 0.32, 0.2),
            2 => Color::srgb(0.34, 0.4, 0.27),
            _ => Color::srgb(0.48, 0.26, 0.23),
        });
        let pants = materials.add(Color::srgb(0.12, 0.15, 0.18));
        let skin = materials.add(match v.vehicle_id % 3 {
            0 => Color::srgb(0.43, 0.29, 0.20),
            1 => Color::srgb(0.75, 0.55, 0.40),
            _ => Color::srgb(0.62, 0.42, 0.28),
        });
        let dark = materials.add(Color::srgb(0.08, 0.07, 0.055));
        for bone in 0..11u8 {
            let mesh = bone_mesh(&mut meshes, bone as usize);
            let material = if bone == 2 || matches!(bone, 4 | 6) {
                skin.clone()
            } else if bone == 0 || bone >= 7 {
                pants.clone()
            } else {
                shirt.clone()
            };
            let root = commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(Vec3::from_array(
                        v.physics.npc.ragdoll[bone as usize].position,
                    ))
                    .with_rotation(Quat::from_array(
                        v.physics.npc.ragdoll[bone as usize].rotation,
                    )),
                    Visibility::Hidden,
                    Detail {
                        vehicle: v.vehicle_id,
                        kind: DetailKind::Bone(bone),
                    },
                ))
                .id();
            extremities(
                &mut commands,
                &mut meshes,
                root,
                bone as usize,
                skin.clone(),
                dark.clone(),
            );
        }
    }
    for (e, d, mut t, mut visible, glass, mesh) in &mut details {
        let Some(v) = world
            .vehicles
            .iter()
            .find(|v| v.vehicle_id == d.vehicle && v.stage != VehicleStage::Exited)
        else {
            commands.entity(e).despawn();
            continue;
        };
        *visible = Visibility::Visible;
        let display = poses.get(&d.vehicle).map(|(_, t)| *t).unwrap_or_else(|| {
            Transform::from_translation(Vec3::from_array(v.position))
                .with_rotation(Quat::from_rotation_y(v.yaw))
        });
        let rotation_delta = display.rotation * Quat::from_rotation_y(v.yaw).inverse();
        let displayed = |point: V3| {
            display.translation
                + rotation_delta * (Vec3::from_array(point) - Vec3::from_array(v.position))
        };
        match d.kind {
            DetailKind::Door(side) => {
                let angle = v.physics.door_angles[side_index(side)];
                t.translation = Vec3::from_array(door_pivot(&v.kind, side));
                t.rotation =
                    Quat::from_rotation_y(if side == Side::Driver { -angle } else { angle });
            }
            DetailKind::Seal(pane) => {
                let mut closed = v.clone();
                closed.physics.glass[pane.index()].lowered = 0.;
                if let Some(f) = pane_frame(&closed, pane) {
                    t.translation = Vec3::from_array(c::local(v, f.center));
                    t.rotation = Quat::from_rotation_y(-v.yaw)
                        * Quat::from_mat3(&Mat3::from_cols(
                            Vec3::from_array(f.u),
                            Vec3::from_array(f.v),
                            Vec3::from_array(f.normal),
                        ));
                    t.scale = Vec3::new(f.half[0], f.half[1], 1.);
                }
            }
            DetailKind::Glass(pane) => {
                let g = &v.physics.glass[pane.index()];
                let revision = g.impacts.len() + if g.shattered { 100 } else { 0 };
                if let Some(mut visual) = glass
                    && let Some(mesh) = mesh
                    && visual.revision != revision
                {
                    let cells: Vec<_> = grid()
                        .into_iter()
                        .filter(|cell| {
                            !g.shattered
                                && !g.impacts.iter().any(|h| {
                                    let center = [
                                        (cell[0][0] + cell[1][0] + cell[2][0]) / 3.,
                                        (cell[0][1] + cell[1][1] + cell[2][1]) / 3.,
                                    ];
                                    (center[0] - h.uv[0]).hypot(center[1] - h.uv[1]) < 0.15
                                })
                        })
                        .collect();
                    if let Some(m) = meshes.get_mut(&mesh.0) {
                        *m = triangle_mesh(&cells, 0.);
                    }
                    if let Some(m) = meshes.get_mut(&visual.cracks) {
                        *m = crack_mesh(&g.impacts);
                    }
                    visual.revision = revision;
                }
                if g.shattered {
                    *visible = Visibility::Hidden;
                }
                if let Some(f) = pane_frame(v, pane) {
                    t.translation = Vec3::from_array(c::local(v, f.center));
                    t.rotation = Quat::from_rotation_y(-v.yaw)
                        * Quat::from_mat3(&Mat3::from_cols(
                            Vec3::from_array(f.u).normalize(),
                            Vec3::from_array(f.v).normalize(),
                            Vec3::from_array(f.normal).normalize(),
                        ));
                    t.scale = Vec3::new(f.half[0], f.half[1], 1.);
                } else {
                    *visible = Visibility::Hidden;
                }
            }
            DetailKind::Bone(bone) => {
                if let Some(b) = v.physics.npc.ragdoll.iter().find(|p| p.bone == bone) {
                    let position = Vec3::from_array(b.position);
                    let rotation = Quat::from_array(b.rotation);
                    t.translation = position;
                    t.rotation = rotation;
                } else {
                    *visible = Visibility::Hidden;
                }
            }
            DetailKind::Npc => {
                let n = &v.physics.npc;
                if !n.ragdoll.is_empty() {
                    *visible = Visibility::Hidden;
                    continue;
                }
                if n.mode == NpcMode::Seated {
                    t.translation = displayed(c::seated_position(v));
                    t.rotation = display.rotation;
                } else {
                    t.translation = t.translation.lerp(
                        Vec3::from_array(n.position),
                        (time.delta_secs() * 20.).min(1.),
                    );
                    t.rotation = t.rotation.slerp(
                        Quat::from_rotation_y(n.yaw),
                        (time.delta_secs() * 20.).min(1.),
                    );
                }
                if n.mode == NpcMode::Dead && !n.died_seated {
                    t.translation.y = 0.22;
                    t.rotation *= Quat::from_rotation_z(-1.48);
                }
            }
        }
    }
    for (part, parent, mut t) in &mut parts {
        let Ok((_, d, ..)) = details.get(parent.parent()) else {
            continue;
        };
        let Some(v) = world.vehicles.iter().find(|v| v.vehicle_id == d.vehicle) else {
            continue;
        };
        let n = &v.physics.npc;
        let seated = n.mode == NpcMode::Seated || n.mode == NpcMode::Dead && n.died_seated;
        let rig = border_net::rig::skeleton(seated);
        let bone = rig.bones[part.bone];
        t.translation = Vec3::from_array(bone.center);
        t.rotation = Quat::from_rotation_x(bone.angle);
    }
}
#[allow(clippy::too_many_arguments)]
pub fn effects(
    shift: Res<shift::Shift>,
    patrol: Res<Patrol>,
    smoke: Option<Res<SmokeTest>>,
    time: Res<Time>,
    mut state: ResMut<Effects>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut debris: Query<(Entity, &mut Transform, &mut Debris)>,
) {
    let Some(world) = &shift.world else {
        return;
    };
    if state.epoch != world.shift_started_ms {
        state.seen.clear();
        state.epoch = world.shift_started_ms;
        for (e, ..) in &debris {
            commands.entity(e).despawn();
        }
    }
    for impact in &world.impacts {
        if !state.seen.insert(impact.id) {
            continue;
        }
        if world.server_time_ms.saturating_sub(impact.at_ms) > 500 {
            continue;
        }
        let glass = impact.kind.starts_with("glass");
        if glass
            && !patrol.muted
            && smoke.is_none()
            && shift
                .me()
                .is_some_and(|p| c::length(c::sub(p.position, impact.position)) < 20.)
        {
            weapon::sound("glass");
        }
        let count = if glass
            || matches!(
                impact.kind.as_str(),
                "npc_hit" | "ragdoll_hit" | "officer_hit"
            ) {
            0
        } else {
            8
        };
        let material = materials.add(StandardMaterial {
            base_color: if glass {
                Color::srgba(0.65, 0.88, 0.92, 0.7)
            } else {
                Color::srgb(1., 0.65, 0.16)
            },
            alpha_mode: if glass {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            metallic: 0.5,
            double_sided: true,
            cull_mode: None,
            unlit: !glass,
            ..default()
        });
        let shard_mesh = meshes.add(triangle_mesh(
            &[[[-0.5, -0.3], [0.5, -0.2], [0.0, 0.65]]],
            0.,
        ));
        for j in 0..count {
            let seed = impact.id.wrapping_mul(7919) + j * 313;
            let a = (seed % 1000) as f32 * 0.013;
            let b = ((seed / 7) % 100) as f32 / 100.;
            commands.spawn((
                Mesh3d(shard_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::from_array(impact.position))
                    .with_scale(Vec3::splat(if glass { 0.035 + b * 0.07 } else { 0.025 })),
                bevy::light::NotShadowCaster,
                Debris {
                    velocity: Vec3::new(
                        a.cos() * (1. + b * 2.),
                        0.7 + b * 2.,
                        a.sin() * (1. + b * 2.),
                    ),
                    spin: Vec3::new(a, 3., a * 0.7),
                    ttl: if glass { 2.4 } else { 0.35 },
                },
            ));
        }
        if !glass
            && impact.kind != "npc_hit"
            && impact.kind != "officer_hit"
            && impact.kind != "ragdoll_hit"
        {
            let m = materials.add(StandardMaterial {
                base_color: Color::srgb(0.025, 0.025, 0.02),
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                Mesh3d(meshes.add(Circle::new(0.035))),
                MeshMaterial3d(m),
                Transform::from_translation(
                    Vec3::from_array(impact.position) + Vec3::from_array(impact.normal) * 0.015,
                )
                .with_rotation(Quat::from_rotation_arc(
                    Vec3::Z,
                    Vec3::from_array(impact.normal),
                )),
                Debris {
                    velocity: Vec3::ZERO,
                    spin: Vec3::ZERO,
                    ttl: 15.,
                },
            ));
        }
    }
    if state.seen.len() > 2048 {
        let live: std::collections::HashSet<_> = world.impacts.iter().map(|i| i.id).collect();
        state.seen.retain(|id| live.contains(id));
    }
    for (e, mut t, mut d) in &mut debris {
        d.ttl -= time.delta_secs();
        if d.ttl <= 0. {
            commands.entity(e).despawn();
            continue;
        }
        if d.spin != Vec3::ZERO {
            d.velocity.y -= 9.8 * time.delta_secs();
            t.translation += d.velocity * time.delta_secs();
            t.rotate(Quat::from_euler(
                EulerRot::XYZ,
                d.spin.x * time.delta_secs(),
                d.spin.y * time.delta_secs(),
                d.spin.z * time.delta_secs(),
            ));
            if t.translation.y < 0.04 {
                t.translation.y = 0.04;
                d.velocity *= 0.25;
                d.velocity.y = d.velocity.y.abs() * 0.3;
            }
        }
    }
}

#[derive(Component)]
pub struct PhysicalShard(pub u64);
#[derive(Resource, Default)]
pub struct ShardAssets {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
}
/// Glass fragments are host-simulated thin rigid bodies, not client-only particle sprites.
pub fn physical_shards(
    shift: Res<shift::Shift>,
    time: Res<Time>,
    mut assets: ResMut<ShardAssets>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shards: Query<(Entity, &PhysicalShard, &mut Transform)>,
) {
    let Some(world) = &shift.world else {
        return;
    };
    if assets.mesh.is_none() {
        assets.mesh = Some(meshes.add(triangle_mesh(
            &[[[-0.5, -0.5], [0.5, -0.3], [0.1, 0.5]]],
            0.,
        )));
        assets.material = Some(materials.add(StandardMaterial {
            base_color: Color::srgba(0.64, 0.85, 0.88, 0.55),
            alpha_mode: AlphaMode::Blend,
            metallic: 0.15,
            perceptual_roughness: 0.18,
            double_sided: true,
            cull_mode: None,
            ..default()
        }));
    }
    for s in &world.shards {
        if !shards.iter().any(|(_, id, _)| id.0 == s.id) {
            commands.spawn((
                Mesh3d(assets.mesh.as_ref().unwrap().clone()),
                MeshMaterial3d(assets.material.as_ref().unwrap().clone()),
                Transform::from_translation(Vec3::from_array(s.position))
                    .with_rotation(Quat::from_array(s.rotation))
                    .with_scale(Vec3::from_array(s.size)),
                bevy::light::NotShadowCaster,
                PhysicalShard(s.id),
            ));
        }
    }
    for (entity, id, mut t) in &mut shards {
        if let Some(s) = world.shards.iter().find(|s| s.id == id.0) {
            t.translation = t.translation.lerp(
                Vec3::from_array(s.position),
                (time.delta_secs() * 25.).min(1.),
            );
            t.rotation = t.rotation.slerp(
                Quat::from_array(s.rotation),
                (time.delta_secs() * 25.).min(1.),
            );
        } else {
            commands.entity(entity).despawn();
        }
    }
}
