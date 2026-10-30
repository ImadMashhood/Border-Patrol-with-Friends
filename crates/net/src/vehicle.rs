//! Shared physical catalog. Local +Z is the rear; vehicles arrive facing north (-Z).
use serde::{Deserialize, Serialize};

pub const BUMPER_GAP: f32 = 3.0;
pub const ENTRY_GATE_Z: f32 = 16.0;
pub const EXIT_GATE_Z: f32 = -5.8;
pub const INTERACTION_RANGE: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehiclePoint {
    Driver,
    Passenger,
    Hood,
    Cargo,
}
impl VehiclePoint {
    pub const ALL: [Self; 4] = [Self::Driver, Self::Passenger, Self::Hood, Self::Cargo];
    pub fn label(self, kind: &str) -> &'static str {
        match self {
            Self::Driver => "Driver window · talk",
            Self::Passenger => "Passenger door · inspect",
            Self::Hood => "Hood · inspect",
            Self::Cargo if pickup(kind) => "Truck tailgate · inspect cargo",
            Self::Cargo => "Trunk · inspect cargo",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct VehicleSpec {
    pub kind: &'static str,
    pub half_width: f32,
    pub half_length: f32,
    pub height: f32,
    pub window_z: f32,
}
pub const KINDS: [&str; 6] = ["sedan", "truck", "suv", "van", "hatchback", "utility_truck"];
pub fn pickup(kind: &str) -> bool {
    matches!(kind, "truck" | "utility_truck")
}
pub fn cab_lift(kind: &str) -> f32 {
    match kind {
        "suv" => 0.22,
        "van" => 0.55,
        _ => 0.,
    }
}
pub fn rear_glass_z(kind: &str) -> f32 {
    match kind {
        "suv" => 1.9,
        "van" => 2.45,
        k if pickup(k) => 0.03,
        _ => 1.16,
    }
}
pub fn spec(kind: &str) -> VehicleSpec {
    let (kind, w, l, h, z) = match kind {
        "truck" => ("truck", 1.17, 3.3, 2.35, -1.2),
        "utility_truck" => ("utility_truck", 1.17, 3.3, 2.55, -1.2),
        "suv" => ("suv", 1.14, 2.55, 2.15, -0.25),
        "van" => ("van", 1.14, 2.9, 2.55, -0.25),
        "hatchback" => ("hatchback", 1.02, 2.1, 1.85, -0.25),
        _ => ("sedan", 1.08, 2.3, 1.85, -0.25),
    };
    VehicleSpec {
        kind,
        half_width: w,
        half_length: l,
        height: h,
        window_z: z,
    }
}
/// Six bodies distributed across both lanes; appearance never encodes guilt.
pub fn scheduled_kind(index: usize) -> &'static str {
    const SHIFT: [&str; 10] = [
        "sedan",
        "truck",
        "suv",
        "van",
        "hatchback",
        "utility_truck",
        "truck",
        "sedan",
        "van",
        "suv",
    ];
    SHIFT[index % SHIFT.len()]
}
impl VehicleSpec {
    pub fn local_point(self, point: VehiclePoint) -> [f32; 3] {
        match point {
            VehiclePoint::Driver => [
                -self.half_width - 0.45,
                1.5 + cab_lift(self.kind),
                self.window_z,
            ],
            VehiclePoint::Passenger => [
                self.half_width + 0.45,
                1.5 + cab_lift(self.kind),
                self.window_z,
            ],
            VehiclePoint::Hood => [0., 1.2, -self.half_length - 0.6],
            VehiclePoint::Cargo => [0., 1.3, self.half_length + 0.65],
        }
    }
    pub fn world_point(self, position: [f32; 3], yaw: f32, point: VehiclePoint) -> [f32; 3] {
        let p = self.local_point(point);
        let (sin, cos) = yaw.sin_cos();
        [
            position[0] + p[0] * cos + p[2] * sin,
            p[1],
            position[2] - p[0] * sin + p[2] * cos,
        ]
    }
    /// Aim at the panel/window itself while range is measured from the accessible standing point.
    pub fn aim_point(self, position: [f32; 3], yaw: f32, point: VehiclePoint) -> [f32; 3] {
        let mut p = self.local_point(point);
        match point {
            VehiclePoint::Driver => p[0] += 0.55,
            VehiclePoint::Passenger => p[0] -= 0.55,
            VehiclePoint::Hood => p[2] += 0.65,
            VehiclePoint::Cargo => p[2] -= 0.65,
        }
        let (sin, cos) = yaw.sin_cos();
        [
            position[0] + p[0] * cos + p[2] * sin,
            p[1],
            position[2] - p[0] * sin + p[2] * cos,
        ]
    }
    pub fn aabb(self, position: [f32; 3], yaw: f32) -> crate::map::Aabb {
        let (sin, cos) = yaw.sin_cos();
        crate::map::Aabb {
            center: position,
            half: [
                self.half_width * cos.abs() + self.half_length * sin.abs(),
                self.height * 0.5,
                self.half_width * sin.abs() + self.half_length * cos.abs(),
            ],
        }
    }
}
pub fn departure_path(lane: u8, admitted: bool) -> Vec<[f32; 2]> {
    let x = (lane - 1) as f32 * 7.;
    if admitted {
        vec![
            [x, -16.],
            [x, -95.],
            [-113.5 + x, -210.],
            [-433.5 + x, -340.],
        ]
    } else {
        vec![[x, -11.], [-4., -16.], [-440., -16.]]
    }
}
