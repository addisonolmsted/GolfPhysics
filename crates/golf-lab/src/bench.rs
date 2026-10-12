//! Load time, resident raster size and the cost of the height queries, on one bundle. Release builds only give
//! meaningful numbers: `cargo run --release -p golf-lab -- bench <dir>`.

use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use golf_core::Vec2;
use golf_course::{Course, LoadOptions};

pub fn run(dir: &Path) -> Result<(), String> {
    let mut best = (f64::MAX, f64::MAX, f64::MAX);
    let mut course = None;
    for verify in [true, true, true, false, false, false] {
        let t = Instant::now();
        let c = Course::open_with(dir, LoadOptions { verify }).map_err(|e| e.to_string())?;
        let ms = t.elapsed().as_secs_f64() * 1e3;
        if verify {
            best.0 = best.0.min(ms);
        } else {
            best.1 = best.1.min(ms);
        }
        best.2 = best.2.min(c.load_stats.decode_ms);
        course = Some(c);
    }
    let c = course.unwrap();
    println!("{}", c.manifest.course_id);
    println!("  load (best of 3): {:.1} ms with blake3 verify, {:.1} ms without; decode alone {:.1} ms", best.0, best.1, best.2);
    println!("  rasters resident: {:.1} MB", c.raster_bytes() as f64 / 1e6);

    // query points: along every line of play, jittered across a 120 m corridor (where shots go)
    let mut s: u64 = 0x9e3779b97f4a7c15;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut pts = Vec::with_capacity(1 << 20);
    while pts.len() < 1 << 20 {
        let h = &c.holes[(rnd() * c.holes.len() as f64) as usize % c.holes.len()];
        let sp = &h.line_of_play.spline;
        let k = (rnd() * (sp.len() - 1) as f64) as usize;
        let t = rnd();
        let (a, b) = (Vec2::new(sp[k][0], sp[k][1]), Vec2::new(sp[k + 1][0], sp[k + 1][1]));
        pts.push(a + (b - a) * t + Vec2::new(rnd() - 0.5, rnd() - 0.5) * 120.0);
    }
    let n = pts.len() as f64;
    let t = Instant::now();
    let mut acc = 0.0;
    for p in &pts {
        acc += c.height(p.x, p.y).unwrap_or(0.0);
    }
    black_box(acc);
    let z_ns = t.elapsed().as_secs_f64() * 1e9 / n;
    let t = Instant::now();
    for p in &pts {
        acc += c.surface_at(p.x, p.y).map_or(0.0, |s| s.dzdx);
    }
    black_box(acc);
    let s_ns = t.elapsed().as_secs_f64() * 1e9 / n;
    let t = Instant::now();
    let mut k = 0u32;
    for p in &pts {
        k += c.surface(p.x, p.y).0 as u32;
    }
    black_box(k);
    let c_ns = t.elapsed().as_secs_f64() * 1e9 / n;
    let t = Instant::now();
    let mut ob = 0;
    for p in pts.iter().take(1 << 18) {
        ob += c.in_bounds(*p, golf_core::ball::RADIUS_M) as u32 + c.penalty_area(*p, golf_core::ball::RADIUS_M).is_some() as u32;
    }
    black_box(ob);
    let r_ns = t.elapsed().as_secs_f64() * 1e9 / (1 << 18) as f64;
    println!("  per query: height {z_ns:.0} ns, height + derivatives {s_ns:.0} ns, class {c_ns:.0} ns, OB + penalty {r_ns:.0} ns");
    println!("  (a 20k-step ground roll with one height + derivative query per step: {:.2} ms)", 20_000.0 * s_ns / 1e6);
    Ok(())
}
