//! The calibration reference (`data/calibration/reference.v1.json`) and flying its rows through the model.

use std::path::{Path, PathBuf};

use golf_core::{Air, LaunchConditions, Vec2};
use golf_flight::{AeroParams, Flat, Flight, Model, Wind};
use serde::Deserialize;

pub const M_PER_YD: f64 = 0.9144;

#[derive(Deserialize, Clone, Debug)]
pub struct FitRow {
    pub id: String,
    pub group: String,
    pub club: String,
    pub club_mph: f64,
    pub ball_mph: f64,
    pub launch_deg: f64,
    pub spin_rpm: f64,
    pub apex_yd: Option<f64>,
    pub land_deg: Option<f64>,
    pub carry_yd: f64,
    /// false, or a string naming the verified fields ("carry", "carry, land")
    pub verified: serde_json::Value,
}

impl FitRow {
    pub fn is_verified(&self, field: &str) -> bool {
        match &self.verified {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => s.split(',').any(|f| f.trim() == field),
            _ => false,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SniffRow {
    pub id: String,
    pub group: String,
    pub club: String,
    pub club_mph: f64,
    pub ball_mph: f64,
    pub launch_deg: f64,
    pub spin_rpm: f64,
    pub carry_band_yd: [f64; 2],
    pub basis: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Reference {
    pub fit_rows: Vec<FitRow>,
    pub sniff_rows: Vec<SniffRow>,
}

/// the repo root (this crate's manifest dir, two up), so the tools find data/ from anywhere
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn load_reference() -> Result<Reference, String> {
    let p = repo_root().join("data/calibration/reference.v1.json");
    serde_json::from_str(&std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?).map_err(|e| e.to_string())
}

/// fly a launch-monitor row on flat ground in TrackMan's normalised air, no wind
pub fn fly(aero: &AeroParams, ball_mph: f64, launch_deg: f64, spin_rpm: f64, axis_deg: f64, wind: Wind) -> Flight {
    let ground = Flat(0.0);
    let m = Model::new(*aero, Air::trackman_normal(), wind, &ground);
    m.fly(Vec2::ZERO, &LaunchConditions::from_monitor(ball_mph, launch_deg, 0.0, spin_rpm, axis_deg))
}

/// carry (yd), apex (yd), landing angle (deg)
pub fn measure(f: &Flight) -> (f64, f64, f64) {
    (f.carry_m() / M_PER_YD, f.apex_m() / M_PER_YD, f.land_angle_rad().to_degrees())
}
