//! M3 follow-up: how receptive should greens be? Release (yards after landing) of every green-landing shot by the
//! green's turf tilt.
use golf_core::{Air, LaunchConditions, Vec2};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, GroundMaterials, TestPlane};
fn main() {
    let green = TestPlane::flat(SurfaceClass::GREEN);
    let shots = [
        ("PGA 7i", 120.0, 16.3, 7097.0, "2..5"),
        ("AMA 7i", 106.0, 17.5, 6500.0, "4..10"),
        ("PGA PW", 102.0, 24.2, 9304.0, "-3..1"),
        ("AMA PW", 90.0, 26.0, 8400.0, "0..3"),
        ("check", 85.0, 24.0, 9500.0, "-1.5..1.5"),
        ("lowspin", 80.0, 26.0, 7000.0, "1..6"),
        ("lob", 65.0, 32.0, 10500.0, "<0"),
        ("chip", 34.0, 30.0, 3000.0, "bump&run"),
    ];
    print!("{:>5} |", "tilt");
    for s in &shots { print!(" {:>8}", s.0); }
    println!();
    print!("{:>5} |", "want");
    for s in &shots { print!(" {:>8}", s.4); }
    println!();
    for tilt in [15.4, 18.0, 21.0, 24.0, 28.0] {
        let mut m = GroundMaterials::builtin();
        let mut g = m.get(SurfaceClass::GREEN).clone();
        g.tilt_deg = tilt;
        g.tilt_max_deg = tilt * 2.0;
        m.set(SurfaceClass::GREEN, g);
        print!("{tilt:>5.1} |");
        for (_, mph, la, rpm, _) in shots {
            let b = BallModel::new(&green, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
            let o = b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
            print!(" {:>8.1}", (o.rest.x - o.first_contact.unwrap().x) / 0.9144);
        }
        println!();
    }
}
