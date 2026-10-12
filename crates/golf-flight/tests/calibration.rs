//! The flight model against the calibration reference (data/calibration/reference.v1.json) and the shape checks of
//! docs/03-success-indicators.md. These run on flat ground in TrackMan's normalised air, so they need no fixtures.

use golf_core::{Air, LaunchConditions, Vec2};
use golf_flight::{AeroParams, End, Flat, Flight, Model, Wind};
use serde_json::Value;

const YD: f64 = 0.9144;

fn reference() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/calibration/reference.v1.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn fly_with(air: Air, wind: Wind, ball_mph: f64, launch_deg: f64, spin_rpm: f64, axis_deg: f64) -> Flight {
    let g = Flat(0.0);
    let m = Model::new(AeroParams::builtin(), air, wind, &g);
    m.fly(Vec2::ZERO, &LaunchConditions::from_monitor(ball_mph, launch_deg, 0.0, spin_rpm, axis_deg))
}

fn fly(ball_mph: f64, launch_deg: f64, spin_rpm: f64) -> Flight {
    fly_with(Air::trackman_normal(), Wind::CALM, ball_mph, launch_deg, spin_rpm, 0.0)
}

fn carry_yd(f: &Flight) -> f64 {
    f.carry_m() / YD
}

#[test]
fn fit_rows_carry_within_4_percent() {
    for r in reference()["fit_rows"].as_array().unwrap() {
        let f = fly(r["ball_mph"].as_f64().unwrap(), r["launch_deg"].as_f64().unwrap(), r["spin_rpm"].as_f64().unwrap());
        assert!(matches!(f.end, End::Contact { .. }));
        let (c, want) = (carry_yd(&f), r["carry_yd"].as_f64().unwrap());
        assert!((c - want).abs() / want < 0.04, "{}: carry {c:.1} vs {want}", r["id"]);
    }
}

#[test]
fn sniff_rows_land_in_their_bands() {
    for r in reference()["sniff_rows"].as_array().unwrap() {
        let f = fly(r["ball_mph"].as_f64().unwrap(), r["launch_deg"].as_f64().unwrap(), r["spin_rpm"].as_f64().unwrap());
        let c = carry_yd(&f);
        let b = &r["carry_band_yd"];
        assert!(c >= b[0].as_f64().unwrap() && c <= b[1].as_f64().unwrap(), "{} ({} {}): carry {c:.1} not in {b}", r["id"], r["group"], r["club"]);
    }
}

#[test]
fn faster_balls_carry_further_for_every_club_shape() {
    for (launch, spin) in [(11.0, 2700.0), (16.0, 7000.0), (25.0, 9000.0)] {
        let mut prev = 0.0;
        for mph in (60..=180).step_by(10) {
            let c = carry_yd(&fly(mph as f64, launch, spin));
            assert!(c > prev, "{mph} mph at {launch} deg: {c} <= {prev}");
            prev = c;
        }
    }
}

#[test]
fn tour_driver_spin_has_an_optimum_and_high_spin_balloons() {
    // 167 mph, 10.9 deg: too little spin falls out of the sky, too much balloons and loses carry
    let c: Vec<f64> = [1500.0, 2700.0, 3500.0, 5000.0].iter().map(|&s| carry_yd(&fly(167.0, 10.9, s))).collect();
    assert!(c[1] > c[0], "1500 rpm {:.1} should carry less than 2700 rpm {:.1}", c[0], c[1]);
    assert!(c[3] < c[2] && c[3] < c[1], "5000 rpm should lose carry: {c:?}");
    let (lo, hi) = (fly(167.0, 10.9, 2000.0), fly(167.0, 10.9, 3500.0));
    assert!(lo.apex_m() < hi.apex_m(), "more spin flies higher");
}

#[test]
fn headwind_costs_more_than_tailwind_gives() {
    let base = carry_yd(&fly(120.0, 16.3, 7097.0));
    let head = carry_yd(&fly_with(Air::trackman_normal(), Wind::new(10.0, std::f64::consts::PI), 120.0, 16.3, 7097.0, 0.0));
    let tail = carry_yd(&fly_with(Air::trackman_normal(), Wind::new(10.0, 0.0), 120.0, 16.3, 7097.0, 0.0));
    let (loss, gain) = (base - head, tail - base);
    assert!(loss > 0.0 && gain > 0.0 && loss > gain, "7 iron, 10 m/s: loses {loss:.1} into, gains {gain:.1} down");
}

#[test]
fn spin_axis_curves_the_ball_the_right_way_by_a_believable_amount() {
    let r = fly_with(Air::trackman_normal(), Wind::CALM, 120.0, 16.3, 7097.0, 3.0);
    let l = fly_with(Air::trackman_normal(), Wind::CALM, 120.0, 16.3, 7097.0, -3.0);
    let (rr, ll) = (r.lateral_m() / YD, l.lateral_m() / YD);
    assert!(rr > 0.0 && ll < 0.0 && (rr + ll).abs() < 1e-6, "{rr} {ll}");
    // a 3 degree axis on a tour 7 iron: a gentle fade, a few yards (launch monitors show roughly 1-3 yd per degree)
    assert!((3.0..=10.0).contains(&rr), "3 deg axis curves {rr:.1} yd");
}

#[test]
fn altitude_adds_carry_like_it_does_in_denver() {
    // ~1 % more carry per 1000 ft is the usual figure: Denver (1600 m, ~5250 ft) adds ~5-8 % to a driver
    let sea = carry_yd(&fly(167.0, 10.9, 2686.0));
    let den = carry_yd(&fly_with(Air::at_altitude(23.9, 1600.0, 0.75), Wind::CALM, 167.0, 10.9, 2686.0, 0.0));
    let gain = den / sea - 1.0;
    assert!((0.04..=0.10).contains(&gain), "Denver gain {:.1} %", gain * 100.0);
}

#[test]
fn deterministic_to_the_bit() {
    let a = fly(150.0, 12.0, 3000.0);
    let b = fly(150.0, 12.0, 3000.0);
    assert_eq!(a.last, b.last);
    assert_eq!(a.samples, b.samples);
}
