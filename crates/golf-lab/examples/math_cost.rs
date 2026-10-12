//! What do the deterministic math calls cost? (M2 investigation)
use std::hint::black_box;
fn main() {
    let n = 2_000_000;
    let xs: Vec<f64> = (0..n).map(|k| 0.05 + (k % 1000) as f64 * 0.0004).collect();
    let time = |name: &str, f: &dyn Fn(f64) -> f64| {
        let t = std::time::Instant::now();
        let mut acc = 0.0;
        for &x in &xs {
            acc += f(black_box(x));
        }
        black_box(acc);
        println!("{name:<14} {:>6.1} ns", t.elapsed().as_secs_f64() * 1e9 / n as f64);
    };
    time("libm::pow", &|x| libm::pow(x, 1.1977));
    time("std powf", &|x| x.powf(1.1977));
    time("libm exp(ln)", &|x| libm::exp(1.1977 * libm::log(x)));
    time("libm::log", &|x| libm::log(x));
    time("libm::exp", &|x| libm::exp(x));
    time("sqrt", &|x| x.sqrt());
}
