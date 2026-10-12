//! Flight time, curve per degree of spin axis, head/tail and cross wind, for review (M2 follow-up).
use golf_core::{Air, LaunchConditions, Vec2};
use golf_flight::{AeroParams, Flat, Flight, Model, Wind};
use std::f64::consts::{FRAC_PI_2, PI};
const YD: f64 = 0.9144;
const MPH: f64 = 0.44704;
fn fly(mph: f64, la: f64, sp: f64, axis: f64, w: Wind) -> Flight {
    let g = Flat(0.0);
    Model::new(AeroParams::builtin(), Air::trackman_normal(), w, &g).fly(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, sp, axis))
}
fn main() {
    let clubs = [("PGA driver", 167.0, 10.9, 2686.0), ("PGA 6 iron", 127.0, 14.1, 6231.0), ("PGA 7 iron", 120.0, 16.3, 7097.0), ("PGA PW", 102.0, 24.2, 9304.0), ("AMA driver", 132.6, 12.6, 3275.0), ("AMA 7 iron", 106.0, 17.5, 6500.0), ("LPGA driver", 140.0, 13.2, 2611.0), ("70 mph driver", 97.0, 15.0, 3200.0)];
    println!("{:<14} {:>6} {:>6} | curve per deg axis (yd) | 10mph into/down (yd) | 20mph into/down | 10mph cross drift | 20mph cross", "shot", "carry", "time");
    for (n, mph, la, sp) in clubs {
        let f = fly(mph, la, sp, 0.0, Wind::CALM);
        let c = f.carry_m() / YD;
        let c5 = fly(mph, la, sp, 5.0, Wind::CALM).lateral_m() / YD / 5.0;
        let w = |s: f64, d: f64| fly(mph, la, sp, 0.0, Wind::new(s * MPH, d));
        let (i10, d10, i20, d20) = (w(10.0, PI).carry_m() / YD - c, w(10.0, 0.0).carry_m() / YD - c, w(20.0, PI).carry_m() / YD - c, w(20.0, 0.0).carry_m() / YD - c);
        let x10 = w(10.0, -FRAC_PI_2).lateral_m() / YD;
        let x20 = w(20.0, -FRAC_PI_2).lateral_m() / YD;
        println!("{n:<14} {c:>6.1} {:>6.2} | {c5:>6.2} | {i10:>+6.1} {d10:>+6.1} | {i20:>+6.1} {d20:>+6.1} | {x10:>6.1} | {x20:>6.1}", f.time_s());
    }
}
