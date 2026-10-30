//! Shared map constants and renderer-exported collision; the host never trusts client coordinates.
use serde::{Deserialize, Serialize};
pub const SPAWNS: [[f32; 3]; 2] = [[-14., 1.7, -6.], [-10., 1.7, -6.]];
pub const DOOR: [f32; 3] = [-12., 1.5, -3.5];
pub const READY_TARGET: [f32; 3] = [-12., 1.7, -4.8];
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub center: [f32; 3],
    pub half: [f32; 3],
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CollisionMap {
    pub schema_version: u16,
    pub solids: Vec<Aabb>,
}
pub fn collision() -> CollisionMap {
    serde_json::from_str(include_str!(
        "../../../assets/maps/san-ignacio-collision.json"
    ))
    .expect("valid baked collision")
}
pub fn walkable(x: f32, z: f32) -> bool {
    ((-32. ..=32.).contains(&x) && (-38. ..=35.).contains(&z))
        || ((-2.6..=9.6).contains(&x) && (35. ..=77.).contains(&z))
}
pub fn blocked(x: f32, z: f32, solid: &Aabb) -> bool {
    (x - solid.center[0]).abs() < solid.half[0] + 0.22
        && (z - solid.center[2]).abs() < solid.half[2] + 0.22
}
pub fn door_distance(position: [f32; 3]) -> f32 {
    ((position[0] - DOOR[0]).powi(2) + (position[2] - DOOR[2]).powi(2)).sqrt()
}
const APPROACH: [[f32; 2]; 4] = [[0., 0.], [0., 130.], [136.5, 235.], [426.5, 350.]];
pub fn incoming(mut distance: f32, lane: u8) -> ([f32; 3], f32) {
    for pair in APPROACH.windows(2) {
        let dx = pair[1][0] - pair[0][0];
        let dz = pair[1][1] - pair[0][1];
        let length = dx.hypot(dz);
        if distance <= length {
            return (
                [
                    pair[0][0] + dx * distance / length + (lane - 1) as f32 * 7.,
                    0.,
                    pair[0][1] + dz * distance / length,
                ],
                dx.atan2(dz),
            );
        }
        distance -= length;
    }
    ([426.5 + (lane - 1) as f32 * 7., 0., 350.], 0.)
}
