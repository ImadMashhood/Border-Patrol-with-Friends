//! Host-owned pedestrian paths with swept collision and periodic dynamic-obstacle replanning.
use super::*;
use border_net::combat as c;
use std::{cmp::Reverse, collections::BinaryHeap};
const CELL: f32 = 0.5;
const RADIUS: f32 = 0.27;
const MIN_X: f32 = -64.;
const MIN_Z: f32 = -38.;
const WIDTH: usize = 193;
const HEIGHT: usize = 233;
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Walker {
    Traveler(u64),
    Porter(u8),
}
#[derive(Default)]
pub struct Route {
    goal: [f32; 3],
    points: VecDeque<[f32; 3]>,
    replan_at: u64,
}
struct Obstacles {
    boxes: Vec<([f32; 3], [f32; 3], f32)>,
    people: Vec<[f32; 3]>,
}
impl Obstacles {
    fn new(w: &WorldSnapshot, solids: &[map::Aabb], walker: Walker) -> Self {
        let mut boxes: Vec<_> = solids
            .iter()
            .filter(|b| b.center[1] + b.half[1] > 0.12 && b.center[1] - b.half[1] < 1.8)
            .map(|b| (b.center, b.half, 0.))
            .collect();
        for v in w
            .vehicles
            .iter()
            .filter(|v| v.stage != VehicleStage::Exited)
        {
            let s = vehicle::spec(&v.kind);
            boxes.push((v.position, [s.half_width, 0., s.half_length], v.yaw));
            for side in [c::Side::Driver, c::Side::Passenger] {
                let angle = v.physics.door_angles[c::side_index(side)];
                if angle > 0.05 {
                    let local = c::door_point(
                        &v.kind,
                        side,
                        angle,
                        c::add(
                            c::door_pivot(&v.kind, side),
                            [0., 0., vehicle::door_layout(&v.kind).length * 0.5],
                        ),
                    );
                    boxes.push((
                        c::world(v, local),
                        [0.06, 0., vehicle::door_layout(&v.kind).length * 0.5],
                        v.yaw
                            + if side == c::Side::Driver {
                                -angle
                            } else {
                                angle
                            },
                    ));
                }
            }
        }
        for lane in &w.lanes {
            for (closed, z) in [
                (!lane.entry_barrier_open, vehicle::ENTRY_GATE_Z),
                (!lane.barrier_open, vehicle::EXIT_GATE_Z),
            ] {
                if closed {
                    boxes.push(([(lane.lane_id - 1) as f32 * 7., 0., z], [2.65, 0., 0.1], 0.));
                }
            }
        }
        if !w.door_open {
            boxes.push((map::DOOR, [1.1, 1.5, 0.16], 0.));
        }
        let mut people: Vec<_> = w
            .peers
            .iter()
            .filter(|p| p.connected && p.health > 0)
            .map(|p| p.position)
            .collect();
        for v in &w.vehicles {
            if walker != Walker::Traveler(v.vehicle_id)
                && v.physics.npc.health > 0
                && matches!(
                    v.physics.npc.mode,
                    c::NpcMode::Outside | c::NpcMode::Fleeing
                )
            {
                people.push(v.physics.npc.position);
            }
        }
        for p in &w.porters {
            if walker != Walker::Porter(p.lane_id) && p.phase != PorterPhase::Driving {
                people.push(p.position);
            }
        }
        Self { boxes, people }
    }
    fn clear(&self, p: [f32; 3]) -> bool {
        let grounds = map::walkable(p[0], p[2])
            || ((MIN_X..=-31.5).contains(&p[0]) && (-35. ..=-12.).contains(&p[2]));
        grounds
            && !self.boxes.iter().any(|(center, half, yaw)| {
                let d = c::rotate(c::sub(p, *center), -*yaw);
                let dx = (d[0].abs() - half[0]).max(0.);
                let dz = (d[2].abs() - half[2]).max(0.);
                dx * dx + dz * dz < RADIUS * RADIUS
            })
            && !self
                .people
                .iter()
                .any(|q| (p[0] - q[0]).hypot(p[2] - q[2]) < 0.52)
    }
    fn segment(&self, a: [f32; 3], b: [f32; 3]) -> bool {
        let steps = ((a[0] - b[0]).hypot(a[2] - b[2]) / 0.08).ceil().max(1.) as usize;
        (1..=steps).all(|i| {
            let t = i as f32 / steps as f32;
            self.clear([a[0] + (b[0] - a[0]) * t, 0., a[2] + (b[2] - a[2]) * t])
        })
    }
}
fn cell(p: [f32; 3]) -> usize {
    let x = ((p[0] - MIN_X) / CELL)
        .round()
        .clamp(0., (WIDTH - 1) as f32) as usize;
    let z = ((p[2] - MIN_Z) / CELL)
        .round()
        .clamp(0., (HEIGHT - 1) as f32) as usize;
    z * WIDTH + x
}
fn point(i: usize) -> [f32; 3] {
    [
        MIN_X + (i % WIDTH) as f32 * CELL,
        0.,
        MIN_Z + (i / WIDTH) as f32 * CELL,
    ]
}
fn plan(obs: &Obstacles, start: [f32; 3], goal: [f32; 3]) -> VecDeque<[f32; 3]> {
    if !obs.clear(goal) {
        return VecDeque::new();
    }
    if obs.segment(start, goal) {
        return [goal].into();
    }
    let begin = cell(start);
    let end = cell(goal);
    let heuristic = |n: usize| {
        let a = point(n);
        ((a[0] - goal[0]).hypot(a[2] - goal[2]) * 1000.) as u32
    };
    let mut costs = vec![u32::MAX; WIDTH * HEIGHT];
    let mut previous = vec![usize::MAX; WIDTH * HEIGHT];
    let mut open = BinaryHeap::new();
    costs[begin] = 0;
    open.push(Reverse((heuristic(begin), 0, begin)));
    let mut found = None;
    let mut visited = 0;
    while let Some(Reverse((_, cost, n))) = open.pop() {
        if cost != costs[n] {
            continue;
        }
        visited += 1;
        if visited > 22000 {
            break;
        }
        let here = if n == begin { start } else { point(n) };
        if (n == end || (here[0] - goal[0]).hypot(here[2] - goal[2]) < 0.8)
            && obs.segment(here, goal)
        {
            found = Some(n);
            break;
        }
        let x = (n % WIDTH) as i32;
        let z = (n / WIDTH) as i32;
        for (dx, dz) in [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ] {
            let nx = x + dx;
            let nz = z + dz;
            if nx < 0 || nz < 0 || nx >= WIDTH as i32 || nz >= HEIGHT as i32 {
                continue;
            }
            let next = nz as usize * WIDTH + nx as usize;
            let next_cost = cost + if dx != 0 && dz != 0 { 707 } else { 500 };
            if next_cost < costs[next] && obs.segment(here, point(next)) {
                costs[next] = next_cost;
                previous[next] = n;
                open.push(Reverse((next_cost + heuristic(next), next_cost, next)));
            }
        }
    }
    let Some(mut n) = found else {
        return VecDeque::new();
    };
    let mut path = VecDeque::from([goal]);
    while n != begin {
        path.push_front(point(n));
        n = previous[n];
    }
    // Safe line-of-sight smoothing removes grid zigzags without cutting corners.
    let mut result = VecDeque::new();
    let mut from = start;
    while !path.is_empty() {
        let mut far = 0;
        for (i, &p) in path.iter().enumerate() {
            if obs.segment(from, p) {
                far = i;
            } else {
                break;
            }
        }
        let p = path[far];
        result.push_back(p);
        from = p;
        for _ in 0..=far {
            path.pop_front();
        }
    }
    result
}
impl Simulation {
    pub(crate) fn walk_actor(
        &mut self,
        walker: Walker,
        position: &mut [f32; 3],
        yaw: &mut f32,
        goal: [f32; 3],
        speed: f32,
        dt: f32,
    ) -> bool {
        let obs = Obstacles::new(&self.world, &self.collision, walker);
        let now = self.world.server_time_ms;
        let route = self.walkers.entry(walker).or_default();
        if route.goal != goal || now >= route.replan_at {
            route.goal = goal;
            route.points = plan(&obs, *position, goal);
            route.replan_at = now + 1000;
        }
        if (position[0] - goal[0]).hypot(position[2] - goal[2]) < 0.12
            && obs.segment(*position, goal)
        {
            *position = goal;
            return true;
        }
        while route
            .points
            .front()
            .is_some_and(|p| (position[0] - p[0]).hypot(position[2] - p[2]) < 0.10)
        {
            route.points.pop_front();
        }
        if let Some(&target) = route.points.front() {
            let d = c::sub(target, *position);
            let length = d[0].hypot(d[2]);
            let step = (speed * dt).min(length) / length.max(0.001);
            let next = [position[0] + d[0] * step, 0., position[2] + d[2] * step];
            if obs.segment(*position, next) {
                *position = next;
                *yaw = (-d[0]).atan2(-d[2]);
            } else {
                route.replan_at = now;
            }
        }
        false
    }
    pub(crate) fn tick_fleeing(&mut self, dt: f32) {
        let ids: Vec<_> = self
            .world
            .vehicles
            .iter()
            .filter(|v| v.physics.npc.mode == c::NpcMode::Fleeing && v.physics.npc.health > 0)
            .map(|v| v.vehicle_id)
            .collect();
        for id in ids {
            let i = self
                .world
                .vehicles
                .iter()
                .position(|v| v.vehicle_id == id)
                .unwrap();
            let mut p = self.world.vehicles[i].physics.npc.position;
            let mut yaw = self.world.vehicles[i].physics.npc.yaw;
            let key = Walker::Traveler(id);
            let goal = if let Some(route) = self.walkers.get(&key) {
                route.goal
            } else {
                let threat = self
                    .world
                    .peers
                    .iter()
                    .filter(|p| p.connected)
                    .min_by(|a, b| {
                        c::length(c::sub(a.position, p))
                            .total_cmp(&c::length(c::sub(b.position, p)))
                    })
                    .map_or(p, |p| p.position);
                let mut exits = vec![
                    [-28., 0., 8.],
                    [-28., 0., 28.],
                    [27., 0., 27.],
                    [27., 0., -32.],
                    [-29., 0., -32.],
                ];
                exits.sort_by(|a, b| {
                    c::length(c::sub(*b, threat)).total_cmp(&c::length(c::sub(*a, threat)))
                });
                let obstacles = Obstacles::new(&self.world, &self.collision, key);
                let fallback = exits[0];
                exits
                    .into_iter()
                    .find(|g| !plan(&obstacles, p, *g).is_empty())
                    .unwrap_or(fallback)
            };
            let arrived = self.walk_actor(key, &mut p, &mut yaw, goal, 3.5, dt);
            let npc = &mut self.world.vehicles[i].physics.npc;
            npc.position = p;
            npc.yaw = yaw;
            if arrived {
                npc.mode = c::NpcMode::Outside;
                self.walkers.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pedestrian_paths_go_around_rotated_cars_and_never_cut_corners() {
        for yaw in [0., 0.7, 1.57] {
            let obs = Obstacles {
                boxes: vec![([0.; 3], [1.2, 0., 3.], yaw)],
                people: vec![],
            };
            let start = [-5., 0., 0.];
            let goal = [5., 0., 0.];
            let path = plan(&obs, start, goal);
            assert!(path.len() > 1, "no car detour: {yaw}");
            let mut p = start;
            for target in path {
                assert!(obs.segment(p, target), "corner cut at {p:?} -> {target:?}");
                p = target;
            }
            assert_eq!(p, goal);
        }
    }
    #[test]
    fn newly_blocked_paths_are_rejected_and_replanned_without_phasing() {
        let start = [-5., 0., 0.];
        let goal = [5., 0., 0.];
        let mut obs = Obstacles {
            boxes: vec![],
            people: vec![],
        };
        assert_eq!(plan(&obs, start, goal).len(), 1);
        obs.boxes.push(([0.; 3], [1.2, 0., 2.5], 0.));
        assert!(!obs.segment(start, goal));
        let path = plan(&obs, start, goal);
        assert!(path.len() > 1);
        obs.boxes.push((goal, [0.5, 0., 0.5], 0.));
        assert!(plan(&obs, start, goal).is_empty());
    }
}
