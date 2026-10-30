//! Shared swept-axis walking rules for authority and local movement prediction.
use crate::{LaneView, VehicleStage, VehicleView, map, vehicle};
#[allow(clippy::too_many_arguments)] // Common movement context keeps authority and prediction collision rules identical.
pub fn step(
    mut position: [f32; 3],
    axis: [f32; 2],
    jog: bool,
    dt: f32,
    solids: &[map::Aabb],
    vehicles: &[VehicleView],
    lanes: &[LaneView],
    door_open: bool,
) -> [f32; 3] {
    let length = axis[0].hypot(axis[1]).max(1.);
    for axis_index in [0, 1] {
        let mut next = position;
        let coordinate = if axis_index == 0 { 0 } else { 2 };
        next[coordinate] += axis[axis_index] / length * if jog { 5.8 } else { 3.2 } * dt;
        let closed_door = map::Aabb {
            center: map::DOOR,
            half: [1.1, 1.5, 0.16],
        };
        if map::walkable(next[0], next[2])
            && !solids.iter().any(|a| map::blocked(next[0], next[2], a))
            && !vehicles
                .iter()
                .filter(|v| v.stage != VehicleStage::Exited)
                .any(|v| {
                    map::blocked(
                        next[0],
                        next[2],
                        &vehicle::spec(&v.kind).aabb(v.position, v.yaw),
                    )
                })
            && !lanes.iter().any(|lane| {
                [
                    (!lane.entry_barrier_open, vehicle::ENTRY_GATE_Z),
                    (!lane.barrier_open, vehicle::EXIT_GATE_Z),
                ]
                .iter()
                .any(|(closed, z)| {
                    *closed
                        && map::blocked(
                            next[0],
                            next[2],
                            &map::Aabb {
                                center: [(lane.lane_id - 1) as f32 * 7., 1.1, *z],
                                half: [2.65, 0.1, 0.1],
                            },
                        )
                })
            })
            && !vehicles.iter().any(|v| {
                let n = &v.physics.npc;
                if matches!(
                    n.mode,
                    crate::combat::NpcMode::Outside
                        | crate::combat::NpcMode::Exiting
                        | crate::combat::NpcMode::Entering
                ) && (next[0] - n.position[0]).hypot(next[2] - n.position[2]) < 0.42
                {
                    return true;
                }
                [crate::combat::Side::Driver, crate::combat::Side::Passenger]
                    .into_iter()
                    .any(|side| {
                        let angle = v.physics.door_angles[crate::combat::side_index(side)]
                            * if side == crate::combat::Side::Driver {
                                -1.
                            } else {
                                1.
                            };
                        if angle.abs() < 0.05 {
                            return false;
                        }
                        let point = crate::combat::local(v, next);
                        let pivot = crate::combat::door_pivot(&v.kind, side);
                        let local = crate::combat::rotate(crate::combat::sub(point, pivot), -angle);
                        local[0].abs() < 0.27 && local[2] > -0.22 && local[2] < 1.37
                    })
            })
            && (door_open || !map::blocked(next[0], next[2], &closed_door))
        {
            position = next;
        }
    }
    position
}
