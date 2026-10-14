//! The hole as geometry: a ball meeting the rim (a circle at the ground), the liner wall, the bottom, and the green
//! around it. Holed, lip-outs, horseshoes, edge-ins and back-wall pop-outs fall out of the contacts; nothing is
//! scripted (docs/models/ground.md#the-cup).
//!
//! Runs in the cup's local frame: the green near the hole is a plane through the cup centre with the terrain's
//! normal there (the hole and the ball span ~25 cm). Contacts are stiff springs with damping set by a restitution,
//! Coulomb friction regularised near zero slip, and the green's rolling resistance on turf contacts. Fixed 20 µs
//! steps, only while the ball is at the hole.

use golf_core::ball::{INERTIA_K, MASS_KG, RADIUS_M};
use golf_core::math::{G_MPS2, ln};
use golf_core::{Vec2, Vec3};
use golf_flight::State;

use crate::record::{EventKind, Phase, Recorder};

pub const CUP_DT_S: f64 = 2e-5;
/// the liner's depth (Rules: at least 4 inches)
pub const CUP_DEPTH_M: f64 = 0.1016;
/// contact stiffness, N/m: a ~3 ms contact, like a ball on firm turf or a plastic liner
const STIFFNESS: f64 = 5e4;
/// restitution of the turf edge, the liner wall and the bottom
const E_EDGE: f64 = 0.3;
const E_WALL: f64 = 0.45;
const E_BOTTOM: f64 = 0.3;
/// Coulomb friction on turf (edge, green) and on the liner
const MU_TURF: f64 = 0.4;
const MU_LINER: f64 = 0.25;
/// friction is viscous below this slip, so a rolling ball doesn't chatter (m/s)
const SLIP_REG_MPS: f64 = 0.01;
/// a ball that stays at the hole this long (resting on the lip) is handed back to the roll
const MAX_CUP_S: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CupEnd {
    Holed,
    /// back on the green, rolling (or at rest on the lip)
    Rolling,
    /// left the hole in the air
    Airborne,
}

/// damping coefficient for a restitution e on the linear spring-dashpot
fn damping(e: f64) -> f64 {
    let l = ln(e);
    let zeta = -l / (std::f64::consts::PI * std::f64::consts::PI + l * l).sqrt();
    2.0 * zeta * (STIFFNESS * MASS_KG).sqrt()
}

struct Frame {
    e1: Vec3,
    e2: Vec3,
    n: Vec3,
}

impl Frame {
    fn to_local(&self, v: Vec3) -> Vec3 {
        Vec3::new(v.dot(self.e1), v.dot(self.e2), v.dot(self.n))
    }
    fn to_world(&self, v: Vec3) -> Vec3 {
        self.e1 * v.x + self.e2 * v.y + self.n * v.z
    }
}

/// One contact's force on the ball and torque about its centre. `nrm` points from the contact into the ball; `pen`
/// is the overlap (> 0 in contact).
#[allow(clippy::too_many_arguments)]
fn contact(nrm: Vec3, pen: f64, v: Vec3, w: Vec3, e: f64, mu: f64, rr_decel: f64, f: &mut Vec3, tq: &mut Vec3) -> f64 {
    let rc = nrm * -RADIUS_M;
    let vc = v + w.cross(rc);
    let vn = vc.dot(nrm);
    let fn_ = (STIFFNESS * pen - damping(e) * vn).max(0.0);
    if fn_ == 0.0 {
        return 0.0;
    }
    let ut = vc - nrm * vn;
    let ul = ut.len();
    let ft = ut * -(mu * fn_ / ul.max(SLIP_REG_MPS));
    // rolling resistance, as a share of the normal load (the green's decel at a 1 g load)
    let vt = v - nrm * v.dot(nrm);
    let vtl = vt.len();
    let frr = if vtl > 1e-4 { vt * -(rr_decel / G_MPS2 * fn_ / vtl) } else { Vec3::ZERO };
    *f += nrm * fn_ + ft + frr;
    *tq += rc.cross(ft);
    fn_
}

/// Simulate the ball at the hole from `s0` (world frame) until it is holed or leaves. `n` is the terrain normal at
/// the cup centre `o` (on the ground), `radius` the rim's radius, `rr_decel` the green's rolling resistance, `pull`
/// the share of gravity acting while the ball is over the hole (the game cup's speed cap, `Cup::hole_pull`).
pub fn at_the_cup(o: Vec3, n: Vec3, radius: f64, rr_decel: f64, pull: f64, s0: State, rec: &mut Recorder) -> (CupEnd, State) {
    let x = if n.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let e1 = (x - n * x.dot(n)).norm();
    let fr = Frame { e1, e2: n.cross(e1), n };
    let inertia = INERTIA_K * MASS_KG * RADIUS_M * RADIUS_M;
    let g = fr.to_local(Vec3::new(0.0, 0.0, -G_MPS2));
    let mut c = fr.to_local(s0.p - o);
    // start resting on the green at the spring's equilibrium
    if c.z <= RADIUS_M + 1e-3 {
        c.z = RADIUS_M - MASS_KG * -g.z / STIFFNESS;
    }
    let mut v = fr.to_local(s0.v);
    v.z = v.z.max(0.0);
    let mut w = fr.to_local(s0.w);
    let mut t = s0.t;
    let world = |c: Vec3, v: Vec3, w: Vec3, t: f64| State { t, p: o + fr.to_world(c), v: fr.to_world(v), w: fr.to_world(w) };
    let mut touched_rim = false;
    let mut lip_logged = false;
    let mut steps: u64 = 0;
    loop {
        let rho = Vec2::new(c.x, c.y).len();
        let dir = if rho > 1e-12 { Vec3::new(c.x / rho, c.y / rho, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
        // holed: the whole ball below the rim
        if c.z < -RADIUS_M {
            let s = world(c, v, w, t);
            rec.sample(s, Phase::Roll);
            let rest = State { t, p: o + fr.to_world(Vec3::new(0.0, 0.0, -CUP_DEPTH_M + RADIUS_M)), v: Vec3::ZERO, w: Vec3::ZERO };
            rec.event(&rest, EventKind::Holed);
            return (CupEnd::Holed, rest);
        }
        // left the hole: past the rim by more than the ball's radius
        if rho > radius + RADIUS_M + 0.005 || t - s0.t > MAX_CUP_S {
            let s = world(c, v, w, t);
            if touched_rim && !lip_logged {
                rec.event(&s, EventKind::LipOut);
            }
            let airborne = c.z > RADIUS_M + 0.003 && v.z > 0.0;
            return (if airborne { CupEnd::Airborne } else { CupEnd::Rolling }, s);
        }

        let mut f = g * (MASS_KG * if rho < radius { pull } else { 1.0 });
        let mut tq = Vec3::ZERO;
        if rho >= radius {
            // the green around the hole
            contact(Vec3::Z, RADIUS_M - c.z, v, w, E_EDGE, MU_TURF, rr_decel, &mut f, &mut tq);
        } else if c.z >= 0.0 {
            // the rim: the nearest point of the circle
            let q = dir * radius;
            let d = c - q;
            let dl = d.len();
            if dl < RADIUS_M && contact(d / dl, RADIUS_M - dl, v, w, E_EDGE, MU_TURF, rr_decel, &mut f, &mut tq) > 0.0 && rho < radius - 1e-3 {
                touched_rim = true;
            }
        } else {
            // the liner wall, and the rim edge from below
            let pen = rho + RADIUS_M - radius;
            if pen > 0.0 {
                contact(dir * -1.0, pen, v, w, E_WALL, MU_LINER, 0.0, &mut f, &mut tq);
            }
            let q = dir * radius;
            let d = c - q;
            let dl = d.len();
            if dl < RADIUS_M && d.z > 0.0 {
                contact(d / dl, RADIUS_M - dl, v, w, E_EDGE, MU_TURF, rr_decel, &mut f, &mut tq);
            }
            let pb = (-CUP_DEPTH_M + RADIUS_M) - c.z;
            if pb > 0.0 {
                contact(Vec3::Z, pb, v, w, E_BOTTOM, MU_LINER, 0.0, &mut f, &mut tq);
            }
        }
        v += f / MASS_KG * CUP_DT_S;
        w += tq / inertia * CUP_DT_S;
        c += v * CUP_DT_S;
        t += CUP_DT_S;
        steps += 1;
        if steps % 100 == 0 {
            rec.dense(world(c, v, w, t), Phase::Roll);
        }
        if touched_rim && !lip_logged && c.z > RADIUS_M + 0.002 && v.z > 0.0 {
            // thrown up off the far edge
            lip_logged = true;
            rec.event(&world(c, v, w, t), EventKind::LipOut);
        }
    }
}
