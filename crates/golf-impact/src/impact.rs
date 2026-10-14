//! Delivery to launch: an oblique impact of the face on the ball (docs/models/impact.md).
//!
//! The club moves along `u` (path and attack); the face points along `n` (face angle and dynamic loft). The angle
//! between them is the spin loft. Along `n` the ball takes the normal impact with the face's restitution; across
//! the face it is dragged along until it rolls on the face, which leaves it with velocity `v_t` down the face and
//! backspin about `ŝ × n`. Measured balls leave with more spin than that (they are compliant tangentially), which
//! the fitted `spin_factor` carries. Start direction (mostly face, some path), launch (mostly loft, some attack)
//! and the spin axis tilt (face to path) all come out of the same vectors: the D-plane.

use golf_core::ball::{INERTIA_K, MASS_KG, RADIUS_M};
use golf_core::math::{self, acos, atan2};
use golf_core::{ClubDelivery, LaunchConditions, Vec3};

use crate::club::{Club, ClubTable};
use crate::lie::Lie;

/// the launch and what produced it
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impact {
    pub launch: LaunchConditions,
    pub spin_loft_rad: f64,
    pub smash: f64,
    pub cor: f64,
}

/// rotate v about the unit axis k by angle a (Rodrigues)
fn rotate(v: Vec3, k: Vec3, a: f64) -> Vec3 {
    let (c, s) = (math::cos(a), math::sin(a));
    v * c + k.cross(v) * s + k * (k.dot(v) * (1.0 - c))
}

/// Delivery -> launch for `club`, aimed along world azimuth `aim_rad`, from `lie`.
pub fn impact(table: &ClubTable, club: &Club, d: &ClubDelivery, aim_rad: f64, lie: &Lie) -> Impact {
    // the aim frame: x along the aim, y to the left, z up. Angles to the right are negative about z
    let u0 = Vec3::new(math::cos(d.attack_rad) * math::cos(d.path_rad), -math::cos(d.attack_rad) * math::sin(d.path_rad), math::sin(d.attack_rad));
    let n0 = Vec3::new(math::cos(d.dynamic_loft_rad) * math::cos(d.face_rad), -math::cos(d.dynamic_loft_rad) * math::sin(d.face_rad), math::sin(d.dynamic_loft_rad));

    // The slope at the ball, in the aim frame: uphill along the aim tilts the swing (partly: slope_follow), a
    // sidehill tilts the face with the sole (in full), which points a lofted face left with the ball above the feet
    let (ca, sa) = (math::cos(aim_rad), math::sin(aim_rad));
    let nl = Vec3::new(lie.normal.x * ca + lie.normal.y * sa, -lie.normal.x * sa + lie.normal.y * ca, lie.normal.z);
    let uphill = atan2(-nl.x, nl.z);
    let sidehill = atan2(nl.y, nl.z);
    let lateral = Vec3::new(0.0, 1.0, 0.0);
    let forward = Vec3::new(1.0, 0.0, 0.0);
    // (a right-hander stands left of the aim line, at +y; with the ball above the feet the ground rises towards -y,
    // the normal leans to +y, and the face, tilted with the sole, points left: a rotation of -sidehill about x)
    let tilt = |v: Vec3| rotate(rotate(v, lateral, -uphill * table.slope_follow), forward, -sidehill);
    let u = tilt(u0);
    let n = tilt(n0);

    // the normal and tangential parts of the club's motion relative to the face
    let cos_sl = u.dot(n).clamp(-1.0, 1.0);
    let spin_loft = acos(cos_sl);
    let t = u - n * cos_sl;
    let tl = t.len();
    let s_hat = if tl > 1e-12 { t / tl } else { Vec3::ZERO };

    let v_club = d.head_speed_mps;
    let mass_ratio = MASS_KG / club.head_kg();
    let vn_impact = v_club * cos_sl;
    let cor = (club.cor_ref + table.cor_slope * (vn_impact - 40.0)).clamp(0.5, 0.9);
    let v_n = vn_impact * (1.0 + cor) / (1.0 + mass_ratio);
    // the tangential impulse that leaves the ball rolling on the face: J = V sinθ / ((1 + 1/k)/m + 1/M)
    let v_t = v_club * tl / (1.0 + 1.0 / INERTIA_K + mass_ratio);
    let mut v = n * v_n + s_hat * v_t;
    let sf = club.spin_factor * math::pow((vn_impact / 40.0).max(0.1), -club.spin_speed_exp);
    let mut w = s_hat.cross(n) * (sf * v_t / (INERTIA_K * RADIUS_M));

    // the strike point: speed lost off the sweet spot, gear effect (toe draws, high spins less), launch from height
    let off = d.strike_m;
    let speed_keep = (1.0 - club.strike_loss_per_m * off.len()).max(0.5);
    v = v * speed_keep;
    let vl = v.len();
    let vhat = v / vl;
    let right = vhat.cross(Vec3::Z).norm();
    let up = right.cross(vhat);
    w += up * (club.gear_h * off.x * vl);
    w -= right * (club.gear_v * off.y * vl);
    v = rotate(v, right, (club.launch_per_m_high * off.y).to_radians());

    // the lie: grass or sand between the face and the ball
    v = v * lie.factors.speed_factor;
    w = w * lie.factors.spin_factor;
    let vl = v.len();
    let vhat = v / vl;
    let right = vhat.cross(Vec3::Z).norm();
    v = rotate(v, right, lie.factors.launch_add_deg.to_radians());

    // as launch-monitor numbers, in the world: the spin's component across the velocity (rifle spin dropped)
    let vl = v.len();
    let vhat = v / vl;
    let right = vhat.cross(Vec3::Z).norm();
    let up = right.cross(vhat);
    let w_perp = w - vhat * w.dot(vhat);
    let launch = LaunchConditions {
        ball_speed_mps: vl,
        launch_rad: math::asin(vhat.z.clamp(-1.0, 1.0)),
        direction_rad: aim_rad + atan2(vhat.y, vhat.x),
        spin_radps: w_perp.len(),
        spin_axis_rad: club.axis_gain * atan2(-w_perp.dot(up), w_perp.dot(right)),
    };
    Impact { launch, spin_loft_rad: spin_loft, smash: vl / v_club.max(1e-9), cor }
}
