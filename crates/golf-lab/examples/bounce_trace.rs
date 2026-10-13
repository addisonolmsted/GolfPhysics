//! M3 tuning: one shot, event by event (speeds and distances), to see where the roll comes from.
use golf_core::{Air, LaunchConditions, Vec2};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, GroundMaterials, TestPlane};
fn main() {
    let args: Vec<f64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let (mph, la, rpm) = if args.len() >= 3 { (args[0], args[1], args[2]) } else { (132.6, 12.6, 3275.0) };
    let class = if args.len() >= 4 { SurfaceClass(args[3] as u8) } else { SurfaceClass::FAIRWAY };
    let fairway = TestPlane::flat(class);
    let m = GroundMaterials::builtin();
    let b = BallModel::new(&fairway, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    let o = b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
    let x0 = o.first_contact.unwrap().x;
    for e in &o.events {
        // the sample nearest the event gives the speed
        let s = o.samples.iter().min_by(|a, c| (a.t - e.t).abs().total_cmp(&(c.t - e.t).abs())).unwrap();
        let bs = s.w.dot(golf_core::Vec3::new(0.0, -1.0, 0.0)) * 30.0 / std::f64::consts::PI;
        println!("t {:6.2}  past landing {:6.2} yd  vx {:6.2} vz {:6.2} m/s  backspin {:6.0} rpm  {:?}", e.t, (e.p.x - x0) / 0.9144, s.v.x, s.v.z, bs, e.kind);
    }
    // speed just after each bounce, from the samples
    let mut prev_air = false;
    for s in &o.samples {
        let air = s.phase == golf_ground::Phase::Air;
        if air && !prev_air && s.t > o.events[0].t {
            println!("  hop starts t {:.2}: vx {:.2} vz {:.2} m/s backspin {:.0} rpm", s.t, s.v.x, s.v.z, s.w.dot(golf_core::Vec3::new(0.0, -1.0, 0.0)) * 30.0 / std::f64::consts::PI);
        }
        prev_air = air;
    }
}
