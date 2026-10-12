//! `LaunchConditions`: what a launch monitor measures about the ball, the flight model's input
//! (docs/00-architecture.md, the hand-off types).

use crate::math::{self, RADPS_PER_RPM};
use crate::vec::Vec3;

pub const MPS_PER_MPH: f64 = 0.44704;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchConditions {
    pub ball_speed_mps: f64,
    /// vertical launch angle above horizontal
    pub launch_rad: f64,
    /// world azimuth of the start direction (from +x towards +y)
    pub direction_rad: f64,
    pub spin_radps: f64,
    /// spin axis tilt: + tilts the axis so the ball curves right (fade / slice for a right-hander)
    pub spin_axis_rad: f64,
}

impl LaunchConditions {
    /// from launch-monitor units: mph, degrees, rpm
    pub fn from_monitor(ball_mph: f64, launch_deg: f64, direction_rad: f64, spin_rpm: f64, spin_axis_deg: f64) -> LaunchConditions {
        LaunchConditions {
            ball_speed_mps: ball_mph * MPS_PER_MPH,
            launch_rad: launch_deg.to_radians(),
            direction_rad,
            spin_radps: spin_rpm * RADPS_PER_RPM,
            spin_axis_rad: spin_axis_deg.to_radians(),
        }
    }

    /// the initial velocity
    pub fn velocity(&self) -> Vec3 {
        let (cl, sl) = (math::cos(self.launch_rad), math::sin(self.launch_rad));
        let (ca, sa) = (math::cos(self.direction_rad), math::sin(self.direction_rad));
        Vec3::new(ca * cl, sa * cl, sl) * self.ball_speed_mps
    }

    /// The initial spin vector. Pure backspin points along `right = v̂ × up` (so ω × v lifts); the tilt rotates it
    /// about the velocity, towards `-up'` (up' = right × v̂) for a positive (rightward-curving) tilt.
    pub fn spin(&self) -> Vec3 {
        let v = self.velocity().norm();
        let right = v.cross(Vec3::Z).norm();
        let up = right.cross(v);
        (right * math::cos(self.spin_axis_rad) - up * math::sin(self.spin_axis_rad)) * self.spin_radps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backspin_lifts_and_positive_tilt_curves_right() {
        // launched along +x: right is -y
        let l = LaunchConditions::from_monitor(100.0, 0.0, 0.0, 3000.0, 0.0);
        let lift = l.spin().cross(l.velocity());
        assert!(lift.z > 0.0 && lift.y.abs() < 1e-9);
        let l = LaunchConditions::from_monitor(100.0, 0.0, 0.0, 3000.0, 20.0);
        let lift = l.spin().cross(l.velocity());
        assert!(lift.y < 0.0, "a + tilt pushes the ball to the right (-y here)");
    }
}
