//! Water: the bodies in `water.json` (outline, surface height, penalty-area links) and the surround's 8 m water
//! surface. Heights under water are the bed; the surface is here.

use golf_core::Vec2;
use serde::Deserialize;

use crate::bundle::Files;
use crate::error::CourseError;
use crate::heights::{Tier, surround_tier};
use crate::holes::Pt;
use crate::polygon::Region;

#[derive(Deserialize)]
struct BodyJson {
    id: String,
    kind: String,
    ring: Vec<Pt>,
    surface_m: [f64; 2],
    /// bodies outside the property list `[null]` (fixtures v1); nulls are dropped
    #[serde(default)]
    penalty_areas: Vec<Option<String>>,
}

#[derive(Deserialize)]
struct WaterFile {
    water_version: u32,
    bodies: Vec<BodyJson>,
}

pub struct WaterBody {
    pub id: String,
    /// creek, pond or lake
    pub kind: String,
    /// [min, max] water surface over the outline; equal for ponds and lakes
    pub surface_m: [f64; 2],
    pub penalty_areas: Vec<String>,
    pub region: Region,
}

pub struct Water {
    pub version: u32,
    pub bodies: Vec<WaterBody>,
    /// the surround's water surface at 8 m (nodata = dry); None for a course with no water at all
    pub surface_8m: Option<Tier>,
}

/// what is at a wet point
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterHit {
    /// index into `Water::bodies`, if the point is inside a listed body
    pub body: Option<usize>,
    pub surface_m: f64,
}

impl Water {
    pub fn load(f: &Files) -> Result<Water, CourseError> {
        let w: WaterFile = f.json("water.json")?;
        let bodies = w
            .bodies
            .into_iter()
            .map(|b| WaterBody { id: b.id, kind: b.kind, surface_m: b.surface_m, penalty_areas: b.penalty_areas.into_iter().flatten().collect(), region: Region::new(&[b.ring], 4.0) })
            .collect();
        Ok(Water { version: w.water_version, bodies, surface_8m: surround_tier(f, "surround/water_8m")? })
    }

    /// The water body containing p, with its surface there. A body with a flat surface gives that height; a sloping
    /// one (a creek) takes the 8 m surface there, clamped to its range.
    pub fn body_at(&self, p: Vec2) -> Option<WaterHit> {
        let (k, b) = self.bodies.iter().enumerate().find(|(_, b)| b.region.contains(p))?;
        let [lo, hi] = b.surface_m;
        let z = if lo == hi { lo } else { self.surface_8m_at(p).unwrap_or(0.5 * (lo + hi)).clamp(lo, hi) };
        Some(WaterHit { body: Some(k), surface_m: z })
    }

    /// the surround's 8 m water surface at the nearest node (None where dry)
    pub fn surface_8m_at(&self, p: Vec2) -> Option<f64> {
        self.surface_8m.as_ref().and_then(|t| t.nearest(p.x, p.y))
    }
}
