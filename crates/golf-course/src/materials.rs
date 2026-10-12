//! Per-class material parameters. Today: the bundle's `materials.json` defaults. From M3: our own
//! `data/materials/physics.v1.json` applied over them (D18).

use serde::Deserialize;

use crate::bundle::Files;
use crate::classes::SurfaceClass;
use crate::error::CourseError;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MaterialPhysics {
    pub restitution: f64,
    pub rolling_resistance: f64,
    pub sliding_friction: f64,
    pub spin_retention: f64,
    pub firmness: f64,
    pub lie_penalty: f64,
    /// green speed in feet (greens only)
    pub stimp: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Material {
    pub id: u8,
    pub name: String,
    pub physics: MaterialPhysics,
}

#[derive(Deserialize)]
struct MaterialsFile {
    materials_version: u32,
    classes: Vec<Material>,
}

/// materials indexed by class id; a class the file doesn't list falls back to class 0
pub struct Materials {
    pub version: u32,
    by_id: Vec<Option<Material>>,
}

impl Materials {
    pub fn load(f: &Files) -> Result<Materials, CourseError> {
        let m: MaterialsFile = f.json("materials.json")?;
        let mut by_id: Vec<Option<Material>> = vec![None; 256];
        for c in m.classes {
            let id = c.id as usize;
            by_id[id] = Some(c);
        }
        if by_id[0].is_none() {
            return Err(CourseError::Missing { what: "materials.json class 0".into() });
        }
        Ok(Materials { version: m.materials_version, by_id })
    }

    pub fn get(&self, c: SurfaceClass) -> &Material {
        self.by_id[c.0 as usize].as_ref().or(self.by_id[0].as_ref()).expect("class 0 checked at load")
    }

    pub fn iter(&self) -> impl Iterator<Item = &Material> {
        self.by_id.iter().flatten()
    }
}
