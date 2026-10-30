//! Continuous character acting with forward kinematics: joints remain connected in every pose.
use super::*;
use std::collections::HashMap;
#[derive(Default)]
struct Actor {
    last: Option<Vec3>,
    stride: f32,
    speed: f32,
    seat: f32,
    gesture: f32,
}
#[derive(Resource, Default)]
pub struct Actors {
    states: HashMap<(u8, u64), Actor>,
}
impl Actors {
    #[allow(clippy::too_many_arguments)]
    pub fn pose(
        &mut self,
        key: (u8, u64),
        position: Vec3,
        seated: bool,
        energy: f32,
        emotion: &str,
        now: f32,
        dt: f32,
    ) -> [Transform; 11] {
        let a = self.states.entry(key).or_default();
        let distance = a.last.map_or(0., |p| {
            Vec2::new(position.x - p.x, position.z - p.z).length()
        });
        a.last = Some(position);
        let speed = if distance < 1. {
            distance / dt.max(0.001)
        } else {
            0.
        };
        let blend = 1. - (-10. * dt).exp();
        a.speed += (speed - a.speed) * blend;
        a.seat += ((seated as u8 as f32) - a.seat) * (1. - (-12. * dt).exp());
        if distance < 1. {
            a.stride += distance * 7.5;
        }
        a.gesture += (energy - a.gesture) * (1. - (-12. * dt).exp());
        pose(
            a.stride,
            a.speed.min(5.),
            a.seat,
            a.gesture,
            emotion,
            now + key.1 as f32 * 1.73,
        )
    }
}
fn pose(phase: f32, speed: f32, seat: f32, talk: f32, emotion: &str, t: f32) -> [Transform; 11] {
    let rig = border_net::rig::skeleton(false);
    let walking = (speed / 1.7).clamp(0., 1.) * (1. - seat);
    let run = ((speed - 1.8) / 2.).clamp(0., 1.);
    let breath = (t * 1.8).sin() * 0.004;
    let mut delta = [Quat::IDENTITY; 11];
    delta[1] = Quat::from_euler(
        EulerRot::XYZ,
        -run * 0.13 + (t * 2.1).sin() * 0.009,
        (phase * 0.5).sin() * 0.045 * walking,
        (phase).sin() * 0.035 * walking,
    );
    delta[2] = Quat::from_euler(
        EulerRot::XYZ,
        (t * 3.1).sin() * 0.018 + (t * 8.).sin() * talk * 0.045,
        (t * 0.7).sin() * 0.07,
        (t * 0.9).sin() * 0.015,
    );
    for (side, upper, fore, thigh, shin) in [(-1., 3, 4, 7, 8), (1., 5, 6, 9, 10)] {
        let cycle = phase + if side < 0. { 0. } else { std::f32::consts::PI };
        let swing = cycle.sin() * walking * (0.38 + run * 0.28);
        delta[thigh] = Quat::from_rotation_x(seat * 1.45 + swing);
        delta[shin] =
            Quat::from_rotation_x(-seat * 0.55 - (cycle.cos().max(0.) * 0.85 + 0.08) * walking);
        let emphasis = (t * 4.5 + side).sin() * talk;
        let alarm = matches!(emotion, "afraid" | "panicked" | "desperate");
        let arm_x = seat * 0.65 - swing * 0.7 + if alarm { talk * 0.45 } else { emphasis * 0.22 };
        delta[upper] = Quat::from_euler(
            EulerRot::XYZ,
            arm_x,
            0.,
            side * if alarm { talk * 0.4 } else { talk * 0.12 },
        );
        delta[fore] = Quat::from_rotation_x(
            seat * 0.7 + walking * (0.25 + run * 0.6) + talk * (0.25 + emphasis * 0.25),
        );
    }
    let mut out = [Transform::IDENTITY; 11];
    out[0] = Transform::from_translation(
        Vec3::from_array(rig.bones[0].center)
            + Vec3::Y * (breath + (phase * 2.).cos().abs() * walking * 0.022),
    )
    .with_rotation(Quat::from_rotation_y(phase.sin() * walking * 0.025));
    for link in &rig.links {
        let parent = link.parent;
        let child = link.child;
        let anchor = Vec3::from_array(link.anchor);
        let parent_local = anchor - Vec3::from_array(rig.bones[parent].center);
        let child_local = anchor - Vec3::from_array(rig.bones[child].center);
        let rotation = out[parent].rotation * delta[child];
        out[child] = Transform::from_translation(
            out[parent].translation + out[parent].rotation * parent_local - rotation * child_local,
        )
        .with_rotation(rotation);
    }
    out
}
#[derive(Component)]
pub struct FacePart {
    pub mouth: bool,
    pub phase: f32,
}
pub fn faces(
    time: Res<Time>,
    net: Res<Network>,
    g: Res<Patrol>,
    heads: Query<(&vehicle_physics::BodyPart, &ChildOf)>,
    details: Query<&vehicle_physics::Detail>,
    mut faces: Query<(&FacePart, &ChildOf, &mut Transform), Without<vehicle_physics::BodyPart>>,
) {
    let energy = net.spatial.lock().unwrap().energy;
    for (part, parent, mut t) in &mut faces {
        if details
            .get(parent.parent())
            .is_ok_and(|d| matches!(d.kind, vehicle_physics::DetailKind::Bone(_)))
        {
            t.scale.y = if part.mouth { 1. } else { 0.12 };
            continue;
        }
        let own = heads
            .get(parent.parent())
            .ok()
            .and_then(|(_, p)| details.get(p.parent()).ok())
            .is_some_and(|d| Some(d.vehicle) == g.vehicle_id);
        if part.mouth {
            t.scale.y = 1. + if own { energy * 5. } else { 0. };
        } else {
            let blink =
                (time.elapsed_secs() + part.phase + (parent.parent().index() % 97) as f32 * 0.37)
                    .rem_euclid(4.1);
            t.scale.y = if blink < 0.12 { 0.12 } else { 1. };
        }
    }
}
pub fn officers(
    time: Res<Time>,
    mut actors: ResMut<Actors>,
    roots: Query<(&shift::RemoteOfficer, &Transform)>,
    mut bones: Query<
        (&vehicle_physics::BodyPart, &ChildOf, &mut Transform),
        Without<shift::RemoteOfficer>,
    >,
) {
    let poses: HashMap<_, _> = roots
        .iter()
        .map(|(officer, t)| {
            (
                officer.0,
                actors.pose(
                    (2, officer.0),
                    t.translation,
                    false,
                    0.,
                    "cooperative",
                    time.elapsed_secs(),
                    time.delta_secs(),
                ),
            )
        })
        .collect();
    for (bone, parent, mut t) in &mut bones {
        if let Ok((officer, _)) = roots.get(parent.parent())
            && let Some(pose) = poses.get(&officer.0)
        {
            *t = pose[bone.bone];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animated_walk_and_conversation_keep_every_joint_connected() {
        let rig = border_net::rig::skeleton(false);
        for frame in 0..200 {
            for seat in [0., 0.5, 1.] {
                let bones = pose(
                    frame as f32 * 0.12,
                    3.5,
                    seat,
                    0.8,
                    "panicked",
                    frame as f32 / 30.,
                );
                for l in &rig.links {
                    let anchor = Vec3::from_array(l.anchor);
                    let a = bones[l.parent]
                        .transform_point(anchor - Vec3::from_array(rig.bones[l.parent].center));
                    let b = bones[l.child]
                        .transform_point(anchor - Vec3::from_array(rig.bones[l.child].center));
                    assert!(a.distance(b) < 0.0001);
                }
            }
        }
    }
}
