//! One putt at the game cup, traced: time, angle about the cup, distance from the centre, centre height (mm).
//! args: line offset as a share of the radius, speed at the hole (m/s)

use golf_core::ball::GAME_CUP_RADIUS_M;
use golf_core::{Air, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, GroundMaterials, TestPlane};

fn main() {
    let a: Vec<f64> = std::env::args().skip(1).filter_map(|x| x.parse().ok()).collect();
    let (f, v) = (a[0], a[1]);
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let dec = m.get(SurfaceClass::GREEN).base_decel();
    let r = GAME_CUP_RADIUS_M;
    let d = 1.0;
    let bm = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(Cup::game(Vec2::new(d, 0.0), 0.0)));
    let o = bm.roll_from(Vec2::new(0.0, f * r), Vec3::new((v * v + 2.0 * dec * (d - r)).sqrt(), 0.0, 0.0));
    for s in o.samples.iter().step_by(8) {
        let p = s.p.xy() - Vec2::new(d, 0.0);
        if p.len() < r + 0.06 {
            println!("{:6.3}s  ang {:5.0}  rho {:5.1} mm  z {:6.1} mm  v {:.2}", s.t, p.y.atan2(p.x).to_degrees(), p.len() * 1e3, s.p.z * 1e3, s.v.len());
        }
    }
    println!("{:?}", o.state);
}
