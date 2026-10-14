//! What the ball sits on: per-class factors from `data/materials/lie.v1.json` (placeholders until feel tuning), and
//! the ground's slope at the ball.

use std::collections::BTreeMap;

use golf_core::Vec3;
use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct LieFactors {
    pub speed_factor: f64,
    pub spin_factor: f64,
    pub launch_add_deg: f64,
}

impl LieFactors {
    pub const CLEAN: LieFactors = LieFactors { speed_factor: 1.0, spin_factor: 1.0, launch_add_deg: 0.0 };
}

/// the lie of one shot: the surface's factors and the ground's unit normal at the ball
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lie {
    pub factors: LieFactors,
    pub normal: Vec3,
}

impl Lie {
    /// a perfect lie on flat ground (the tee, launch-monitor tables)
    pub const CLEAN: Lie = Lie { factors: LieFactors::CLEAN, normal: Vec3::Z };
}

#[derive(Deserialize)]
struct Entry {
    #[allow(dead_code)]
    name: String,
    #[serde(flatten)]
    f: LieFactors,
}

#[derive(Deserialize)]
struct File {
    classes: BTreeMap<String, Entry>,
}

pub struct LieTable {
    by_class: BTreeMap<u8, LieFactors>,
}

const BUILTIN: &str = include_str!("../../../data/materials/lie.v1.json");

impl LieTable {
    pub fn builtin() -> LieTable {
        let f: File = serde_json::from_str(BUILTIN).expect("data/materials/lie.v1.json parses");
        LieTable { by_class: f.classes.into_iter().filter_map(|(k, e)| k.parse().ok().map(|id| (id, e.f))).collect() }
    }

    /// the factors for a C0 class id; a clean lie for a class without an entry
    pub fn get(&self, class_id: u8) -> LieFactors {
        self.by_class.get(&class_id).copied().unwrap_or(LieFactors::CLEAN)
    }
}
