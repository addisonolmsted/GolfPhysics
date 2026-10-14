//! The engine behind a few exported functions, for the lab pages (Impact Lab). One call plays a whole shot on a flat
//! fairway: delivery -> impact -> flight -> bounces and roll. Results go into a buffer the page reads from linear
//! memory: no bindings generator, no imports.

use std::sync::Mutex;

use golf_core::{Air, ClubDelivery, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, GroundMaterials, Phase, TestPlane};
use golf_impact::{ClubTable, Lie, LieTable, impact};

const YD: f64 = 0.9144;
const MPH: f64 = 0.44704;

/// the last shot: 16 summary values, then 5 per sample (x yd, right yd, height yd, t s, phase)
static OUT: Mutex<Vec<f64>> = Mutex::new(Vec::new());

/// the number of clubs in the table (club indices follow its order)
#[unsafe(no_mangle)]
pub extern "C" fn club_count() -> u32 {
    ClubTable::builtin().clubs.len() as u32
}

/// Play a shot. Angles in degrees, + = right (face open, path in-to-out), attack + = up. Strike in mm (x towards
/// the toe, y up). `lie_class` a C0 class id; `uphill` and `sidehill` slopes in % (sidehill + = ball above the
/// feet). Wind: mph at 10 m, `wind_from` the direction it blows from, degrees clockwise from straight at the
/// player (0 = into the face, 180 = from behind). Returns the number of floats written; read them at `out_ptr()`.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn shot(club: u32, mph: f64, attack: f64, loft: f64, face: f64, path: f64, strike_x_mm: f64, strike_y_mm: f64, lie_class: u32, uphill: f64, sidehill: f64, wind_mph: f64, wind_from: f64) -> u32 {
    let table = ClubTable::builtin();
    let Some(c) = table.clubs.get(club as usize) else { return 0 };
    let d = ClubDelivery {
        head_speed_mps: mph * MPH,
        path_rad: path.to_radians(),
        attack_rad: attack.to_radians(),
        face_rad: face.to_radians(),
        dynamic_loft_rad: loft.to_radians(),
        strike_m: Vec2::new(strike_x_mm / 1000.0, strike_y_mm / 1000.0),
    };
    let normal = Vec3::new(-uphill / 100.0, sidehill / 100.0, 1.0).norm();
    let lie = Lie { factors: LieTable::builtin().get(lie_class as u8), normal };
    let i = impact(&table, c, &d, 0.0, &lie);
    let l = i.launch;

    // the player aims along +x and right is -y: a wind from `wind_from` degrees clockwise of the target blows
    // towards the world azimuth 180° - wind_from (0 = into the face, blowing along -x; 180 = from behind, along +x)
    let wind = Wind { downwind_rad: (180.0 - wind_from).to_radians(), speed_10m_mps: wind_mph * MPH, ..Wind::CALM };
    let mats = GroundMaterials::builtin();
    let plane = TestPlane::flat(SurfaceClass::FAIRWAY);
    let m = BallModel::new(&plane, &mats, AeroParams::builtin(), Air::trackman_normal(), wind, None);
    let o = m.play(Vec2::ZERO, &l);

    let first = o.first_contact.unwrap_or(o.rest);
    let apex = o.samples.iter().map(|s| s.p.z).fold(0.0, f64::max) - o.start.z;
    let land = o.samples.iter().take_while(|s| s.phase == Phase::Air).last();
    let land_deg = land.map_or(0.0, |s| (-s.v.z).atan2(s.v.xy().len()).to_degrees());
    let hang = o.events.iter().find(|e| matches!(e.kind, golf_ground::EventKind::Land { .. })).map_or(0.0, |e| e.t);
    let mut out = vec![
        l.ball_speed_mps / MPH,
        l.launch_rad.to_degrees(),
        -l.direction_rad.to_degrees(),
        l.spin_radps * 30.0 / std::f64::consts::PI,
        l.spin_axis_rad.to_degrees(),
        i.smash,
        i.spin_loft_rad.to_degrees(),
        i.cor,
        first.x / YD,
        -first.y / YD,
        apex / YD,
        land_deg,
        hang,
        o.rest.x / YD,
        -o.rest.y / YD,
        0.0,
    ];
    for s in &o.samples {
        let ph = match s.phase {
            Phase::Air => 0.0,
            Phase::Slide => 1.0,
            Phase::Roll => 2.0,
            Phase::Rest => 3.0,
        };
        out.extend_from_slice(&[s.p.x / YD, -s.p.y / YD, (s.p.z - o.start.z) / YD, s.t, ph]);
    }
    out[15] = o.samples.len() as f64;
    let n = out.len() as u32;
    *OUT.lock().unwrap() = out;
    n
}

/// where the last shot's floats are (valid until the next call)
#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> *const f64 {
    OUT.lock().unwrap().as_ptr()
}
