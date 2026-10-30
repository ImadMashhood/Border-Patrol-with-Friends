use super::*;
// Everything is built from primitives, so the checkout has no art downloads.
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.74, 0.84, 0.91),
        brightness: 420.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 12500.,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-8., 14., 8.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    macro_rules! block {
        ($commands:expr, $size:expr, $pos:expr, $color:expr, $solid:expr $(,)?) => {{
            let mut e = $commands.spawn((
                Mesh3d(meshes.add(Cuboid::from_size($size))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: $color,
                    perceptual_roughness: 0.88,
                    ..default()
                })),
                Transform::from_translation($pos),
            ));
            if $solid {
                e.insert(Solid { half: $size * 0.5 });
            }
            let id = e.id();
            id
        }};
    }
    let sand = Color::srgb(0.52, 0.53, 0.39);
    let concrete = Color::srgb(0.69, 0.69, 0.60);
    let dark = Color::srgb(0.11, 0.16, 0.17);
    let cream = Color::srgb(0.91, 0.86, 0.68);
    let red = Color::srgb(0.78, 0.24, 0.15);
    let yellow = Color::srgb(0.96, 0.67, 0.25);
    block!(
        &mut commands,
        Vec3::new(160., 0.2, 160.),
        Vec3::new(0., -0.16, 0.),
        sand,
        false,
    );
    block!(
        &mut commands,
        Vec3::new(6., 0.08, 160.),
        Vec3::new(0., -0.02, 0.),
        Color::srgb(0.22, 0.26, 0.26),
        false,
    );
    for z in -22..22 {
        block!(
            &mut commands,
            Vec3::new(0.09, 0.02, 1.2),
            Vec3::new(0., 0.04, z as f32 * 3.4),
            cream,
            false,
        );
    }
    // Road edges and the inspection stop line.
    for x in [-2.8, 2.8] {
        block!(
            &mut commands,
            Vec3::new(0.08, 0.025, 140.),
            Vec3::new(x, 0.04, 0.),
            cream,
            false,
        );
    }
    block!(
        &mut commands,
        Vec3::new(5.5, 0.025, 0.22),
        Vec3::new(0., 0.045, -2.8),
        yellow,
        false,
    );
    block!(
        &mut commands,
        Vec3::new(4.2, 0.22, 11.),
        Vec3::new(-5.1, 0.04, -0.3),
        concrete,
        false,
    );
    // Booth with a broad window and contrasting cladding.
    block!(
        &mut commands,
        Vec3::new(2.9, 2.8, 3.5),
        Vec3::new(-6., 1.4, -1.6),
        cream,
        true,
    );
    block!(
        &mut commands,
        Vec3::new(3.1, 0.18, 3.8),
        Vec3::new(-6., 2.88, -1.6),
        dark,
        false,
    );
    block!(
        &mut commands,
        Vec3::new(0.05, 1.1, 2.3),
        Vec3::new(-4.53, 1.8, -1.5),
        Color::srgb(0.17, 0.34, 0.36),
        false,
    );
    for z in [-2.3, -1.5, -0.7] {
        block!(
            &mut commands,
            Vec3::new(0.07, 1.1, 0.045),
            Vec3::new(-4.48, 1.8, z),
            cream,
            false,
        );
    }
    block!(
        &mut commands,
        Vec3::new(0.22, 0.12, 2.6),
        Vec3::new(-4.43, 1.22, -1.5),
        dark,
        false,
    );
    for y in [0.4, 0.7] {
        block!(
            &mut commands,
            Vec3::new(0.06, 0.13, 3.45),
            Vec3::new(-4.5, y, -1.6),
            Color::srgb(0.29, 0.42, 0.35),
            false,
        );
    }
    // Barrier pivot. Child geometry keeps hinge-based movement believable.
    block!(
        &mut commands,
        Vec3::new(0.5, 1.2, 0.5),
        Vec3::new(2.9, 0.6, -4.),
        yellow,
        true,
    );
    let pivot = commands
        .spawn((
            Transform::from_xyz(2.9, 1.1, -4.),
            Visibility::default(),
            Barrier,
        ))
        .id();
    for i in 0..12 {
        let e = block!(
            &mut commands,
            Vec3::new(0.46, 0.17, 0.18),
            Vec3::new(-0.25 - i as f32 * 0.46, 0., 0.),
            if i % 2 == 0 { cream } else { red },
            false,
        );
        commands.entity(pivot).add_child(e);
    }
    // Fencing, bollards, signs, security gantry.
    for side in [-1., 1.] {
        for i in -7..8 {
            block!(
                &mut commands,
                Vec3::new(0.09, 2.5, 0.09),
                Vec3::new(side * 9., 1.25, i as f32 * 2.),
                dark,
                true,
            );
        }
        for y in [0.6, 1.6, 2.4] {
            block!(
                &mut commands,
                Vec3::new(0.06, 0.04, 30.),
                Vec3::new(side * 9., y, 0.),
                Color::srgb(0.36, 0.41, 0.37),
                false,
            );
        }
        for i in -30..31 {
            block!(
                &mut commands,
                Vec3::new(0.025, 2.35, 0.025),
                Vec3::new(side * 9., 1.2, i as f32 * 0.5),
                Color::srgb(0.38, 0.43, 0.38),
                false,
            );
        }
        for z in [-5.5, -3.8, 3.5] {
            block!(
                &mut commands,
                Vec3::new(0.23, 0.9, 0.23),
                Vec3::new(side * 3.3, 0.45, z),
                yellow,
                true,
            );
            block!(
                &mut commands,
                Vec3::new(0.24, 0.15, 0.24),
                Vec3::new(side * 3.3, 0.65, z),
                dark,
                false,
            );
        }
        block!(
            &mut commands,
            Vec3::new(0.2, 5.8, 0.2),
            Vec3::new(side * 3.65, 2.9, -6.),
            dark,
            true,
        );
    }
    block!(
        &mut commands,
        Vec3::new(7.5, 0.28, 0.35),
        Vec3::new(0., 5.7, -6.),
        dark,
        false,
    );
    block!(
        &mut commands,
        Vec3::new(4.7, 0.95, 0.14),
        Vec3::new(0., 4.9, -5.98),
        Color::srgb(0.14, 0.3, 0.27),
        false,
    );
    // Simple abstract border crest on the gantry.
    for x in [-0.6, 0., 0.6] {
        block!(
            &mut commands,
            Vec3::new(0.26, 0.42, 0.04),
            Vec3::new(x, 4.9, -5.88),
            cream,
            false,
        );
    }
    block!(
        &mut commands,
        Vec3::new(0.08, 2.7, 0.08),
        Vec3::new(4.2, 1.35, 2.3),
        dark,
        true,
    );
    block!(
        &mut commands,
        Vec3::new(1.2, 0.85, 0.12),
        Vec3::new(4.2, 2.25, 2.3),
        yellow,
        false,
    );
    block!(
        &mut commands,
        Vec3::new(0.75, 0.12, 0.03),
        Vec3::new(4.2, 2.25, 2.38),
        dark,
        false,
    );
    // Landscape: low-poly cypress silhouettes and angular hills.
    for i in 0..32 {
        let x = if i % 2 == 0 { -1. } else { 1. } * (14. + (i % 5) as f32 * 3.5);
        let z = (i as f32 * 7.1) % 100. - 48.;
        block!(
            &mut commands,
            Vec3::new(0.35, 2.7, 0.35),
            Vec3::new(x, 1.3, z),
            Color::srgb(0.31, 0.29, 0.22),
            false,
        );
        let h = 3. + (i % 3) as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cone::new(1.5, h).mesh().resolution(5))),
            MeshMaterial3d(materials.add(Color::srgb(0.25, 0.36 + (i % 3) as f32 * 0.025, 0.28))),
            Transform::from_xyz(x, 2.4 + h * 0.3, z),
        ));
    }
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
    let eye = Vec3::new(-4.5, 1.7, 5.8);
    let target = Vec3::new(0., 1.23, -1.2);
    let transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    commands.spawn((
        Camera3d::default(),
        transform,
        Player { yaw, pitch },
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
    ));
    // Keep the sunlit scene deterministic; no procedural texture or asset loading.
}
