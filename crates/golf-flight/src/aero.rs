//! Drag, lift and spin decay (docs/models/flight.md). The coefficients live in `data/aero/ball.v<N>.json`, written
//! by `golf-lab fit-aero`.

use golf_core::math;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AeroParams {
    /// drag at zero spin
    pub cd0: f64,
    /// drag rise per unit spin factor
    pub cd_s: f64,
    /// drag rise per unit spin factor squared
    #[serde(default)]
    pub cd_s2: f64,
    /// drag change with air speed: + cd_v min(0, cd_v_ref / |u| - 1), so drag falls as the ball goes faster than
    /// `cd_v_ref` and is flat below it. Empirical. A Reynolds-number form cancelled half the altitude gain (M2), and
    /// a two-sided speed form raised drag on slow balls enough that a tailwind shortened a 70 mph drive (M2b).
    #[serde(default)]
    pub cd_v: f64,
    /// the air speed above which the speed term lowers drag, m/s
    #[serde(default = "default_v_ref")]
    pub cd_v_ref: f64,
    /// lift at saturation: Cl = cl_max S^p / (cl_s_half^p + S^p)
    pub cl_max: f64,
    /// the spin factor at half the saturated lift
    pub cl_s_half: f64,
    /// the lift curve's exponent: Cl grows as S^p at low spin
    pub cl_p: f64,
    /// spin decay: dω/dt = -(spin_decay_per_s + spin_decay_per_m |u|) ω
    pub spin_decay_per_m: f64,
    /// the part of the spin decay that doesn't depend on speed (M2b: keeps spin falling late in the flight)
    #[serde(default)]
    pub spin_decay_per_s: f64,
}

fn default_v_ref() -> f64 {
    36.5
}

#[derive(Deserialize)]
struct AeroFile {
    params: AeroParams,
}

/// the built-in coefficients: the repo's current `data/aero/ball.v1.json`
const BUILTIN: &str = include_str!("../../../data/aero/ball.v1.json");

impl AeroParams {
    pub fn builtin() -> AeroParams {
        serde_json::from_str::<AeroFile>(BUILTIN).expect("data/aero/ball.v1.json parses").params
    }

    pub fn from_json(s: &str) -> Result<AeroParams, String> {
        serde_json::from_str::<AeroFile>(s).map(|f| f.params).map_err(|e| e.to_string())
    }

    /// (Cd, Cl) at spin factor S and air speed |u|
    #[inline]
    pub fn coefficients(&self, s: f64, speed_mps: f64) -> (f64, f64) {
        self.prepare().coefficients(s, speed_mps)
    }

    /// the coefficients with their constant parts computed once (the integrator calls this four times a step)
    pub fn prepare(&self) -> Prepared {
        Prepared { p: *self, s_half_p: math::pow(self.cl_s_half, self.cl_p) }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Prepared {
    pub p: AeroParams,
    s_half_p: f64,
}

impl Prepared {
    #[inline]
    pub fn coefficients(&self, s: f64, speed_mps: f64) -> (f64, f64) {
        let p = &self.p;
        let cd = p.cd0 + (p.cd_s + p.cd_s2 * s) * s + p.cd_v * (p.cd_v_ref / speed_mps.max(5.0) - 1.0).min(0.0);
        // S^p as exp(p ln S): the same value as libm's pow to rounding, at a third of the cost
        let sp = if s > 0.0 { math::exp(p.cl_p * math::ln(s)) } else { 0.0 };
        (cd, p.cl_max * sp / (self.s_half_p + sp))
    }
}
