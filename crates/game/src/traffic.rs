//! Timed traffic for the graphical prototype; provider sessions remain authoritative for cases.
use super::*;
pub const GAP: f32 = 9.;
pub const INCOMING: [Vec2; 4] = [
    Vec2::new(0., 0.),
    Vec2::new(0., 130.),
    Vec2::new(136.5, 235.),
    Vec2::new(426.5, 350.),
];
const OUTGOING: [Vec2; 4] = [
    Vec2::new(0., -16.),
    Vec2::new(0., -95.),
    Vec2::new(-113.5, -210.),
    Vec2::new(-433.5, -340.),
];
#[derive(Component)]
pub struct Vehicle {
    pub lane: u8,
    pub ordinal: usize,
    pub distance: f32,
    pub speed: f32,
    pub release_at: f32,
    wait: f32,
    exit: Option<bool>,
    waypoint: usize,
}
impl Vehicle {
    pub fn new(lane: u8, ordinal: usize) -> Self {
        Self {
            lane,
            ordinal,
            distance: if lane == 0 {
                18. + ordinal as f32 * 30.
            } else {
                75. + ordinal as f32 * 35.
            },
            speed: 0.,
            release_at: ordinal as f32 * 2.7,
            wait: 0.,
            exit: None,
            waypoint: 0,
        }
    }
}
#[derive(Component)]
pub struct LaneBarrier(pub u8);
#[derive(Component)]
pub struct VehicleDust {
    pub lane: u8,
    pub ordinal: usize,
    pub phase: f32,
}
#[derive(Resource, Default)]
pub struct Traffic {
    pub restart: bool,
    elapsed: f32,
    ambient_head: usize,
}
pub fn incoming(distance: f32, lane: u8) -> (Vec3, Quat) {
    let mut remaining = distance.max(0.);
    for pair in INCOMING.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.length();
        if remaining <= length {
            let p = pair[0] + delta.normalize() * remaining;
            return (
                Vec3::new(p.x + lane as f32 * 7., 0., p.y),
                Quat::from_rotation_y(delta.x.atan2(delta.y)),
            );
        }
        remaining -= length;
    }
    let end = INCOMING[3];
    (
        Vec3::new(end.x + lane as f32 * 7., 0., end.y),
        Quat::IDENTITY,
    )
}
pub fn near_road(point: Vec2, margin: f32) -> bool {
    let center = point - Vec2::new(3.5, 0.);
    INCOMING.windows(2).chain(OUTGOING.windows(2)).any(|p| {
        let d = p[1] - p[0];
        let t = ((center - p[0]).dot(d) / d.length_squared()).clamp(0., 1.);
        center.distance(p[0] + d * t) < margin
    }) || (point.x < 2. && point.x > -450. && (point.y + 16.).abs() < margin)
}
fn person_blocks(position: Vec3, target: Vec3, person: Vec3) -> bool {
    let direction = (target - position).normalize_or_zero();
    let offset = Vec3::new(person.x, 0., person.z) - position;
    let ahead = offset.dot(direction);
    ahead > 0. && ahead < 3.5 && (offset - direction * ahead).length() < 1.35
}
/// Braking-distance speed limit plus a hard spacing clamp prevents queue overlap at low frame rates.
fn approach(distance: &mut f32, speed: &mut f32, target: f32, dt: f32) {
    let remaining = (*distance - target).max(0.);
    let desired = (remaining * 3.6).sqrt().min(10.);
    let change = (desired - *speed).clamp(-4. * dt, 2. * dt);
    *speed = (*speed + change).max(0.);
    *distance = (*distance - *speed * dt).max(target);
    if *distance - target < 0.025 {
        *distance = target;
        *speed = 0.;
    }
}
#[allow(clippy::type_complexity)] // Query filters describe disjoint moving cars, barrier roots and player.
pub fn progress(
    time: Res<Time>,
    shift: Option<Res<shift::Shift>>,
    mut traffic: ResMut<Traffic>,
    mut g: ResMut<Patrol>,
    mut cars: Query<(&mut Transform, &mut Vehicle), Without<Barrier>>,
    mut barriers: Query<(&mut Transform, &LaneBarrier), (With<Barrier>, Without<Vehicle>)>,
    player: Query<&Transform, (With<Player>, Without<Vehicle>, Without<Barrier>)>,
) {
    if shift.is_some_and(|s| s.connected) {
        return;
    }
    if traffic.restart {
        *traffic = Traffic::default();
        for (mut t, mut v) in &mut cars {
            let lane = v.lane;
            let ordinal = v.ordinal % if lane == 0 { 10 } else { 4 };
            *v = Vehicle::new(lane, ordinal);
            let (position, rotation) = incoming(v.distance, lane);
            t.translation = position;
            t.rotation = rotation;
        }
    }
    let dt = time.delta_secs().min(0.05);
    traffic.elapsed += dt;
    let old: Vec<_> = cars
        .iter()
        .map(|(_, v)| (v.lane, v.ordinal, v.distance, v.exit.is_some()))
        .collect();
    let player = player.single().ok().map(|p| p.translation);
    let mut advance = false;
    for (mut transform, mut v) in &mut cars {
        let head = if v.lane == 0 {
            g.position
        } else {
            traffic.ambient_head
        };
        if v.exit.is_none() && v.ordinal == head && v.distance == 0. {
            v.wait += dt;
            if v.lane == 0 && g.phase == Phase::Departing {
                v.exit = Some(
                    g.reveal
                        .as_ref()
                        .is_some_and(|r| r.decision == border_scenarios::Decision::Admit),
                );
            } else if v.lane == 1 && v.wait > 5. {
                v.exit = Some(true);
            }
        }
        if let Some(admitted) = v.exit {
            let point = if admitted {
                OUTGOING[v.waypoint.min(3)] + Vec2::new(v.lane as f32 * 7., 0.)
            } else {
                [
                    Vec2::new(0., -11.),
                    Vec2::new(-4., -16.),
                    Vec2::new(-440., -16.),
                ][v.waypoint.min(2)]
            };
            let target = Vec3::new(point.x, 0., point.y);
            let offset = target - transform.translation;
            let remaining = offset.length();
            let blocked = player.is_some_and(|p| person_blocks(transform.translation, target, p));
            v.speed = if blocked {
                0.
            } else {
                (v.speed + dt * 2.).min(12.)
            };
            if remaining > 0.01 {
                let direction = offset / remaining;
                transform.translation += direction * (v.speed * dt).min(remaining);
                let yaw = (-direction.x).atan2(-direction.z);
                transform.rotation = transform
                    .rotation
                    .slerp(Quat::from_rotation_y(yaw), (dt * 3.).min(1.));
            }
            if remaining < 0.4 {
                let last = if admitted { 3 } else { 2 };
                if v.waypoint < last {
                    v.waypoint += 1;
                } else if v.lane == 1 {
                    v.ordinal += 4;
                    v.distance = 590.;
                    v.exit = None;
                    v.waypoint = 0;
                    v.wait = 0.;
                    v.release_at = traffic.elapsed + 6.;
                    let (p, r) = incoming(v.distance, v.lane);
                    transform.translation = p;
                    transform.rotation = r;
                }
            }
            if v.ordinal == head
                && ((admitted && transform.translation.z < -19.)
                    || (!admitted && transform.translation.x < -13.))
            {
                if v.lane == 0 {
                    advance = true;
                } else {
                    traffic.ambient_head += 1;
                }
            }
        } else {
            if v.ordinal < head {
                continue;
            }
            let mut target = (v.ordinal - head) as f32 * GAP;
            if let Some((_, _, distance, _)) = old.iter().find(|(lane, index, _, exiting)| {
                *lane == v.lane && *index + 1 == v.ordinal && !*exiting
            }) {
                target = target.max(*distance + GAP);
            }
            if traffic.elapsed >= v.release_at {
                let mut distance = v.distance;
                let mut speed = v.speed;
                approach(&mut distance, &mut speed, target, dt);
                let (p, rotation) = incoming(distance, v.lane);
                let blocked =
                    player.is_some_and(|person| person_blocks(transform.translation, p, person));
                if !blocked {
                    v.distance = distance;
                    v.speed = speed;
                    transform.translation = p;
                    transform.rotation = rotation;
                } else {
                    v.speed = 0.;
                }
            }
        }
    }
    for (mut t, lane) in &mut barriers {
        let open = cars.iter().any(|(t, v)| {
            v.lane == lane.0 && v.exit.is_some() && t.translation.z > -13. && t.translation.x > -4.
        });
        let target = Quat::from_rotation_z(if open { -1.35 } else { 0. });
        t.rotation = t.rotation.slerp(target, (dt * 4.).min(1.));
    }
    if advance {
        g.position += 1;
        if g.position >= g.ids.len() {
            g.phase = Phase::Complete;
            g.overlay = true;
            g.looking = false;
        } else {
            g.reset_case();
            g.looking = true;
        }
    }
}
pub fn dust(
    time: Res<Time>,
    cars: Query<(&Transform, &Vehicle), Without<VehicleDust>>,
    mut dust: Query<(&mut Transform, &VehicleDust), Without<Vehicle>>,
) {
    for (mut t, d) in &mut dust {
        if let Some((car, v)) = cars.iter().find(|(_, v)| {
            v.lane == d.lane && v.ordinal % if v.lane == 0 { 10 } else { 4 } == d.ordinal
        }) {
            let strength = (v.speed / 10.).clamp(0., 1.);
            let back = car.rotation * Vec3::Z;
            let age = (time.elapsed_secs() * 0.8 + d.phase).fract();
            t.translation = car.translation
                + back * (3. + age * 4.)
                + Vec3::new(age * 0.6, 0.25 + age * 0.8, 0.);
            t.scale = Vec3::new(0.7 + age, 0.25 + age * 0.4, 1.6 + age) * strength.max(0.001);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_cars_brake_without_overrunning_the_stop() {
        for step in [0.016, 0.05, 0.2] {
            let mut distance = 80.;
            let mut speed = 0.;
            for _ in 0..3000 {
                approach(&mut distance, &mut speed, 9., step);
                assert!(distance >= 9.);
                assert!(speed >= 0.);
            }
            assert_eq!(distance, 9.);
            assert_eq!(speed, 0.);
        }
    }
    #[test]
    fn incoming_route_crosses_bridge_and_continues_around_bend() {
        let (bridge, _) = incoming(50., 0);
        assert_eq!(bridge, Vec3::new(0., 0., 50.));
        let (distant, _) = incoming(350., 0);
        assert!(distant.x > 100. && distant.z > 200.);
    }
    #[test]
    fn traffic_processes_all_cases_and_denials_use_the_return_road() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Traffic>()
            .insert_resource(Time::<()>::default())
            .insert_resource(Patrol::new((0..10).map(|i| i.to_string()).collect(), 42))
            .add_systems(Startup, scene::setup)
            .add_systems(Update, progress);
        app.update();
        let pack = border_scenarios::load_pack(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios"),
        )
        .unwrap();
        let mut handled = 0;
        let mut saw_return = false;
        for _ in 0..16000 {
            let index = app.world().resource::<Patrol>().position;
            let phase = app.world().resource::<Patrol>().phase;
            let stopped = {
                let world = app.world_mut();
                let mut q = world.query::<&Vehicle>();
                q.iter(world).any(|v| {
                    v.lane == 0 && v.ordinal == index && v.distance == 0. && v.exit.is_none()
                })
            };
            if phase == Phase::Approach && stopped {
                let decision = if index.is_multiple_of(2) {
                    border_scenarios::Decision::Admit
                } else {
                    border_scenarios::Decision::Deny
                };
                let mut patrol = app.world_mut().resource_mut::<Patrol>();
                patrol.reveal = Some(pack[0].resolve(decision, false));
                patrol.phase = Phase::Departing;
                handled += 1;
            }
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            app.update();
            let world = app.world_mut();
            let mut q = world.query::<(&Transform, &Vehicle)>();
            let mut queue: Vec<_> = q
                .iter(world)
                .filter(|(_, v)| v.lane == 0 && v.exit.is_none())
                .map(|(_, v)| (v.ordinal, v.distance))
                .collect();
            queue.sort_by_key(|v| v.0);
            for pair in queue.windows(2) {
                assert!(pair[1].1 - pair[0].1 >= GAP - 0.01, "queue overlap");
            }
            saw_return |= q
                .iter(world)
                .any(|(t, v)| v.lane == 0 && v.exit == Some(false) && t.translation.x < -15.);
            if world.resource::<Patrol>().phase == Phase::Complete {
                break;
            }
        }
        assert_eq!(
            handled, 10,
            "every car must arrive before its case is processed"
        );
        assert_eq!(app.world().resource::<Patrol>().phase, Phase::Complete);
        assert!(saw_return, "denied travelers must leave via the side road");
        {
            let mut patrol = app.world_mut().resource_mut::<Patrol>();
            patrol.position = 0;
            patrol.reset_case();
        }
        app.world_mut().resource_mut::<Traffic>().restart = true;
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&Vehicle>();
        assert!(q.iter(world).all(|v| v.exit.is_none()));
        assert!(
            q.iter(world)
                .any(|v| v.lane == 0 && v.ordinal == 0 && v.distance > 17.)
        );
    }
    #[test]
    fn cars_yield_to_people_in_the_lane_but_can_pass_an_officer_at_the_window() {
        assert!(person_blocks(
            Vec3::new(0., 0., 5.),
            Vec3::ZERO,
            Vec3::new(0., 1.7, 3.)
        ));
        assert!(!person_blocks(
            Vec3::new(0., 0., 5.),
            Vec3::ZERO,
            Vec3::new(-1.8, 1.7, 3.)
        ));
    }
}
