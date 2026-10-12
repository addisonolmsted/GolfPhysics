//! Our height function against the generator's reference reader (`course_bundle::reader`), at 100k random points
//! per bundle, including the tier seams. The M1 gate: no difference above 1e-9 m.
//!
//! The reference reader evaluates the three C0 tiers (play 0.5 m, core 2 m, full 8 m) and not the surround, so
//! points where it has no answer are counted, not compared.

use std::path::Path;

use golf_course::Course;

pub fn run(dir: &Path) -> Result<(), String> {
    let ours = Course::open(dir).map_err(|e| e.to_string())?;
    let theirs = course_bundle::reader::Bundle::open(dir)?;
    let th = theirs.heights()?;
    let b = ours.manifest.bounds;
    let mut s: u64 = 0x2545f4914f6cdd1d ^ ours.manifest.seed;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut n, mut worst, mut only_ours, mut only_theirs, mut tier_diff) = (0u64, 0.0f64, 0u64, 0u64, 0u64);
    let mut seam = 0u64;
    for k in 0..100_000 {
        // a third of the points on tile seams (multiples of 255 nodes of the 0.5 m tier) and tier edges
        let (mut x, mut y) = (b.x0 - 100.0 + (b.x1 - b.x0 + 200.0) * rnd(), b.y0 - 100.0 + (b.y1 - b.y0 + 200.0) * rnd());
        if k % 3 == 0 {
            x = (x / 127.5).round() * 127.5 + (rnd() - 0.5) * 2.0;
            seam += 1;
        }
        if k % 3 == 1 {
            y = (y / 127.5).round() * 127.5 + (rnd() - 0.5) * 2.0;
            seam += 1;
        }
        match (ours.heights.at(x, y), th.at(x, y)) {
            (Some((a, ta)), Some((bz, tb))) => {
                n += 1;
                worst = worst.max((a - bz).abs());
                tier_diff += (ta != tb) as u64;
            }
            (Some((_, t)), None) => {
                // the reference reader stops at full_8m; ours goes on to the surround (tier 3)
                if t != 3 {
                    return Err(format!("({x}, {y}): ours answers from tier {t}, the reference reader has nothing"));
                }
                only_ours += 1;
            }
            (None, Some(_)) => only_theirs += 1,
            (None, None) => {}
        }
    }
    println!(
        "{}: {n} points compared ({seam} near seams), max |diff| {worst:.3e} m, {tier_diff} answered by a different tier, {only_ours} from the surround only (ours), {only_theirs} only theirs",
        ours.manifest.course_id
    );
    if worst > 1e-9 || only_theirs > 0 || tier_diff > 0 {
        return Err("conformance failed".into());
    }
    Ok(())
}
