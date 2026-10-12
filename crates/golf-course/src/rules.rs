//! `rules.json` as geometry: the OB rings and the penalty areas. The engine reports facts; the game applies the
//! rules (D17).

use golf_core::Vec2;
use serde::Deserialize;

use crate::bundle::Files;
use crate::error::CourseError;
use crate::holes::Pt;
use crate::polygon::Region;

const CELL_M: f64 = 8.0;

#[derive(Deserialize)]
struct ObJson {
    id: String,
    /// what the inside of the ring is: "in_bounds" (the property) or "out_of_bounds" (e.g. the clubhouse)
    inside: String,
    ring: Vec<Pt>,
    source: String,
}

#[derive(Deserialize)]
struct PaJson {
    id: String,
    colour: String,
    rings: Vec<Vec<Pt>>,
    fill: String,
    #[serde(default)]
    holes: Vec<u8>,
}

#[derive(Deserialize)]
struct RulesFile {
    rules_version: u32,
    out_of_bounds: Vec<ObJson>,
    penalty_areas: Vec<PaJson>,
}

pub struct ObArea {
    pub id: String,
    pub source: String,
    /// true: inside the ring is in bounds (the property); false: inside is out of bounds
    pub inside_is_in_bounds: bool,
    pub region: Region,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenaltyColour {
    Red,
    Yellow,
}

pub struct PenaltyArea {
    pub id: String,
    pub colour: PenaltyColour,
    pub holes: Vec<u8>,
    pub region: Region,
}

pub struct Rules {
    pub version: u32,
    pub ob: Vec<ObArea>,
    pub penalty: Vec<PenaltyArea>,
}

impl Rules {
    pub fn load(f: &Files) -> Result<Rules, CourseError> {
        let r: RulesFile = f.json("rules.json")?;
        let mut ob = Vec::new();
        for o in r.out_of_bounds {
            let inside_is_in_bounds = match o.inside.as_str() {
                "in_bounds" => true,
                "out_of_bounds" => false,
                x => return Err(CourseError::Version { what: format!("rules.json OB '{}' inside", o.id), found: x.into() }),
            };
            ob.push(ObArea { id: o.id, source: o.source, inside_is_in_bounds, region: Region::new(&[o.ring], CELL_M) });
        }
        let mut penalty = Vec::new();
        for p in r.penalty_areas {
            if p.fill != "even_odd" {
                return Err(CourseError::Version { what: format!("penalty area '{}' fill", p.id), found: p.fill });
            }
            let colour = match p.colour.as_str() {
                "red" => PenaltyColour::Red,
                "yellow" => PenaltyColour::Yellow,
                x => return Err(CourseError::Version { what: format!("penalty area '{}' colour", p.id), found: x.into() }),
            };
            penalty.push(PenaltyArea { id: p.id, colour, holes: p.holes, region: Region::new(&p.rings, CELL_M) });
        }
        if !ob.iter().any(|o| o.inside_is_in_bounds) {
            return Err(CourseError::Missing { what: "rules.json: an in-bounds (property) OB ring".into() });
        }
        Ok(Rules { version: r.rules_version, ob, penalty })
    }

    /// C0's rule: a ball is out of bounds only when the **whole ball** is out. With the centre `c` and radius `r`:
    /// outside an in-bounds ring and more than `r` from it, or inside an out-of-bounds ring and more than `r` from it.
    pub fn in_bounds(&self, c: Vec2, r: f64) -> bool {
        !self.ob.iter().any(|o| o.region.contains(c) != o.inside_is_in_bounds && !o.region.near_boundary(c, r))
    }

    /// the penalty area the ball is in: any part of the ball in it or touching it (the Rules of Golf)
    pub fn penalty_area(&self, c: Vec2, r: f64) -> Option<&PenaltyArea> {
        self.penalty.iter().find(|p| p.region.contains(c) || p.region.near_boundary(c, r))
    }
}
