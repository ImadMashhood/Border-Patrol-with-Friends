//! Shared interaction geometry and authoritative combat state; no renderer or client hit reports.
use crate::VehicleView;
use serde::{Deserialize, Serialize};
pub type V3 = [f32; 3];
pub const PROJECTILE_SPEED: f32 = 95.;
pub const MAGAZINE: u8 = 12;
pub const FIRE_INTERVAL_MS: u64 = 220;
pub const RELOAD_MS: u64 = 1500;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Driver,
    Passenger,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pane {
    Windshield,
    Rear,
    DriverFront,
    DriverRear,
    PassengerFront,
    PassengerRear,
}
impl Pane {
    pub const ALL: [Self; 6] = [
        Self::Windshield,
        Self::Rear,
        Self::DriverFront,
        Self::DriverRear,
        Self::PassengerFront,
        Self::PassengerRear,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|v| *v == self).unwrap()
    }
    pub fn side(self) -> Option<Side> {
        match self {
            Self::DriverFront | Self::DriverRear => Some(Side::Driver),
            Self::PassengerFront | Self::PassengerRear => Some(Side::Passenger),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlassImpact {
    pub uv: [f32; 2],
    pub at_ms: u64,
    pub seed: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlassState {
    pub pane: Pane,
    pub lowered: f32,
    pub target_down: bool,
    pub shattered: bool,
    pub impacts: Vec<GlassImpact>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcMode {
    Seated,
    Exiting,
    Entering,
    Outside,
    Dead,
    Fleeing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub health: u8,
    pub mode: NpcMode,
    pub position: V3,
    pub yaw: f32,
    pub exit_progress: f32,
    pub died_seated: bool,
    pub ragdoll: Vec<BonePose>,
    pub extracted: bool,
    pub body_pull_active: bool,
    pub death_impulse: V3,
    pub body_holder: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePhysics {
    pub glass: Vec<GlassState>,
    pub door_open: [bool; 2],
    pub door_angles: [f32; 2],
    pub npc: NpcState,
    pub disabled_at: Option<u64>,
    pub tow_until: Option<u64>,
    pub manual_window_until: u64,
}
impl Default for VehiclePhysics {
    fn default() -> Self {
        Self {
            glass: Pane::ALL
                .into_iter()
                .map(|pane| GlassState {
                    pane,
                    lowered: 0.,
                    target_down: false,
                    shattered: false,
                    impacts: vec![],
                })
                .collect(),
            door_open: [false; 2],
            door_angles: [0.; 2],
            npc: NpcState {
                health: 100,
                mode: NpcMode::Seated,
                position: [0.; 3],
                yaw: 0.,
                exit_progress: 0.,
                died_seated: false,
                ragdoll: vec![],
                extracted: false,
                body_pull_active: false,
                death_impulse: [0.; 3],
                body_holder: None,
            },
            disabled_at: None,
            tow_until: None,
            manual_window_until: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponState {
    pub drawn: bool,
    pub magazine: u8,
    pub reserve: u16,
    pub reload_until: Option<u64>,
    pub last_fired_ms: Option<u64>,
}
impl Default for WeaponState {
    fn default() -> Self {
        Self {
            drawn: false,
            magazine: MAGAZINE,
            reserve: 60,
            reload_until: None,
            last_fired_ms: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub id: u64,
    pub shooter: u64,
    pub position: V3,
    pub velocity: V3,
    pub energy: f32,
    pub born_ms: u64,
    pub origin: V3,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub id: u64,
    pub projectile: u64,
    pub position: V3,
    pub normal: V3,
    pub kind: String,
    pub vehicle_id: Option<u64>,
    pub at_ms: u64,
    pub origin: V3,
}
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
pub fn rotate(p: V3, yaw: f32) -> V3 {
    let (s, c) = yaw.sin_cos();
    [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c]
}
pub fn world(v: &VehicleView, p: V3) -> V3 {
    add(v.position, rotate(p, v.yaw))
}
pub fn local(v: &VehicleView, p: V3) -> V3 {
    rotate(sub(p, v.position), -v.yaw)
}
pub fn side_index(side: Side) -> usize {
    if side == Side::Driver { 0 } else { 1 }
}
pub fn door_pivot(kind: &str, side: Side) -> V3 {
    let spec = crate::vehicle::spec(kind);
    [
        if side == Side::Driver {
            -spec.half_width + 0.04
        } else {
            spec.half_width - 0.04
        },
        0.,
        spec.window_z
            - if crate::vehicle::pickup(kind) {
                0.85
            } else {
                0.65
            },
    ]
}
pub fn door_point(kind: &str, side: Side, angle: f32, p: V3) -> V3 {
    let pivot = door_pivot(kind, side);
    add(
        pivot,
        rotate(
            sub(p, pivot),
            if side == Side::Driver { -angle } else { angle },
        ),
    )
}
#[derive(Debug, Clone, Copy)]
pub struct PaneFrame {
    pub center: V3,
    pub u: V3,
    pub v: V3,
    pub normal: V3,
    pub half: [f32; 2],
}
pub fn pane_frame(v: &VehicleView, pane: Pane) -> Option<PaneFrame> {
    let truck = crate::vehicle::pickup(&v.kind);
    let state = &v.physics.glass[pane.index()];
    let (mut center, mut u, mut up, half) = match pane {
        Pane::Windshield => (
            [
                0.,
                if truck { 1.7 } else { 1.28 },
                if truck { -2.1 } else { -0.98 },
            ],
            [1., 0., 0.],
            if truck {
                [0., 1., 0.]
            } else {
                [0., 0.878, 0.479]
            },
            [
                crate::vehicle::spec(&v.kind).half_width - 0.07,
                if truck { 0.41 } else { 0.325 },
            ],
        ),
        Pane::Rear => (
            [
                0.,
                if truck { 1.7 } else { 1.27 },
                if truck { 0.03 } else { 1.16 },
            ],
            [1., 0., 0.],
            if truck {
                [0., 1., 0.]
            } else {
                [0., 0.9, -0.435]
            },
            [
                crate::vehicle::spec(&v.kind).half_width - 0.07,
                if truck { 0.35 } else { 0.31 },
            ],
        ),
        p => {
            let driver = p.side() == Some(Side::Driver);
            let front = matches!(p, Pane::DriverFront | Pane::PassengerFront);
            (
                [
                    if driver { -1. } else { 1. }
                        * (crate::vehicle::spec(&v.kind).half_width - 0.04),
                    if truck { 1.69 } else { 1.27 },
                    if truck {
                        if front { -1.475 } else { -0.35 }
                    } else if front {
                        -0.325
                    } else {
                        0.73
                    },
                ],
                [0., 0., 1.],
                [0., 1., 0.],
                [
                    if front { 0.515 } else { 0.34 },
                    if truck { 0.385 } else { 0.31 },
                ],
            )
        }
    };
    let lift = crate::vehicle::cab_lift(&v.kind);
    center[1] += lift;
    if pane == Pane::Rear {
        center[2] = crate::vehicle::rear_glass_z(&v.kind);
    }
    if matches!(pane, Pane::DriverRear | Pane::PassengerRear)
        && matches!(v.kind.as_str(), "suv" | "van")
    {
        center[2] = if v.kind == "van" { 1.4 } else { 1.1 };
    }
    let mut half = half;
    if matches!(pane, Pane::DriverRear | Pane::PassengerRear)
        && matches!(v.kind.as_str(), "suv" | "van")
    {
        half[0] = if v.kind == "van" { 0.95 } else { 0.70 };
    }

    if let Some(side) = pane.side() {
        let bottom = if truck { 1.3 } else { 0.96 + lift };
        let top = center[1] + half[1] - state.lowered * 0.9;
        let low = (center[1] - half[1] - state.lowered * 0.9).max(bottom);
        if top <= low {
            return None;
        }
        half[1] = (top - low) * 0.5;
        center[1] = (top + low) * 0.5;
        if matches!(pane, Pane::DriverFront | Pane::PassengerFront) {
            let angle = v.physics.door_angles[side_index(side)];
            center = door_point(&v.kind, side, angle, center);
            u = rotate(u, if side == Side::Driver { -angle } else { angle });
            up = rotate(up, if side == Side::Driver { -angle } else { angle });
        }
    }
    let normal = [
        u[1] * up[2] - u[2] * up[1],
        u[2] * up[0] - u[0] * up[2],
        u[0] * up[1] - u[1] * up[0],
    ];
    Some(PaneFrame {
        center: world(v, center),
        u: rotate(u, v.yaw),
        v: rotate(up, v.yaw),
        normal: rotate(normal, v.yaw),
        half,
    })
}
pub fn pane_hit(origin: V3, dir: V3, max: f32, frame: PaneFrame) -> Option<(f32, [f32; 2])> {
    let denom = dot(dir, frame.normal);
    if denom.abs() < 0.0001 {
        return None;
    }
    let t = dot(sub(frame.center, origin), frame.normal) / denom;
    if t < 0.001 || t > max {
        return None;
    }
    let offset = sub(add(origin, mul(dir, t)), frame.center);
    let uv = [
        dot(offset, frame.u) / frame.half[0],
        dot(offset, frame.v) / frame.half[1],
    ];
    (uv[0].abs() <= 1. && uv[1].abs() <= 1.).then_some((t, uv))
}
/// Match the exact removed triangle cells used by the glass renderer.
pub fn glass_hole(uv: [f32; 2], impacts: &[GlassImpact]) -> bool {
    let fx = ((uv[0] + 1.) * 6.).clamp(0., 11.99999);
    let fy = ((uv[1] + 1.) * 4.).clamp(0., 7.99999);
    let a = [-1. + fx.floor() / 6., -1. + fy.floor() / 4.];
    let center = if fy.fract() <= fx.fract() {
        [a[0] + 1. / 9., a[1] + 1. / 12.]
    } else {
        [a[0] + 1. / 18., a[1] + 1. / 6.]
    };
    impacts
        .iter()
        .any(|h| (center[0] - h.uv[0]).hypot(center[1] - h.uv[1]) < 0.15)
}
/// Slab intersection for swept projectiles, including the origin inside a volume.
pub fn ray_box(origin: V3, dir: V3, center: V3, half: V3, max: f32) -> Option<f32> {
    let mut near: f32 = 0.;
    let mut far = max;
    for i in 0..3 {
        let o = origin[i] - center[i];
        if dir[i].abs() < 0.00001 {
            if o.abs() > half[i] {
                return None;
            }
        } else {
            let a = (-half[i] - o) / dir[i];
            let b = (half[i] - o) / dir[i];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}
pub fn seated_position(v: &VehicleView) -> V3 {
    world(
        v,
        [
            -0.48,
            if crate::vehicle::pickup(&v.kind) {
                0.20
            } else {
                crate::vehicle::cab_lift(&v.kind) - 0.15
            },
            crate::vehicle::spec(&v.kind).window_z,
        ],
    )
}
/// Solid vehicle shell pieces leave real openings for glass and door apertures.
pub fn shell_boxes(v: &VehicleView) -> Vec<(V3, V3)> {
    let truck = crate::vehicle::pickup(&v.kind);
    let spec = crate::vehicle::spec(&v.kind);
    let belt = if truck {
        1.05
    } else {
        0.86 + crate::vehicle::cab_lift(&v.kind)
    };
    let mut boxes = vec![
        (
            [0., 0.45, 0.],
            [
                spec.half_width,
                0.35,
                crate::vehicle::body_half_length(&v.kind),
            ],
        ),
        (
            [
                0.,
                if truck {
                    2.18
                } else {
                    1.65 + crate::vehicle::cab_lift(&v.kind)
                },
                if truck { -1.05 } else { 0.1 },
            ],
            [
                if truck { 1.025 } else { 0.87 },
                0.065,
                if truck { 1.125 } else { 0.94 },
            ],
        ),
    ];
    if matches!(v.kind.as_str(), "suv" | "van") {
        let rear = crate::vehicle::rear_glass_z(&v.kind);
        boxes[1].0[2] = (rear - 1.1) * 0.5;
        boxes[1].1[2] = (rear + 1.1) * 0.5;
    }
    // Hood, trunk/bed and seat backs; do not use a whole-car box above the belt.
    boxes.push((
        [0., belt, if truck { -2.7 } else { -1.6 }],
        [spec.half_width - 0.1, 0.12, if truck { 0.5 } else { 0.6 }],
    ));
    boxes.push((
        [0., belt, if truck { 1.8 } else { 1.75 }],
        [spec.half_width - 0.1, 0.12, if truck { 1.4 } else { 0.5 }],
    ));
    for x in [-1., 1.] {
        for z in if truck {
            [-2.08, -0.98, 0.02]
        } else {
            [-0.96, 0.25, 1.15]
        } {
            boxes.push((
                [
                    x * (spec.half_width - 0.16),
                    if truck {
                        1.65
                    } else {
                        1.28 + crate::vehicle::cab_lift(&v.kind)
                    },
                    z,
                ],
                [0.055, if truck { 0.52 } else { 0.35 }, 0.055],
            ));
        }
    }
    boxes.extend(crate::vehicle::assembly_boxes(&v.kind));
    boxes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonePose {
    pub bone: u8,
    pub position: V3,
    pub rotation: [f32; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPose {
    pub id: u64,
    pub position: V3,
    pub rotation: [f32; 4],
    pub size: V3,
    pub born_ms: u64,
}
