//! M3 exploration: the ground model against the gates in docs/03-success-indicators.md §2.
use golf_core::{Air, LaunchConditions, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, EventKind, GroundMaterials, Outcome, TestPlane, capture_speed};

const YD: f64 = 0.9144;

fn shot(plane: &TestPlane, m: &GroundMaterials, mph: f64, la: f64, rpm: f64) -> Outcome {
    let b = BallModel::new(plane, m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0))
}

fn describe(name: &str, o: &Outcome) {
    let roll = o.total_m() - o.carry_m();
    let first_bounce = o.events.iter().find_map(|e| if let EventKind::Bounce { cor, gripped, .. } = e.kind { Some((cor, gripped)) } else { None });
    println!(
        "{name:<34} carry {:6.1} yd  roll {:+6.1} yd  total {:6.1} yd  bounces {:2}  time {:5.1} s  end {:?}  first bounce {:?}",
        o.carry_m() / YD, roll / YD, o.total_m() / YD, o.bounces(), o.time_s, o.state, first_bounce
    );
}

fn main() {
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let fairway = TestPlane::flat(SurfaceClass::FAIRWAY);
    let rough = TestPlane::flat(SurfaceClass::ROUGH);
    let bunker = TestPlane::flat(SurfaceClass::BUNKER_SAND);

    // stimp: 1.83 m/s rolling on a flat green should roll the stimp distance (10.5 ft = 3.20 m)
    let b = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    let o = b.roll_from(Vec2::ZERO, Vec3::new(1.83, 0.0, 0.0));
    println!("stimp: rolled {:.3} m (target {:.3} m), {:?}", o.total_m(), 10.5 * 0.3048, o.state);

    // a ball at rest on a sloped green
    for slope in [0.02, 0.04, 0.06, 0.08] {
        let p = TestPlane { z0: 0.0, gx: slope, gy: 0.0, class: SurfaceClass::GREEN };
        let b = BallModel::new(&p, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
        let o = b.roll_from(Vec2::ZERO, Vec3::ZERO);
        println!("at rest on a {:.0} % green slope: moved {:.2} m, {:?}", slope * 100.0, o.total_m(), o.state);
    }

    println!();
    describe("amateur driver -> fairway", &shot(&fairway, &m, 132.6, 12.6, 3275.0));
    describe("PGA driver -> fairway", &shot(&fairway, &m, 167.0, 10.9, 2686.0));
    describe("LPGA driver -> fairway", &shot(&fairway, &m, 140.0, 13.2, 2611.0));
    describe("70 mph driver -> fairway", &shot(&fairway, &m, 97.0, 15.0, 3200.0));
    describe("amateur driver -> rough", &shot(&rough, &m, 132.6, 12.6, 3275.0));
    describe("PGA 7 iron -> green", &shot(&green, &m, 120.0, 16.3, 7097.0));
    describe("amateur 7 iron -> green", &shot(&green, &m, 106.0, 17.5, 6500.0));
    describe("PGA PW -> green", &shot(&green, &m, 102.0, 24.2, 9304.0));
    describe("LPGA PW -> green", &shot(&green, &m, 86.0, 25.6, 8403.0));
    describe("amateur PW -> green", &shot(&green, &m, 90.0, 26.0, 8400.0));
    describe("PGA 7 iron -> fairway", &shot(&fairway, &m, 120.0, 16.3, 7097.0));
    describe("PGA 7 iron -> bunker", &shot(&bunker, &m, 120.0, 16.3, 7097.0));
    describe("amateur PW -> bunker", &shot(&bunker, &m, 90.0, 26.0, 8400.0));
    describe("chip 15 m/s, 30 deg, 3000 rpm -> green", &shot(&green, &m, 15.0 / 0.44704, 30.0, 3000.0));

    println!("\ncup: capture speed through the centre {:.2} m/s, at half the radius {:.2} m/s", capture_speed(0.0), capture_speed(0.027));
    for (v, off) in [(1.0, 0.0), (1.5, 0.0), (1.8, 0.0), (1.0, 0.03), (1.0, 0.05), (0.6, 0.06)] {
        // a putt from 3 m, aimed off-centre by `off`, with speed v as it reaches the hole
        let d = 3.0;
        let a = (green.base_decel(&m)).max(0.0);
        let v0 = (v * v + 2.0 * a * d).sqrt();
        let b = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(Cup { xy: Vec2::new(d, off), z: 0.0 }));
        let o = b.roll_from(Vec2::ZERO, Vec3::new(v0, 0.0, 0.0));
        let lip = o.events.iter().any(|e| e.kind == EventKind::LipOut);
        println!("  putt reaching the hole at {v:.1} m/s, {:.0} mm off-centre: {:?}{}", off * 1000.0, o.state, if lip { " (lipped out)" } else { "" });
    }
}

trait Decel {
    fn base_decel(&self, m: &GroundMaterials) -> f64;
}
impl Decel for TestPlane {
    fn base_decel(&self, m: &GroundMaterials) -> f64 {
        m.get(self.class).base_decel()
    }
}
