//! Local equipment selection; all case decisions still go through the authoritative host.
use super::*;
use bevy_egui::{EguiContexts, egui};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Item {
    #[default]
    Hands,
    Pistol,
    Tablet,
}
impl Item {
    pub const ALL: [Self; 3] = [Self::Pistol, Self::Tablet, Self::Hands];
    pub fn name(self) -> &'static str {
        match self {
            Self::Hands => "HANDS",
            Self::Pistol => "PISTOL",
            Self::Tablet => "TABLET",
        }
    }
}
#[derive(Resource, Default)]
pub struct Equipment {
    pub selected: Item,
    pub wheel: bool,
    pub hover: Item,
    pub record: Option<u64>,
    pub page: usize,
    pub awaiting: Option<(u64, f32)>,
    root: Option<Entity>,
}
impl Equipment {
    pub fn equip(&mut self, item: Item, g: &mut Patrol, shift: &shift::Shift) {
        self.selected = item;
        self.wheel = false;
        shift.send(border_net::ClientCommand::SetWeaponDrawn {
            drawn: item == Item::Pistol,
        });
        g.overlay = item == Item::Tablet;
        g.looking = !g.overlay;
        g.settings_open = false;
        g.map_open = false;
        g.request_focus = false;
        if item == Item::Tablet
            && let Some(id) = g.vehicle_id
        {
            self.record = Some(id);
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut e: ResMut<Equipment>,
    mut g: ResMut<Patrol>,
    shift: Res<shift::Shift>,
    mut window: Single<&mut Window>,
    mut contexts: EguiContexts,
    smoke: Option<Res<SmokeTest>>,
) {
    if smoke.is_some() {
        return;
    }
    if !window.focused || shift.me().is_none_or(|p| p.health == 0) {
        e.wheel = false;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        e.wheel = false;
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        e.wheel = true;
        e.hover = e.selected;
        g.overlay = false;
        g.looking = false;
        g.map_open = false;
        g.settings_open = false;
        let center = Vec2::new(window.width() * 0.5, window.height() * 0.5);
        window.set_cursor_position(Some(center));
    }
    if e.wheel {
        for (key, item) in [
            (KeyCode::Digit1, Item::Pistol),
            (KeyCode::Digit2, Item::Tablet),
            (KeyCode::Digit3, Item::Hands),
        ] {
            if keys.just_pressed(key) {
                e.hover = item;
            }
        }
        if keys.just_released(KeyCode::Tab) || mouse.just_pressed(MouseButton::Left) {
            let item = e.hover;
            e.equip(item, &mut g, &shift);
        }
        return;
    }
    let typing = contexts.ctx_mut().is_ok_and(|c| c.wants_keyboard_input());
    if !typing && keys.just_pressed(KeyCode::KeyI) {
        if g.overlay {
            g.overlay = false;
            g.looking = true;
        } else {
            e.equip(Item::Tablet, &mut g, &shift);
        }
    }
    if !g.overlay && keys.just_pressed(KeyCode::KeyB) {
        let item = if e.selected == Item::Pistol {
            Item::Hands
        } else {
            Item::Pistol
        };
        e.equip(item, &mut g, &shift);
    }
    if !g.overlay && e.selected == Item::Tablet && keys.just_pressed(KeyCode::KeyE) {
        g.overlay = true;
        g.looking = false;
    }
}
#[derive(Component)]
pub struct Tablet;
#[allow(clippy::too_many_arguments)]
pub fn render(
    mut e: ResMut<Equipment>,
    g: Res<Patrol>,
    shift: Res<shift::Shift>,
    camera: Query<Entity, With<Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tablets: Query<&mut Visibility, With<Tablet>>,
) {
    if e.root.is_none()
        && let Ok(camera) = camera.single()
    {
        let root = commands
            .spawn((
                Transform::from_xyz(0.14, -0.26, -0.65).with_rotation(Quat::from_rotation_x(-0.24)),
                Visibility::Hidden,
                Tablet,
            ))
            .id();
        commands.entity(camera).add_child(root);
        let shell = materials.add(Color::srgb(0.045, 0.055, 0.05));
        let screen = materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.20, 0.18),
            unlit: true,
            ..default()
        });
        let hand = materials.add(Color::srgb(0.55, 0.39, 0.26));
        for (size, pos, mat) in [
            (Vec3::new(0.39, 0.28, 0.028), Vec3::ZERO, shell),
            (
                Vec3::new(0.345, 0.225, 0.008),
                Vec3::new(0., 0.005, 0.02),
                screen,
            ),
            (
                Vec3::new(0.075, 0.14, 0.085),
                Vec3::new(-0.18, -0.08, 0.025),
                hand.clone(),
            ),
            (
                Vec3::new(0.075, 0.14, 0.085),
                Vec3::new(0.18, -0.08, 0.025),
                hand,
            ),
        ] {
            let child = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::from_size(size))),
                    MeshMaterial3d(mat),
                    Transform::from_translation(pos),
                ))
                .id();
            commands.entity(root).add_child(child);
        }
        e.root = Some(root);
    }
    for mut v in &mut tablets {
        *v = if e.selected == Item::Tablet
            && !g.overlay
            && !e.wheel
            && shift.me().is_some_and(|p| p.health > 0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
pub fn wheel(ctx: &egui::Context, e: &mut Equipment) {
    let center = ctx.viewport_rect().center();
    if let Some(pos) = ctx.pointer_latest_pos() {
        let delta = pos - center;
        if delta.length() > 48. && delta.length() < 245. {
            let angle =
                (delta.y.atan2(delta.x) + std::f32::consts::PI * 0.5 + std::f32::consts::PI / 3.)
                    .rem_euclid(std::f32::consts::TAU);
            e.hover = Item::ALL[(angle / (std::f32::consts::TAU / 3.)).floor() as usize % 3];
        }
    }
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("equipment-wheel"),
    ));
    p.rect_filled(
        ctx.viewport_rect(),
        0.,
        egui::Color32::from_black_alpha(100),
    );
    for (i, item) in Item::ALL.into_iter().enumerate() {
        let angle = -std::f32::consts::PI * 0.5 + i as f32 * std::f32::consts::TAU / 3.;
        let a = angle - std::f32::consts::PI / 3. + 0.025;
        let b = angle + std::f32::consts::PI / 3. - 0.025;
        // Convex wedges, with an opaque center laid over their tips.
        let mut points = vec![center];
        for j in 0..=28 {
            let t = a + (b - a) * j as f32 / 28.;
            points.push(center + egui::vec2(t.cos(), t.sin()) * 190.);
        }
        let selected = e.hover == item;
        p.add(egui::Shape::convex_polygon(
            points,
            if selected {
                egui::Color32::from_rgb(68, 96, 84)
            } else {
                egui::Color32::from_rgb(21, 32, 30)
            },
            egui::Stroke::new(
                1.,
                if selected {
                    ui::GOLD
                } else {
                    egui::Color32::from_gray(75)
                },
            ),
        ));
        let pos = center + egui::vec2(angle.cos(), angle.sin()) * 126.;
        p.text(
            pos,
            egui::Align2::CENTER_CENTER,
            format!("{}\n{}", i + 1, item.name()),
            egui::FontId::monospace(17.),
            egui::Color32::WHITE,
        );
    }
    p.circle_filled(center, 58., egui::Color32::from_rgb(12, 21, 20));
    p.text(
        center,
        egui::Align2::CENTER_CENTER,
        e.hover.name(),
        egui::FontId::monospace(13.),
        ui::GOLD,
    );
    p.text(
        center + egui::vec2(0., 225.),
        egui::Align2::CENTER_CENTER,
        "MOVE MOUSE TO SELECT · RELEASE TAB TO EQUIP\n1 / 2 / 3 select · Esc cancels",
        egui::FontId::proportional(13.),
        egui::Color32::WHITE,
    );
}
