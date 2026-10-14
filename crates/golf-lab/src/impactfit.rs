//! `fit-impact`: fit the club table's impact coefficients (restitution, spin factor, tour shaft lean) to TrackMan's
//! PGA Tour rows, then check the LPGA rows and the average amateur's driver with the same coefficients, and the
//! face/path shot shapes through the flight model. Writes data/clubs/standard.v1.json.

use golf_core::ClubDelivery;
use golf_flight::{AeroParams, Wind};
use golf_impact::{Club, ClubTable, Lie, impact};
use serde_json::Value;

use crate::calib::{self, FitRow};

const MPS_PER_MPH: f64 = 0.44704;

/// the club a reference row was hit with
pub fn club_id(club: &str) -> Option<&'static str> {
    Some(match club {
        "Driver" => "dr",
        "3-wood" | "3 wood" => "3w",
        "5-wood" | "5 wood" => "5w",
        "7-wood" | "7 wood" => "7w",
        "Hybrid" | "hybrid" => "3h",
        "3-iron" | "3 iron" => "3i",
        "4-iron" | "4 iron" => "4i",
        "5-iron" | "5 iron" => "5i",
        "6-iron" | "6 iron" => "6i",
        "7-iron" | "7 iron" => "7i",
        "8-iron" | "8 iron" => "8i",
        "9-iron" | "9 iron" => "9i",
        "PW" | "Pitching wedge" => "pw",
        _ => return None,
    })
}

/// (ball mph, launch deg, spin rpm) for a row's club speed and attack, at dynamic loft = static + lean
pub fn predict(table: &ClubTable, club: &Club, club_mph: f64, attack_deg: f64, lean_deg: f64) -> (f64, f64, f64) {
    let d = ClubDelivery::square(club_mph * MPS_PER_MPH, attack_deg, club.loft_deg + lean_deg);
    let i = impact(table, club, &d, 0.0, &Lie::CLEAN);
    (i.launch.ball_speed_mps / MPS_PER_MPH, i.launch.launch_rad.to_degrees(), i.launch.spin_radps * 30.0 / std::f64::consts::PI)
}

/// fit one club to one row exactly: lean from the launch angle, restitution from the ball speed, spin factor from
/// the spin, iterated (each moves the others a little)
fn fit_club(table: &ClubTable, club: &mut Club, r: &FitRow) {
    let attack = r.attack_deg.unwrap_or(0.0);
    for _ in 0..40 {
        let (mut lo, mut hi) = (-25.0, 25.0);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if predict(table, club, r.club_mph, attack, mid).1 < r.launch_deg { lo = mid } else { hi = mid }
        }
        club.tour_lean_deg = 0.5 * (lo + hi);
        let (ball, _, spin) = predict(table, club, r.club_mph, attack, club.tour_lean_deg);
        club.cor_ref = ((1.0 + club.cor_ref) * r.ball_mph / ball - 1.0).clamp(0.3, 0.95);
        club.spin_factor *= r.spin_rpm / spin;
    }
}

/// copy the fitted coefficients onto clubs without a row, from the nearest fitted club(s) of the right kind
fn fill(table: &mut ClubTable) {
    let get = |t: &ClubTable, id: &str| t.get(id).cloned().unwrap();
    let copy = |t: &mut ClubTable, to: &str, from: &Club| {
        let c = t.get_mut(to).unwrap();
        c.cor_ref = from.cor_ref;
        c.spin_factor = from.spin_factor;
        c.tour_lean_deg = from.tour_lean_deg;
    };
    let (w5, h3, pw) = (get(table, "5w"), get(table, "3h"), get(table, "pw"));
    let mut w7 = w5.clone();
    w7.cor_ref = 0.5 * (w5.cor_ref + h3.cor_ref);
    w7.spin_factor = 0.5 * (w5.spin_factor + h3.spin_factor);
    w7.tour_lean_deg = 0.5 * (w5.tour_lean_deg + h3.tour_lean_deg);
    copy(table, "7w", &w7);
    copy(table, "4h", &h3);
    for id in ["gw", "sw", "lw"] {
        copy(table, id, &pw);
    }
}

fn fit_all(table: &mut ClubTable, rows: &[FitRow]) {
    for r in rows.iter().filter(|r| r.group == "PGA Tour") {
        let Some(id) = club_id(&r.club) else { continue };
        let mut c = table.get(id).unwrap().clone();
        fit_club(table, &mut c, r);
        *table.get_mut(id).unwrap() = c;
    }
    fill(table);
}

/// the lean that gives a row's launch angle with this club (the player's own delivery)
fn lean_for(table: &ClubTable, c: &Club, r: &FitRow) -> f64 {
    let (mut lo, mut hi) = (-25.0, 25.0);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if predict(table, c, r.club_mph, r.attack_deg.unwrap_or(0.0), mid).1 < r.launch_deg { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

/// errors on the rows not fitted, each at the dynamic loft that gives its launch (a player's lean is the player's,
/// measured by the phone): (id, lean, ball speed %, spin %)
fn check(table: &ClubTable, rows: &[FitRow]) -> Vec<(String, f64, f64, f64)> {
    rows.iter()
        .filter(|r| r.group != "PGA Tour")
        .filter_map(|r| {
            let c = table.get(club_id(&r.club)?)?;
            let lean = lean_for(table, c, r);
            let (b, _, s) = predict(table, c, r.club_mph, r.attack_deg.unwrap_or(0.0), lean);
            Some((r.id.clone(), lean, 100.0 * (b / r.ball_mph - 1.0), 100.0 * (s / r.spin_rpm - 1.0)))
        })
        .collect()
}

fn is_wood(c: &Club) -> bool {
    matches!(c.kind, golf_impact::ClubKind::Wood | golf_impact::ClubKind::Hybrid)
}

/// the exponent families: 0 the driver, 1 the other woods and hybrids, 2 irons and wedges
fn family(c: &Club) -> usize {
    if c.id == "dr" { 0 } else if is_wood(c) { 1 } else { 2 }
}

fn set_exps(t: &mut ClubTable, exps: [f64; 3]) {
    for c in &mut t.clubs {
        c.spin_speed_exp = if c.kind == golf_impact::ClubKind::Putter { 0.0 } else { exps[family(c)] };
    }
}

/// spin rms (%) over the checked rows of one family
fn spin_rms(t: &ClubTable, rows: &[FitRow], fam: usize) -> f64 {
    let e: Vec<f64> = check(t, rows)
        .into_iter()
        .filter(|(id, ..)| rows.iter().find(|r| &r.id == id).and_then(|r| club_id(&r.club)).and_then(|c| t.get(c)).is_some_and(|c| family(c) == fam))
        .map(|x| x.3)
        .collect();
    (e.iter().map(|x| x * x).sum::<f64>() / e.len().max(1) as f64).sqrt()
}

pub fn run() -> Result<(), String> {
    let reference = calib::load_reference()?;
    let rows = &reference.fit_rows;
    for r in rows {
        if club_id(&r.club).is_none() {
            println!("(no club for row {} '{}')", r.id, r.club);
        }
    }
    // the one shared parameter, the restitution's fall with speed, from the slower LPGA ball speeds
    let mut best = (f64::MAX, 0.0);
    for k in 0..=40 {
        let slope = -0.008 + 0.0002 * k as f64;
        let mut t = ClubTable::builtin();
        t.cor_slope = slope;
        fit_all(&mut t, rows);
        let e = check(&t, rows);
        let rms = (e.iter().map(|x| x.2 * x.2).sum::<f64>() / e.len() as f64).sqrt();
        if rms < best.0 {
            best = (rms, slope);
        }
    }
    // How spin per degree of spin loft falls with impact speed, per family, from the slower rows (the tour rows
    // stay exact). The driver has its own: tour players strike it high on low-spin heads, so its tour row spins far
    // less than the LPGA and amateur drivers at their spin lofts.
    let mut exps = [0.0; 3];
    for (fam, name) in [(0, "driver"), (1, "other woods and hybrids"), (2, "irons and wedges")] {
        let mut b = (f64::MAX, 0.0);
        for k in 0..=30 {
            let x = 0.1 * k as f64;
            let mut t = ClubTable::builtin();
            t.cor_slope = best.1;
            let mut e = [0.0; 3];
            e[fam] = x;
            set_exps(&mut t, e);
            fit_all(&mut t, rows);
            let err = spin_rms(&t, rows, fam);
            if err < b.0 {
                b = (err, x);
            }
        }
        exps[fam] = b.1;
        println!("spin speed exponent, {name}: {:.1} (spin rms {:.1} %)", b.1, b.0);
    }
    let mut table = ClubTable::builtin();
    table.cor_slope = best.1;
    set_exps(&mut table, exps);
    fit_all(&mut table, rows);
    println!("cor_slope {:.4} per m/s (LPGA ball-speed rms {:.2} %)", best.1, best.0);
    fit_axis_gain(&mut table);
    println!("\nfitted on the PGA Tour rows:");
    println!("  club   loft  tour lean  dyn loft  cor@40  spin factor");
    for c in &table.clubs {
        println!("  {:<5} {:5.1}  {:+8.1}  {:8.1}  {:6.3}  {:6.2}", c.id, c.loft_deg, c.tour_lean_deg, c.loft_deg + c.tour_lean_deg, c.cor_ref, c.spin_factor);
    }
    println!("\nchecked with the same coefficients, at the dynamic loft that gives each row's launch: lean, ball speed, spin");
    let gate = |b: f64, s: f64| b.abs() <= 2.0 && s.abs() <= 10.0;
    for (id, lean, b, s) in check(&table, rows) {
        let tour = rows.iter().find(|r| r.id == id).and_then(|r| club_id(&r.club)).and_then(|c| table.get(c)).map_or(0.0, |c| c.tour_lean_deg);
        println!("  {id:<8} lean {lean:+5.1}° (tour {tour:+5.1}°)  {b:+5.1} %  {s:+6.1} %  {}", if gate(b, s) { "ok" } else { "outside ±2 % / ±10 %" });
    }
    // through the flight: carry against the table
    let aero = AeroParams::builtin();
    println!("\ncarry through the flight model (yd): table / launch from impact");
    for r in rows {
        let Some(c) = club_id(&r.club).and_then(|id| table.get(id)) else { continue };
        let (b, l, s) = predict(&table, c, r.club_mph, r.attack_deg.unwrap_or(0.0), c.tour_lean_deg);
        let f = calib::fly(&aero, b, l, s, 0.0, Wind::CALM);
        println!("  {:<8} {:5.0} / {:5.1}", r.id, r.carry_yd, calib::measure(&f).0);
    }
    write(&table)
}

/// Each club's axis gain: with its tour delivery and the face 1° open to the path, the flight should curve ~0.7 %
/// of carry per degree of the D-plane's spin axis (TrackMan's rule, docs/models/flight.md M2b).
fn fit_axis_gain(table: &mut ClubTable) {
    use golf_core::{Air, Vec2};
    use golf_flight::{Flat, Model};
    let ground = Flat(0.0);
    let m = Model::new(AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, &ground);
    // a typical tour speed per club, by loft, for the measurement
    let mph = |loft: f64| (118.0 - 0.75 * loft).clamp(70.0, 115.0);
    let attack = |c: &Club| if c.id == "dr" { -1.3 } else { -1.0 - 0.09 * c.loft_deg };
    println!("\naxis gain (curve per degree of D-plane axis → TrackMan's 0.7 % of carry):");
    let ids: Vec<String> = table.clubs.iter().filter(|c| c.kind != golf_impact::ClubKind::Putter).map(|c| c.id.clone()).collect();
    for id in ids {
        let mut c = table.get(&id).unwrap().clone();
        c.axis_gain = 1.0;
        let mut d = ClubDelivery::square(mph(c.loft_deg) * MPS_PER_MPH, attack(&c), c.loft_deg + c.tour_lean_deg);
        d.face_rad = 0.5f64.to_radians();
        d.path_rad = (-0.5f64).to_radians();
        let i = impact(table, &c, &d, 0.0, &Lie::CLEAN);
        let f = m.fly(Vec2::ZERO, &i.launch);
        let carry = f.carry_m();
        let side = -f.last.p.y - carry * (-i.launch.direction_rad).sin();
        let axis_deg = i.launch.spin_axis_rad.to_degrees();
        let target = 0.007 * carry * axis_deg;
        c.axis_gain = (target / side).clamp(0.3, 1.5);
        println!("  {:<4} axis {axis_deg:4.2}° per 1° face to path, curve {:4.1} → {:4.1} m, gain {:.2}", c.id, side, target, c.axis_gain);
        *table.get_mut(&id).unwrap() = c;
    }
}

/// write the fitted fields into the data file, leaving everything else as it is
fn write(table: &ClubTable) -> Result<(), String> {
    let p = calib::repo_root().join("data/clubs/standard.v1.json");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let r4 = |x: f64| (x * 10000.0).round() / 10000.0;
    v["cor_slope"] = Value::from(r4(table.cor_slope));
    for c in v["clubs"].as_array_mut().ok_or("no clubs")? {
        let id = c["id"].as_str().unwrap_or("").to_string();
        if let Some(t) = table.get(&id) {
            c["cor_ref"] = Value::from(r4(t.cor_ref));
            c["spin_factor"] = Value::from(r4(t.spin_factor));
            c["tour_lean_deg"] = Value::from((t.tour_lean_deg * 100.0).round() / 100.0);
            c["spin_speed_exp"] = Value::from(r4(t.spin_speed_exp));
            c["axis_gain"] = Value::from(r4(t.axis_gain));
        }
    }
    std::fs::write(&p, serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
    println!("\nwrote {}", p.display());
    Ok(())
}
