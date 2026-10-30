//! Shared, fixed-length skeleton used by both the renderer and the host.
//! +Y runs up each bone; flexion is around local X, with -Z facing forward.
use crate::combat::{V3, add};
pub const BONE_COUNT: usize = 11;
#[derive(Clone, Copy, Debug)]
pub struct Bone {
    pub center: V3,
    pub angle: f32,
    pub length: f32,
    pub radius: f32,
    pub mass: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub parent: usize,
    pub child: usize,
    pub anchor: V3,
    /// Anatomical reference angles, not relative to the death pose.
    pub limits: [[f32; 2]; 3],
    pub hinge: bool,
    pub stiffness: f32,
    pub damping: f32,
}
pub struct Rig {
    pub bones: [Bone; BONE_COUNT],
    pub links: Vec<Link>,
}
fn down(length: f32, angle: f32) -> V3 {
    [0., -length * angle.cos(), -length * angle.sin()]
}
fn segment(top: V3, length: f32, angle: f32, radius: f32, mass: f32) -> Bone {
    let d = down(length * 0.5, angle);
    Bone {
        center: add(top, d),
        angle,
        length,
        radius,
        mass,
    }
}
pub fn skeleton(seated: bool) -> Rig {
    let mut bones = [segment([0., 0.99, 0.], 0.21, 0., 0.14, 12.); BONE_COUNT];
    bones[1] = segment([0., 1.40, 0.], 0.41, 0., 0.18, 25.);
    bones[2] = segment([0., 1.70, 0.], 0.30, 0., 0.15, 5.);
    let mut links = vec![
        Link {
            parent: 0,
            child: 1,
            anchor: [0., 0.99, 0.],
            limits: [[-0.22, 0.30], [-0.16, 0.16], [-0.16, 0.16]],
            hinge: false,
            stiffness: 20.,
            damping: 12.,
        },
        Link {
            parent: 1,
            child: 2,
            anchor: [0., 1.40, 0.],
            limits: [[-0.45, 0.65], [-0.65, 0.65], [-0.35, 0.35]],
            hinge: false,
            stiffness: 0.4,
            damping: 3.,
        },
    ];
    for (side, upper, lower, thigh, shin) in [(-1., 3, 4, 7, 8), (1., 5, 6, 9, 10)] {
        let shoulder = [side * 0.235, 1.31, 0.];
        let arm_angle = if seated { 0.65 } else { 0. };
        let elbow = add(shoulder, down(0.30, arm_angle));
        bones[upper] = segment(shoulder, 0.30, arm_angle, 0.075, 2.2);
        bones[lower] = segment(elbow, 0.28, if seated { 1.35 } else { 0. }, 0.065, 1.7);
        links.push(Link {
            parent: 1,
            child: upper,
            anchor: shoulder,
            limits: [[-1.1, 2.2], [-0.8, 0.8], [-1.35, 1.35]],
            hinge: false,
            stiffness: 2.,
            damping: 1.5,
        });
        links.push(Link {
            parent: upper,
            child: lower,
            anchor: elbow,
            limits: [[0., 2.45], [0., 0.], [0., 0.]],
            hinge: true,
            stiffness: 1.,
            damping: 1.,
        });
        let hip = [side * 0.11, 0.84, 0.];
        let leg_angle = if seated { 1.45 } else { 0. };
        let knee = add(hip, down(0.36, leg_angle));
        bones[thigh] = segment(hip, 0.36, leg_angle, 0.10, 8.);
        bones[shin] = segment(knee, 0.36, if seated { 0.90 } else { 0. }, 0.085, 4.1);
        links.push(Link {
            parent: 0,
            child: thigh,
            anchor: hip,
            limits: [[-0.4, 1.95], [-0.4, 0.4], [-0.65, 0.65]],
            hinge: false,
            stiffness: 4.,
            damping: 2.,
        });
        links.push(Link {
            parent: thigh,
            child: shin,
            anchor: knee,
            limits: [[-2.45, 0.02], [0., 0.], [0., 0.]],
            hinge: true,
            stiffness: 1.,
            damping: 1.5,
        });
    }
    Rig { bones, links }
}

/// Walking/running pose keeps each distal bone anchored to its parent's endpoint.
pub fn walking(phase: f32) -> Rig {
    let mut rig = skeleton(false);
    let swing = phase.sin() * 0.55;
    for (side, thigh, shin, upper, fore) in [(-1., 7, 8, 3, 4), (1., 9, 10, 5, 6)] {
        let angle = swing * side;
        let hip = [side * 0.11, 0.84, 0.];
        rig.bones[thigh] = segment(hip, 0.36, angle, 0.10, 8.);
        let knee = add(hip, down(0.36, angle));
        rig.bones[shin] = segment(knee, 0.36, angle - angle.max(0.) * 0.8, 0.085, 4.1);
        let shoulder = [side * 0.235, 1.31, 0.];
        let arm = -angle;
        rig.bones[upper] = segment(shoulder, 0.30, arm, 0.075, 2.2);
        rig.bones[fore] = segment(add(shoulder, down(0.30, arm)), 0.28, arm + 0.3, 0.065, 1.7);
    }
    rig
}
