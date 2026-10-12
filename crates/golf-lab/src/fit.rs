//! `fit-aero`: fit the aero coefficients to every target at once (docs/models/flight.md):
//! - the fit rows' carry (3 %), apex (10 %) and landing angle (3°), unverified fields at half weight;
//! - the extra targets (M2b): driver hang time, curve per degree of spin axis, the published 7-iron wind pair;
//! - "a tailwind never shortens a full shot" on every fit and sniff row, at 10 and 20 mph.
//!
//! Nelder-Mead, deterministic; the jobs are flown on a fixed pool of 4 threads and summed in job order.

use golf_flight::{AeroParams, Wind};
use serde_json::Value;

use crate::calib::{self, FitRow, M_PER_YD};

const N: usize = 10;
const NAMES: [&str; N] = ["cd0", "cd_s", "cd_s2", "cd_v", "cd_v_ref", "cl_max", "cl_s_half", "cl_p", "spin_decay_per_m", "spin_decay_per_s"];
const LO: [f64; N] = [0.10, -0.5, -3.0, -0.2, 20.0, 0.2, 0.05, 0.3, 0.0, 0.0];
const HI: [f64; N] = [0.40, 2.0, 3.0, 0.5, 60.0, 1.0, 1.0, 3.0, 0.0015, 0.15];
/// the parameters are moved in units where each is ~1
const SCALE: [f64; N] = [0.1, 0.3, 0.5, 0.1, 5.0, 0.1, 0.1, 0.2, 0.0003, 0.02];
const MPH: f64 = 0.44704;

fn to_params(x: &[f64; N]) -> AeroParams {
    AeroParams { cd0: x[0], cd_s: x[1], cd_s2: x[2], cd_v: x[3], cd_v_ref: x[4], cl_max: x[5], cl_s_half: x[6], cl_p: x[7], spin_decay_per_m: x[8], spin_decay_per_s: x[9] }
}

fn from_params(p: &AeroParams) -> [f64; N] {
    [p.cd0, p.cd_s, p.cd_s2, p.cd_v, p.cd_v_ref, p.cl_max, p.cl_s_half, p.cl_p, p.spin_decay_per_m, p.spin_decay_per_s]
}

/// one launch: ball mph, launch deg, spin rpm
type Launch = (f64, f64, f64);

#[derive(Clone, Debug)]
pub enum Job {
    /// a fit row: carry, apex, landing angle (and hang time where a target exists)
    Row { row: FitRow, hang: Option<(f64, f64)> },
    /// curve per degree of spin axis as a share of carry: (target %, tolerance fraction)
    Curve { id: String, l: Launch, pct: f64, tol: f64 },
    /// carry change into and down a wind: (mph, into yd, down yd, tolerance yd)
    Wind { id: String, l: Launch, mph: f64, into: f64, down: f64, tol: f64 },
    /// a tailwind must not shorten this shot
    Tail { id: String, l: Launch, mph: f64 },
}

/// what a job measured, for printing
#[derive(Clone, Debug)]
pub struct Outcome {
    pub cost: f64,
    pub text: String,
}

fn carry(a: &AeroParams, l: Launch, axis: f64, w: Wind) -> golf_flight::Flight {
    calib::fly(a, l.0, l.1, l.2, axis, w)
}

pub fn run_job(j: &Job, a: &AeroParams) -> Outcome {
    match j {
        Job::Row { row: r, hang } => {
            let f = carry(a, (r.ball_mph, r.launch_deg, r.spin_rpm), 0.0, Wind::CALM);
            let (c, h, l) = calib::measure(&f);
            let w = |field: &str| if r.is_verified(field) { 1.0 } else { 0.5 };
            let mut cost = w("carry") * ((c - r.carry_yd) / (0.03 * r.carry_yd)).powi(2);
            if let Some(ap) = r.apex_yd {
                cost += w("apex") * ((h - ap) / (0.10 * ap)).powi(2);
            }
            if let Some(la) = r.land_deg {
                cost += w("land") * ((l - la) / 3.0).powi(2);
            }
            let mut text = format!("{}: carry {c:.1}/{} apex {h:.1}/{:?} land {l:.1}/{:?}", r.id, r.carry_yd, r.apex_yd, r.land_deg);
            if let Some((t, tol)) = hang {
                let ht = f.time_s();
                cost += ((ht - t) / tol).powi(2);
                text += &format!(" hang {ht:.2}/{t}");
            }
            Outcome { cost, text }
        }
        Job::Curve { id, l, pct, tol } => {
            let f = carry(a, *l, 5.0, Wind::CALM);
            let got = 100.0 * (f.lateral_m() / 5.0) / f.carry_m();
            Outcome { cost: ((got - pct) / (tol * pct)).powi(2), text: format!("{id} curve: {got:.2} % of carry per deg (target {pct})") }
        }
        Job::Wind { id, l, mph, into, down, tol } => {
            let c0 = carry(a, *l, 0.0, Wind::CALM).carry_m() / M_PER_YD;
            let ci = carry(a, *l, 0.0, Wind::new(mph * MPH, std::f64::consts::PI)).carry_m() / M_PER_YD - c0;
            let cd = carry(a, *l, 0.0, Wind::new(mph * MPH, 0.0)).carry_m() / M_PER_YD - c0;
            Outcome { cost: ((ci - into) / tol).powi(2) + ((cd - down) / tol).powi(2), text: format!("{id} {mph} mph: into {ci:+.1} (target {into}), down {cd:+.1} (target {down})") }
        }
        Job::Tail { id, l, mph } => {
            let c0 = carry(a, *l, 0.0, Wind::CALM).carry_m();
            let cd = carry(a, *l, 0.0, Wind::new(mph * MPH, 0.0)).carry_m();
            let short = ((c0 - cd) / M_PER_YD).max(0.0);
            Outcome { cost: 100.0 * short * short, text: format!("{id} {mph} mph down: {:+.1} yd", (cd - c0) / M_PER_YD) }
        }
    }
}

/// every job the reference defines
pub fn jobs() -> Result<Vec<Job>, String> {
    let r = calib::load_reference()?;
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(calib::repo_root().join("data/calibration/reference.v1.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let x = &raw["extra_targets"];
    let row = |id: &str| r.fit_rows.iter().find(|f| f.id == id).cloned().ok_or_else(|| format!("extra target names unknown row {id}"));
    let launch = |f: &FitRow| (f.ball_mph, f.launch_deg, f.spin_rpm);
    let mut out = Vec::new();
    for f in &r.fit_rows {
        let hang = x["hang"].as_array().and_then(|a| a.iter().find(|h| h["row"] == f.id.as_str())).map(|h| (h["hang_s"].as_f64().unwrap(), h["tol_s"].as_f64().unwrap()));
        out.push(Job::Row { row: f.clone(), hang });
    }
    for c in x["curve"].as_array().cloned().unwrap_or_default() {
        let f = row(c["row"].as_str().unwrap_or(""))?;
        out.push(Job::Curve { id: f.id.clone(), l: launch(&f), pct: c["pct_of_carry_per_deg"].as_f64().unwrap(), tol: c["tol_frac"].as_f64().unwrap() });
    }
    for w in x["wind"].as_array().cloned().unwrap_or_default() {
        let f = row(w["row"].as_str().unwrap_or(""))?;
        out.push(Job::Wind { id: f.id.clone(), l: launch(&f), mph: w["mph"].as_f64().unwrap(), into: w["into_yd"].as_f64().unwrap(), down: w["down_yd"].as_f64().unwrap(), tol: w["tol_yd"].as_f64().unwrap() });
    }
    let mphs: Vec<f64> = x["tailwind_never_shortens"]["mph"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
    for mph in mphs {
        for f in &r.fit_rows {
            out.push(Job::Tail { id: f.id.clone(), l: launch(f), mph });
        }
        for s in &r.sniff_rows {
            out.push(Job::Tail { id: s.id.clone(), l: (s.ball_mph, s.launch_deg, s.spin_rpm), mph });
        }
    }
    Ok(out)
}

pub fn run_all(jobs: &[Job], a: &AeroParams) -> Vec<Outcome> {
    let mut out: Vec<Option<Outcome>> = vec![None; jobs.len()];
    let chunk = jobs.len().div_ceil(4);
    std::thread::scope(|sc| {
        for (js, os) in jobs.chunks(chunk).zip(out.chunks_mut(chunk)) {
            sc.spawn(move || {
                for (j, o) in js.iter().zip(os.iter_mut()) {
                    *o = Some(run_job(j, a));
                }
            });
        }
    });
    out.into_iter().map(|o| o.unwrap()).collect()
}

fn total_cost(jobs: &[Job], x: &[f64; N]) -> f64 {
    let mut pen = 0.0;
    for k in 0..N {
        if x[k] < LO[k] {
            pen += 1e3 * ((LO[k] - x[k]) / SCALE[k]).powi(2);
        }
        if x[k] > HI[k] {
            pen += 1e3 * ((x[k] - HI[k]) / SCALE[k]).powi(2);
        }
    }
    let clamped: [f64; N] = std::array::from_fn(|k| x[k].clamp(LO[k] - SCALE[k], HI[k] + SCALE[k]));
    let a = to_params(&clamped);
    // a weak prior keeping the total spin decay near a few % per second at 60 m/s
    let prior = ((x[9] + 60.0 * x[8] - 0.03) / 0.025).powi(2);
    run_all(jobs, &a).iter().map(|o| o.cost).sum::<f64>() + pen + prior
}

fn nelder_mead(jobs: &[Job], x0: [f64; N], max_evals: usize) -> ([f64; N], f64, usize) {
    let f = |x: &[f64; N]| total_cost(jobs, x);
    let mut simplex: Vec<([f64; N], f64)> = vec![(x0, f(&x0))];
    for k in 0..N {
        let mut x = x0;
        x[k] += 0.5 * SCALE[k];
        simplex.push((x, f(&x)));
    }
    let mut evals = N + 1;
    while evals < max_evals {
        simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
        if (simplex[N].1 - simplex[0].1).abs() < 1e-6 * (1.0 + simplex[0].1.abs()) {
            break;
        }
        let mut c = [0.0; N];
        for (x, _) in &simplex[..N] {
            for k in 0..N {
                c[k] += x[k] / N as f64;
            }
        }
        let worst = simplex[N].0;
        let pt = |t: f64| -> [f64; N] { std::array::from_fn(|k| c[k] + t * (worst[k] - c[k])) };
        let xr = pt(-1.0);
        let fr = f(&xr);
        evals += 1;
        if fr < simplex[0].1 {
            let xe = pt(-2.0);
            let fe = f(&xe);
            evals += 1;
            simplex[N] = if fe < fr { (xe, fe) } else { (xr, fr) };
        } else if fr < simplex[N - 1].1 {
            simplex[N] = (xr, fr);
        } else {
            let xc = if fr < simplex[N].1 { pt(-0.5) } else { pt(0.5) };
            let fc = f(&xc);
            evals += 1;
            if fc < fr.min(simplex[N].1) {
                simplex[N] = (xc, fc);
            } else {
                let best = simplex[0].0;
                for s in simplex.iter_mut().skip(1) {
                    s.0 = std::array::from_fn(|k| best[k] + 0.5 * (s.0[k] - best[k]));
                    s.1 = f(&s.0);
                    evals += 1;
                }
            }
        }
    }
    simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
    (simplex[0].0, simplex[0].1, evals)
}

/// `only`: fit just the jobs whose id contains it; `dry`: print, don't write the data file
pub fn run(only: Option<&str>, dry: bool) -> Result<(), String> {
    let all = jobs()?;
    let jobs: Vec<Job> = all
        .into_iter()
        .filter(|j| {
            only.is_none_or(|o| match j {
                Job::Row { row, .. } => row.id.contains(o),
                Job::Curve { id, .. } | Job::Wind { id, .. } | Job::Tail { id, .. } => id.contains(o),
            })
        })
        .collect();
    let start = AeroParams::builtin();
    let t = std::time::Instant::now();
    let c0 = total_cost(&jobs, &from_params(&start));
    let (mut x, mut cost, mut evals) = nelder_mead(&jobs, from_params(&start), 2500);
    // restarts from the best point: Nelder-Mead stalls in long valleys
    for _ in 0..3 {
        let (x2, c2, e2) = nelder_mead(&jobs, x, 2500);
        evals += e2;
        if c2 < cost - 1e-6 {
            x = x2;
            cost = c2;
        } else {
            break;
        }
    }
    let a = to_params(&x);
    println!("fit: {} jobs, cost {c0:.2} -> {cost:.2} in {evals} evaluations, {:.1} s", jobs.len(), t.elapsed().as_secs_f64());
    for k in 0..N {
        println!("  {:<18} {:.5}", NAMES[k], x[k]);
    }
    let outcomes = run_all(&jobs, &a);
    let worst: Vec<_> = {
        let mut v: Vec<_> = outcomes.iter().filter(|o| o.cost > 1.0).collect();
        v.sort_by(|a, b| b.cost.total_cmp(&a.cost));
        v
    };
    println!("jobs costing more than 1 (outside tolerance):");
    for o in &worst {
        println!("  {:6.2}  {}", o.cost, o.text);
    }
    if dry || only.is_some() {
        return Ok(());
    }
    let residuals: Vec<Value> = outcomes.iter().map(|o| serde_json::json!({"cost": (o.cost * 100.0).round() / 100.0, "what": o.text})).collect();
    let out = serde_json::json!({
        "version": 1,
        "status": "fitted (M2b)",
        "fitted": "2026-10-12",
        "command": "cargo run --release -p golf-lab -- fit-aero",
        "reference": "data/calibration/reference.v1.json (fit_rows, extra_targets, and every sniff row for the tailwind rule)",
        "objective": "fit rows: ((carry err) / 3 %)^2 + ((apex err) / 10 %)^2 + ((land err) / 3 deg)^2, unverified fields at half weight; driver hang time / 0.25 s; curve per degree / 25 %; wind deltas / 3 yd; 100 x (yards a tailwind shortens a shot)^2; a weak prior on spin decay",
        "objective_value": (cost * 100.0).round() / 100.0,
        "model": "Cd = cd0 + cd_s S + cd_s2 S^2 + cd_v min(0, cd_v_ref / |u| - 1); Cl = cl_max S^p / (cl_s_half^p + S^p); dω/dt = -(spin_decay_per_s + spin_decay_per_m |u|) ω  (S = r |ω⊥| / |u|, u = ball velocity relative to the air)",
        "params": serde_json::to_value(a).unwrap(),
        "residuals": residuals,
        "air": "TrackMan normal: sea level, 23.9 C, 75 % humidity"
    });
    let p = calib::repo_root().join("data/aero/ball.v1.json");
    std::fs::write(&p, serde_json::to_string_pretty(&out).unwrap() + "\n").map_err(|e| e.to_string())?;
    println!("wrote {}", p.display());
    Ok(())
}
