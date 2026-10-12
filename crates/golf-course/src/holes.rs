//! `holes.json`: tees, greens, pins and the line of play, in the C0 frame (`[x, y]` points).

use serde::Deserialize;

use crate::bundle::Files;
use crate::error::CourseError;

pub type Pt = [f64; 2];

#[derive(Deserialize, Debug, Clone)]
pub struct Tee {
    /// 0 = the longest tee position on the hole
    pub index: u8,
    /// the tee set's name: tips, back, middle_back, middle, middle_forward, forward, front
    pub set: String,
    /// which tee ground (pad) it sits on
    pub ground: String,
    pub plate: Pt,
    pub aim: Pt,
    pub yardage_m: f64,
    pub z: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Pin {
    pub xy: Pt,
    pub z: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Green {
    pub polygon: Vec<Pt>,
    pub pin: Pin,
    pub target: Pt,
    pub z_mean: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LandingZone {
    pub center: Pt,
    pub radius_m: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LineOfPlay {
    /// back tee, landing zones, green target
    pub spline: Vec<Pt>,
    #[serde(default)]
    pub landing_zones: Vec<LandingZone>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hole {
    pub number: u8,
    pub par: u8,
    pub length_m: f64,
    pub tees: Vec<Tee>,
    pub green: Green,
    pub line_of_play: LineOfPlay,
}

#[derive(Deserialize)]
struct HolesFile {
    holes: Vec<Hole>,
}

pub fn load(f: &Files) -> Result<Vec<Hole>, CourseError> {
    let mut h: HolesFile = f.json("holes.json")?;
    h.holes.sort_by_key(|h| h.number);
    Ok(h.holes)
}
