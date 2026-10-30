use super::*;
use bevy::light::NotShadowCaster;

// X is east/west; +Z leads south across the river. Texas is the north bank.
pub const YARD_MIN: Vec2 = Vec2::new(-27., -32.);
pub const YARD_MAX: Vec2 = Vec2::new(24., 35.);
pub fn walkable(p: Vec3) -> bool {
    (p.x >= YARD_MIN.x && p.x <= YARD_MAX.x && p.z >= YARD_MIN.y && p.z <= YARD_MAX.y)
        || (p.x >= -2.6 && p.x <= 9.6 && p.z >= 35. && p.z <= 77.)
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
const HILL: usize = 15;
const HAZE: usize = 16;
const WHITE: usize = 17;
const PAINT: usize = 18;
const RUBBER: usize = 19;
struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    cube: Handle<Mesh>,
    round: Handle<Mesh>,
    cone: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}
impl<'a, 'w, 's> Builder<'a, 'w, 's> {
    fn new(
        commands: &'a mut Commands<'w, 's>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
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
        ];
        let materials = colors
            .into_iter()
            .enumerate()
            .map(|(i, (r, g, b))| {
                materials.add(StandardMaterial {
                    base_color: if i == HAZE {
                        Color::srgba(r, g, b, 0.12)
                    } else {
                        Color::srgb(r, g, b)
                    },
                    perceptual_roughness: if i == WATER { 0.23 } else { 0.87 },
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
        Self {
            commands,
            cube: meshes.add(Cuboid::new(1., 1., 1.)),
            round: meshes.add(Sphere::new(1.).mesh().ico(1).expect("valid sphere")),
            cone: meshes.add(Cone::new(1., 1.).mesh().resolution(6)),
            cylinder: meshes.add(Cylinder::new(1., 1.).mesh().resolution(8)),
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
    fn fence_z(&mut self, x: f32, start: f32, end: f32, height: f32) {
        self.block(
            Vec3::new(0.12, height, end - start),
            Vec3::new(x, height * 0.5, (start + end) * 0.5),
            DARK,
            true,
        );
        for i in 0..=((end - start) / 1.5) as usize {
            self.block(
                Vec3::new(0.22, height + 0.15, 0.22),
                Vec3::new(x, height * 0.5, start + i as f32 * 1.5),
                RUST,
                false,
            );
        }
        for y in [0.25, height - 0.2] {
            self.block(
                Vec3::new(0.28, 0.06, end - start),
                Vec3::new(x, y, (start + end) * 0.5),
                STONE,
                false,
            );
        }
    }
    fn parked_car(&mut self, x: f32, z: f32, color: usize) {
        self.block(Vec3::new(2., 0.55, 4.3), Vec3::new(x, 0.66, z), color, true);
        self.block(
            Vec3::new(1.75, 0.65, 2.1),
            Vec3::new(x, 1.24, z + 0.1),
            GLASS,
            false,
        );
        self.block(
            Vec3::new(1.85, 0.12, 2.25),
            Vec3::new(x, 1.62, z + 0.1),
            color,
            false,
        );
        for dz in [-2.2, 2.2] {
            self.block(
                Vec3::new(1.85, 0.14, 0.12),
                Vec3::new(x, 0.43, z + dz),
                STONE,
                false,
            );
        }
        for side in [-1., 1.] {
            for dz in [-1.35, 1.35] {
                let e = self.column(0.36, 0.22, Vec3::new(x + side, 0.38, z + dz), RUBBER);
                self.commands.entity(e).insert(
                    Transform::from_xyz(x + side, 0.38, z + dz)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
                        .with_scale(Vec3::new(0.36, 0.22, 0.36)),
                );
            }
        }
        for dx in [-0.65, 0.65] {
            self.block(
                Vec3::new(0.34, 0.18, 0.07),
                Vec3::new(x + dx, 0.74, z - 2.18),
                CREAM,
                false,
            );
            self.block(
                Vec3::new(0.34, 0.18, 0.07),
                Vec3::new(x + dx, 0.74, z + 2.18),
                RED,
                false,
            );
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
        // Ribs and a few warm flower tips make the silhouette readable at walking distance.
        self.block(
            Vec3::new(0.055, h * 0.87, 0.03),
            Vec3::new(x - 0.08, h * 0.48, z + 0.19),
            SAGE,
            false,
        );
        self.ball(Vec3::splat(0.10), Vec3::new(x, h + 0.13, z), GOLD);
    }
}
pub fn atmosphere(
    time: Res<Time>,
    mut ripples: Query<(&mut Transform, &RiverRipple), Without<Dust>>,
    mut dust: Query<(&mut Transform, &Dust), Without<RiverRipple>>,
) {
    for (mut t, r) in &mut ripples {
        t.translation = r.origin
            + Vec3::new(
                (time.elapsed_secs() * 0.22 + r.phase).sin() * 1.5,
                (time.elapsed_secs() + r.phase).sin() * 0.018,
                0.,
            );
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
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        let mut b = Builder::new(&mut commands, &mut meshes, &mut materials);
        // Separate banks leave a genuine river channel under a raised road deck.
        b.block(
            Vec3::new(1000., 0.5, 650.),
            Vec3::new(0., -0.31, -289.),
            SAND,
            false,
        ); // north bank ends z=36
        b.block(
            Vec3::new(1000., 0.5, 650.),
            Vec3::new(0., -0.31, 390.),
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
                b.ball(
                    Vec3::new(3.4, 1.25, 1.7),
                    Vec3::new(i as f32 * 7.1, -1.1, bank),
                    if i % 3 == 0 { CLAY } else { SAND },
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
            Vec3::new(13.5, 0.08, 114.),
            Vec3::new(3.5, -0.01, 22.),
            ROAD,
            false,
        );
        for x in [-2.65, 3.5, 9.65] {
            for i in -9..20 {
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
        // Second lane barrier is static set dressing until Part 6 lane lifecycle integration.
        b.block(
            Vec3::new(0.5, 1.2, 0.5),
            Vec3::new(9.9, 0.6, -4.),
            GOLD,
            true,
        );
        for i in 0..12 {
            b.block(
                Vec3::new(0.46, 0.17, 0.18),
                Vec3::new(9.65 - i as f32 * 0.46, 1.1, -4.),
                if i % 2 == 0 { CREAM } else { RED },
                false,
            );
        }
        // Existing interactive barrier keeps its animated hinge and original coordinates.
        b.block(
            Vec3::new(0.5, 1.2, 0.5),
            Vec3::new(2.9, 0.6, -4.),
            GOLD,
            true,
        );
        let pivot = b
            .commands
            .spawn((
                Transform::from_xyz(2.9, 1.1, -4.),
                Visibility::default(),
                Barrier,
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
                CREAM,
                true,
            );
        }
        b.block(
            Vec3::new(10., 3.5, 0.25),
            Vec3::new(-12., 1.75, -12.5),
            CREAM,
            true,
        );
        for x in [-15.05, -8.95] {
            b.block(
                Vec3::new(3.9, 3.5, 0.25),
                Vec3::new(x, 1.75, -3.5),
                CREAM,
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
        // Detention yard: perimeter, open east gate, benches and shade.
        b.block(
            Vec3::new(12., 0.1, 10.),
            Vec3::new(-14., 0., -23.),
            STONE,
            false,
        );
        for z in [-28., -18.] {
            b.fence_x(-20., -8., z, 2.5);
        }
        b.fence_z(-20., -28., -18., 2.5);
        b.fence_z(-8., -28., -24.5, 2.5);
        b.fence_z(-8., -21.5, -18., 2.5);
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
            Vec3::new(-18., 0.8, -14.),
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
        // Static queue and utility van; later vehicle lifecycle owns their activation.
        for (x, z, c) in [
            (0., 12., CLAY),
            (0., 20., CREAM),
            (0., 28., GREEN),
            (7., 0., PAINT),
            (7., 13., WHITE),
            (7., 23., RED),
            (0., 70., CREAM),
        ] {
            b.parked_car(x, z, c);
        }
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
        // Visible yard limits and a closed gate at the far end of the walkable bridge.
        b.fence_z(-27.6, -33., 32., 2.2);
        b.fence_z(24.6, -33., 32., 2.2);
        b.fence_x(-27.6, -3., -33., 2.2);
        b.fence_x(10., 24.6, -33., 2.2);
        for x in [0., 7.] {
            b.block(
                Vec3::new(5.7, 0.7, 0.35),
                Vec3::new(x, 0.45, -32.8),
                GOLD,
                true,
            );
        }
        b.block(
            Vec3::new(13., 1.1, 0.35),
            Vec3::new(3.5, 0.55, 77.5),
            GOLD,
            true,
        );
        // Reserved eastern apron: no extra operational lanes until the expansion milestone.
        b.block(
            Vec3::new(26., 0.03, 30.),
            Vec3::new(41., -0.02, -4.),
            CLAY,
            false,
        );
        b.sign(Vec3::new(30., 0., 13.), 4., GOLD);
        landmarks
            .0
            .push(("FUTURE LANES  /  CLOSED", Vec3::new(30., 2.4, 13.15)));
        // Layered distant mesas, dunes and desert scrub. Deterministic placement keeps captures reproducible.
        for i in 0..38 {
            let angle = i as f32 * 2.39996;
            let radius = 150. + (i % 6) as f32 * 31.;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let e = b.shape(
                b.cone.clone(),
                Vec3::new(
                    32. + (i % 4) as f32 * 8.,
                    24. + (i % 7) as f32 * 4.,
                    27. + (i % 5) as f32 * 8.,
                ),
                Vec3::new(x, 6., z),
                if i % 3 == 0 { CLAY } else { HILL },
            );
            b.commands.entity(e).insert(NotShadowCaster);
            b.ball(
                Vec3::new(25., 4., 18.),
                Vec3::new(x * 0.58, 0., z * 0.58),
                SAND,
            );
        }
        for i in 0..195 {
            let x = ((i * 47 + 19) % 283) as f32 - 141.;
            let z = ((i * 71 + 31) % 235) as f32 - 105.;
            if (x > -30. && x < 49. && z > -36. && z < 82.) || (35. ..66.).contains(&z) {
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
                b.ball(
                    Vec3::new(0.6, 0.23, 0.4),
                    Vec3::new(x + 1.8, 0.12, z - 1.),
                    STONE,
                );
            }
        }
        // Close plants around walkways and buildings, kept out of the vehicle/door clearances.
        for (x, z, h) in [
            (-24., 13., 2.8),
            (-20., 5., 2.1),
            (-6., 18., 2.5),
            (20., 22., 3.2),
            (-23., -16., 1.9),
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
        // Cones, barrels, walking route markers and a water tank add human scale.
        for (x, z) in [
            (-5., 8.),
            (-5., 11.),
            (11., 12.),
            (12., 15.),
            (-3., -16.),
            (10., -16.),
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
        for z in [-14., -15.] {
            b.column(0.34, 0.9, Vec3::new(-23., 0.45, z), GREEN);
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
    let cream = Color::srgb(0.91, 0.84, 0.65);
    let dark = Color::srgb(0.13, 0.20, 0.20);
    let red = Color::srgb(0.72, 0.24, 0.16);
    // Sedan: dark teal paint, open driver's side, visible driver.
    let car = commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            Car,
            Solid {
                half: Vec3::new(1.08, 0.9, 2.3),
            },
        ))
        .id();
    // Car solids stay on the moving root, geometry uses local transforms.
    let mut part = |size: Vec3, pos: Vec3, color: Color| {
        let e = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.6,
                    ..default()
                })),
                Transform::from_translation(pos),
            ))
            .id();
        commands.entity(car).add_child(e);
    };
    let paint = Color::srgb(0.12, 0.36, 0.34);
    let glass = Color::srgb(0.2, 0.31, 0.32);
    part(Vec3::new(2.05, 0.58, 4.45), Vec3::new(0., 0.65, 0.), paint);
    part(Vec3::new(1.84, 0.12, 2.2), Vec3::new(0., 1.65, 0.1), paint);
    part(
        Vec3::new(1.77, 0.65, 0.07),
        Vec3::new(0., 1.28, -0.98),
        glass,
    );
    part(
        Vec3::new(1.77, 0.62, 0.07),
        Vec3::new(0., 1.27, 1.16),
        glass,
    );
    part(
        Vec3::new(0.06, 0.62, 1.95),
        Vec3::new(0.92, 1.27, 0.1),
        glass,
    );
    for z in [-0.96, 0.25, 1.15] {
        part(Vec3::new(0.1, 0.7, 0.1), Vec3::new(-0.91, 1.28, z), paint);
    }
    part(
        Vec3::new(0.06, 0.55, 0.78),
        Vec3::new(-0.92, 1.24, 0.73),
        glass,
    );
    part(
        Vec3::new(0.06, 0.06, 0.27),
        Vec3::new(-1.05, 0.85, -0.22),
        cream,
    );
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
    part(
        Vec3::new(0.45, 0.55, 0.35),
        Vec3::new(-0.48, 1.02, -0.18),
        Color::srgb(0.71, 0.46, 0.24),
    );
    part(
        Vec3::new(0.28, 0.32, 0.28),
        Vec3::new(-0.48, 1.47, -0.25),
        Color::srgb(0.72, 0.54, 0.4),
    );
    part(
        Vec3::new(0.3, 0.10, 0.3),
        Vec3::new(-0.48, 1.66, -0.25),
        Color::srgb(0.19, 0.16, 0.12),
    );
    part(
        Vec3::new(0.08, 0.07, 0.06),
        Vec3::new(-0.65, 1.5, -0.31),
        dark,
    );
    part(
        Vec3::new(0.18, 0.12, 0.4),
        Vec3::new(-0.72, 1.08, -0.55),
        Color::srgb(0.71, 0.46, 0.24),
    );
    for x in [-1.02, 1.02] {
        for z in [-1.45, 1.43] {
            let wheel = commands
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.38, 0.22).mesh().resolution(12))),
                    MeshMaterial3d(materials.add(dark)),
                    Transform::from_xyz(x, 0.38, z)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ))
                .id();
            commands.entity(car).add_child(wheel);
        }
    }
    let view = std::env::args()
        .find_map(|a| a.strip_prefix("--scene-view=").map(str::to_owned))
        .unwrap_or_default();
    let (eye, target) = match view.as_str() {
        "overview" => (Vec3::new(-46., 39., 72.), Vec3::new(1., 0., 9.)),
        "river" => (Vec3::new(35., 15., 75.), Vec3::new(0., 0., 35.)),
        "office" => (Vec3::new(-12., 1.7, -4.5), Vec3::new(-12., 1.4, -11.)),
        "yard" => (Vec3::new(-4., 2.8, -15.), Vec3::new(-15., 1.1, -24.)),
        "desert" => (Vec3::new(18., 2., 26.), Vec3::new(-25., 2., 41.)),
        _ => (Vec3::new(-4.5, 1.7, 5.8), Vec3::new(0., 1.23, -1.2)),
    };
    let transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    commands.spawn((
        Camera3d::default(),
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
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};
    #[test]
    fn every_inspection_space_and_bridge_is_reachable_without_crossing_water_or_walls() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
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
            ("far bridge gate", (7, 148)),
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
