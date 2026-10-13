//! The M3 gates (docs/03-success-indicators.md §2) on test planes, plus the course checks on the v1 fixtures (skipped
//! when they aren't mounted).

use std::path::PathBuf;

use golf_core::ball::{CUP_RADIUS_M, RADIUS_M};
use golf_core::{Air, LaunchConditions, Vec2, Vec3};
use golf_course::{Course, SurfaceClass};
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, EventKind, GroundMaterials, Outcome, RestState, TestPlane, capture_speed};

const YD: f64 = 0.9144;

fn shot(plane: &TestPlane, mph: f64, la: f64, rpm: f64) -> Outcome {
    let m = GroundMaterials::builtin();
    let b = BallModel::new(plane, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0))
}

fn roll_yd(o: &Outcome) -> f64 {
    (o.total_m() - o.carry_m()) / YD
}

#[test]
fn stimp_rolls_the_stimp_distance() {
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let b = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    let o = b.roll_from(Vec2::ZERO, Vec3::new(1.83, 0.0, 0.0));
    let want = 10.5 * 0.3048;
    assert!((o.total_m() - want).abs() / want < 0.03, "rolled {:.3} m, want {want:.3}", o.total_m());
    assert_eq!(o.state, RestState::OnGround);
}

#[test]
fn a_ball_holds_on_a_gentle_green_slope_and_rolls_off_a_steep_one() {
    let m = GroundMaterials::builtin();
    for (slope, holds) in [(0.03, true), (0.05, true), (0.10, false)] {
        let p = TestPlane { z0: 0.0, gx: slope, gy: 0.0, class: SurfaceClass::GREEN };
        let b = BallModel::new(&p, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
        let o = b.roll_from(Vec2::ZERO, Vec3::ZERO);
        assert_eq!(o.total_m() < 0.01, holds, "{slope}: moved {:.2} m", o.total_m());
    }
}

#[test]
fn amateur_driver_rolls_like_trackman() {
    // TrackMan's average male amateur: carry 204, total 226 yd on its normalised fairway: 22 yd of roll
    let r = roll_yd(&shot(&TestPlane::flat(SurfaceClass::FAIRWAY), 132.6, 12.6, 3275.0));
    assert!((17.0..=27.0).contains(&r), "amateur driver rolls {r:.1} yd");
}

#[test]
fn drivers_roll_15_to_40_yards_on_fairway_and_less_in_rough() {
    let fw = TestPlane::flat(SurfaceClass::FAIRWAY);
    for (mph, la, rpm) in [(167.0, 10.9, 2686.0), (140.0, 13.2, 2611.0), (97.0, 15.0, 3200.0)] {
        let r = roll_yd(&shot(&fw, mph, la, rpm));
        assert!((15.0..=40.0).contains(&r), "{mph} mph driver rolls {r:.1} yd");
    }
    let fairway = roll_yd(&shot(&fw, 132.6, 12.6, 3275.0));
    let rough = roll_yd(&shot(&TestPlane::flat(SurfaceClass::ROUGH), 132.6, 12.6, 3275.0));
    assert!(rough < 0.6 * fairway, "rough {rough:.1} vs fairway {fairway:.1}");
}

#[test]
fn wedges_stop_on_greens() {
    let green = TestPlane::flat(SurfaceClass::GREEN);
    for (who, mph, la, rpm) in [("PGA PW", 102.0, 24.2, 9304.0), ("LPGA PW", 86.0, 25.6, 8403.0), ("amateur PW", 90.0, 26.0, 8400.0)] {
        let r = roll_yd(&shot(&green, mph, la, rpm));
        assert!((-3.0..=5.0).contains(&r), "{who} rolls {r:.1} yd on a green");
    }
    let r = roll_yd(&shot(&green, 120.0, 16.3, 7097.0));
    assert!((0.0..=8.0).contains(&r), "tour 7 iron rolls {r:.1} yd on a green");
}

#[test]
fn bunkers_stop_the_ball_and_only_steep_fast_landings_plug() {
    let sand = TestPlane::flat(SurfaceClass::BUNKER_SAND);
    // steep iron and wedge landings stop; a shallow, fast driver landing may run a few yards (feel item, D21)
    for (mph, la, rpm, most) in [(120.0, 16.3, 7097.0, 1.5), (90.0, 26.0, 8400.0, 1.5), (132.6, 12.6, 3275.0, 8.0)] {
        let o = shot(&sand, mph, la, rpm);
        assert_ne!(o.state, RestState::Plugged, "{mph} mph plugged");
        assert!(roll_yd(&o).abs() < most, "{mph} mph into sand moved {:.1} yd", roll_yd(&o));
    }
    // a ball dropping almost straight down, fast: plugs
    let m = GroundMaterials::builtin();
    let b = BallModel::new(&sand, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    let o = b.play(Vec2::ZERO, &LaunchConditions::from_monitor(110.0, 85.0, 0.0, 2000.0, 0.0));
    assert_eq!(o.state, RestState::Plugged);
}

#[test]
fn the_cup_takes_putts_below_the_capture_speed() {
    assert!((capture_speed(0.0) - 1.64).abs() < 0.02, "Holmes measured ~1.63 m/s through the centre");
    assert_eq!(capture_speed(CUP_RADIUS_M), 0.0);
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let a = m.get(SurfaceClass::GREEN).base_decel();
    let putt = |at_hole: f64, offset: f64| {
        let d = 3.0;
        let b = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(Cup::regulation(Vec2::new(d, offset), 0.0)));
        b.roll_from(Vec2::ZERO, Vec3::new((at_hole * at_hole + 2.0 * a * d).sqrt(), 0.0, 0.0))
    };
    assert_eq!(putt(1.0, 0.0).state, RestState::Holed);
    assert_eq!(putt(1.4, 0.0).state, RestState::Holed);
    assert_eq!(putt(1.0, 0.03).state, RestState::Holed);
    let fast = putt(1.9, 0.0);
    assert_ne!(fast.state, RestState::Holed);
    assert!(fast.events.iter().any(|e| e.kind == EventKind::LipOut));
    assert_ne!(putt(1.0, CUP_RADIUS_M + 0.005).state, RestState::Holed);
    // a holed ball rests in the cup
    let o = putt(0.8, 0.0);
    assert!((o.rest.xy() - Vec2::new(3.0, 0.0)).len() < 1e-9);
}

#[test]
fn deterministic_to_the_bit() {
    let fw = TestPlane::flat(SurfaceClass::FAIRWAY);
    let a = shot(&fw, 150.0, 12.0, 3000.0);
    let b = shot(&fw, 150.0, 12.0, 3000.0);
    assert_eq!(a.rest, b.rest);
    assert_eq!(a.samples, b.samples);
}

// ---- the course (fixtures) ----

fn fixture(seed: u64) -> Option<Course> {
    let d = PathBuf::from(std::env::var("GOLF_FIXTURES").unwrap_or_else(|_| "/Volumes/GolfOut/out/physics_fixtures/v1".into()));
    let p = d.join(format!("course_4_{seed}"));
    if !p.is_dir() {
        eprintln!("SKIP: fixtures not found at {}", d.display());
        return None;
    }
    Some(Course::open(&p).unwrap())
}

#[test]
fn a_ball_settles_next_to_every_fixture_pin() {
    let m = GroundMaterials::builtin();
    for seed in [7667961, 604486, 5746282, 8337594, 1920628, 7011100] {
        let Some(c) = fixture(seed) else { return };
        for h in &c.holes {
            let pin = Vec2::new(h.green.pin.xy[0], h.green.pin.xy[1]);
            let b = BallModel::new(&c, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
            for dir in 0..4 {
                let p = pin + Vec2::from_azimuth(dir as f64 * std::f64::consts::FRAC_PI_2) * 0.5;
                let o = b.roll_from(p, Vec3::ZERO);
                assert_eq!(o.state, RestState::OnGround, "{seed} h{}: dropped by the pin, ended {:?}", h.number, o.state);
                assert!(o.total_m() < 3.0, "{seed} h{}: a ball dropped 0.5 m from the pin rolled {:.1} m", h.number, o.total_m());
            }
        }
    }
}

#[test]
fn random_shots_on_every_fixture_end_cleanly() {
    let m = GroundMaterials::builtin();
    let mut s: u64 = 0x5eed;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    for seed in [7667961, 1920628, 7011100] {
        let Some(c) = fixture(seed) else { return };
        for k in 0..300 {
            let h = &c.holes[k % c.holes.len()];
            let tee = &h.tees[0];
            let start = Vec2::new(tee.plate[0], tee.plate[1]) + Vec2::new(rnd() - 0.5, rnd() - 0.5) * 60.0;
            let wind = Wind::new(rnd() * 10.0, rnd() * std::f64::consts::TAU);
            let b = BallModel::new(&c, &m, AeroParams::builtin(), Air::trackman_normal(), wind, Some(Cup::game(Vec2::new(h.green.pin.xy[0], h.green.pin.xy[1]), h.green.pin.z)));
            let l = LaunchConditions::from_monitor(40.0 + rnd() * 130.0, 5.0 + rnd() * 30.0, rnd() * std::f64::consts::TAU, 1500.0 + rnd() * 9000.0, (rnd() - 0.5) * 30.0);
            let o = b.play(start, &l);
            assert_ne!(o.state, RestState::Timeout, "{seed} shot {k} timed out at t {:.1} s, rest {:?} class {:?}; last events {:?}",
                o.time_s, o.rest, c.surface(o.rest.x, o.rest.y), &o.events[o.events.len().saturating_sub(6)..]);
            assert!(o.rest.x.is_finite() && o.rest.y.is_finite() && o.rest.z.is_finite(), "{seed} shot {k}: NaN");
            if o.state == RestState::OnGround {
                let g = c.height(o.rest.x, o.rest.y).unwrap();
                assert!(o.rest.z - RADIUS_M - g > -0.002, "{seed} shot {k}: rest {:.4} m below the ground", g - (o.rest.z - RADIUS_M));
            }
            for w in o.samples.windows(2) {
                assert!(w[1].t >= w[0].t, "{seed} shot {k}: samples out of order");
            }
        }
    }
}
