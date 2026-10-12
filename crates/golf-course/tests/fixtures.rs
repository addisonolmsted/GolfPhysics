//! Fixture tests on the six frozen v1 bundles. They skip (with a message) when the fixtures aren't mounted; see
//! fixtures/README.md.

use std::path::PathBuf;

use golf_core::{Vec2, ball::RADIUS_M};
use golf_course::{Course, SurfaceClass};

const SEEDS: [u64; 6] = [7667961, 604486, 5746282, 8337594, 1920628, 7011100];

fn fixture_dir() -> Option<PathBuf> {
    let d = PathBuf::from(std::env::var("GOLF_FIXTURES").unwrap_or_else(|_| "/Volumes/GolfOut/out/physics_fixtures/v1".into()));
    if d.join(format!("course_4_{}", SEEDS[0])).is_dir() {
        Some(d)
    } else {
        eprintln!("SKIP: fixtures not found at {} (set GOLF_FIXTURES)", d.display());
        None
    }
}

fn courses() -> Vec<Course> {
    let Some(d) = fixture_dir() else { return vec![] };
    SEEDS.iter().map(|s| Course::open(&d.join(format!("course_4_{s}"))).unwrap_or_else(|e| panic!("{s}: {e}"))).collect()
}

/// points every `step` m along a polyline
fn walk(pts: &[[f64; 2]], step: f64) -> Vec<Vec2> {
    let mut out = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (Vec2::new(w[0][0], w[0][1]), Vec2::new(w[1][0], w[1][1]));
        let n = (a.dist(b) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            out.push(a + (b - a) * (k as f64 / n as f64));
        }
    }
    out
}

#[test]
fn every_fixture_loads_and_verifies() {
    for c in courses() {
        assert_eq!(c.holes.len(), 9, "{}", c.manifest.course_id);
        assert_eq!(c.holes.iter().map(|h| h.par as u32).sum::<u32>(), c.manifest.par);
        assert!(!c.instances.records.is_empty());
        assert_eq!(c.heights.tiers.len(), 4);
        assert_eq!(c.heights.tiers[0].res_m, 0.5);
    }
}

#[test]
fn heights_exist_along_every_line_of_play_on_the_finest_tier() {
    for c in courses() {
        for h in &c.holes {
            for p in walk(&h.line_of_play.spline, 2.0) {
                let (z, tier) = c.heights.at(p.x, p.y).unwrap_or_else(|| panic!("{} h{} no height at {p:?}", c.manifest.course_id, h.number));
                assert!(z.is_finite());
                assert_eq!(tier, 0, "{} h{} at {p:?}: not the 0.5 m tier", c.manifest.course_id, h.number);
            }
        }
    }
}

#[test]
fn pins_sit_on_green_and_match_their_height() {
    for c in courses() {
        for h in &c.holes {
            let p = h.green.pin.xy;
            assert_eq!(c.surface(p[0], p[1]), SurfaceClass::GREEN, "{} h{}", c.manifest.course_id, h.number);
            let z = c.height(p[0], p[1]).unwrap();
            // the pin's z is the 0.5 m ground at the cup, written to 1 mm
            assert!((z - h.green.pin.z).abs() < 0.002, "{} h{}: {z} vs {}", c.manifest.course_id, h.number, h.green.pin.z);
            assert!(c.in_bounds(Vec2::new(p[0], p[1]), RADIUS_M));
            assert!(c.material(SurfaceClass::GREEN).physics.stimp.is_some());
        }
    }
}

#[test]
fn tees_are_in_bounds_and_outside_the_world_is_not() {
    for c in courses() {
        for h in &c.holes {
            for t in &h.tees {
                assert!(c.in_bounds(Vec2::new(t.plate[0], t.plate[1]), RADIUS_M), "{} h{} tee {}", c.manifest.course_id, h.number, t.index);
            }
        }
        assert!(!c.in_bounds(Vec2::new(5.0, 5.0), RADIUS_M));
    }
}

#[test]
fn the_ob_test_is_whole_ball() {
    for c in courses() {
        let ob = c.rules.ob.iter().find(|o| o.inside_is_in_bounds).unwrap();
        let mut checked = 0;
        // probe across the ring near its own bbox centre line: step along x until containment flips
        let (lo, hi) = ob.region.area_bbox;
        let y = 0.5 * (lo.y + hi.y);
        let mut x = lo.x - 5.0;
        let mut prev = ob.region.contains(Vec2::new(x, y));
        while x < hi.x + 5.0 {
            let now = ob.region.contains(Vec2::new(x + 0.01, y));
            if now != prev {
                // the boundary is within 1 cm of x: a ball centred there touches the line, so it is in bounds
                assert!(c.in_bounds(Vec2::new(x + 0.005, y), RADIUS_M));
                checked += 1;
            }
            prev = now;
            x += 0.01;
        }
        assert!(checked >= 2, "{}", c.manifest.course_id);
    }
}

#[test]
fn water_bodies_report_their_penalty_area() {
    for c in courses() {
        for b in &c.water.bodies {
            let (lo, hi) = b.region.area_bbox;
            // the first grid point inside the body and in bounds (a creek can run on past the property, where its
            // penalty area, clipped to the property, does not follow it)
            let Some(p) = (0..400)
                .flat_map(|j| (0..400).map(move |i| (i, j)))
                .map(|(i, j)| Vec2::new(lo.x + (hi.x - lo.x) * (i as f64 + 0.5) / 400.0, lo.y + (hi.y - lo.y) * (j as f64 + 0.5) / 400.0))
                .find(|p| b.region.contains(*p) && c.in_bounds(*p, RADIUS_M) && c.surface(p.x, p.y) == SurfaceClass::WATER)
            else {
                continue;
            };
            let w = c.water_at(p).unwrap();
            assert!(w.body.is_some());
            if b.region.boundary_dist(p, 2.0).is_infinite() {
                assert!(w.surface_m >= c.height(p.x, p.y).unwrap() - 0.05, "{} {}: surface below the bed", c.manifest.course_id, b.id);
            }
            if let Some(pa) = b.penalty_areas.first() {
                assert_eq!(c.penalty_area(p, RADIUS_M).map(|a| a.id.as_str()), Some(pa.as_str()), "{} {} {p:?}", c.manifest.course_id, b.id);
            }
        }
    }
}

#[test]
fn analytic_gradient_matches_finite_differences() {
    for c in courses().into_iter().take(2) {
        for h in &c.holes {
            for p in walk(&h.line_of_play.spline, 7.3) {
                let s = c.surface_at(p.x, p.y).unwrap();
                let e = 1e-4;
                let gx = (c.height(p.x + e, p.y).unwrap() - c.height(p.x - e, p.y).unwrap()) / (2.0 * e);
                let gy = (c.height(p.x, p.y + e).unwrap() - c.height(p.x, p.y - e).unwrap()) / (2.0 * e);
                assert!((s.dzdx - gx).abs() < 1e-5 && (s.dzdy - gy).abs() < 1e-5, "{p:?}: {s:?} vs {gx} {gy}");
                assert_eq!(s.z, c.height(p.x, p.y).unwrap());
            }
        }
    }
}
