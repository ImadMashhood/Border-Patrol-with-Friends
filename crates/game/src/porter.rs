use super::*;
use border_net::{PorterPhase, combat as c, porter_chair};
#[derive(Component)]
pub struct Limb(usize);
fn cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    parent: Entity,
    size: Vec3,
    pos: Vec3,
    mat: Handle<StandardMaterial>,
) {
    let e = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
        ))
        .id();
    commands.entity(parent).add_child(e);
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn render(
    shift: Res<shift::Shift>,
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut people: Query<(Entity, &interactions::ServiceNpc, &mut Transform), Without<Limb>>,
    mut limbs: Query<(&Limb, &ChildOf, &mut Transform), Without<interactions::ServiceNpc>>,
) {
    let Some(w) = &shift.world else {
        return;
    };
    for p in &w.porters {
        if !people.iter().any(|(_, n, _)| n.0 == p.lane_id) {
            let root = commands
                .spawn((
                    Transform::from_translation(Vec3::from_array(p.position)),
                    Visibility::Inherited,
                    interactions::ServiceNpc(p.lane_id),
                ))
                .id();
            let shirt = materials.add(Color::srgb(0.20, 0.38, 0.62));
            let pants = materials.add(Color::srgb(0.18, 0.20, 0.23));
            let skin = materials.add(Color::srgb(0.60, 0.43, 0.30));
            for bone in 0..11 {
                let material = if bone == 2 || matches!(bone, 4 | 6) {
                    skin.clone()
                } else if bone == 0 || bone >= 7 {
                    pants.clone()
                } else {
                    shirt.clone()
                };
                let e = commands
                    .spawn((
                        Mesh3d(vehicle_physics::bone_mesh(&mut meshes, bone)),
                        MeshMaterial3d(material),
                        Transform::default(),
                        Limb(bone),
                    ))
                    .id();
                commands.entity(root).add_child(e);
            }
            let chair = commands
                .spawn((
                    Transform::from_translation(Vec3::from_array(porter_chair(p.lane_id))),
                    Visibility::Inherited,
                ))
                .id();
            let metal = materials.add(Color::srgb(0.22, 0.25, 0.23));
            cube(
                &mut commands,
                &mut meshes,
                chair,
                Vec3::new(0.65, 0.10, 0.60),
                Vec3::new(0., 0.44, 0.),
                metal.clone(),
            );
            cube(
                &mut commands,
                &mut meshes,
                chair,
                Vec3::new(0.65, 0.70, 0.08),
                Vec3::new(0., 0.80, 0.28),
                metal.clone(),
            );
            for x in [-0.26, 0.26] {
                for z in [-0.23, 0.23] {
                    cube(
                        &mut commands,
                        &mut meshes,
                        chair,
                        Vec3::new(0.055, 0.44, 0.055),
                        Vec3::new(x, 0.22, z),
                        metal.clone(),
                    );
                }
            }
        }
    }
    for (_, npc, mut t) in &mut people {
        if let Some(p) = w.porters.iter().find(|p| p.lane_id == npc.0) {
            let mut position = Vec3::from_array(p.position);
            let mut yaw = p.yaw;
            if let Some(v) = p
                .vehicle_id
                .and_then(|id| w.vehicles.iter().find(|v| v.vehicle_id == id))
            {
                if p.phase == PorterPhase::Driving {
                    position = Vec3::from_array(c::seated_position(v));
                    yaw = v.yaw;
                } else if p.phase == PorterPhase::Entering {
                    position = position.lerp(
                        Vec3::from_array(c::seated_position(v)),
                        (p.progress / 1.4).min(1.),
                    );
                    yaw = v.yaw;
                }
            }
            if p.phase == PorterPhase::Seated {
                position.y -= 0.3;
            }
            t.translation = t
                .translation
                .lerp(position, (time.delta_secs() * 20.).min(1.));
            t.rotation = Quat::from_rotation_y(yaw);
        }
    }
    for (limb, parent, mut t) in &mut limbs {
        let Ok((_, npc, _)) = people.get(parent.parent()) else {
            continue;
        };
        let Some(p) = w.porters.iter().find(|p| p.lane_id == npc.0) else {
            continue;
        };
        let seated = matches!(
            p.phase,
            PorterPhase::Seated | PorterPhase::Driving | PorterPhase::Entering
        );
        let rig = if matches!(p.phase, PorterPhase::Approaching | PorterPhase::Returning) {
            border_net::rig::walking(time.elapsed_secs() * 11.)
        } else {
            border_net::rig::skeleton(seated)
        };
        let b = rig.bones[limb.0];
        t.translation = Vec3::from_array(b.center);
        t.rotation = Quat::from_rotation_x(b.angle);
    }
}
