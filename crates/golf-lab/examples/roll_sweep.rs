//! M3 tuning: fairway rolling resistance a = c0 + c1 v against TrackMan's amateur driver (carry 204, total 226 yd:
//! 22 yd of roll on its normalised fairway).
use golf_core::{Air, LaunchConditions, Vec2};
use golf_course::SurfaceClass;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, GroundMaterials, TestPlane};
fn main() {
    let fairway = TestPlane::flat(SurfaceClass::FAIRWAY);
    let shots = [("AMA dr", 132.6, 12.6, 3275.0), ("PGA dr", 167.0, 10.9, 2686.0), ("LPGA dr", 140.0, 13.2, 2611.0), ("70mph dr", 97.0, 15.0, 3200.0), ("PGA 7i", 120.0, 16.3, 7097.0)];
    print!("{:>5} {:>5} {:>5} |", "tilt", "c0", "c1");
    for s in &shots { print!(" {:>9}", s.0); }
    println!("   (roll, yd)");
    for tilt in [12.0, 18.0, 24.0, 30.0] {
    for c0 in [0.8, 1.2] {
        for c1 in [0.2, 0.45] {
            let mut m = GroundMaterials::builtin();
            let mut f = m.get(SurfaceClass::FAIRWAY).clone();
            f.roll_decel = c0;
            f.roll_c1 = c1;
            f.tilt_deg = tilt;
            f.tilt_max_deg = tilt * 2.0;
            m.set(SurfaceClass::FAIRWAY, f);
            print!("{tilt:>5.0} {c0:>5.2} {c1:>5.2} |");
            for (_, mph, la, rpm) in shots {
                let b = BallModel::new(&fairway, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
                let o = b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
                print!(" {:>9.1}", (o.total_m() - o.carry_m()) / 0.9144);
            }
            println!();
        }
    }
    }
}
