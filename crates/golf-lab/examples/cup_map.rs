//! What the physical cup does with a putt, by its speed at the hole and how far its line passes from the centre:
//! H holed, L lipped out (touched the rim and left), . missed. Flat green, stimp 10.5.

use golf_core::ball::{CUP_RADIUS_M, GAME_CUP_RADIUS_M};
use golf_core::{Air, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, EventKind, GroundMaterials, RestState, TestPlane};

fn main() {
    let m = GroundMaterials::builtin();
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let a = m.get(SurfaceClass::GREEN).base_decel();
    for (name, r, cap) in [("regulation", CUP_RADIUS_M, None), ("10 in, no cap", GAME_CUP_RADIUS_M, None), ("game: 10 in, cap", GAME_CUP_RADIUS_M, golf_ground::GAME_CAPTURE_CAP_MPS)] {
        println!("{name}: rows = speed at the hole (m/s), columns = line offset as a share of the radius 0.0 .. 0.95");
        let t0 = std::time::Instant::now();
        let mut n = 0;
        for k in 0..16 {
            let v = 0.2 + 0.2 * k as f64;
            let mut row = String::new();
            for j in 0..20 {
                let b = r * j as f64 * 0.05;
                let d = 0.5;
                let cup = Cup { xy: Vec2::new(d, b), z: 0.0, radius_m: r, capture_cap_mps: cap, flagstick: None };
                let bm = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(cup));
                let o = bm.roll_from(Vec2::ZERO, Vec3::new((v * v + 2.0 * a * d).sqrt(), 0.0, 0.0));
                n += 1;
                row.push(if o.state == RestState::Holed { 'H' } else if o.events.iter().any(|e| e.kind == EventKind::LipOut) { 'L' } else { '.' });
            }
            println!("  {v:4.2} {row}");
        }
        println!("  {:.2} ms per putt", t0.elapsed().as_secs_f64() * 1e3 / n as f64);
    }
}
