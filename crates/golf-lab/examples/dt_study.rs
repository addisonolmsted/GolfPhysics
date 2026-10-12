//! How large a fixed step can RK4 take before the flight changes? (M2 investigation)
use golf_core::{Air, LaunchConditions, Vec2};
use golf_flight::{AeroParams, Flat, Model, Wind};
fn main() {
    let g = Flat(0.0);
    let shots = [("tour driver", 167.0, 10.9, 2686.0), ("amateur 7 iron", 106.0, 17.5, 6500.0), ("tour PW", 102.0, 24.2, 9304.0), ("chip", 40.0, 30.0, 4000.0)];
    for (name, mph, la, sp) in shots {
        let l = LaunchConditions::from_monitor(mph, la, 0.3, sp, 4.0);
        let base = Model::new(AeroParams::builtin(), Air::trackman_normal(), Wind::new(6.0, 2.0), &g).with_dt(0.25e-3).fly(Vec2::ZERO, &l);
        print!("{name:<15}");
        for dt in [1e-3, 2e-3, 4e-3, 8e-3, 16e-3] {
            let m = Model::new(AeroParams::builtin(), Air::trackman_normal(), Wind::new(6.0, 2.0), &g).with_dt(dt);
            let t = std::time::Instant::now();
            let mut f = m.fly(Vec2::ZERO, &l);
            for _ in 0..199 {
                f = m.fly(Vec2::ZERO, &l);
            }
            let us = t.elapsed().as_secs_f64() * 1e6 / 200.0;
            let err_mm = (f.last.p - base.last.p).len() * 1000.0;
            print!(" | {:>2.0} ms: {err_mm:>7.2} mm {us:>6.0} us", dt * 1e3);
        }
        println!();
    }
}
