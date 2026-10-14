//! The M4 gates (docs/03-success-indicators.md §1, impact): the tour delivery reproduces TrackMan's PGA Tour rows,
//! the same coefficients carry to the LPGA and amateur rows, face and path give the D-plane's shapes, and strike,
//! lie and slope push the ball the right way.

use golf_core::{ClubDelivery, Vec2, Vec3};
use golf_impact::{ClubTable, Lie, LieFactors, LieTable, impact};
use serde_json::Value;

const MPH: f64 = 0.44704;

fn rows() -> Vec<Value> {
    let v: Value = serde_json::from_str(include_str!("../../../data/calibration/reference.v1.json")).unwrap();
    v["fit_rows"].as_array().unwrap().clone()
}

fn club_id(name: &str) -> &'static str {
    match name {
        "Driver" => "dr",
        "3 wood" => "3w",
        "5 wood" => "5w",
        "7 wood" => "7w",
        "Hybrid" => "3h",
        "3 iron" => "3i",
        "4 iron" => "4i",
        "5 iron" => "5i",
        "6 iron" => "6i",
        "7 iron" => "7i",
        "8 iron" => "8i",
        "9 iron" => "9i",
        "PW" => "pw",
        _ => panic!("no club for {name}"),
    }
}

/// (ball mph, launch deg, spin rpm)
fn launch(t: &ClubTable, id: &str, mph: f64, attack: f64, loft: f64) -> (f64, f64, f64) {
    let i = impact(t, t.get(id).unwrap(), &ClubDelivery::square(mph * MPH, attack, loft), 0.0, &Lie::CLEAN);
    (i.launch.ball_speed_mps / MPH, i.launch.launch_rad.to_degrees(), i.launch.spin_radps * 30.0 / std::f64::consts::PI)
}

#[test]
fn the_tour_delivery_reproduces_every_pga_tour_row() {
    let t = ClubTable::builtin();
    for r in rows().iter().filter(|r| r["group"] == "PGA Tour") {
        let id = club_id(r["club"].as_str().unwrap());
        let c = t.get(id).unwrap();
        let (b, l, s) = launch(&t, id, r["club_mph"].as_f64().unwrap(), r["attack_deg"].as_f64().unwrap(), c.loft_deg + c.tour_lean_deg);
        let (rb, rl, rs) = (r["ball_mph"].as_f64().unwrap(), r["launch_deg"].as_f64().unwrap(), r["spin_rpm"].as_f64().unwrap());
        assert!((b / rb - 1.0).abs() <= 0.02, "{id}: ball {b:.1} vs {rb}");
        assert!((l - rl).abs() <= 1.0, "{id}: launch {l:.1} vs {rl}");
        assert!((s / rs - 1.0).abs() <= 0.10, "{id}: spin {s:.0} vs {rs}");
    }
}

#[test]
fn the_same_coefficients_fit_the_slower_rows() {
    // each row at the dynamic loft that gives its launch (the lean is the player's): ball speed within 3 % (the
    // amateur driver 5 %: TrackMan's average includes off-centre strikes) and spin within 15 %
    let t = ClubTable::builtin();
    for r in rows().iter().filter(|r| r["group"] != "PGA Tour") {
        let id = club_id(r["club"].as_str().unwrap());
        let (mph, attack, rl) = (r["club_mph"].as_f64().unwrap(), r["attack_deg"].as_f64().unwrap(), r["launch_deg"].as_f64().unwrap());
        let (mut lo, mut hi) = (0.0, 70.0);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if launch(&t, id, mph, attack, mid).1 < rl { lo = mid } else { hi = mid }
        }
        let (b, _, s) = launch(&t, id, mph, attack, 0.5 * (lo + hi));
        let (rb, rs) = (r["ball_mph"].as_f64().unwrap(), r["spin_rpm"].as_f64().unwrap());
        let ball_tol = if r["id"] == "ama-dr" { 0.05 } else { 0.03 };
        assert!((b / rb - 1.0).abs() <= ball_tol, "{}: ball {b:.1} vs {rb}", r["id"]);
        assert!((s / rs - 1.0).abs() <= 0.15, "{}: spin {s:.0} vs {rs}", r["id"]);
    }
}

fn shot(id: &str, mph: f64, face: f64, path: f64) -> golf_core::LaunchConditions {
    let t = ClubTable::builtin();
    let c = t.get(id).unwrap();
    let mut d = ClubDelivery::square(mph * MPH, -2.0, c.loft_deg + c.tour_lean_deg);
    d.face_rad = face.to_radians();
    d.path_rad = path.to_radians();
    impact(&t, c, &d, 0.0, &Lie::CLEAN).launch
}

#[test]
fn face_and_path_give_the_d_plane_shapes() {
    for id in ["dr", "7i", "pw"] {
        // start: right is a negative azimuth; between the path and the face, nearer the face
        let start = |l: &golf_core::LaunchConditions| -l.direction_rad.to_degrees();
        let s = shot(id, 90.0, 0.0, 0.0);
        assert!(start(&s).abs() < 1e-9 && s.spin_axis_rad.abs() < 1e-9, "{id}: square is straight");
        let push_fade = shot(id, 90.0, 2.0, -2.0);
        assert!(start(&push_fade) > 0.5 && start(&push_fade) < 2.0 && push_fade.spin_axis_rad > 0.0, "{id}: push-fade");
        let pull_draw = shot(id, 90.0, -2.0, 2.0);
        assert!(start(&pull_draw) < -0.5 && pull_draw.spin_axis_rad < 0.0, "{id}: pull-draw");
        let fade = shot(id, 90.0, 0.0, -4.0);
        assert!(start(&fade).abs() < 1.2 && fade.spin_axis_rad > 0.0, "{id}: straight-ish start, fade");
        let slice = shot(id, 90.0, 6.0, -4.0);
        assert!(start(&slice) > 3.0 && slice.spin_axis_rad > push_fade.spin_axis_rad * 2.0, "{id}: slice");
    }
    // the same face to path tilts a driver's axis more than an iron's (less spin loft)
    assert!(shot("dr", 90.0, 0.0, -2.0).spin_axis_rad > shot("7i", 90.0, 0.0, -2.0).spin_axis_rad);
}

#[test]
fn off_centre_strikes_lose_speed_and_gear() {
    let t = ClubTable::builtin();
    let c = t.get("dr").unwrap();
    let base = ClubDelivery::square(45.0, 0.0, 13.0);
    let at = |x: f64, y: f64| impact(&t, c, &ClubDelivery { strike_m: Vec2::new(x, y), ..base }, 0.0, &Lie::CLEAN).launch;
    let centre = at(0.0, 0.0);
    let toe = at(0.015, 0.0);
    let heel = at(-0.015, 0.0);
    assert!(toe.ball_speed_mps < centre.ball_speed_mps && heel.ball_speed_mps < centre.ball_speed_mps);
    assert!(toe.spin_axis_rad < 0.0, "a toe strike draws (gear effect)");
    assert!(heel.spin_axis_rad > 0.0, "a heel strike fades");
    let high = at(0.0, 0.01);
    assert!(high.spin_radps < centre.spin_radps && high.launch_rad > centre.launch_rad, "high on the face: more launch, less spin");
}

#[test]
fn the_lie_and_the_slope_change_the_launch() {
    let t = ClubTable::builtin();
    let c = t.get("7i").unwrap();
    let d = ClubDelivery::square(38.0, -4.0, 22.0);
    let clean = impact(&t, c, &d, 0.0, &Lie::CLEAN).launch;
    let rough = LieTable::builtin().get(6);
    let r = impact(&t, c, &d, 0.0, &Lie { factors: rough, normal: Vec3::Z }).launch;
    assert!(r.spin_radps < 0.7 * clean.spin_radps && r.ball_speed_mps < clean.ball_speed_mps && r.launch_rad > clean.launch_rad);
    // the ground rising along the aim (uphill) launches higher
    let up = impact(&t, c, &d, 0.0, &Lie { factors: LieFactors::CLEAN, normal: Vec3::new(-0.1, 0.0, 1.0).norm() }).launch;
    assert!(up.launch_rad > clean.launch_rad + 2f64.to_radians());
    // ball above the feet (the ground rising to the right of the aim, away from a right-hander): a lofted face points
    // left, and more so for a wedge
    let above = Lie { factors: LieFactors::CLEAN, normal: Vec3::new(0.0, 0.15, 1.0).norm() };
    let w = t.get("sw").unwrap();
    let iron = impact(&t, c, &d, 0.0, &above).launch.direction_rad;
    let wedge = impact(&t, w, &ClubDelivery::square(35.0, -4.0, 50.0), 0.0, &above).launch.direction_rad;
    assert!(iron > 0.01 && wedge > iron, "left is a positive azimuth: iron {iron}, wedge {wedge}");
    // the aim turns everything with it
    let aimed = impact(&t, c, &d, 1.0, &Lie::CLEAN).launch;
    assert!((aimed.direction_rad - 1.0).abs() < 1e-12 && (aimed.ball_speed_mps - clean.ball_speed_mps).abs() < 1e-12);
}

#[test]
fn deterministic_to_the_bit() {
    let a = shot("5i", 87.0, 1.3, -0.7);
    let b = shot("5i", 87.0, 1.3, -0.7);
    assert_eq!(a, b);
}
