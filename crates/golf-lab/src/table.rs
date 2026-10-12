//! `table`: the model against every reference row (fit rows with their targets, sniff rows with their bands), with
//! the aero coefficients the engine is built with.

use golf_flight::{AeroParams, Wind};

use crate::calib;

pub fn run() -> Result<(), String> {
    let r = calib::load_reference()?;
    let a = AeroParams::builtin();
    println!("aero: {a:?}\n");
    println!("{:<9} {:<17} {:<7} | {:>5} {:>5} {:>5} | {:>6} {:>6} {:>5} | {:>5} {:>5} {:>5} | {:>4}", "id", "group", "club", "mph", "ball", "launch", "carry", "ref", "err%", "apex", "ref", "land", "ref");
    let mut worst = 0.0f64;
    for row in &r.fit_rows {
        let f = calib::fly(&a, row.ball_mph, row.launch_deg, row.spin_rpm, 0.0, Wind::CALM);
        let (c, h, l) = calib::measure(&f);
        let e = 100.0 * (c - row.carry_yd) / row.carry_yd;
        worst = worst.max(e.abs());
        println!(
            "{:<9} {:<17} {:<7} | {:>5.0} {:>5.0} {:>5.1} | {:>6.1} {:>6.0} {:>+5.1} | {:>5.1} {:>5} {:>5.1} | {:>4}",
            row.id,
            row.group,
            row.club,
            row.club_mph,
            row.ball_mph,
            row.launch_deg,
            c,
            row.carry_yd,
            e,
            h,
            row.apex_yd.map_or("-".into(), |v| format!("{v:.0}")),
            l,
            row.land_deg.map_or("-".into(), |v| format!("{v:.0}"))
        );
    }
    println!("worst carry error on the fit rows: {worst:.1} %\n");
    let mut fails = 0;
    println!("sniff rows (never fitted; launch conditions assumed, see the reference file)");
    for s in &r.sniff_rows {
        let f = calib::fly(&a, s.ball_mph, s.launch_deg, s.spin_rpm, 0.0, Wind::CALM);
        let (c, h, l) = calib::measure(&f);
        let ok = c >= s.carry_band_yd[0] && c <= s.carry_band_yd[1];
        fails += !ok as u32;
        println!(
            "{:<10} {:<19} {:<7} {:>4.0} mph club -> carry {:>5.1} yd (band {:.0}-{:.0}) apex {:>4.1} land {:>4.1}  {}",
            s.id,
            s.group,
            s.club,
            s.club_mph,
            c,
            s.carry_band_yd[0],
            s.carry_band_yd[1],
            h,
            l,
            if ok { "ok" } else { "OUT OF BAND" }
        );
        if !ok {
            println!("           basis: {}", s.basis);
        }
    }
    if fails > 0 { Err(format!("{fails} sniff rows out of band")) } else { Ok(()) }
}
