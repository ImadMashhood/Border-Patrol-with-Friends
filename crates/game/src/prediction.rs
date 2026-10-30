//! Frame-rate movement prediction with bounded, smooth host reconciliation.
use super::*;
#[derive(Default)]
pub struct Prediction {
    pub position: Vec3,
    pub initialized: bool,
    pub received_at: f32,
    correction: Vec3,
}
impl Prediction {
    pub fn reconcile(&mut self, server: Vec3, velocity: Vec3, now: f32, reset: bool) {
        self.received_at = now;
        if !self.initialized || reset || self.position.distance(server) > 2.5 {
            self.position = server;
            self.initialized = true;
            self.correction = Vec3::ZERO;
            return;
        }
        let error = server + velocity * 0.05 - self.position;
        // Leave sub-poll movement ahead of the server alone; correct larger errors smoothly.
        let length = error.length();
        let dead_zone = if velocity.length_squared() < 0.0001 {
            0.005
        } else {
            0.10
        };
        self.correction = if length > dead_zone {
            error * (1. - dead_zone / length)
        } else {
            Vec3::ZERO
        };
    }
    pub fn correct(&mut self, dt: f32) {
        let delta = (self.correction * (1. - (-12. * dt).exp())).clamp_length_max(0.04);
        self.position += delta;
        self.correction -= delta;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconciliation_keeps_small_network_lag_smooth_and_resets_large_teleports() {
        let mut p = Prediction::default();
        p.reconcile(Vec3::ZERO, Vec3::ZERO, 0., false);
        p.position = Vec3::X * 0.16;
        p.reconcile(Vec3::ZERO, Vec3::X * 3.2, 0.05, false);
        let before = p.position;
        p.correct(1. / 60.);
        assert_eq!(before, p.position);
        p.reconcile(Vec3::X, Vec3::ZERO, 0.1, false);
        let before = p.position;
        p.correct(1. / 60.);
        assert!(p.position.distance(before) <= 0.041);
        assert!(p.position.x > before.x);
        p.reconcile(Vec3::X * 10., Vec3::ZERO, 0.2, false);
        assert_eq!(p.position, Vec3::X * 10.);
    }
}
