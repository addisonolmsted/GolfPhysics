//! Why a 20 mph tailwind gives a slow drive no gain: lift, drag, time and speed, calm against downwind (M2b).
use golf_core::{Air, LaunchConditions, Vec2, ball::RADIUS_M};
use golf_flight::{AeroParams, Flat, Model, Wind};
fn main() {
    let a = AeroParams::builtin();
    let air = Air::trackman_normal();
    let k = 0.5 * air.density_kgpm3 * std::f64::consts::PI * RADIUS_M * RADIUS_M / golf_core::ball::MASS_KG;
    for (name, mph, la, rpm) in [("70 mph driver", 97.0, 15.0, 3200.0), ("tour driver", 167.0, 10.9, 2686.0)] {
        for (wl, w) in [("calm", 0.0), ("20 mph down", 20.0 * 0.44704)] {
            let g = Flat(0.0);
            let f = Model::new(a, air, Wind::new(w, 0.0), &g).fly(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
            // forces at launch, in the air's frame
            let u = mph * 0.44704 - w * (la.to_radians().cos());
            let s = RADIUS_M * rpm * std::f64::consts::PI / 30.0 / u;
            let (cd, cl) = a.coefficients(s, u);
            println!(
                "{name:<14} {wl:<12} airspeed {u:5.1} m/s  lift {:.2} g  drag {:.2} g | carry {:6.1} yd  time {:.2} s  apex {:4.1} yd  mean ground speed {:4.1} m/s",
                k * cl * u * u / 9.80665, k * cd * u * u / 9.80665, f.carry_m() / 0.9144, f.time_s(), f.apex_m() / 0.9144, f.carry_m() / f.time_s()
            );
        }
    }
}
