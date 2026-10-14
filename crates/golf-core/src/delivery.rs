//! `ClubDelivery`: what the club is doing at impact, the seam between the swing (golf-swing) and the physics
//! (golf-impact). Launch-monitor club numbers (D9). Angles relative to the aim line; a left-hander is mirrored at
//! the phone boundary (docs/01-conventions.md).

use crate::vec::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClubDelivery {
    pub head_speed_mps: f64,
    /// the club's horizontal path relative to the aim line, + = right (in-to-out for a right-hander)
    pub path_rad: f64,
    /// the club's vertical path at impact, + = up (hitting up), - = down
    pub attack_rad: f64,
    /// the face angle relative to the aim line, + = open (right)
    pub face_rad: f64,
    /// the face's loft at impact: static loft, shaft lean, and how the hands deliver it
    pub dynamic_loft_rad: f64,
    /// where on the face the ball is struck, from the sweet spot: x towards the toe, y up (m)
    pub strike_m: Vec2,
}

impl ClubDelivery {
    /// a centred, square strike along the aim line (degrees in, for tables and tests)
    pub fn square(head_speed_mps: f64, attack_deg: f64, dynamic_loft_deg: f64) -> ClubDelivery {
        ClubDelivery {
            head_speed_mps,
            path_rad: 0.0,
            attack_rad: attack_deg.to_radians(),
            face_rad: 0.0,
            dynamic_loft_rad: dynamic_loft_deg.to_radians(),
            strike_m: Vec2::ZERO,
        }
    }

    /// the face relative to the path, + = open to the path (a fade's spin for a right-hander)
    pub fn face_to_path_rad(&self) -> f64 {
        self.face_rad - self.path_rad
    }
}
