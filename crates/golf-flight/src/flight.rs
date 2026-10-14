//! The integrator: classic RK4 at a fixed 10 ms, spin as a vector, ground contact refined by a fixed-count
//! bisection (deterministic: docs/01-conventions.md).

use golf_core::ball::{CONTACT_RADIUS_M, MASS_KG, RADIUS_M};
use golf_core::math::{self, G_MPS2};
use golf_core::{Air, LaunchConditions, Vec3};

use crate::aero::{AeroParams, Prepared};
use crate::ground::{Ground, Surface};
use crate::hole::Hole;
use crate::wind::Wind;

/// The fixed step. RK4 with the wind taken at each stage's own height is 4th order: against a 0.25 ms reference
/// the landing point moves under 0.03 mm at 10 ms on every test shot (golf-lab/examples/dt_study.rs, 2026-10-12),
/// and a driver flight costs ~650 steps instead of ~6,500.
pub const DT_S: f64 = 0.010;
/// samples for drawing: the first step past each 1/30 s (P2)
pub const SAMPLE_HZ: f64 = 30.0;
const MAX_T_S: f64 = 20.0;
/// halvings of a 10 ms step: 0.6 µs, under 0.1 mm at any ball speed
const BISECT_ITERS: u32 = 24;
/// contact is tested from this long after the start at the latest, s
const AIRBORNE_AFTER_S: f64 = 0.05;
/// the steepest ground the contact check assumes between queries (rise over run)
const MAX_GROUND_SLOPE: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub t: f64,
    /// the ball's centre
    pub p: Vec3,
    pub v: Vec3,
    pub w: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    /// the ball touched the ground or water: hand over to golf-ground
    Contact { surface: Surface, ground_z: f64 },
    /// it hit the flagstick: the state is at the contact, `normal` the pole's outward normal there
    Flagstick { normal: Vec3 },
    /// it flew past every height tier
    OffWorld,
    Timeout,
}

#[derive(Clone, Debug)]
pub struct Flight {
    pub samples: Vec<State>,
    pub apex: State,
    pub last: State,
    pub end: End,
    pub launch: State,
}

impl Flight {
    /// horizontal distance from launch to the last state
    pub fn carry_m(&self) -> f64 {
        (self.last.p.xy() - self.launch.p.xy()).len()
    }
    /// apex height above the launch point (ball centre)
    pub fn apex_m(&self) -> f64 {
        self.apex.p.z - self.launch.p.z
    }
    /// descent angle below horizontal at the last state
    pub fn land_angle_rad(&self) -> f64 {
        math::atan2(-self.last.v.z, self.last.v.xy().len())
    }
    /// offset of the last state from the start line, + = right
    pub fn lateral_m(&self) -> f64 {
        let d = self.launch.v.xy().norm();
        let off = self.last.p.xy() - self.launch.p.xy();
        -d.cross(off)
    }
    pub fn time_s(&self) -> f64 {
        self.last.t - self.launch.t
    }
}

pub struct Model<'a> {
    pub aero: AeroParams,
    pub air: Air,
    pub wind: Wind,
    pub ground: &'a dyn Ground,
    /// the fixed step (DT_S unless set with `with_dt`)
    pub dt_s: f64,
    prepared: Prepared,
    /// ½ ρ A / m
    k: f64,
    /// the wind's horizontal unit vector times its 10 m speed, and the profile's 1 / ln(10 / z0)
    wind_10m: Vec3,
    wind_inv_ln: f64,
}

impl<'a> Model<'a> {
    pub fn new(aero: AeroParams, air: Air, wind: Wind, ground: &'a dyn Ground) -> Model<'a> {
        let k = 0.5 * air.density_kgpm3 * std::f64::consts::PI * RADIUS_M * RADIUS_M / MASS_KG;
        let wind_10m = golf_core::Vec2::from_azimuth(wind.downwind_rad).extend(0.0) * wind.speed_10m_mps;
        let wind_inv_ln = 1.0 / math::ln(10.0 / wind.z0_m);
        Model { aero, air, wind, ground, dt_s: DT_S, prepared: aero.prepare(), k, wind_10m, wind_inv_ln }
    }

    pub fn with_dt(self, dt_s: f64) -> Model<'a> {
        Model { dt_s, ..self }
    }

    /// the wind at a height above the ground (the same log profile as `Wind::at`, with its constants precomputed)
    #[inline]
    fn wind_at(&self, z_agl: f64) -> Vec3 {
        if self.wind.speed_10m_mps == 0.0 {
            return Vec3::ZERO;
        }
        self.wind_10m * (math::ln(z_agl.max(crate::wind::Z_MIN_M) / self.wind.z0_m) * self.wind_inv_ln)
    }

    /// accelerations (dv/dt, dω/dt) at a state; `z_agl` is the state's height above the ground for the wind
    #[inline]
    fn deriv(&self, v: Vec3, w: Vec3, z_agl: f64) -> (Vec3, Vec3) {
        let u = v - self.wind_at(z_agl);
        let speed = u.len();
        let mut a = Vec3::new(0.0, 0.0, -G_MPS2);
        if speed < 1e-9 {
            return (a, Vec3::ZERO);
        }
        let uh = u / speed;
        let w_perp = w - uh * w.dot(uh);
        let s = RADIUS_M * w_perp.len() / speed;
        let (cd, cl) = self.prepared.coefficients(s, speed);
        let k = self.k;
        a -= u * (k * cd * speed);
        let lift_dir = w_perp.cross(u).norm();
        a += lift_dir * (k * cl * speed * speed);
        (a, w * (-(self.aero.spin_decay_per_s + self.aero.spin_decay_per_m * speed)))
    }

    /// one classic RK4 step; `z_agl` is the height above the ground at the start (the ground height is taken as
    /// constant across the step, the stages' own heights set the wind)
    #[inline]
    fn rk4(&self, s: &State, dt: f64, z_agl: f64) -> State {
        let h = 0.5 * dt;
        let (a1, b1) = self.deriv(s.v, s.w, z_agl);
        let (v2, w2) = (s.v + a1 * h, s.w + b1 * h);
        let (a2, b2) = self.deriv(v2, w2, z_agl + s.v.z * h);
        let (v3, w3) = (s.v + a2 * h, s.w + b2 * h);
        let (a3, b3) = self.deriv(v3, w3, z_agl + v2.z * h);
        let (v4, w4) = (s.v + a3 * dt, s.w + b3 * dt);
        let (a4, b4) = self.deriv(v4, w4, z_agl + v3.z * dt);
        State {
            t: s.t + dt,
            p: s.p + (s.v + v2 * 2.0 + v3 * 2.0 + v4) * (dt / 6.0),
            v: s.v + (a1 + a2 * 2.0 + a3 * 2.0 + a4) * (dt / 6.0),
            w: s.w + (b1 + b2 * 2.0 + b3 * 2.0 + b4) * (dt / 6.0),
        }
    }

    /// Fly from a ball resting on the ground at `start` (x, y; z is taken from the ground) until it touches down.
    pub fn fly(&self, start: golf_core::Vec2, launch: &LaunchConditions) -> Flight {
        self.fly_at(start, launch, None)
    }

    /// `fly` with the hole in play: its opening (contact over it is `Surface::Hole`) and the flagstick if in
    pub fn fly_at(&self, start: golf_core::Vec2, launch: &LaunchConditions, hole: Option<&Hole>) -> Flight {
        let z0 = self.ground.contact(start.x, start.y).map_or(0.0, |g| g.0);
        let s0 = State { t: 0.0, p: start.extend(z0 + CONTACT_RADIUS_M), v: launch.velocity(), w: launch.spin() };
        self.fly_from_at(s0, hole)
    }

    /// Fly from any state (a launch, or a bounce hop handed back by golf-ground).
    pub fn fly_from(&self, s0: State) -> Flight {
        self.fly_from_at(s0, None)
    }

    /// the ground under (x, y), or the cup's opening at rim height when the ball is within reach of it
    #[inline]
    fn contact_at(&self, x: f64, y: f64, hole: Option<&Hole>) -> Option<(f64, Surface)> {
        if let Some(h) = hole
            && h.over(golf_core::Vec2::new(x, y))
        {
            return Some((h.z, Surface::Hole));
        }
        self.ground.contact(x, y)
    }

    /// `fly_from` with the hole in play
    pub fn fly_from_at(&self, s0: State, hole: Option<&Hole>) -> Flight {
        let mut s = s0;
        let mut apex = s0;
        let mut samples = vec![s0];
        let mut gz = self.contact_at(s.p.x, s.p.y, hole).map_or(s.p.z - CONTACT_RADIUS_M, |g| g.0);
        let mut next_check = 0.0;
        // leave the ground before testing for contact (the launch starts touching it): 1 cm of clearance or 50 ms,
        // whichever comes first. A hop off a slope can start a hair below the surface and never clear 1 cm; without
        // the time limit it would fall through the ground (found by the M3 fuzz)
        let mut airborne = false;
        while s.t < MAX_T_S {
            let n = self.rk4(&s, self.dt_s, s.p.z - gz);
            if n.p.z > apex.p.z {
                apex = n;
            }
            if let Some(h) = hole
                && let Some((frac, normal)) = h.pin_hit(s.p, n.p)
            {
                let c = self.rk4(&s, self.dt_s * frac, s.p.z - gz);
                samples.push(c);
                return Flight { samples, apex, last: c, end: End::Flagstick { normal }, launch: s0 };
            }
            // query the ground only when it could be within reach
            if n.t >= next_check || !airborne {
                match self.contact_at(n.p.x, n.p.y, hole) {
                    None => return Flight { samples, apex, last: n, end: End::OffWorld, launch: s0 },
                    Some((g, surf)) => {
                        gz = g;
                        let clear = n.p.z - CONTACT_RADIUS_M - g;
                        if clear > 0.01 || n.t - s0.t > AIRBORNE_AFTER_S {
                            airborne = true;
                        }
                        // in contact: or, before that, more than 2 cm into the ground
                        if (airborne && clear <= 0.0) || clear < -0.02 {
                            let (c, g, surf) = self.bisect(&s, g, surf, hole);
                            samples.push(c);
                            return Flight { samples, apex, last: c, end: End::Contact { surface: surf, ground_z: g }, launch: s0 };
                        }
                        let closing = n.v.xy().len() * MAX_GROUND_SLOPE + (-n.v.z).max(0.0) + 1.0;
                        next_check = n.t + (clear.max(0.0) / closing).min(0.25);
                    }
                }
            }
            s = n;
            if (s.t * SAMPLE_HZ).floor() > ((s.t - self.dt_s) * SAMPLE_HZ).floor() {
                samples.push(s);
            }
        }
        Flight { samples, apex, last: s, end: End::Timeout, launch: s0 }
    }

    /// the contact inside the step that starts at `s`: a fixed number of halvings of the step length
    fn bisect(&self, s: &State, mut g: f64, mut surf: Surface, hole: Option<&Hole>) -> (State, f64, Surface) {
        let (mut lo, mut hi) = (0.0, self.dt_s);
        let z_agl = s.p.z - g;
        for _ in 0..BISECT_ITERS {
            let mid = 0.5 * (lo + hi);
            let m = self.rk4(s, mid, z_agl);
            let (gm, sm) = self.contact_at(m.p.x, m.p.y, hole).unwrap_or((g, surf));
            if m.p.z - CONTACT_RADIUS_M - gm > 0.0 {
                lo = mid;
            } else {
                hi = mid;
                g = gm;
                surf = sm;
            }
        }
        (self.rk4(s, hi, z_agl), g, surf)
    }
}
