//! Per-class ground parameters: `data/materials/physics.v1.json` (D18), compiled in. The model each field feeds is
//! described in the file itself and in docs/models/ground.md.

use std::collections::BTreeMap;

use golf_course::SurfaceClass;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct GroundMaterial {
    pub name: String,
    pub cor_scale: f64,
    pub cor_min: f64,
    pub tilt_deg: f64,
    pub tilt_max_deg: f64,
    pub bounce_mu: f64,
    pub slide_mu: f64,
    #[serde(default)]
    pub roll_decel: f64,
    #[serde(default)]
    pub stimp_ft: Option<f64>,
    #[serde(default)]
    pub roll_c1: f64,
    pub static_factor: f64,
    #[serde(default)]
    pub plug_speed: Option<f64>,
}

/// the stimpmeter's release speed, m/s
pub const STIMP_V0_MPS: f64 = 1.83;

impl GroundMaterial {
    /// normal coefficient of restitution at a normal impact speed: Penner's green curve, scaled and clamped
    pub fn cor(&self, vn: f64) -> f64 {
        let v = vn.abs();
        let penner = if v <= 20.0 { 0.510 - 0.0375 * v + 0.000903 * v * v } else { 0.120 };
        (self.cor_scale * penner).clamp(self.cor_min, 0.8)
    }

    /// how far the effective normal leans back towards the incoming ball (radians)
    pub fn tilt_rad(&self, speed: f64, landing_rad: f64) -> f64 {
        (self.tilt_deg * (speed / 18.6) * (landing_rad.to_degrees() / 44.4)).clamp(0.0, self.tilt_max_deg).to_radians()
    }

    /// the constant part of the rolling deceleration (m/s^2): from stimp on greens
    pub fn base_decel(&self) -> f64 {
        match self.stimp_ft {
            Some(s) => STIMP_V0_MPS * STIMP_V0_MPS / (2.0 * s * 0.3048),
            None => self.roll_decel,
        }
    }

    /// the rolling deceleration at a speed (m/s^2)
    #[inline]
    pub fn decel(&self, speed: f64) -> f64 {
        self.base_decel() + self.roll_c1 * speed
    }

    /// the largest slope pull a ball at rest resists (m/s^2)
    pub fn static_decel(&self) -> f64 {
        self.static_factor * self.base_decel()
    }
}

#[derive(Deserialize)]
struct MaterialsFile {
    version: u32,
    classes: BTreeMap<String, GroundMaterial>,
}

pub struct GroundMaterials {
    pub version: u32,
    by_id: Vec<Option<GroundMaterial>>,
}

const BUILTIN: &str = include_str!("../../../data/materials/physics.v1.json");

impl GroundMaterials {
    pub fn builtin() -> GroundMaterials {
        GroundMaterials::from_json(BUILTIN).expect("data/materials/physics.v1.json parses")
    }

    pub fn from_json(s: &str) -> Result<GroundMaterials, String> {
        let f: MaterialsFile = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let mut by_id: Vec<Option<GroundMaterial>> = vec![None; 256];
        for (k, m) in f.classes {
            let id: usize = k.parse().map_err(|_| format!("class id {k}"))?;
            by_id[id] = Some(m);
        }
        if by_id[0].is_none() {
            return Err("materials: class 0 is required (the fallback)".into());
        }
        Ok(GroundMaterials { version: f.version, by_id })
    }

    /// replace one class's parameters (tuning tools)
    pub fn set(&mut self, c: SurfaceClass, m: GroundMaterial) {
        self.by_id[c.0 as usize] = Some(m);
    }

    #[inline]
    pub fn get(&self, c: SurfaceClass) -> &GroundMaterial {
        self.by_id[c.0 as usize].as_ref().or(self.by_id[0].as_ref()).expect("class 0 checked at load")
    }
}
