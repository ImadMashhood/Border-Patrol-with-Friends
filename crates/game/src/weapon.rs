//! Player pistol, input, tracers and vehicle mechanism shortcuts.
use super::*;
use border_net::{ClientCommand, VehicleStage, combat::*};
#[derive(Resource, Default)]
pub struct WeaponUi {
    pub next_fire: f32,
    pub last_shot: Option<u64>,
    pub recoil: f32,
    pub root: Option<Entity>,
    tracer_mesh: Option<Handle<Mesh>>,
    tracer_material: Option<Handle<StandardMaterial>>,
    was_looking: bool,
    pub aim_blend: f32,
}
#[derive(Component)]
pub struct Gun;
#[derive(Component)]
pub struct Slide(pub f32);
#[derive(Component)]
pub struct Flash;
#[derive(Component)]
pub struct Tracer(pub u64);
fn part(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    parent: Entity,
    size: Vec3,
    pos: Vec3,
    material: Handle<StandardMaterial>,
    slide: bool,
) {
    let e = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
            Transform::from_translation(pos),
        ))
        .id();
    if slide {
        commands.entity(e).insert(Slide(pos.z));
    }
    commands.entity(parent).add_child(e);
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn render(
    shift: Res<shift::Shift>,
    patrol: Res<Patrol>,
    smoke: Option<Res<SmokeTest>>,
    time: Res<Time>,
    mut ui: ResMut<WeaponUi>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera: Query<(Entity, &mut Projection), With<Player>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut guns: Query<(&mut Transform, &mut Visibility), With<Gun>>,
    mut slides: Query<(&Slide, &mut Transform), Without<Gun>>,
    mut flashes: Query<&mut Visibility, (With<Flash>, Without<Gun>)>,
    mut tracers: Query<(Entity, &Tracer, &mut Transform), (Without<Gun>, Without<Slide>)>,
) {
    let Some(world) = &shift.world else {
        return;
    };
    let Some(me) = shift.me() else {
        return;
    };
    if ui.root.is_none()
        && let Ok((camera, _)) = camera.single()
    {
        let root = commands
            .spawn((
                Transform::from_xyz(0.17, -0.15, -0.60),
                Visibility::Hidden,
                Gun,
            ))
            .id();
        commands.entity(camera).add_child(root);
        ui.root = Some(root);
        let metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.12, 0.14),
            metallic: 0.82,
            perceptual_roughness: 0.30,
            ..default()
        });
        let grip = materials.add(Color::srgb(0.06, 0.055, 0.05));
        let skin = materials.add(Color::srgb(0.64, 0.44, 0.31));
        let sleeve = materials.add(Color::srgb(0.15, 0.23, 0.18));
        let sight = materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.95, 0.65),
            unlit: true,
            ..default()
        });
        for (size, pos, mat, slide) in [
            (
                Vec3::new(0.065, 0.065, 0.25),
                Vec3::new(0., 0.035, -0.08),
                metal.clone(),
                true,
            ),
            (
                Vec3::new(0.06, 0.045, 0.22),
                Vec3::new(0., -0.012, -0.06),
                metal.clone(),
                false,
            ),
            (
                Vec3::new(0.06, 0.13, 0.075),
                Vec3::new(0., -0.08, 0.012),
                grip,
                false,
            ),
            (
                Vec3::new(0.008, 0.009, 0.014),
                Vec3::new(0., 0.075, -0.175),
                sight,
                false,
            ),
            (
                Vec3::new(0.008, 0.015, 0.025),
                Vec3::new(-0.015, 0.075, 0.025),
                metal.clone(),
                false,
            ),
            (
                Vec3::new(0.008, 0.015, 0.025),
                Vec3::new(0.015, 0.075, 0.025),
                metal.clone(),
                false,
            ),
            (
                Vec3::new(0.095, 0.11, 0.09),
                Vec3::new(0., -0.065, 0.06),
                skin.clone(),
                false,
            ),
            (
                Vec3::new(0.1, 0.095, 0.10),
                Vec3::new(-0.06, -0.07, -0.015),
                skin,
                false,
            ),
            (
                Vec3::new(0.09, 0.11, 0.26),
                Vec3::new(0.035, -0.12, 0.2),
                sleeve.clone(),
                false,
            ),
            (
                Vec3::new(0.09, 0.11, 0.29),
                Vec3::new(-0.12, -0.13, 0.13),
                sleeve,
                false,
            ),
        ] {
            part(&mut commands, &mut meshes, root, size, pos, mat, slide);
        }
        for i in 0..7 {
            part(
                &mut commands,
                &mut meshes,
                root,
                Vec3::new(0.068, 0.004, 0.005),
                Vec3::new(0., 0.05, 0.02 - i as f32 * 0.012),
                metal.clone(),
                true,
            );
        }
        let black = materials.add(Color::srgb(0.01, 0.01, 0.01));
        part(
            &mut commands,
            &mut meshes,
            root,
            Vec3::new(0.028, 0.028, 0.003),
            Vec3::new(0., 0.035, -0.207),
            black,
            false,
        );
        let flash_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0.75, 0.18),
            unlit: true,
            ..default()
        });
        let flash = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(0.05).mesh().ico(1).unwrap())),
                MeshMaterial3d(flash_mat),
                Transform::from_xyz(0., 0.035, -0.27).with_scale(Vec3::new(0.7, 0.7, 1.6)),
                Visibility::Hidden,
                Flash,
            ))
            .id();
        commands.entity(root).add_child(flash);
    }
    let recent = me
        .weapon
        .last_fired_ms
        .is_some_and(|t| world.server_time_ms.saturating_sub(t) < 85);
    if ui.last_shot != me.weapon.last_fired_ms {
        ui.last_shot = me.weapon.last_fired_ms;
        if recent {
            ui.recoil = 1.;
            if !patrol.muted && smoke.is_none() {
                sound("pistol");
            }
        }
    }
    // Adapted from Rusty Shooter's weapon offset follow/recovery (MIT), made frame-rate independent.
    ui.recoil *= 0.8_f32.powf(time.delta_secs() * 60.);
    let aimed = buttons.pressed(MouseButton::Right)
        && me.weapon.drawn
        && me.weapon.reload_until.is_none()
        && !patrol.overlay
        && patrol.looking;
    let target = if aimed { 1. } else { 0. };
    ui.aim_blend += (target - ui.aim_blend) * (1. - (-15. * time.delta_secs()).exp());
    if let Ok((_, mut projection)) = camera.single_mut()
        && let Projection::Perspective(ref mut p) = *projection
    {
        p.fov = (70. - 20. * ui.aim_blend).to_radians();
    }

    for (mut t, mut visible) in &mut guns {
        *visible = if me.weapon.drawn
            && me.health > 0
            && !smoke
                .as_ref()
                .is_some_and(|s| s.physics && s.elapsed > 48.5)
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let reload = me.weapon.reload_until.is_some();
        t.translation = Vec3::new(0.17, -0.15, -0.60)
            .lerp(Vec3::new(0., -0.075, -0.48), ui.aim_blend)
            + Vec3::new(0., if reload { -0.18 } else { 0. }, ui.recoil * 0.065);
        t.rotation = Quat::from_rotation_x(ui.recoil * 0.18 + if reload { -0.35 } else { 0. });
    }
    for (base, mut slide) in &mut slides {
        slide.translation.z = base.0 + ui.recoil * 0.06;
    }
    for mut flash in &mut flashes {
        *flash = if recent {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if ui.tracer_material.is_none() {
        ui.tracer_material = Some(materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0.83, 0.34),
            unlit: true,
            ..default()
        }));
        ui.tracer_mesh = Some(meshes.add(Cuboid::new(0.014, 0.014, 0.65)));
    }
    let tracer_material = ui.tracer_material.as_ref().unwrap().clone();
    for shot in &world.projectiles {
        if !tracers.iter().any(|(_, t, _)| t.0 == shot.id) {
            commands.spawn((
                Mesh3d(ui.tracer_mesh.as_ref().unwrap().clone()),
                MeshMaterial3d(tracer_material.clone()),
                Transform::from_translation(Vec3::from_array(shot.position)),
                Tracer(shot.id),
            ));
        }
    }
    for (e, id, mut t) in &mut tracers {
        if let Some(shot) = world.projectiles.iter().find(|s| s.id == id.0) {
            t.translation = Vec3::from_array(shot.position);
            t.rotation =
                Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::from_array(shot.velocity).normalize());
        } else {
            commands.entity(e).despawn();
        }
    }
}
/// Closest stationary vehicle side, independently of whether the driver is alive.
pub fn nearby(shift: &shift::Shift, position: Vec3) -> Option<(u64, Side)> {
    shift
        .world
        .as_ref()?
        .vehicles
        .iter()
        .filter(|v| {
            matches!(
                v.stage,
                VehicleStage::Stopped | VehicleStage::Resolved | VehicleStage::Disabled
            )
        })
        .flat_map(|v| {
            [Side::Driver, Side::Passenger]
                .into_iter()
                .map(move |side| (v, side))
        })
        .filter_map(|(v, side)| {
            let p = border_net::vehicle::spec(&v.kind).world_point(
                v.position,
                v.yaw,
                if side == Side::Driver {
                    border_net::vehicle::VehiclePoint::Driver
                } else {
                    border_net::vehicle::VehiclePoint::Passenger
                },
            );
            let d = (position.x - p[0]).hypot(position.z - p[2]);
            (d <= 2.8).then_some((v.vehicle_id, side, d))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(id, side, _)| (id, side))
}
#[allow(clippy::too_many_arguments)]
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut ui: ResMut<WeaponUi>,
    shift: Res<shift::Shift>,
    g: Res<Patrol>,
    camera: Query<&Transform, With<Player>>,
    mut contexts: EguiContexts,
    smoke: Option<Res<SmokeTest>>,
) {
    if !ui.was_looking && g.looking {
        ui.next_fire = time.elapsed_secs() + 0.15;
    }
    ui.was_looking = g.looking;
    if smoke.is_some()
        || g.overlay
        || g.settings_open
        || g.map_open
        || shift.me().is_none_or(|p| p.health == 0)
    {
        return;
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyB) {
        shift.send(ClientCommand::SetWeaponDrawn {
            drawn: !shift.me().unwrap().weapon.drawn,
        });
    }
    if keys.just_pressed(KeyCode::KeyR) {
        shift.send(ClientCommand::Reload);
    }
    if g.looking
        && mouse.just_pressed(MouseButton::Left)
        && time.elapsed_secs() >= ui.next_fire
        && shift.me().is_some_and(|p| p.weapon.drawn)
        && !contexts.ctx_mut().is_ok_and(|c| c.is_pointer_over_area())
    {
        shift.send(ClientCommand::Fire {
            direction: camera.forward().to_array(),
        });
        ui.next_fire = time.elapsed_secs() + 0.25;
    }
    if let Some((id, side)) = nearby(&shift, camera.translation) {
        if keys.just_pressed(KeyCode::KeyQ) {
            shift.send(ClientCommand::ToggleDoor {
                vehicle_id: id,
                side,
            });
        }
        if keys.just_pressed(KeyCode::KeyV) {
            shift.send(ClientCommand::ToggleWindow {
                vehicle_id: id,
                pane: match (side, keys.pressed(KeyCode::ShiftLeft)) {
                    (Side::Driver, false) => Pane::DriverFront,
                    (Side::Driver, true) => Pane::DriverRear,
                    (Side::Passenger, false) => Pane::PassengerFront,
                    (Side::Passenger, true) => Pane::PassengerRear,
                },
            });
        }
        if keys.just_pressed(KeyCode::KeyX) {
            if shift.world.as_ref().is_some_and(|w| {
                w.vehicles
                    .iter()
                    .any(|v| v.vehicle_id == id && v.physics.npc.mode == NpcMode::Outside)
            }) {
                shift.send(ClientCommand::OrderEnter { vehicle_id: id });
            } else {
                shift.send(ClientCommand::OrderExit { vehicle_id: id });
            }
        }
        if keys.just_pressed(KeyCode::KeyY) {
            shift.send(ClientCommand::PullBody { vehicle_id: id });
        }
        if keys.just_pressed(KeyCode::KeyT) {
            shift.send(ClientCommand::TowVehicle { vehicle_id: id });
        }
    }
}
pub fn hud(ctx: &bevy_egui::egui::Context, shift: &shift::Shift, position: Vec3) {
    use bevy_egui::egui;
    let Some(p) = shift.me() else {
        return;
    };
    egui::Area::new("weapon_status".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-25., -55.])
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(if p.health == 0 {
                    "INCAPACITATED · wait for next shift".into()
                } else {
                    format!(
                        "PISTOL  {} / {}{}",
                        p.weapon.magazine,
                        p.weapon.reserve,
                        if p.weapon.reload_until.is_some() {
                            "  RELOADING"
                        } else if p.weapon.drawn {
                            "  DRAWN"
                        } else {
                            "  HOLSTERED"
                        }
                    )
                })
                .monospace()
                .size(16.),
            );
            ui.label("B draw / holster   LMB fire   RMB aim   R reload");
            if let Some((id, side)) = nearby(shift, position) {
                ui.label(format!(
                    "#{id} {side:?} · Q door · V front / Shift+V rear window · X exit / return · Y pull body · T recover"
                ));
            }
        });
}

/// Short procedural effects use the same installed player as voice playback, outside the render loop.
pub fn sound(name: &'static str) {
    std::thread::spawn(move || {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/audio")
            .join(format!("{name}.wav"));
        if let Ok(mut child) = std::process::Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-volume",
                "45",
            ])
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            let _ = child.wait();
        }
    });
}
