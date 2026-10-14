//! How far putts travel around the cup's rim: for each line and pace, the angle the ball's centre sweeps about the
//! cup's centre while it is at the hole, and how it ends. The game cup (10 in) and the 2x contact ball.

use golf_core::ball::{CONTACT_RADIUS_M, GAME_CUP_RADIUS_M};
use golf_core::{Air, Vec2, Vec3};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, GroundMaterials, RestState, TestPlane};

fn main() {
    let m = GroundMaterials::builtin();
    let slope: f64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(0.0);
    let green = TestPlane { z0: 0.0, gx: 0.0, gy: slope, class: SurfaceClass::GREEN };
    let a = m.get(SurfaceClass::GREEN).base_decel();
    let r = GAME_CUP_RADIUS_M;
    let d = 1.0;
    let mut rows = Vec::new();
    for j in 0..=60 {
        let f = 0.55 + 0.0075 * j as f64;
        for k in 0..=40 {
            let v = 0.2 + 0.05 * k as f64;
            let cup = Cup::game(Vec2::new(d, 0.0), 0.0);
            let bm = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, Some(cup));
            let o = bm.roll_from(Vec2::new(0.0, f * r), Vec3::new((v * v + 2.0 * a * (d - r)).sqrt(), 0.0, 0.0));
            // the angle swept about the cup centre while near the rim
            let mut swept = 0.0;
            let mut last: Option<f64> = None;
            for s in &o.samples {
                let p = s.p.xy() - Vec2::new(d, 0.0);
                if p.len() < r + CONTACT_RADIUS_M {
                    let ang = p.y.atan2(p.x);
                    if let Some(l) = last {
                        let mut da = ang - l;
                        while da > std::f64::consts::PI { da -= 2.0 * std::f64::consts::PI; }
                        while da < -std::f64::consts::PI { da += 2.0 * std::f64::consts::PI; }
                        swept += da;
                    }
                    last = Some(ang);
                } else {
                    last = None;
                }
            }
            rows.push((swept.abs().to_degrees(), f, v, o.state == RestState::Holed));
        }
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let n_in = rows.iter().filter(|r| r.0 > 180.0 && r.3).count();
    let n_out = rows.iter().filter(|r| r.0 > 180.0 && !r.3).count();
    println!("slope {slope}: {} putts; swept > 180 deg: {n_in} in, {n_out} out; > 270: {}", rows.len(), rows.iter().filter(|r| r.0 > 270.0).count());
    for (sw, f, v, holed) in rows.iter().take(14) {
        println!("  {sw:6.0} deg  line {:.4} out  {v:.2} m/s  {}", f, if *holed { "IN" } else { "out" });
    }
}
