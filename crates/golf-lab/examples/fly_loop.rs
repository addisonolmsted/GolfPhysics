//! A long loop of flights, for profiling (M2 investigation).
use golf_core::{Air, LaunchConditions, Vec2};
use golf_flight::{AeroParams, Flat, Model, Wind};
fn main() {
    let g = Flat(0.0);
    let m = Model::new(AeroParams::builtin(), Air::trackman_normal(), Wind::new(6.0, 2.0), &g);
    let l = LaunchConditions::from_monitor(167.0, 10.9, 0.3, 2686.0, 4.0);
    let mut c = 0.0;
    for _ in 0..3000 {
        c += m.fly(Vec2::ZERO, &l).carry_m();
    }
    println!("{c}");
}
