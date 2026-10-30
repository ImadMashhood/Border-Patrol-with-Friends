use super::*;
use bevy::{
    asset::RenderAssetUsages,
    light::NotShadowCaster,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};

// X is east/west; +Z leads south across the river. Texas is the north bank.
pub fn walkable(p: Vec3) -> bool {
    border_net::map::walkable(p.x, p.z)
}
#[derive(Resource, Default)]
pub struct Landmarks(pub Vec<(&'static str, Vec3)>);
#[derive(Component)]
pub struct RiverRipple {
    origin: Vec3,
    phase: f32,
}
#[derive(Component)]
pub struct Dust {
    origin: Vec3,
    phase: f32,
}

const SAND: usize = 0;
const STONE: usize = 1;
const ROAD: usize = 2;
const CREAM: usize = 3;
const DARK: usize = 4;
const RUST: usize = 5;
const GOLD: usize = 6;
const GREEN: usize = 7;
const GLASS: usize = 8;
const RED: usize = 9;
const WATER: usize = 10;
const FOAM: usize = 11;
const SAGE: usize = 12;
const CACTUS: usize = 13;
const CLAY: usize = 14;
const HAZE: usize = 16;
const WHITE: usize = 17;

const RUBBER: usize = 19;
const ROCK: usize = 20;
const ROCK_DARK: usize = 21;
const WOOD: usize = 22;
const CLOUD: usize = 23;
const CLOUD_SHADE: usize = 24;
const GRAVEL: usize = 25;
const CINDER: usize = 26;
#[derive(Component)]
pub struct Cloud {
    origin: Vec3,
    phase: f32,
}
struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    cube: Handle<Mesh>,
    round: Handle<Mesh>,
    cone: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
    rocks: Vec<Handle<Mesh>>,
    cloud: Handle<Mesh>,
    material_assets: &'a mut Assets<StandardMaterial>,
    wall_materials: std::collections::HashMap<(usize, u32, u32), Handle<StandardMaterial>>,
}
impl<'a, 'w, 's> Builder<'a, 'w, 's> {
    fn new(
        commands: &'a mut Commands<'w, 's>,
        meshes: &mut Assets<Mesh>,
        material_assets: &'a mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
    ) -> Self {
        let colors = [
            (0.68, 0.51, 0.32),
            (0.72, 0.68, 0.55),
            (0.20, 0.23, 0.23),
            (0.91, 0.84, 0.65),
            (0.13, 0.20, 0.20),
            (0.42, 0.25, 0.17),
            (0.96, 0.66, 0.19),
            (0.17, 0.31, 0.25),
            (0.22, 0.41, 0.43),
            (0.72, 0.24, 0.16),
            (0.20, 0.47, 0.48),
            (0.49, 0.67, 0.60),
            (0.39, 0.43, 0.25),
            (0.26, 0.43, 0.28),
            (0.65, 0.36, 0.23),
            (0.48, 0.40, 0.34),
            (0.72, 0.59, 0.43),
            (0.93, 0.89, 0.76),
            (0.22, 0.43, 0.42),
            (0.09, 0.12, 0.12),
            (0.60, 0.49, 0.37),
            (0.39, 0.35, 0.31),
            (0.32, 0.26, 0.19),
            (0.95, 0.94, 0.88),
            (0.82, 0.87, 0.91),
            (0.56, 0.45, 0.31),
            (1., 1., 1.),
        ];
        let maps: Vec<_> = (0..4).map(|kind| surfaces::maps(images, kind)).collect();
        let masonry = surfaces::masonry(images);
        let materials: Vec<Handle<StandardMaterial>> = colors
            .into_iter()
            .enumerate()
            .map(|(i, (r, g, b))| {
                material_assets.add(StandardMaterial {
                    base_color: if i == HAZE {
                        Color::srgba(r, g, b, 0.04)
                    } else {
                        Color::srgb(r, g, b)
                    },
                    base_color_texture: match i {
                        CINDER => Some(masonry.0.clone()),
                        SAND | GRAVEL => Some(maps[0].0.clone()),
                        ROAD => Some(maps[1].0.clone()),
                        STONE | CREAM | ROCK | ROCK_DARK => Some(maps[2].0.clone()),
                        RUST | GREEN | CLAY | WOOD => Some(maps[3].0.clone()),
                        _ => None,
                    },
                    normal_map_texture: match i {
                        CINDER => Some(masonry.1.clone()),
                        SAND | GRAVEL => Some(maps[0].1.clone()),
                        ROAD => Some(maps[1].1.clone()),
                        STONE | CREAM | ROCK | ROCK_DARK => Some(maps[2].1.clone()),
                        RUST | GREEN | CLAY | WOOD => Some(maps[3].1.clone()),
                        _ => None,
                    },
                    uv_transform: bevy::math::Affine2::from_scale(Vec2::splat(match i {
                        SAND => 100.,
                        ROAD => 12.,
                        STONE => 4.,
                        _ => 1.,
                    })),
                    perceptual_roughness: if i == WATER { 0.23 } else { 0.95 },
                    unlit: i == CLOUD || i == CLOUD_SHADE || i == HAZE,
                    fog_enabled: i != CLOUD && i != CLOUD_SHADE,
                    metallic: if i == WATER { 0.18 } else { 0. },
                    alpha_mode: if i == HAZE {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Opaque
                    },
                    ..default()
                })
            })
            .collect();
        let rocks = (0..7)
            .map(|seed| {
                let mut mesh = Sphere::new(1.).mesh().ico(3).expect("rock sphere");
                if let Some(VertexAttributeValues::Float32x3(vertices)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    for p in vertices {
                        let v = Vec3::from_array(*p);
                        let n = surfaces::hash(
                            (v.x * 917.) as i32,
                            (v.z * 761. + v.y * 431.) as i32,
                            seed * 71 + 8,
                        );
                        let mut v = v * (0.76 + n * 0.45);
                        v.y = v.y.max(-0.65);
                        *p = v.to_array();
                    }
                }
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let _ = mesh.generate_tangents();
                meshes.add(mesh)
            })
            .collect();
        let mut cube = Cuboid::new(1., 1., 1.).mesh().build();
        let _ = cube.generate_tangents();
        let mut cloud_mesh = Sphere::new(1.).mesh().ico(3).expect("cloud sphere");
        if let Some(VertexAttributeValues::Float32x3(vertices)) =
            cloud_mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            let colors: Vec<[f32; 4]> = vertices
                .iter()
                .map(|v| {
                    let light = (v[1] * 0.5 + 0.5) * 0.23 + 0.77;
                    [light, (light + 0.015).min(1.), (light + 0.025).min(1.), 1.]
                })
                .collect();
            cloud_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        Self {
            commands,
            material_assets,
            wall_materials: std::collections::HashMap::new(),
            rocks,
            cloud: meshes.add(cloud_mesh),
            cube: meshes.add(cube),
            round: meshes.add(Sphere::new(1.).mesh().ico(2).expect("valid sphere")),
            cone: meshes.add(Cone::new(1., 1.).mesh().resolution(6)),
            cylinder: meshes.add(Cylinder::new(1., 1.).mesh().resolution(12)),
            materials,
        }
    }
    fn shape(&mut self, mesh: Handle<Mesh>, size: Vec3, pos: Vec3, material: usize) -> Entity {
        self.commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(self.materials[material].clone()),
                Transform::from_translation(pos).with_scale(size),
            ))
            .id()
    }
    fn block(&mut self, size: Vec3, pos: Vec3, material: usize, solid: bool) -> Entity {
        let e = self.shape(self.cube.clone(), size, pos, material);
        if material == CINDER || material == ROAD {
            let scale = if material == CINDER {
                Vec2::new(size.x.max(size.z) / 0.8, size.y / 0.8)
            } else {
                Vec2::new(size.x / 4., size.z / 4.)
            };
            let key = (material, scale.x.to_bits(), scale.y.to_bits());
            let handle = self
                .wall_materials
                .entry(key)
                .or_insert_with(|| {
                    let mut mat = self
                        .material_assets
                        .get(&self.materials[material])
                        .expect("surface material")
                        .clone();
                    mat.uv_transform = bevy::math::Affine2::from_scale(scale);
                    self.material_assets.add(mat)
                })
                .clone();
            self.commands.entity(e).insert(MeshMaterial3d(handle));
        }
        if solid {
            self.commands.entity(e).insert(Solid { half: size * 0.5 });
        }
        e
    }
    fn ball(&mut self, size: Vec3, pos: Vec3, material: usize) -> Entity {
        self.shape(self.round.clone(), size, pos, material)
    }
    fn column(&mut self, r: f32, h: f32, pos: Vec3, material: usize) -> Entity {
        self.shape(self.cylinder.clone(), Vec3::new(r, h, r), pos, material)
    }
    fn beam(&mut self, a: Vec3, b: Vec3, width: f32, material: usize) {
        let e = self.block(
            Vec3::new(width, a.distance(b), width),
            (a + b) * 0.5,
            material,
            false,
        );
        self.commands.entity(e).insert(
            Transform::from_translation((a + b) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, (b - a).normalize()))
                .with_scale(Vec3::new(width, a.distance(b), width)),
        );
    }
    fn sign(&mut self, position: Vec3, width: f32, material: usize) {
        for x in [-width * 0.4, width * 0.4] {
            self.block(
                Vec3::new(0.09, 2.4, 0.09),
                position + Vec3::new(x, 1.2, 0.),
                DARK,
                true,
            );
        }
        self.block(
            Vec3::new(width, 0.9, 0.14),
            position + Vec3::new(0., 2.25, 0.),
            material,
            false,
        );
    }
    fn fence_x(&mut self, start: f32, end: f32, z: f32, height: f32) {
        let length = end - start;
        self.block(
            Vec3::new(length, height, 0.16),
            Vec3::new((start + end) * 0.5, height * 0.5, z),
            RUST,
            true,
        );
        // Close panels have separate slats; the dark backing gives a readable distant silhouette.
        for y in [0.25, height - 0.25] {
            self.block(
                Vec3::new(length, 0.09, 0.22),
                Vec3::new((start + end) * 0.5, y, z),
                DARK,
                false,
            );
        }
        for i in 0..=(length / 0.46) as usize {
            let x = start + i as f32 * 0.46;
            self.block(
                Vec3::new(0.12, height + 0.2, 0.24),
                Vec3::new(x, height * 0.5, z),
                if i % 9 == 0 { DARK } else { RUST },
                false,
            );
        }
    }
    fn rock(&mut self, size: Vec3, pos: Vec3, seed: usize) {
        let e = self.shape(
            self.rocks[seed % self.rocks.len()].clone(),
            size,
            pos,
            if seed.is_multiple_of(3) {
                ROCK_DARK
            } else {
                ROCK
            },
        );
        self.commands.entity(e).insert(
            Transform::from_translation(pos)
                .with_scale(size)
                .with_rotation(Quat::from_rotation_y(seed as f32 * 1.71)),
        );
        if size.x > 1.3 {
            for j in 0..3 {
                let q = pos
                    + Vec3::new(
                        (j as f32 - 1.) * size.x * 0.46,
                        -size.y * 0.45,
                        size.z * 0.8,
                    );
                self.shape(
                    self.rocks[(seed + j + 2) % 7].clone(),
                    size * 0.18,
                    q,
                    ROCK_DARK,
                );
            }
        }
    }
    fn road(&mut self, a: Vec2, z: Vec2, width: f32) {
        let d = z - a;
        let center = (a + z) * 0.5;
        let yaw = d.x.atan2(d.y);
        let e = self.block(
            Vec3::new(width, 0.07, d.length() + 1.),
            Vec3::new(center.x, 0.0, center.y),
            ROAD,
            false,
        );
        self.commands.entity(e).insert(
            Transform::from_xyz(center.x, 0., center.y)
                .with_scale(Vec3::new(width, 0.07, d.length() + 1.))
                .with_rotation(Quat::from_rotation_y(yaw)),
        );
        let normal = Vec2::new(d.y, -d.x).normalize();
        for offset in [-width * 0.46, 0., width * 0.46] {
            for i in 0..(d.length() / 5.) as usize {
                let p = a + d.normalize() * (i as f32 * 5. + 1.) + normal * offset;
                let mark = self.block(
                    Vec3::new(0.1, 0.02, 2.),
                    Vec3::new(p.x, 0.055, p.y),
                    CREAM,
                    false,
                );
                self.commands.entity(mark).insert(
                    Transform::from_xyz(p.x, 0.055, p.y)
                        .with_scale(Vec3::new(0.1, 0.02, 2.))
                        .with_rotation(Quat::from_rotation_y(yaw)),
                );
            }
        }
    }
    fn dead_tree(&mut self, x: f32, z: f32, seed: usize) {
        let h = 2.4 + (seed % 4) as f32 * 0.4;
        let base = Vec3::new(x, 0., z);
        let top = base + Vec3::new(0.2, h, 0.1);
        self.beam(base, top, 0.16, WOOD);
        for j in 0..5 {
            let angle = j as f32 * 2.3 + seed as f32;
            let a = base + Vec3::Y * (0.8 + j as f32 * 0.29);
            let end = a + Vec3::new(angle.cos() * 1.1, 0.65, angle.sin() * 1.1);
            self.beam(a, end, 0.075, WOOD);
            self.beam(end, end + Vec3::new(0.4, 0.5, -0.2), 0.035, WOOD);
        }
    }
    fn cactus(&mut self, x: f32, z: f32, h: f32) {
        self.column(0.20, h, Vec3::new(x, h * 0.5, z), CACTUS);
        self.ball(Vec3::splat(0.2), Vec3::new(x, h, z), CACTUS);
        for (side, level) in [(-1., 0.48), (1., 0.66)] {
            let y = h * level;
            self.beam(
                Vec3::new(x, y, z),
                Vec3::new(x + side * 0.64, y, z),
                0.26,
                CACTUS,
            );
            self.column(
                0.14,
                h * 0.28,
                Vec3::new(x + side * 0.64, y + h * 0.14, z),
                CACTUS,
            );
            self.ball(
                Vec3::splat(0.14),
                Vec3::new(x + side * 0.64, y + h * 0.28, z),
                CACTUS,
            );
        }
        // Raised ribs, spine clusters and flower buds keep close plants from reading as cylinders.
        for i in 0..9 {
            let angle = i as f32 * std::f32::consts::TAU / 9.;
            let offset = Vec3::new(angle.cos() * 0.198, 0., angle.sin() * 0.198);
            self.beam(
                Vec3::new(x, 0.15, z) + offset,
                Vec3::new(x, h - 0.12, z) + offset,
                0.017,
                SAGE,
            );
        }
        for i in 0..15 {
            let angle = i as f32 * 2.39996;
            let y = 0.3 + (i % 6) as f32 * (h - 0.5) / 6.;
            let direction = Vec3::new(angle.cos(), 0.1, angle.sin());
            let start = Vec3::new(x, y, z) + direction * 0.2;
            self.beam(start, start + direction * 0.09, 0.009, CREAM);
        }
        for i in 0..3 {
            self.ball(
                Vec3::new(0.08, 0.1, 0.07),
                Vec3::new(x - 0.1 + i as f32 * 0.1, h + 0.06, z),
                if i == 1 { RED } else { GOLD },
            );
        }
    }
}
#[allow(clippy::type_complexity)] // Explicit disjoint Bevy queries prevent overlapping mutable transforms.
pub fn atmosphere(
    time: Res<Time>,
    mut ripples: Query<(&mut Transform, &RiverRipple), Without<Dust>>,
    mut dust: Query<(&mut Transform, &Dust), Without<RiverRipple>>,
    mut clouds: Query<(&mut Transform, &Cloud), (Without<Dust>, Without<RiverRipple>)>,
) {
    for (mut t, r) in &mut ripples {
        t.translation = r.origin
            + Vec3::new(
                (time.elapsed_secs() * 0.22 + r.phase).sin() * 1.5,
                (time.elapsed_secs() + r.phase).sin() * 0.018,
                0.,
            );
    }
    for (mut t, c) in &mut clouds {
        t.translation =
            c.origin + Vec3::new((time.elapsed_secs() * 0.008 + c.phase).sin() * 18., 0., 0.);
    }
    for (mut t, d) in &mut dust {
        t.translation = d.origin
            + Vec3::new(
                (time.elapsed_secs() * 0.07 + d.phase).sin() * 2.,
                (time.elapsed_secs() * 0.14 + d.phase).sin() * 0.18,
                0.,
            );
    }
}
pub fn export_collision(path: &str) {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Image>>()
        .add_systems(Startup, setup);
    app.update();
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Transform, &Solid), Without<traffic::Vehicle>>();
    let map = border_net::map::CollisionMap {
        schema_version: 1,
        solids: q
            .iter(world)
            .map(|(t, s)| border_net::map::Aabb {
                center: t.translation.to_array(),
                half: s.half.to_array(),
            })
            .collect(),
    };
    std::fs::write(path, serde_json::to_string_pretty(&map).unwrap()).unwrap();
}
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    shift: Option<Res<shift::Shift>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.93, 0.82, 0.65),
        brightness: 330.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1., 0.86, 0.66),
            illuminance: 16000.,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-45., 65., 35.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let mut landmarks = Landmarks::default();
    {
        let mut b = Builder::new(&mut commands, &mut meshes, &mut materials, &mut images);
        // Separate banks leave a genuine river channel under a raised road deck.
        b.block(
            Vec3::new(1000., 0.5, 650.),
            Vec3::new(0., -2.5, -289.),
            SAND,
            false,
        ); // north bank ends z=36
        b.block(
            Vec3::new(1000., 0.5, 650.),
            Vec3::new(0., -2.5, 390.),
            SAND,
            false,
        ); // south bank starts z=65
        b.block(
            Vec3::new(1000., 0.4, 29.),
            Vec3::new(0., -3.2, 50.5),
            CLAY,
            false,
        );
        b.block(
            Vec3::new(1000., 0.12, 27.8),
            Vec3::new(0., -2.35, 50.5),
            WATER,
            false,
        );
        for bank in [36.2, 64.8] {
            for i in -25..26 {
                b.rock(
                    Vec3::new(3.4, 1.25, 1.7),
                    Vec3::new(i as f32 * 7.1, -1.1, bank),
                    (i + 25) as usize,
                );
            }
        }
        for i in 0..34 {
            let origin = Vec3::new(-110. + i as f32 * 6.5, -2.27, 39. + (i % 8) as f32 * 3.1);
            let e = b.block(
                Vec3::new(2.2 + (i % 4) as f32, 0.015, 0.12),
                origin,
                FOAM,
                false,
            );
            b.commands.entity(e).insert((
                RiverRipple {
                    origin,
                    phase: i as f32,
                },
                NotShadowCaster,
            ));
        }
        // Two paved lanes, shoulder lines, stop lines and a shared canopy.
        b.block(
            Vec3::new(13.5, 0.08, 230.),
            Vec3::new(3.5, -0.01, 15.),
            ROAD,
            false,
        );
        for x in [-2.65, 3.5, 9.65] {
            for i in -25..33 {
                b.block(
                    Vec3::new(0.11, 0.02, 1.9),
                    Vec3::new(x, 0.045, i as f32 * 4.),
                    CREAM,
                    false,
                );
            }
        }
        for x in [0., 7.] {
            b.block(
                Vec3::new(5.3, 0.025, 0.32),
                Vec3::new(x, 0.055, 3.1),
                GOLD,
                false,
            );
            // Painted approach chevrons and lane arrows.
            for z in [7., 24., 31.] {
                b.block(
                    Vec3::new(0.19, 0.02, 1.7),
                    Vec3::new(x, 0.05, z),
                    WHITE,
                    false,
                );
                for side in [-1., 1.] {
                    b.beam(
                        Vec3::new(x, 0.06, z - 0.85),
                        Vec3::new(x + side * 0.7, 0.06, z - 0.1),
                        0.12,
                        WHITE,
                    );
                }
            }
            for z in [-5., 5.] {
                b.block(
                    Vec3::new(0.25, 5.2, 0.25),
                    Vec3::new(x - 2.7, 2.6, z),
                    GREEN,
                    true,
                );
            }
            b.block(
                Vec3::new(6.3, 0.28, 12.),
                Vec3::new(x, 5.2, 0.),
                CREAM,
                false,
            );
            for z in [-4., 0., 4.] {
                b.block(Vec3::new(5.7, 0.07, 0.22), Vec3::new(x, 5., z), DARK, false);
                b.block(
                    Vec3::new(1.5, 0.04, 0.15),
                    Vec3::new(x, 4.94, z),
                    WHITE,
                    false,
                );
            }
            b.block(
                Vec3::new(4., 0.9, 0.13),
                Vec3::new(x, 4.45, 5.3),
                GREEN,
                false,
            );
            landmarks.0.push((
                if x == 0. {
                    "01  •  DOCUMENT CHECK"
                } else {
                    "02  •  INSPECTION LANE"
                },
                Vec3::new(x, 4.5, 5.4),
            ));
            // Protective bollards flank each stop without blocking the driver-side approach.
            for side in [-1., 1.] {
                for z in [-3.6, 3.5] {
                    b.column(0.12, 0.85, Vec3::new(x + side * 2.5, 0.425, z), GOLD);
                }
            }
        }
        b.block(
            Vec3::new(0.5, 1.2, 0.5),
            Vec3::new(9.9, 0.6, -5.8),
            GOLD,
            true,
        );
        let second = b
            .commands
            .spawn((
                Transform::from_xyz(9.9, 1.1, -5.8),
                Visibility::default(),
                Barrier,
                traffic::LaneBarrier(1),
            ))
            .id();
        for i in 0..12 {
            let e = b.block(
                Vec3::new(0.46, 0.17, 0.18),
                Vec3::new(-0.25 - i as f32 * 0.46, 0., 0.),
                if i % 2 == 0 { CREAM } else { RED },
                false,
            );
            b.commands.entity(second).add_child(e);
        }
        // Existing interactive barrier keeps its animated hinge and original coordinates.
        b.block(
            Vec3::new(0.5, 1.2, 0.5),
            Vec3::new(2.9, 0.6, -5.8),
            GOLD,
            true,
        );
        let pivot = b
            .commands
            .spawn((
                Transform::from_xyz(2.9, 1.1, -5.8),
                Visibility::default(),
                Barrier,
                traffic::LaneBarrier(0),
            ))
            .id();
        for i in 0..12 {
            let e = b.block(
                Vec3::new(0.46, 0.17, 0.18),
                Vec3::new(-0.25 - i as f32 * 0.46, 0., 0.),
                if i % 2 == 0 { CREAM } else { RED },
                false,
            );
            b.commands.entity(pivot).add_child(e);
        }
        // Entry gates meter approaching traffic independently of each lane's exit gate.
        for lane in 0..2 {
            let x = 2.9 + lane as f32 * 7.;
            b.block(
                Vec3::new(0.5, 1.2, 0.5),
                Vec3::new(x, 0.6, border_net::vehicle::ENTRY_GATE_Z),
                GOLD,
                true,
            );
            let pivot = b
                .commands
                .spawn((
                    Transform::from_xyz(x, 1.1, border_net::vehicle::ENTRY_GATE_Z),
                    Visibility::default(),
                    Barrier,
                    traffic::LaneBarrier(lane),
                    vehicles::EntryGate,
                ))
                .id();
            for i in 0..12 {
                let e = b.block(
                    Vec3::new(0.46, 0.17, 0.18),
                    Vec3::new(-0.25 - i as f32 * 0.46, 0., 0.),
                    if i % 2 == 0 { CREAM } else { RED },
                    false,
                );
                b.commands.entity(pivot).add_child(e);
            }
        }
        // Administration office: a genuinely hollow room with a broad south-facing doorway.
        b.block(
            Vec3::new(10., 0.12, 9.),
            Vec3::new(-12., 0., -8.),
            STONE,
            false,
        );
        for x in [-17., -7.] {
            b.block(
                Vec3::new(0.25, 3.5, 9.),
                Vec3::new(x, 1.75, -8.),
                CINDER,
                true,
            );
        }
        b.block(
            Vec3::new(10., 3.5, 0.25),
            Vec3::new(-12., 1.75, -12.5),
            CINDER,
            true,
        );
        for x in [-15.05, -8.95] {
            b.block(
                Vec3::new(3.9, 3.5, 0.25),
                Vec3::new(x, 1.75, -3.5),
                CINDER,
                true,
            );
        }
        b.block(
            Vec3::new(2.3, 0.5, 0.27),
            Vec3::new(-12., 3.25, -3.5),
            GREEN,
            false,
        );
        b.block(
            Vec3::new(10.7, 0.23, 9.6),
            Vec3::new(-12., 3.6, -8.),
            GREEN,
            false,
        );
        for x in [-15.05, -8.95] {
            b.block(
                Vec3::new(2.5, 1.1, 0.04),
                Vec3::new(x, 1.9, -3.35),
                GLASS,
                false,
            );
            for dx in [-0.9, 0., 0.9] {
                b.block(
                    Vec3::new(0.05, 1.2, 0.05),
                    Vec3::new(x + dx, 1.9, -3.3),
                    GREEN,
                    false,
                );
            }
        }
        b.block(
            Vec3::new(5., 0.55, 0.15),
            Vec3::new(-12., 2.9, -3.32),
            GREEN,
            false,
        );
        landmarks
            .0
            .push(("ADMINISTRATION  /  SAN IGNACIO", Vec3::new(-12., 3., -3.2)));
        // Desks, chairs, terminals, filing cabinets, bulletin board and a very necessary coffee station.
        for x in [-14.5, -9.5] {
            b.block(
                Vec3::new(2.5, 0.13, 1.1),
                Vec3::new(x, 0.85, -10.7),
                CLAY,
                true,
            );
            for dx in [-1., 1.] {
                b.block(
                    Vec3::new(0.09, 0.8, 0.8),
                    Vec3::new(x + dx, 0.4, -10.7),
                    DARK,
                    false,
                );
            }
            b.block(
                Vec3::new(0.9, 0.55, 0.10),
                Vec3::new(x, 1.3, -10.95),
                DARK,
                false,
            );
            b.block(
                Vec3::new(0.75, 0.4, 0.02),
                Vec3::new(x, 1.32, -10.88),
                GLASS,
                false,
            );
            b.block(
                Vec3::new(0.65, 0.05, 0.22),
                Vec3::new(x, 0.95, -10.35),
                CREAM,
                false,
            );
            b.block(
                Vec3::new(0.65, 0.12, 0.7),
                Vec3::new(x, 0.52, -9.2),
                GREEN,
                true,
            );
            b.block(
                Vec3::new(0.65, 0.8, 0.12),
                Vec3::new(x, 0.9, -8.9),
                GREEN,
                false,
            );
            b.column(0.08, 0.5, Vec3::new(x, 0.25, -9.2), DARK);
            b.block(
                Vec3::new(0.36, 0.04, 0.45),
                Vec3::new(x + 0.8, 0.95, -10.45),
                WHITE,
                false,
            );
        }
        for x in [-16., -15.2] {
            b.block(
                Vec3::new(0.65, 1.6, 0.7),
                Vec3::new(x, 0.8, -6.),
                SAGE,
                true,
            );
            for y in [0.3, 0.8, 1.3] {
                b.block(
                    Vec3::new(0.25, 0.04, 0.03),
                    Vec3::new(x, y, -5.63),
                    DARK,
                    false,
                );
            }
        }
        b.block(
            Vec3::new(0.7, 0.9, 1.2),
            Vec3::new(-8., 0.45, -5.),
            CLAY,
            true,
        );
        b.column(0.18, 0.3, Vec3::new(-8., 1.05, -5.), DARK);
        b.column(0.10, 0.18, Vec3::new(-8., 1., -4.6), WHITE);
        b.block(
            Vec3::new(2.3, 1.25, 0.05),
            Vec3::new(-12., 2.1, -12.34),
            CLAY,
            false,
        );
        for i in 0..6 {
            b.block(
                Vec3::new(0.4, 0.5, 0.025),
                Vec3::new(
                    -12.8 + (i % 3) as f32 * 0.75,
                    1.8 + (i / 3) as f32 * 0.55,
                    -12.29,
                ),
                if i == 2 { GOLD } else { WHITE },
                false,
            );
        }
        for x in [-14.5, -9.5] {
            b.block(
                Vec3::new(1.5, 0.06, 0.3),
                Vec3::new(x, 3.43, -8.),
                WHITE,
                false,
            );
        }
        landmarks.0.push((
            "COFFEE IS NOT A SECURITY CLEARANCE",
            Vec3::new(-8., 1.5, -4.8),
        ));
        // Ready door opens outwards from the west hinge; collision remains host-owned.
        let door = b
            .commands
            .spawn((
                Transform::from_xyz(-13.1, 0., -3.5),
                Visibility::default(),
                shift::ReadyDoor,
            ))
            .id();
        for (size, pos, material) in [
            (Vec3::new(2.16, 2.95, 0.12), Vec3::new(1.1, 1.48, 0.), GREEN),
            (
                Vec3::new(1.2, 0.85, 0.025),
                Vec3::new(1.1, 2., -0.075),
                GLASS,
            ),
            (
                Vec3::new(0.15, 0.12, 0.16),
                Vec3::new(1.9, 1.15, -0.12),
                GOLD,
            ),
        ] {
            let e = b.block(size, pos, material, false);
            b.commands.entity(door).add_child(e);
        }
        // Equipment shelter with open frontage and visible shelves/crates.
        b.block(
            Vec3::new(7., 0.1, 8.),
            Vec3::new(-22., 0., -8.),
            STONE,
            false,
        );
        for x in [-25.5, -18.5] {
            b.block(Vec3::new(0.2, 3.1, 8.), Vec3::new(x, 1.55, -8.), CLAY, true);
        }
        b.block(
            Vec3::new(7., 3.1, 0.2),
            Vec3::new(-22., 1.55, -12.),
            CLAY,
            true,
        );
        b.block(
            Vec3::new(7.6, 0.2, 8.5),
            Vec3::new(-22., 3.2, -8.),
            RUST,
            false,
        );
        for y in [0.25, 1.25, 2.25] {
            b.block(
                Vec3::new(5.8, 0.13, 1.1),
                Vec3::new(-22., y, -10.7),
                DARK,
                true,
            );
            for i in 0..4 {
                b.block(
                    Vec3::new(0.9, 0.65, 0.8),
                    Vec3::new(-24. + i as f32 * 1.3, y + 0.4, -10.7),
                    if i % 2 == 0 { GREEN } else { CLAY },
                    false,
                );
            }
        }
        landmarks
            .0
            .push(("EQUIPMENT  /  STORAGE", Vec3::new(-22., 2.8, -3.8)));
        // Open holding pad: painted bounds, benches and shade.
        b.block(
            Vec3::new(12., 0.1, 10.),
            Vec3::new(-14., 0., -23.),
            STONE,
            false,
        );
        for z in [-28., -18.] {
            b.block(
                Vec3::new(12., 0.02, 0.09),
                Vec3::new(-14., 0.065, z),
                GOLD,
                false,
            );
        }
        b.block(
            Vec3::new(0.09, 0.02, 10.),
            Vec3::new(-20., 0.065, -23.),
            GOLD,
            false,
        );
        for x in [-18., -10.] {
            b.block(
                Vec3::new(0.15, 3.1, 0.15),
                Vec3::new(x, 1.55, -26.),
                DARK,
                true,
            );
        }
        b.block(
            Vec3::new(9., 0.16, 3.),
            Vec3::new(-14., 3.2, -26.),
            CREAM,
            false,
        );
        for x in [-17., -12.] {
            b.block(
                Vec3::new(2.7, 0.16, 0.65),
                Vec3::new(x, 0.6, -26.),
                CLAY,
                true,
            );
            for dx in [-1., 1.] {
                b.block(
                    Vec3::new(0.14, 0.6, 0.5),
                    Vec3::new(x + dx, 0.3, -26.),
                    DARK,
                    false,
                );
            }
        }
        b.sign(Vec3::new(-6.8, 0., -23.), 1.7, GOLD);
        landmarks
            .0
            .push(("HOLDING YARD", Vec3::new(-6.8, 2.35, -22.85)));
        // Condenser units, service piping, utility cabinet and cameras.
        for x in [-15.5, -8.5] {
            b.block(
                Vec3::new(1., 0.8, 0.7),
                Vec3::new(x, 0.4, -13.1),
                STONE,
                true,
            );
            for i in 0..6 {
                b.block(
                    Vec3::new(0.85, 0.025, 0.03),
                    Vec3::new(x, 0.15 + i as f32 * 0.1, -13.47),
                    DARK,
                    false,
                );
            }
            b.column(0.07, 3.4, Vec3::new(x + 0.65, 1.7, -12.8), RUST);
        }
        b.block(
            Vec3::new(0.8, 1.6, 0.5),
            Vec3::new(-18., 0.8, -12.95),
            GREEN,
            true,
        );
        for x in [-2.7, 4.3] {
            b.beam(
                Vec3::new(x, 4.2, 4.5),
                Vec3::new(x + 0.5, 4.2, 4.5),
                0.1,
                DARK,
            );
            b.block(
                Vec3::new(0.25, 0.22, 0.55),
                Vec3::new(x + 0.5, 4.1, 4.5),
                WHITE,
                false,
            );
            b.block(
                Vec3::new(0.16, 0.14, 0.04),
                Vec3::new(x + 0.5, 4.1, 4.8),
                DARK,
                false,
            );
        }
        b.sign(Vec3::new(-4.6, 0., 20.), 2.6, GOLD);
        landmarks.0.push((
            "DECLARE ALL SNACKS. YES, ALL OF THEM.",
            Vec3::new(-4.6, 2.35, 20.15),
        ));
        // Secondary inspection apron and open-sided shade with work tables.
        b.block(
            Vec3::new(10., 0.1, 22.),
            Vec3::new(18., 0., -1.),
            STONE,
            false,
        );
        for x in [13.5, 22.5] {
            for z in [-10., 8.] {
                b.block(Vec3::new(0.2, 4.1, 0.2), Vec3::new(x, 2.05, z), DARK, true);
            }
        }
        b.block(
            Vec3::new(10., 0.19, 20.),
            Vec3::new(18., 4.2, -1.),
            RUST,
            false,
        );
        for x in [15., 21.] {
            b.block(
                Vec3::new(2., 0.12, 0.85),
                Vec3::new(x, 0.9, -8.5),
                CREAM,
                true,
            );
            for dx in [-0.8, 0.8] {
                b.block(
                    Vec3::new(0.1, 0.9, 0.7),
                    Vec3::new(x + dx, 0.45, -8.5),
                    DARK,
                    false,
                );
            }
        }
        for x in [14., 22.] {
            b.block(
                Vec3::new(0.13, 0.02, 15.),
                Vec3::new(x, 0.065, -0.5),
                GOLD,
                false,
            );
        }
        b.sign(Vec3::new(18., 0., 10.), 3.5, GREEN);
        landmarks
            .0
            .push(("SECONDARY INSPECTION", Vec3::new(18., 2.35, 10.15)));
        // Bridge deck, abutments, piers, guard rails and diagonal braces over visible water.
        b.block(
            Vec3::new(14., 0.8, 32.),
            Vec3::new(3.5, -0.48, 50.5),
            STONE,
            false,
        );
        b.block(
            Vec3::new(12.8, 0.06, 33.),
            Vec3::new(3.5, 0.0, 50.5),
            ROAD,
            false,
        );
        for z in [36., 46., 56., 65.] {
            for x in [-2.8, 9.8] {
                b.block(
                    Vec3::new(1.1, 3.1, 1.6),
                    Vec3::new(x, -1.7, z),
                    STONE,
                    false,
                );
            }
        }
        for x in [-3.35, 10.35] {
            b.block(
                Vec3::new(0.4, 0.35, 33.),
                Vec3::new(x, 0.18, 50.5),
                STONE,
                true,
            );
            for y in [0.65, 1.15] {
                b.block(
                    Vec3::new(0.13, 0.12, 33.),
                    Vec3::new(x, y, 50.5),
                    DARK,
                    false,
                );
            }
            for i in 0..17 {
                let z = 34.5 + i as f32 * 2.;
                b.block(
                    Vec3::new(0.15, 1.25, 0.15),
                    Vec3::new(x, 0.62, z),
                    DARK,
                    false,
                );
            }
            for z in [36., 46., 56.] {
                b.beam(
                    Vec3::new(x, -0.8, z),
                    Vec3::new(x, -2.8, z + 8.),
                    0.28,
                    RUST,
                );
            }
        }
        // Border fence sits entirely on the Texas (north) bank, with an opening at the bridge.
        b.fence_x(-92., -3.7, 32., 4.);
        b.fence_x(10.7, 92., 32., 4.);
        for side in [-1., 1.] {
            for i in 0..34 {
                let x = side * (97. + i as f32 * 10.);
                b.block(
                    Vec3::new(9.8, 3.7, 0.18),
                    Vec3::new(x, 1.85, 32.),
                    RUST,
                    false,
                );
                b.block(
                    Vec3::new(0.2, 4., 0.3),
                    Vec3::new(x - 5., 2., 32.),
                    DARK,
                    false,
                );
            }
        }
        b.sign(Vec3::new(-8., 0., 30.8), 5., GREEN);
        landmarks
            .0
            .push(("SAN IGNACIO  •  TEXAS", Vec3::new(-8., 2.35, 30.95)));
        b.sign(Vec3::new(14., 0., 29.8), 3., GOLD);
        landmarks
            .0
            .push(("RIVER CROSSING  /  15 MPH", Vec3::new(14., 2.35, 29.95)));
        // Open compound: only the riverbank border wall remains. Posted foot limits replace enclosure fencing.
        b.sign(Vec3::new(-5., 0., -30.), 2.3, GOLD);
        b.sign(Vec3::new(12., 0., 76.), 2.3, GOLD);
        landmarks.0.push((
            "TRAFFIC ONLY  /  NO FOOT ACCESS",
            Vec3::new(12., 2.35, 76.15),
        ));
        b.sign(Vec3::new(-31., 0., 10.), 2.2, GOLD);
        b.sign(Vec3::new(31., 0., 24.), 2.2, GOLD);
        landmarks
            .0
            .push(("PATROL FOOT LIMIT", Vec3::new(-31., 2.35, 10.15)));
        landmarks
            .0
            .push(("PATROL FOOT LIMIT", Vec3::new(31., 2.35, 24.15)));
        // Reserved eastern apron: no extra operational lanes until the expansion milestone.
        b.block(
            Vec3::new(26., 0.03, 30.),
            Vec3::new(41., -0.02, -4.),
            GRAVEL,
            false,
        );
        b.sign(Vec3::new(30., 0., 13.), 4., GOLD);
        landmarks
            .0
            .push(("FUTURE LANES  /  CLOSED", Vec3::new(30., 2.4, 13.15)));
        // No dunes: angular weathered outcrops and eroded rocky relief.
        for (a, z) in [
            (Vec2::new(3.5, -100.), Vec2::new(-110., -210.)),
            (Vec2::new(-110., -210.), Vec2::new(-430., -340.)),
            (Vec2::new(3.5, 130.), Vec2::new(140., 235.)),
            (Vec2::new(140., 235.), Vec2::new(430., 350.)),
        ] {
            b.road(a, z, 13.5);
            b.column(7., 0.06, Vec3::new(a.x, 0., a.y), ROAD);
        }
        b.road(Vec2::new(-2., -16.), Vec2::new(-440., -16.), 4.5);
        for i in 0..35 {
            let angle = i as f32 * 2.39996;
            let radius = 175. + (i % 5) as f32 * 27.;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            if traffic::near_road(Vec2::new(x, z), 35.) || (34. ..67.).contains(&z) {
                continue;
            }
            b.rock(
                Vec3::new(
                    18. + (i % 4) as f32 * 5.,
                    10. + (i % 7) as f32 * 2.,
                    15. + (i % 5) as f32 * 4.,
                ),
                Vec3::new(x, 1., z),
                i,
            );
        }
        for i in 0..195 {
            let x = ((i * 47 + 19) % 283) as f32 - 141.;
            let z = ((i * 71 + 31) % 235) as f32 - 105.;
            if (x > -30. && x < 49. && z > -36. && z < 82.)
                || (35. ..66.).contains(&z)
                || traffic::near_road(Vec2::new(x, z), 11.)
            {
                continue;
            }
            if i % 5 == 0 {
                b.cactus(x, z, 1.8 + (i % 4) as f32 * 0.6);
            } else {
                for j in 0..3 {
                    b.ball(
                        Vec3::new(0.55 + (i % 3) as f32 * 0.12, 0.38, 0.5),
                        Vec3::new(x + j as f32 * 0.4, 0.3, z + (j % 2) as f32 * 0.35),
                        SAGE,
                    );
                }
                b.rock(
                    Vec3::new(0.6, 0.23, 0.4),
                    Vec3::new(x + 1.8, 0.12, z - 1.),
                    i,
                );
            }
        }
        // Close plants around walkways and buildings, kept out of the vehicle/door clearances.
        for (x, z, h) in [
            (-24., 13., 2.8),
            (-20., 5., 2.1),
            (-6., 18., 2.5),
            (20., 22., 3.2),
            (-29., -22., 1.9),
            (22., -24., 2.6),
            (-15., 27., 2.4),
        ] {
            b.cactus(x, z, h);
            for j in 0..4 {
                b.ball(
                    Vec3::new(0.65, 0.42, 0.55),
                    Vec3::new(
                        x + 1.5 + (j % 2) as f32 * 0.5,
                        0.35,
                        z + (j / 2) as f32 * 0.5,
                    ),
                    SAGE,
                );
            }
        }
        for i in 0..15 {
            let origin = Vec3::new(
                -75. + i as f32 * 11.,
                0.9 + (i % 3) as f32 * 0.3,
                -38. - (i % 4) as f32 * 11.,
            );
            let e = b.ball(Vec3::new(6., 0.8, 1.8), origin, HAZE);
            b.commands.entity(e).insert((
                Dust {
                    origin,
                    phase: i as f32,
                },
                NotShadowCaster,
            ));
        }
        // Multi-part cloud banks: shaded bases, overlapping high-detail lobes and wind drift.
        for i in 0..30 {
            let origin = Vec3::new(
                -375. + (i % 6) as f32 * 150.,
                66. + (i % 3) as f32 * 12.,
                -400. + (i / 6) as f32 * 200.,
            );
            let root = b
                .commands
                .spawn((
                    Transform::from_translation(origin),
                    Visibility::default(),
                    Cloud {
                        origin,
                        phase: i as f32,
                    },
                ))
                .id();
            for j in 0..17 {
                let f = j as f32;
                let angle = f * 2.39996;
                let radial = if j < 4 {
                    5.
                } else if j > 12 {
                    9.
                } else {
                    15.
                };
                let local = Vec3::new(
                    angle.cos() * radial,
                    if j < 4 {
                        -3.
                    } else if j > 12 {
                        6. + (j % 3) as f32 * 2.
                    } else {
                        f.sin().abs() * 4. - 1.
                    },
                    angle.sin() * radial * 0.47,
                );
                let size = Vec3::new(
                    8. + (j % 4) as f32 * 1.8,
                    if j < 4 {
                        2.
                    } else if j > 12 {
                        7.
                    } else {
                        4. + (j % 3) as f32
                    },
                    6. + (j % 3) as f32,
                );
                let e = b.shape(
                    b.cloud.clone(),
                    size,
                    local,
                    if j < 4 { CLOUD_SHADE } else { CLOUD },
                );
                b.commands.entity(e).insert(NotShadowCaster);
                b.commands.entity(root).add_child(e);
            }
        }
        // Scattered scree, twigs, deadwood and sparse plant islands replace empty sand sheets.
        for i in 0..740 {
            let x = -155. + surfaces::hash(i, 3, 82) * 310.;
            let z = -120. + surfaces::hash(i, 9, 76) * 250.;
            let p = Vec2::new(x, z);
            if (34. ..67.).contains(&z)
                || traffic::near_road(p, 9.)
                || (!open_dressing(p) && x > -28. && x < 25.)
            {
                continue;
            }
            if i % 9 == 0 {
                b.dead_tree(x, z, i as usize);
            } else if i % 3 == 0 {
                b.rock(
                    Vec3::new(
                        0.35 + (i % 4) as f32 * 0.18,
                        0.22 + (i % 3) as f32 * 0.16,
                        0.4,
                    ),
                    Vec3::new(x, 0.05, z),
                    i as usize,
                );
            } else {
                let angle = i as f32 * 1.71;
                let start = Vec3::new(x, 0.065, z);
                let end = start + Vec3::new(angle.cos() * 0.7, 0.04, angle.sin() * 0.7);
                b.beam(start, end, 0.025, WOOD);
                b.beam(
                    (start + end) * 0.5,
                    end + Vec3::new(0.2, 0.08, -0.3),
                    0.017,
                    WOOD,
                );
            }
        }
        for i in 0..90 {
            let x = -26. + surfaces::hash(i, 18, 72) * 49.;
            let z = -29. + surfaces::hash(i, 8, 98) * 59.;
            let p = Vec2::new(x, z);
            if !open_dressing(p) {
                continue;
            }
            // Thin, irregular ground patches and pebble fans add relief without blocking foot routes.
            b.rock(
                Vec3::new(0.8 + (i % 3) as f32 * 0.35, 0.07, 0.65),
                Vec3::new(x, 0.025, z),
                i as usize,
            );
            for j in 0..5 {
                let dx = surfaces::hash(i, j, 17) * 2. - 1.;
                let dz = surfaces::hash(i, j, 51) * 2. - 1.;
                b.rock(
                    Vec3::new(0.06, 0.04, 0.09),
                    Vec3::new(x + dx, 0.05, z + dz),
                    (i + j) as usize,
                );
            }
            if i % 4 == 0 {
                b.dead_tree(x, z, i as usize);
            }
            if i % 3 == 0 {
                for j in 0..7 {
                    let a = j as f32 * 0.9;
                    b.beam(
                        Vec3::new(x, 0.05, z),
                        Vec3::new(
                            x + a.cos() * 0.5,
                            0.5 + (j % 3) as f32 * 0.15,
                            z + a.sin() * 0.5,
                        ),
                        0.035,
                        SAGE,
                    );
                }
            }
        }
        for i in 0..22 {
            let x = if i % 2 == 0 { -18. } else { 20. };
            let z = -24. + (i % 8) as f32 * 7.;
            if !open_dressing(Vec2::new(x, z)) {
                continue;
            }
            for j in 0..4 {
                let origin = Vec3::new(x + j as f32 * 0.8, 0.5 + j as f32 * 0.2, z);
                let e = b.ball(
                    Vec3::new(1.9 + j as f32 * 0.3, 0.7 + j as f32 * 0.25, 1.1),
                    origin,
                    HAZE,
                );
                b.commands.entity(e).insert((
                    Dust {
                        origin,
                        phase: (i + j) as f32,
                    },
                    NotShadowCaster,
                ));
            }
        }
        // Roadside services: leaning timber poles, crossarms, insulators and sagging cables.
        let mut previous = None;
        for z in [-90., -55., -20., 15., 82., 117.] {
            let top = Vec3::new(-7., 6.5, z);
            b.beam(Vec3::new(-7.4, 0., z), top, 0.19, WOOD);
            b.block(Vec3::new(2., 0.14, 0.18), top, DARK, false);
            for dx in [-0.7, 0.7] {
                b.column(0.07, 0.3, top + Vec3::new(dx, 0.18, 0.), CREAM);
            }
            if let Some(old) = previous {
                for dx in [-0.7, 0.7] {
                    let start: Vec3 = old + Vec3::new(dx, 0.25, 0.);
                    let end = top + Vec3::new(dx, 0.25, 0.);
                    let midpoint = (start + end) * 0.5 - Vec3::Y * 0.8;
                    b.beam(start, midpoint, 0.025, DARK);
                    b.beam(midpoint, end, 0.025, DARK);
                }
            }
            previous = Some(top);
        }
        // Branching asphalt cracks, repair seams, tire marks and dusty shoulders.
        for i in 0..65 {
            let lane = (i % 2) as f32 * 7.;
            let x = lane - 1.9 + surfaces::hash(i, 3, 4) * 3.8;
            let z = -28. + surfaces::hash(i, 4, 8) * 106.;
            let mut from = Vec3::new(x, 0.054, z);
            for j in 0..4 {
                let to = from + Vec3::new((surfaces::hash(i, j, 9) - 0.5) * 0.75, 0., 0.3);
                b.beam(from, to, 0.016, RUBBER);
                if j == 2 {
                    b.beam(from, from + Vec3::new(0.35, 0., -0.2), 0.012, RUBBER);
                }
                from = to;
            }
        }
        for x in [-1.3, 1.3, 5.7, 8.3] {
            for z in [4., 12., 24., 34.] {
                b.block(
                    Vec3::new(0.18, 0.012, 2.6),
                    Vec3::new(x, 0.055, z),
                    ROCK_DARK,
                    false,
                );
            }
        }
        for x in [14., 17., 20., 23.] {
            b.block(
                Vec3::new(0.018, 0.012, 20.),
                Vec3::new(x, 0.061, -1.),
                ROCK_DARK,
                false,
            );
        }
        for z in [-10., -5., 0., 5.] {
            b.block(
                Vec3::new(9.5, 0.012, 0.018),
                Vec3::new(18., 0.061, z),
                ROCK_DARK,
                false,
            );
        }
        // Roof vents and service hardware retain practical detail on the clean masonry office.
        for x in [-14., -10.] {
            b.block(
                Vec3::new(1.1, 0.32, 0.8),
                Vec3::new(x, 3.85, -10.),
                STONE,
                false,
            );
            for j in 0..6 {
                b.block(
                    Vec3::new(0.9, 0.025, 0.03),
                    Vec3::new(x, 3.73 + j as f32 * 0.045, -9.58),
                    DARK,
                    false,
                );
            }
        }
        // Cones, barrels, walking route markers and a water tank add human scale.
        for (x, z) in [
            (-5., 8.),
            (-5., 11.),
            (11., 12.),
            (12., 15.),
            (-3., -19.5),
            (12., -19.5),
        ] {
            b.block(
                Vec3::new(0.55, 0.06, 0.55),
                Vec3::new(x, 0.04, z),
                DARK,
                false,
            );
            b.shape(
                b.cone.clone(),
                Vec3::new(0.24, 0.7, 0.24),
                Vec3::new(x, 0.41, z),
                GOLD,
            );
        }
        for z in [-20., -21.] {
            b.column(0.34, 0.9, Vec3::new(-27., 0.45, z), GREEN);
        }
        b.column(1.1, 2.1, Vec3::new(-23., 1.05, -21.), CREAM);
        b.column(1.15, 0.12, Vec3::new(-23., 2.16, -21.), GREEN);
        for i in 0..10 {
            b.block(
                Vec3::new(1.5, 0.025, 0.12),
                Vec3::new(-11.8, 0.065, -1.6 + i as f32 * 0.45),
                WHITE,
                false,
            );
        }
    }
    for x in [-14.5, -9.5] {
        commands.spawn((
            PointLight {
                color: Color::srgb(1., 0.89, 0.72),
                intensity: 65000.,
                range: 9.,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(x, 3.05, -8.),
        ));
    }
    commands.insert_resource(landmarks);
    terrain(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_fleet(&mut commands, &mut meshes, &mut materials);
    let view = std::env::args()
        .find_map(|a| a.strip_prefix("--scene-view=").map(str::to_owned))
        .unwrap_or_default();
    let (eye, target) = match view.as_str() {
        "overview" => (Vec3::new(-46., 39., 72.), Vec3::new(1., 0., 9.)),
        "cinematic" => (Vec3::new(-36., 12., 70.), Vec3::new(0., 1.5, 10.)),
        "river" => (Vec3::new(35., 15., 75.), Vec3::new(0., 0., 35.)),
        "office" => (Vec3::new(-12., 1.7, -4.5), Vec3::new(-12., 1.4, -11.)),
        "office-exterior" => (Vec3::new(-10., 2., 4.), Vec3::new(-12., 1.6, -7.)),
        "cars" => (Vec3::new(-3.1, 1.3, -3.2), Vec3::new(0., 1.1, 0.)),
        "yard" => (Vec3::new(-4., 2.8, -15.), Vec3::new(-15., 1.1, -24.)),
        "desert" => (Vec3::new(18., 2., 26.), Vec3::new(-25., 2., 41.)),
        "landscape" => (Vec3::new(26., 6., 95.), Vec3::new(40., 18., 190.)),
        "service-road" => (Vec3::new(-5., 2.1, -16.), Vec3::new(-40., 1., -16.)),
        "north-road" => (Vec3::new(18., 6., -30.), Vec3::new(-35., 10., -155.)),
        "rocks" => (Vec3::new(18., 1.8, 30.), Vec3::new(25., 0., 36.)),
        _ => (Vec3::new(-4.5, 1.7, -1.5), Vec3::new(0., 1.23, 2.)),
    };
    let (eye, target) = if let Some(shift) = &shift
        && shift.connected
    {
        (
            Vec3::from_array(
                shift
                    .me()
                    .map(|p| p.position)
                    .unwrap_or(border_net::map::SPAWNS[0]),
            ),
            Vec3::new(-12., 1.7, -3.5),
        )
    } else {
        (eye, target)
    };
    let transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let camera = commands
        .spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 70_f32.to_radians(),
                ..default()
            }),
            transform,
            Player { yaw, pitch },
            bevy::pbr::DistanceFog {
                color: Color::srgb(0.75, 0.69, 0.58),
                falloff: bevy::pbr::FogFalloff::Linear {
                    start: 85.,
                    end: 360.,
                },
                ..default()
            },
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            Msaa::Off,
            bevy::core_pipeline::oit::OrderIndependentTransparencySettings {
                layer_count: 16,
                ..default()
            },
        ))
        .id();
    let light = commands
        .spawn((
            SpotLight {
                intensity: 12000.,
                range: 20.,
                outer_angle: 0.45,
                inner_angle: 0.25,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.12, -0.12, -0.1),
            Visibility::Hidden,
            shift::Flashlight,
        ))
        .id();
    commands.entity(camera).add_child(light);
}

fn spawn_fleet(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cream = Color::srgb(0.91, 0.84, 0.65);
    let dark = Color::srgb(0.045, 0.05, 0.055);
    let red = Color::srgb(0.72, 0.24, 0.16);
    let paint = Color::srgb(0.08, 0.32, 0.30);
    let glass = Color::srgb(0.075, 0.15, 0.20);
    let mut template = Vec::new();
    // Car solids stay on the moving root, geometry uses local transforms.
    let mut part = |size: Vec3, pos: Vec3, color: Color| {
        let mesh = meshes.add(if color == paint && size.z > 4. {
            car_body()
        } else {
            Cuboid::from_size(size).mesh().build()
        });
        let rotation = if color == glass && size.x > 1.5
            || color == paint && size.x < 0.15 && size.y > 0.6 && pos.z.abs() > 0.8
        {
            Quat::from_rotation_x(if pos.z < 0. { 0.5 } else { -0.45 })
        } else {
            Quat::IDENTITY
        };
        let local = Transform::from_translation(pos)
            .with_rotation(rotation)
            .with_scale(if color == paint && size.z > 4. {
                Vec3::new(1., 0.55, 1.)
            } else {
                Vec3::ONE
            });
        let material = materials.add(StandardMaterial {
            base_color: color,
            metallic: if color == paint { 0.42 } else { 0. },
            perceptual_roughness: if color == paint {
                0.27
            } else if color == glass {
                0.12
            } else {
                0.65
            },
            clearcoat: if color == paint { 1. } else { 0. },
            clearcoat_perceptual_roughness: 0.16,
            ..default()
        });
        template.push((mesh.clone(), material.clone(), local, color == paint));
    };

    part(Vec3::new(2.05, 0.58, 4.45), Vec3::new(0., 0.5, 0.), paint);
    part(Vec3::new(1.74, 0.12, 1.88), Vec3::new(0., 1.65, 0.1), paint);
    for z in [-2.26, 2.26] {
        part(Vec3::new(1.9, 0.14, 0.1), Vec3::new(0., 0.43, z), dark);
    }
    for x in [-0.73, 0.73] {
        part(
            Vec3::new(0.4, 0.2, 0.06),
            Vec3::new(x, 0.76, -2.25),
            Color::srgb(1., 0.94, 0.68),
        );
        part(Vec3::new(0.4, 0.2, 0.06), Vec3::new(x, 0.76, 2.25), red);
    }
    part(Vec3::new(0.6, 0.18, 0.06), Vec3::new(0., 0.52, 2.32), cream);
    part(Vec3::new(0.9, 0.16, 0.04), Vec3::new(0., 0.74, -2.29), dark);
    for x in [-1.02, 1.02] {
        for z in [-1.45, 1.43] {
            let mesh = meshes.add(Cylinder::new(0.38, 0.22).mesh().resolution(12));
            let material = materials.add(StandardMaterial {
                base_color: dark,
                perceptual_roughness: 0.95,
                ..default()
            });
            let transform = Transform::from_xyz(x, 0.38, z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
            template.push((mesh.clone(), material.clone(), transform, false));
            let mesh = meshes.add(Cylinder::new(0.22, 0.025).mesh().resolution(16));
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.46, 0.49, 0.52),
                metallic: 0.85,
                perceptual_roughness: 0.28,
                ..default()
            });
            let transform = Transform::from_xyz(x + x.signum() * 0.13, 0.38, z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
            template.push((mesh.clone(), material.clone(), transform, false));
        }
    }
    for lane in 0..2 {
        for ordinal in 0..if lane == 0 { 10 } else { 5 } {
            let vehicle = traffic::Vehicle::new(lane, ordinal);
            let (position, rotation) = traffic::incoming(vehicle.distance, lane);
            let root = commands
                .spawn((
                    Transform::from_translation(position).with_rotation(rotation),
                    Visibility::default(),
                    vehicle,
                    Solid {
                        half: Vec3::new(1.08, 0.9, 2.3),
                    },
                ))
                .id();
            let paint = materials.add(StandardMaterial {
                base_color: match (ordinal + lane as usize) % 5 {
                    0 => Color::srgb(0.22, 0.41, 0.39),
                    1 => Color::srgb(0.69, 0.42, 0.28),
                    2 => Color::srgb(0.81, 0.76, 0.60),
                    3 => Color::srgb(0.36, 0.40, 0.44),
                    _ => Color::srgb(0.55, 0.25, 0.18),
                },
                metallic: 0.42,
                clearcoat: 1.,
                clearcoat_perceptual_roughness: 0.16,
                perceptual_roughness: 0.27,
                ..default()
            });
            let kind = border_net::vehicle::scheduled_kind(lane as usize + ordinal * 2);
            if !border_net::vehicle::pickup(kind) {
                let sedan = commands
                    .spawn((
                        Transform::default(),
                        Visibility::default(),
                        vehicles::Body {
                            lane,
                            ordinal,
                            kind,
                        },
                    ))
                    .id();
                commands.entity(root).add_child(sedan);
                for (mesh, material, transform, body) in &template {
                    if matches!(kind, "suv" | "van")
                        && (transform.translation.y - 1.65).abs() < 0.01
                    {
                        continue;
                    }
                    let mut transform = *transform;
                    let profile = border_net::vehicle::spec(kind);
                    if transform.translation.y > 0.95 {
                        transform.translation.y += border_net::vehicle::cab_lift(kind);
                    }
                    if transform.translation.z.abs() > 2.1 {
                        transform.translation.z = transform.translation.z.signum()
                            * (profile.half_length + (transform.translation.z.abs() - 2.3));
                    }
                    if transform.translation.y < 0.55
                        && transform.translation.z.abs() < 1.6
                        && transform.translation.z.abs() > 1.3
                    {
                        transform.translation.z *= profile.half_length / 2.3;
                    }
                    if (transform.translation.y - 0.5).abs() < 0.01 {
                        transform.scale.z *= profile.half_length / 2.3;
                        transform.scale.x *= profile.half_width / 1.08;
                    }
                    let part = commands
                        .spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(if *body {
                                paint.clone()
                            } else {
                                material.clone()
                            }),
                            transform,
                        ))
                        .id();
                    commands.entity(sedan).add_child(part);
                }
                vehicles::sedan_panels(commands, meshes, sedan, paint.clone(), lane, ordinal, kind);
            } else {
                vehicles::truck(
                    commands,
                    meshes,
                    materials,
                    root,
                    lane,
                    ordinal,
                    paint.clone(),
                    kind,
                );
            }
            vehicles::dressing(
                commands,
                meshes,
                materials,
                root,
                lane,
                ordinal,
                kind,
                paint.clone(),
            );
        }
    }
    let mesh = meshes.add(Sphere::new(1.).mesh().ico(2).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.65, 0.53, 0.37, 0.055),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    for lane in 0..2 {
        for ordinal in 0..if lane == 0 { 10 } else { 5 } {
            for i in 0..3 {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_scale(Vec3::splat(0.001)),
                    traffic::VehicleDust {
                        lane,
                        ordinal,
                        phase: i as f32 / 3.,
                    },
                    NotShadowCaster,
                ));
            }
        }
    }
}

fn open_dressing(p: Vec2) -> bool {
    !(p.x > -3.7 && p.x < 11.5
        || p.x > -26.5 && p.x < -6. && p.y > -29. && p.y < -2.5
        || p.x > 12.5 && p.x < 23.5 && p.y > -12. && p.y < 12.
        || p.x > -18. && p.x < -6. && p.y > -4. && p.y < 6.)
        && (p.y + 16.).abs() > 3.
}
fn terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let (albedo, normal) = surfaces::maps(images, 0);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.66, 0.51, 0.34),
        base_color_texture: Some(albedo),
        normal_map_texture: Some(normal),
        perceptual_roughness: 0.99,
        ..default()
    });
    for (start, end) in [(-400., 36.), (65., 440.)] {
        let width = 150;
        let depth = ((end - start) / 6.) as usize;
        let mut positions = Vec::new();
        let mut uv = Vec::new();
        let mut indices = Vec::new();
        for z in 0..=depth {
            for x in 0..=width {
                let px = -450. + x as f32 * 6.;
                let pz = start + (end - start) * z as f32 / depth as f32;
                let p = Vec2::new(px, pz);
                let developed = px > -38. && px < 50. && pz > -44. && pz < 82.;
                let nearroad = traffic::near_road(p, 18.);
                let mut height = if developed || nearroad {
                    -0.055
                } else {
                    (surfaces::hash(x, z as i32, 44) - 0.3) * 1.8
                        + (surfaces::hash(x / 4, z as i32 / 4, 97) - 0.4) * 1.5
                };
                let edge = if end == 36. { 36. - pz } else { pz - 65. };
                if edge < 5. && !nearroad {
                    height -= 2.1 * (1. - edge / 5.);
                }
                positions.push([px, height, pz]);
                uv.push([px / 5., pz / 5.]);
            }
        }
        let stride = width as u32 + 1;
        for z in 0..depth as u32 {
            for x in 0..width as u32 {
                let a = z * stride + x;
                indices.extend_from_slice(&[
                    a,
                    a + stride,
                    a + 1,
                    a + 1,
                    a + stride,
                    a + stride + 1,
                ]);
            }
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(indices));
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();
        let _ = mesh.generate_tangents();
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::default(),
        ));
    }
}

fn car_body() -> Mesh {
    let rings = [
        (-2.22, 0.89, 0.10),
        (-1.85, 1.015, 0.29),
        (-0.9, 1.025, 0.29),
        (1.25, 1.02, 0.25),
        (2.22, 0.93, 0.15),
    ];
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for (z, w, top) in rings {
        let bottom = -0.29;
        for (j, (x, y)) in [
            (-w + 0.1, bottom),
            (-w, bottom + 0.09),
            (-w, top - 0.09),
            (-w + 0.1, top),
            (w - 0.1, top),
            (w, top - 0.09),
            (w, bottom + 0.09),
            (w - 0.1, bottom),
        ]
        .into_iter()
        .enumerate()
        {
            positions.push([x, y, z]);
            uv.push([j as f32 / 8., (z + 2.22) / 4.44]);
        }
    }
    for ring in 0..4u32 {
        for i in 0..8u32 {
            let a = ring * 8 + i;
            let b = ring * 8 + (i + 1) % 8;
            indices.extend_from_slice(&[a, b + 8, b, a, a + 8, b + 8]);
        }
    }
    for i in 1..7 {
        indices.extend_from_slice(&[0, i, i + 1, 32, 32 + i + 1, 32 + i]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices));
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};
    #[test]
    fn service_road_keeps_a_clear_driving_corridor() {
        for solid in border_net::map::collision().solids {
            if solid.center[0] + solid.half[0] > -31.
                && solid.center[0] - solid.half[0] < -5.
                && solid.center[1] + solid.half[1] > 0.15
            {
                assert!(
                    (solid.center[2] + 16.).abs() >= solid.half[2] + 2.,
                    "solid obstructs service road: {solid:?}"
                );
            }
        }
    }
    #[test]
    fn static_collision_matches_the_host_map() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Startup, setup);
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Transform, &Solid), Without<traffic::Vehicle>>();
        let mut actual: Vec<_> = query
            .iter(world)
            .map(|(t, s)| border_net::map::Aabb {
                center: t.translation.to_array(),
                half: s.half.to_array(),
            })
            .collect();
        let mut baked = border_net::map::collision().solids;
        let order = |a: &border_net::map::Aabb, b: &border_net::map::Aabb| {
            a.center[0]
                .total_cmp(&b.center[0])
                .then(a.center[1].total_cmp(&b.center[1]))
                .then(a.center[2].total_cmp(&b.center[2]))
        };
        actual.sort_by(order);
        baked.sort_by(order);
        assert_eq!(
            actual, baked,
            "regenerate with --export-collision=assets/maps/san-ignacio-collision.json after changing level solids"
        );
    }
    #[test]
    fn every_inspection_space_and_bridge_is_reachable_without_crossing_water_or_walls() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Startup, setup);
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<(&Transform, &Solid)>();
        let solids: Vec<_> = query
            .iter(world)
            .map(|(t, s)| (t.translation, s.half))
            .collect();
        let clear = |x: i32, z: i32| {
            let p = Vec3::new(x as f32 * 0.5, 1.7, z as f32 * 0.5);
            walkable(p)
                && !solids.iter().any(|(center, half)| {
                    (p.x - center.x).abs() < half.x + 0.22 && (p.z - center.z).abs() < half.z + 0.22
                })
        };
        let start = (-9, 12);
        assert!(clear(start.0, start.1));
        let mut visited = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some((x, z)) = queue.pop_front() {
            for next in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
                if !visited.contains(&next) && clear(next.0, next.1) {
                    visited.insert(next);
                    queue.push_back(next);
                }
            }
        }
        for (name, target) in [
            ("office", (-24, -12)),
            ("storage", (-44, -12)),
            ("holding yard", (-28, -42)),
            ("secondary bay", (36, 0)),
            ("bridge", (7, 110)),
            ("south pedestrian limit", (7, 148)),
            ("driver", (-3, 0)),
        ] {
            assert!(visited.contains(&target), "unreachable {name}");
        }
        assert!(
            !visited.contains(&(-20, 100)),
            "must not walk on river west of bridge"
        );
        assert!(
            !visited.contains(&(30, 100)),
            "must not walk on river east of bridge"
        );
        assert!(
            !visited.contains(&(82, 0)),
            "future lanes remain outside the playable yard"
        );
    }
}
