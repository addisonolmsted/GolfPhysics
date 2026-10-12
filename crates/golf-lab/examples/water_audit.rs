//! How well do water.json's rings and the class raster agree on what is wet? (M1 investigation)
use golf_core::Vec2;
use golf_course::{Course, SurfaceClass};
fn main() {
    let dir = std::env::args().nth(1).expect("bundle dir");
    let c = Course::open(std::path::Path::new(&dir)).unwrap();
    for (k, b) in c.water.bodies.iter().enumerate() {
        let (lo, hi) = b.region.area_bbox;
        let (mut inside, mut wet_in, mut inb) = (0u64, 0u64, 0u64);
        let mut y = lo.y.floor();
        while y <= hi.y {
            let mut x = lo.x.floor();
            while x <= hi.x {
                let p = Vec2::new(x + 0.5, y + 0.5);
                if b.region.contains(p) {
                    inside += 1;
                    wet_in += (c.surface(p.x, p.y) == SurfaceClass::WATER) as u64;
                    inb += c.in_bounds(p, 0.0) as u64;
                }
                x += 1.0;
            }
            y += 1.0;
        }
        println!("{} #{k} {} {}: {inside} m2 inside the ring, {:.1}% class water, {:.1}% in bounds, surface {:?}", c.manifest.course_id, b.id, b.kind, 100.0 * wet_in as f64 / inside as f64, 100.0 * inb as f64 / inside as f64, b.surface_m);
    }
    // water cells of the play raster outside every ring
    let bb = c.manifest.bounds;
    let (mut wet, mut wet_out) = (0u64, 0u64);
    let mut y = bb.y0;
    while y < bb.y1 {
        let mut x = bb.x0;
        while x < bb.x1 {
            let p = Vec2::new(x + 0.5, y + 0.5);
            if c.classes.play.at(p.x, p.y) == Some(9) {
                wet += 1;
                wet_out += c.water.bodies.iter().all(|b| !b.region.contains(p)) as u64;
            }
            x += 1.0;
        }
        y += 1.0;
    }
    println!("{}: {wet} m2 class water in the play raster, {wet_out} m2 of it outside every ring", c.manifest.course_id);
}
