//! The club table: `data/clubs/standard.v1.json`, compiled in. Static values per club (loft, lie, length, head
//! mass) and the fitted impact coefficients (docs/models/impact.md).

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClubKind {
    Wood,
    Hybrid,
    Iron,
    Wedge,
    Putter,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Club {
    pub id: String,
    pub name: String,
    pub kind: ClubKind,
    pub loft_deg: f64,
    pub lie_deg: f64,
    pub length_in: f64,
    pub head_g: f64,
    /// the face's restitution at a 40 m/s normal impact speed (fitted)
    pub cor_ref: f64,
    /// measured spin over the rigid rolling-off-the-face spin (fitted)
    pub spin_factor: f64,
    /// spin per degree of spin loft falls with the normal impact speed: spin_factor * (v_n / 40)^-spin_speed_exp
    #[serde(default)]
    pub spin_speed_exp: f64,
    /// the spin axis tilt as a share of the D-plane's: fitted so each club curves TrackMan's ~0.7 % of carry per
    /// degree of D-plane axis (the flight model's sideways lift is ~45 % strong for irons: docs/models/flight.md M2b)
    #[serde(default = "one")]
    pub axis_gain: f64,
    /// a tour player's dynamic loft minus the static loft (fitted from the PGA Tour launch angles)
    pub tour_lean_deg: f64,
    pub gear_h: f64,
    pub gear_v: f64,
    pub strike_loss_per_m: f64,
    pub launch_per_m_high: f64,
}

fn one() -> f64 {
    1.0
}

impl Club {
    pub fn head_kg(&self) -> f64 {
        self.head_g / 1000.0
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ClubTable {
    /// the restitution's change per m/s of normal impact speed (one value for every club)
    pub cor_slope: f64,
    /// how much of an uphill or downhill slope the swing follows (0: the club moves as on flat ground, 1: along the
    /// slope). The sidehill tilt always applies in full: the club soles on the ground.
    pub slope_follow: f64,
    pub clubs: Vec<Club>,
}

const BUILTIN: &str = include_str!("../../../data/clubs/standard.v1.json");

impl ClubTable {
    pub fn builtin() -> ClubTable {
        serde_json::from_str(BUILTIN).expect("data/clubs/standard.v1.json parses")
    }

    pub fn from_json(s: &str) -> Result<ClubTable, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    pub fn get(&self, id: &str) -> Option<&Club> {
        self.clubs.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Club> {
        self.clubs.iter_mut().find(|c| c.id == id)
    }
}
