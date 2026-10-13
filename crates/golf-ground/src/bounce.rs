//! One bounce (docs/models/ground.md, after Penner's model of a ball landing on turf):
//! 1. the effective normal leans back towards the incoming ball (the turf gives way under it);
//! 2. the normal speed comes back scaled by a restitution that falls with impact speed;
//! 3. the tangential impulse is whatever stops the contact point slipping (the ball leaves rolling), capped by
//!    Coulomb friction (then it slips); the same impulse changes the spin. Check and spin-back come out of this.

use golf_core::ball::{INERTIA_K, MASS_KG, RADIUS_M};
use golf_core::{Vec3, math};

use crate::materials::GroundMaterial;

#[derive(Clone, Copy, Debug)]
pub struct BounceOut {
    pub v: Vec3,
    pub w: Vec3,
    /// the effective normal used
    pub normal: Vec3,
    pub cor: f64,
    pub plugged: bool,
    /// true when the contact point stopped slipping (grip), false when it slid (Coulomb)
    pub gripped: bool,
}

pub fn bounce(v: Vec3, w: Vec3, n: Vec3, mat: &GroundMaterial) -> BounceOut {
    let speed = v.len();
    let vn = v.dot(n);
    if vn >= 0.0 || speed == 0.0 {
        return BounceOut { v, w, normal: n, cor: 0.0, plugged: false, gripped: false };
    }
    // the turf tilt
    let landing = math::asin((-vn / speed).min(1.0));
    let vt = v - n * vn;
    // The tilt models the crater's far wall pushing back. It can never lean the normal past the ball's own incoming
    // direction (the angle between n and -v is 90° - landing): a head-on hit into a steep face has no far wall, and
    // an uncapped tilt there pushed the ball down the face at 9 m/s (found by the Ground Lab's pitch into a face)
    let tilt = mat.tilt_rad(speed, landing).min(0.9 * (std::f64::consts::FRAC_PI_2 - landing));
    let ne = if vt.len() > 1e-9 { (n * math::cos(tilt) - vt.norm() * math::sin(tilt)).norm() } else { n };
    let vne = v.dot(ne);
    if vne >= 0.0 {
        return BounceOut { v, w, normal: ne, cor: 0.0, plugged: false, gripped: false };
    }
    // plugging goes by how hard the ball meets the actual ground (before the turf's give), so only steep, fast
    // landings plug: a fried egg, not every iron into sand
    if let Some(p) = mat.plug_speed
        && -vn > p
    {
        return BounceOut { v: Vec3::ZERO, w: Vec3::ZERO, normal: ne, cor: 0.0, plugged: true, gripped: false };
    }
    let e = mat.cor(-vne);
    let jn = MASS_KG * (1.0 + e) * (-vne);
    // the contact point and its slip velocity
    let rc = ne * -RADIUS_M;
    let vc = v + w.cross(rc);
    let vct = vc - ne * vc.dot(ne);
    let inertia = INERTIA_K * MASS_KG * RADIUS_M * RADIUS_M;
    // the impulse that stops the slip: Δv_c = J (1/m + r²/I) = J (1 + 1/k) / m
    let j_roll = vct * (-MASS_KG / (1.0 + 1.0 / INERTIA_K));
    let (jt, gripped) = if j_roll.len() <= mat.bounce_mu * jn {
        (j_roll, true)
    } else {
        (vct.norm() * (-mat.bounce_mu * jn), false)
    };
    let v2 = v + (ne * jn + jt) / MASS_KG;
    let w2 = w + rc.cross(jt) / inertia;
    BounceOut { v: v2, w: w2, normal: ne, cor: e, plugged: false, gripped }
}
