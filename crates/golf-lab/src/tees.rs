//! Hole lengths per par and tee set over a folder of bundles: the table behind the baseline carries
//! (docs/gameplay/assists.md). "middle" is a hole's `middle` set, else the mean of `middle_back` and
//! `middle_forward`, else the mean of its longest and shortest tees.

use std::collections::BTreeMap;
use std::path::Path;

use golf_core::math::M_PER_YD;
use golf_course::Course;

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q).round() as usize]
}

pub fn run(dir: &Path) -> Result<(), String> {
    let mut by: BTreeMap<(String, u8), Vec<f64>> = BTreeMap::new();
    let mut totals: Vec<(String, f64)> = Vec::new();
    let mut ents: Vec<_> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).filter(|p| p.join("manifest.json").is_file()).collect();
    ents.sort();
    for p in ents {
        let c = Course::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut total = 0.0;
        for h in &c.holes {
            let set = |n: &str| h.tees.iter().find(|t| t.set == n).map(|t| t.yardage_m);
            for t in &h.tees {
                by.entry((t.set.clone(), h.par)).or_default().push(t.yardage_m);
            }
            let ys: Vec<f64> = h.tees.iter().map(|t| t.yardage_m).collect();
            let mid = set("middle")
                .or_else(|| Some(0.5 * (set("middle_back")? + set("middle_forward")?)))
                .unwrap_or_else(|| 0.5 * (ys.iter().cloned().fold(f64::MIN, f64::max) + ys.iter().cloned().fold(f64::MAX, f64::min)));
            by.entry(("=middle".into(), h.par)).or_default().push(mid);
            total += mid;
        }
        totals.push((c.manifest.course_id.clone(), total));
    }
    println!("{:<16} {:>3} {:>4} {:>8} {:>8} {:>8} {:>14}", "tee set", "par", "n", "p25 yd", "med yd", "p75 yd", "range yd");
    for ((set, par), v) in by.iter_mut() {
        let (lo, hi) = (pct(v, 0.0), pct(v, 1.0));
        println!(
            "{:<16} {:>3} {:>4} {:>8.0} {:>8.0} {:>8.0} {:>6.0}-{:<7.0}",
            set,
            par,
            v.len(),
            pct(v, 0.25) / M_PER_YD,
            pct(v, 0.5) / M_PER_YD,
            pct(v, 0.75) / M_PER_YD,
            lo / M_PER_YD,
            hi / M_PER_YD
        );
    }
    println!("\n=middle course totals:");
    for (id, t) in totals {
        println!("  {id}: {:.0} yd", t / M_PER_YD);
    }
    Ok(())
}
