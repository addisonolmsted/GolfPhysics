//! On the ground: sliding until the contact point stops slipping, then rolling on the height function with slope
//! gravity and rolling resistance, until rest, the cup, water, or a crest the ball leaves (docs/models/ground.md).
//! Semi-implicit Euler at 1 ms with the exact height and normal each step.

use golf_core::ball::{CUP_RADIUS_M, INERTIA_K, MASS_KG, RADIUS_M};
use golf_core::math::G_MPS2;
use golf_core::{Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::State;

use crate::materials::GroundMaterials;
use crate::record::{EventKind, Phase, Recorder};
use crate::terrain::Terrain;

pub const ROLL_DT_S: f64 = 1e-3;
/// the contact point counts as not slipping below this (m/s)
pub const SLIP_EPS_MPS: f64 = 0.02;
/// a rolling ball slower than this may come to rest (m/s)
pub const REST_SPEED_MPS: f64 = 0.01;
pub const MAX_ROLL_S: f64 = 60.0;

/// the hole: the pin's position, the ground height at the cup, its size, and an optional cap on the capture speed
#[derive(Clone, Copy, Debug)]
pub struct Cup {
    pub xy: Vec2,
    pub z: f64,
    pub radius_m: f64,
    /// a big game cup captures fast putts (3.9 m/s through the centre at 10 inches); a cap keeps pace mattering
    pub capture_cap_mps: Option<f64>,
}

impl Cup {
    /// the regulation 108 mm cup
    pub fn regulation(xy: Vec2, z: f64) -> Cup {
        Cup { xy, z, radius_m: CUP_RADIUS_M, capture_cap_mps: None }
    }

    /// the game's cup (D47): 10 inches across, capture speed capped at GAME_CAPTURE_CAP_MPS
    pub fn game(xy: Vec2, z: f64) -> Cup {
        Cup { xy, z, radius_m: golf_core::ball::GAME_CUP_RADIUS_M, capture_cap_mps: GAME_CAPTURE_CAP_MPS }
    }

    /// the fastest a ball crossing on a line b from the centre can drop
    pub fn capture_speed(&self, b: f64) -> f64 {
        let v = capture_speed_for(b, self.radius_m);
        self.capture_cap_mps.map_or(v, |c| v.min(c))
    }
}

/// the game cup's capture-speed cap (None: the physical limit for its size). To be set by the owner from the Ground
/// Lab (D47)
pub const GAME_CAPTURE_CAP_MPS: Option<f64> = None;

/// The fastest a ball can cross a cup of radius R and still drop: it must fall about its own radius while it crosses
/// the chord at offset b from the centre. For the regulation cup at b = 0 this is 1.64 m/s, close to Holmes'
/// measured 1.63 m/s.
pub fn capture_speed_for(b: f64, radius: f64) -> f64 {
    if b >= radius {
        return 0.0;
    }
    2.0 * (radius * radius - b * b).sqrt() * (G_MPS2 / (2.0 * RADIUS_M)).sqrt()
}

/// the regulation cup's capture speed
pub fn capture_speed(b: f64) -> f64 {
    capture_speed_for(b, CUP_RADIUS_M)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollEnd {
    Rest,
    Holed,
    InWater,
    /// the ground fell away faster than gravity: back to flight
    Liftoff,
    OffWorld,
    Timeout,
}

/// distance from c to segment a-b
fn seg_dist(c: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let t = if l2 > 0.0 { ((c - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (c - (a + ab * t)).len()
}

pub fn roll<T: Terrain + ?Sized>(terrain: &T, mats: &GroundMaterials, s0: State, cup: Option<Cup>, rec: &mut Recorder) -> (RollEnd, State) {
    let inertia = INERTIA_K * MASS_KG * RADIUS_M * RADIUS_M;
    let dt = ROLL_DT_S;
    let mut s = s0;
    let mut sliding = true;
    let mut crossing_cup = false;
    rec.event(&s, EventKind::StartSlide);
    loop {
        if s.t - s0.t > MAX_ROLL_S {
            rec.event(&s, EventKind::Timeout);
            return (RollEnd::Timeout, s);
        }
        let Some(sf) = terrain.surface_at(s.p.x, s.p.y) else {
            rec.event(&s, EventKind::OffWorld);
            return (RollEnd::OffWorld, s);
        };
        let n = Vec3::new(-sf.dzdx, -sf.dzdy, 1.0).norm();
        let class = terrain.class_at(s.p.x, s.p.y);
        if class == SurfaceClass::WATER
            && let Some(wz) = terrain.water_at(s.p.xy())
            && wz > sf.z
        {
            s.p.z = wz;
            rec.event(&s, EventKind::Water);
            return (RollEnd::InWater, s);
        }
        let mat = mats.get(class);
        // stay on the surface, moving along it
        s.p.z = sf.z + RADIUS_M / n.z;
        s.v -= n * s.v.dot(n);
        let speed = s.v.len();
        rec.sample(s, if sliding { Phase::Slide } else { Phase::Roll });

        // A crest that drops away faster than gravity throws the ball back into the air. Following z = h(x) at
        // horizontal speed v_h needs a vertical acceleration h'' v_h²; the ground can only push, so the ball leaves
        // when h'' v_h² < -g. (The curvature and the slope's cosine cancel: v² κ > g cos θ is the same condition.)
        let vh = s.v.xy();
        let vhl = vh.len();
        if vhl > 0.2 {
            let d = vh / vhl;
            let hss = sf.dxx * d.x * d.x + 2.0 * sf.dxy * d.x * d.y + sf.dyy * d.y * d.y;
            if hss * vhl * vhl < -G_MPS2 {
                rec.event(&s, EventKind::Liftoff);
                return (RollEnd::Liftoff, s);
            }
        }

        let g_t = Vec3::new(0.0, 0.0, -G_MPS2) + n * (G_MPS2 * n.z);
        let rc = n * -RADIUS_M;
        let vc = s.v + s.w.cross(rc);
        let vct = vc - n * vc.dot(n);
        if sliding && vct.len() > SLIP_EPS_MPS {
            // kinetic friction against the slip, on the centre and as a torque on the spin
            let a_f = vct.norm() * -(mat.slide_mu * G_MPS2 * n.z);
            s.v += (g_t + a_f) * dt;
            s.w += rc.cross(a_f * MASS_KG) / inertia * dt;
            let vc2 = s.v + s.w.cross(rc);
            if (vc2 - n * vc2.dot(n)).dot(vct) <= 0.0 {
                sliding = false;
            }
        } else {
            if sliding {
                sliding = false;
                rec.event(&s, EventKind::StartRoll);
            }
            let pull = g_t / (1.0 + INERTIA_K);
            if speed < REST_SPEED_MPS {
                if pull.len() <= mat.static_decel() {
                    s.v = Vec3::ZERO;
                    s.w = Vec3::ZERO;
                    rec.sample(s, Phase::Rest);
                    rec.event(&s, EventKind::Rest);
                    return (RollEnd::Rest, s);
                }
                s.v += pull * dt;
            } else {
                let vhat = s.v / speed;
                let mut v2 = s.v + (pull - vhat * mat.decel(speed)) * dt;
                // rolling resistance stops a ball; it never pushes it backwards
                if v2.dot(vhat) < 0.0 && pull.len() <= mat.static_decel() {
                    v2 = Vec3::ZERO;
                }
                s.v = v2;
            }
            s.w = n.cross(s.v) / RADIUS_M;
        }

        let p0 = s.p;
        s.p.x += s.v.x * dt;
        s.p.y += s.v.y * dt;
        s.t += dt;

        // the cup: the centre's path crossing the hole. What decides capture is how far the line of travel passes
        // from the centre (b), not where the ball first meets the rim
        if let Some(c) = cup {
            let near = seg_dist(c.xy, p0.xy(), s.p.xy());
            if near < c.radius_m {
                if !crossing_cup {
                    crossing_cup = true;
                    let v = s.v.len();
                    let b = if v > 1e-9 { (s.v.xy() / s.v.xy().len()).cross(c.xy - p0.xy()).abs() } else { 0.0 };
                    if v < c.capture_speed(b) {
                        s.p = Vec3::new(c.xy.x, c.xy.y, c.z - 0.05);
                        s.v = Vec3::ZERO;
                        s.w = Vec3::ZERO;
                        rec.sample(s, Phase::Rest);
                        rec.event(&s, EventKind::Holed);
                        return (RollEnd::Holed, s);
                    }
                    // lip-out (placeholder, M3): turned away from the centre, more the nearer the edge, and slowed
                    let side = s.v.xy().cross(c.xy - s.p.xy()).signum();
                    let ang = -side * 0.6 * (b / c.radius_m);
                    let (ca, sa) = (golf_core::math::cos(ang), golf_core::math::sin(ang));
                    s.v = Vec3::new(s.v.x * ca - s.v.y * sa, s.v.x * sa + s.v.y * ca, s.v.z) * 0.8;
                    rec.event(&s, EventKind::LipOut);
                }
            } else if near > c.radius_m + 0.01 {
                crossing_cup = false;
            }
        }
    }
}
