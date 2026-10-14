//! A ball rolling into the flagstick at a given speed (m/s at the hole), traced near the cup.
use golf_core::{Air, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, GroundMaterials, TestPlane};

fn main() {
    let v: f64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(2.6);
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let a = m.get(SurfaceClass::GREEN).base_decel();
    let d = 1.0;
    let cup = Cup { flagstick: Some(true), ..Cup::game(Vec2::new(d, 0.0), 0.0) };
    let bm = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(cup));
    let o = bm.roll_from(Vec2::ZERO, Vec3::new((v * v + 2.0 * a * d).sqrt(), 0.0, 0.0));
    for s in &o.samples {
        let p = s.p.xy() - Vec2::new(d, 0.0);
        if p.len() < 0.3 {
            println!("{:6.3}s  x {:6.1} mm  z {:6.1} mm  vx {:5.2} vz {:5.2}  wy {:6.0}", s.t, p.x * 1e3, s.p.z * 1e3, s.v.x, s.v.z, s.w.y);
        }
    }
    for e in &o.events { println!("{:?} at {:.3}s x {:.3}", e.kind, e.t, e.p.x - d); }
    println!("{:?}", o.state);
}
