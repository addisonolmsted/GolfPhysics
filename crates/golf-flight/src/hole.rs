//! The hole as the flight sees it: the cup's opening, where a ball coming down is handed to golf-ground's cup model
//! (slam dunks), and the flagstick when it is in, a vertical pole a flying ball can hit.

use golf_core::ball::CONTACT_RADIUS_M;
use golf_core::{Vec2, Vec3};

/// the flagstick: Rules of Golf maximum 1 inch thick; 7 ft above the green is the usual height
pub const PIN_RADIUS_M: f64 = 0.0127;
pub const PIN_HEIGHT_M: f64 = 2.134;
/// A flagstick is a flexible fibreglass pole sitting loosely in its ferrule: it soaks up most of a ball's pace, which
/// is why a firm putt or chip that hits it usually drops. Restitution and friction for a ball hitting it.
pub const PIN_COR: f64 = 0.12;
pub const PIN_MU: f64 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub xy: Vec2,
    /// the ground height at the rim
    pub z: f64,
    pub radius: f64,
    pub pin_in: bool,
}

impl Hole {
    /// within reach of the opening: a ball here can touch the rim, so its contact goes to the cup model
    #[inline]
    pub fn over(&self, p: Vec2) -> bool {
        (p - self.xy).len() < self.radius + CONTACT_RADIUS_M
    }

    /// Where the ball's centre, moving from a to b in a straight line, first touches the flagstick: the fraction of
    /// the way, and the pole's outward normal there. Only from outside the pole and above the rim.
    pub fn pin_hit(&self, a: Vec3, b: Vec3) -> Option<(f64, Vec3)> {
        if !self.pin_in {
            return None;
        }
        let rr = PIN_RADIUS_M + CONTACT_RADIUS_M;
        let p0 = a.xy() - self.xy;
        let d = b.xy() - a.xy();
        let c = p0.dot(p0) - rr * rr;
        if c < 0.0 {
            return None;
        }
        let aa = d.dot(d);
        if aa < 1e-18 {
            return None;
        }
        let bb = p0.dot(d);
        let disc = bb * bb - aa * c;
        if bb >= 0.0 || disc < 0.0 {
            return None;
        }
        let t = (-bb - disc.sqrt()) / aa;
        if !(0.0..=1.0).contains(&t) {
            return None;
        }
        let z = a.z + (b.z - a.z) * t;
        if z < self.z || z > self.z + PIN_HEIGHT_M + CONTACT_RADIUS_M {
            return None;
        }
        let n = (p0 + d * t) / rr;
        Some((t, Vec3::new(n.x, n.y, 0.0)))
    }
}
