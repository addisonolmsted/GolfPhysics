//! The generator's physics-starter smoke test, steps 1-5, on one bundle.

use std::path::Path;

use golf_core::{Vec2, ball::RADIUS_M};
use golf_course::{Course, SurfaceClass};

fn class_name(c: &Course, k: SurfaceClass) -> String {
    format!("{} {}", k.0, c.material(k).name)
}

/// points every `step` m along a polyline, with their distance from the start
fn walk(pts: &[[f64; 2]], step: f64) -> Vec<(f64, Vec2)> {
    let mut out = Vec::new();
    let mut s0 = 0.0;
    for w in pts.windows(2) {
        let (a, b) = (Vec2::new(w[0][0], w[0][1]), Vec2::new(w[1][0], w[1][1]));
        let l = a.dist(b);
        let mut s = (step - s0 % step) % step;
        while s < l {
            out.push((s0 + s, a + (b - a) * (s / l)));
            s += step;
        }
        s0 += l;
    }
    out
}

/// the first point of a grid over [lo, hi] that satisfies f
fn find(lo: Vec2, hi: Vec2, n: usize, f: impl Fn(Vec2) -> bool) -> Option<Vec2> {
    (0..n).flat_map(|j| (0..n).map(move |i| (i, j))).map(|(i, j)| {
        Vec2::new(lo.x + (hi.x - lo.x) * (i as f64 + 0.5) / n as f64, lo.y + (hi.y - lo.y) * (j as f64 + 0.5) / n as f64)
    }).find(|p| f(*p))
}

fn lie(c: &Course, p: Vec2) -> String {
    let s = c.surface_at(p.x, p.y);
    let k = c.surface(p.x, p.y);
    format!(
        "({:.2}, {:.2}) z {} slope {} class {} | in bounds {} | penalty {} | water {}",
        p.x,
        p.y,
        s.map_or("-".into(), |s| format!("{:.3}", s.z)),
        s.map_or("-".into(), |s| format!("{:.1}%", 100.0 * (s.dzdx * s.dzdx + s.dzdy * s.dzdy).sqrt())),
        class_name(c, k),
        c.in_bounds(p, RADIUS_M),
        c.penalty_area(p, RADIUS_M).map_or("-".into(), |a| format!("{} ({:?})", a.id, a.colour)),
        c.water_at(p).map_or("-".into(), |w| format!("surface {:.3} body {:?}", w.surface_m, w.body.map(|k| c.water.bodies[k].id.as_str()))),
    )
}

pub fn run(dir: &Path) -> Result<(), String> {
    // 1. open and verify
    let c = Course::open(dir).map_err(|e| e.to_string())?;
    let st = c.load_stats;
    println!("## 1. open + verify: {}", c.manifest.course_id);
    println!(
        "   {} physics files, {:.2} MB read + blake3 in {:.0} ms, decoded in {:.0} ms; rasters {:.1} MB resident",
        st.files,
        st.bytes_read as f64 / 1e6,
        st.read_ms,
        st.decode_ms,
        c.raster_bytes() as f64 / 1e6
    );
    println!(
        "   {} holes, par {}; {} instances ({} in play); {} water bodies; {} penalty areas; {} OB rings",
        c.holes.len(),
        c.manifest.par,
        c.instances.records.len(),
        c.instances.n_in_play(),
        c.water.bodies.len(),
        c.rules.penalty.len(),
        c.rules.ob.len()
    );

    // 2. walk hole 1's line of play
    let h = c.hole(1).ok_or("no hole 1")?;
    println!("\n## 2. hole 1 (par {}, {:.0} m) line of play every 5 m", h.par, h.length_m);
    println!("   {:>6}  {:>9} {:>9}  {:>8}  {:>6}  class", "s (m)", "x", "y", "z (m)", "slope");
    for (s, p) in walk(&h.line_of_play.spline, 5.0) {
        let sf = c.surface_at(p.x, p.y).ok_or_else(|| format!("no height at {p:?}"))?;
        println!(
            "   {s:6.1}  {:9.2} {:9.2}  {:8.3}  {:5.1}%  {}",
            p.x,
            p.y,
            sf.z,
            100.0 * (sf.dzdx * sf.dzdx + sf.dzdy * sf.dzdy).sqrt(),
            class_name(&c, c.surface(p.x, p.y))
        );
    }

    // 3. the green at the pin (rolling is M3)
    let pin = Vec2::new(h.green.pin.xy[0], h.green.pin.xy[1]);
    let m = c.material(c.surface(pin.x, pin.y));
    println!("\n## 3. hole 1 green at the pin");
    println!("   {}", lie(&c, pin));
    println!("   normal {:?}; material: {} stimp {:?} rolling_resistance {}", c.normal(pin.x, pin.y).unwrap(), m.name, m.physics.stimp, m.physics.rolling_resistance);
    println!("   (rolling a ball needs the ground model: M3)");

    // 4. OB and penalty tests
    println!("\n## 4. OB and penalty areas");
    let tee = h.tees[0].plate;
    println!("   hole 1 back tee: {}", lie(&c, Vec2::new(tee[0], tee[1])));
    println!("   world SW corner: {}", lie(&c, Vec2::new(5.0, 5.0)));
    let ob = c.rules.ob.iter().find(|o| o.inside_is_in_bounds).unwrap();
    let (lo, hi) = ob.region.area_bbox;
    let y = 0.5 * (lo.y + hi.y);
    let mut x = lo.x - 2.0;
    while ob.region.contains(Vec2::new(x, y)) == ob.region.contains(Vec2::new(lo.x - 2.0, y)) && x < hi.x {
        x += 0.001;
    }
    println!("   OB line crossing at y = {y:.1}, x ~ {x:.3}:");
    // x is within 1 mm of the line, outside to its left: in bounds until the whole ball is past the line
    for dx in [-1.0, -(RADIUS_M + 0.003), -(RADIUS_M - 0.003), -0.5 * RADIUS_M, 0.0, 0.5 * RADIUS_M, 1.0] {
        println!("     x {:+.4}: in bounds {}", dx, c.in_bounds(Vec2::new(x + dx, y), RADIUS_M));
    }
    for pa in &c.rules.penalty {
        let (lo, hi) = pa.region.area_bbox;
        if let Some(p) = find(lo, hi, 300, |p| pa.region.contains(p) && c.in_bounds(p, RADIUS_M)) {
            println!("   inside {} ({:?}, holes {:?}): {}", pa.id, pa.colour, pa.holes, lie(&c, p));
        }
    }

    // 5. landing in water and in bunkers
    println!("\n## 5. landing points");
    for b in &c.water.bodies {
        let (lo, hi) = b.region.area_bbox;
        if let Some(p) = find(lo, hi, 300, |p| b.region.contains(p) && b.region.boundary_dist(p, 3.0).is_infinite() && c.surface(p.x, p.y) == SurfaceClass::WATER && c.in_bounds(p, RADIUS_M)) {
            println!("   in {} ({}): {}", b.id, b.kind, lie(&c, p));
        }
    }
    let mut n = 0;
    for h in &c.holes {
        let found = walk(&h.line_of_play.spline, 2.0).into_iter().find_map(|(_, p)| {
            (-40..=40).map(|k| p + Vec2::new(0.0, k as f64)).chain((-40..=40).map(|k| p + Vec2::new(k as f64, 0.0))).find(|q| c.surface(q.x, q.y) == SurfaceClass::BUNKER_SAND)
        });
        if let Some(p) = found {
            println!("   bunker near hole {}'s line: {}", h.number, lie(&c, p));
            n += 1;
            if n == 3 {
                break;
            }
        }
    }
    if n == 0 {
        println!("   no bunker sand within 40 m of a line of play");
    }
    Ok(())
}
