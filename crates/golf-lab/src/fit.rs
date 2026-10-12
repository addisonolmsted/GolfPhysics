//! `fit-aero`: fit the aero coefficients to every fit row at once, on carry, apex and landing angle together
//! (fitting carry alone is degenerate: docs/models/flight.md). Nelder-Mead, deterministic; rows are flown on a
//! fixed pool of 4 threads and summed in row order.

use golf_flight::{AeroParams, Wind};

use crate::calib::{self, FitRow};

const N: usize = 8;
const NAMES: [&str; N] = ["cd0", "cd_s", "cd_v", "cl_max", "cl_p", "spin_decay_per_m", "cd_s2", "cl_s_half"];
const LO: [f64; N] = [0.10, -0.5, -0.2, 0.2, 0.3, 0.0001, -3.0, 0.05];
const HI: [f64; N] = [0.40, 2.0, 0.3, 1.0, 3.0, 0.0015, 3.0, 1.0];
/// the parameters are fitted in units where each is ~1
const SCALE: [f64; N] = [0.1, 0.3, 0.1, 0.1, 0.2, 0.0003, 0.5, 0.1];

fn to_params(x: &[f64; N]) -> AeroParams {
    AeroParams { cd0: x[0], cd_s: x[1], cd_s2: x[6], cd_v: x[2], cl_max: x[3], cl_s_half: x[7], cl_p: x[4], spin_decay_per_m: x[5] }
}

fn from_params(p: &AeroParams) -> [f64; N] {
    [p.cd0, p.cd_s, p.cd_v, p.cl_max, p.cl_p, p.spin_decay_per_m, p.cd_s2, p.cl_s_half]
}

/// per-row weighted squared residuals: carry within 3 %, apex within 10 %, landing angle within 3 degrees;
/// unverified fields count half
pub fn row_cost(r: &FitRow, a: &AeroParams) -> (f64, (f64, f64, f64)) {
    let f = calib::fly(a, r.ball_mph, r.launch_deg, r.spin_rpm, 0.0, Wind::CALM);
    let (c, h, l) = calib::measure(&f);
    let w = |field: &str| if r.is_verified(field) { 1.0 } else { 0.5 };
    let mut cost = w("carry") * ((c - r.carry_yd) / (0.03 * r.carry_yd)).powi(2);
    if let Some(ap) = r.apex_yd {
        cost += w("apex") * ((h - ap) / (0.10 * ap)).powi(2);
    }
    if let Some(la) = r.land_deg {
        cost += w("land") * ((l - la) / 3.0).powi(2);
    }
    (cost, (c, h, l))
}

fn total_cost(rows: &[FitRow], x: &[f64; N]) -> f64 {
    let mut pen = 0.0;
    for k in 0..N {
        if x[k] < LO[k] {
            pen += 1e3 * ((LO[k] - x[k]) / SCALE[k]).powi(2);
        }
        if x[k] > HI[k] {
            pen += 1e3 * ((x[k] - HI[k]) / SCALE[k]).powi(2);
        }
    }
    let a = to_params(&std::array::from_fn(|k| if k == 2 || k == 1 || k == 6 { x[k] } else { x[k].max(1e-6) }));
    let mut costs = vec![0.0; rows.len()];
    std::thread::scope(|sc| {
        for (chunk_rows, chunk_out) in rows.chunks(rows.len().div_ceil(4)).zip(costs.chunks_mut(rows.len().div_ceil(4))) {
            sc.spawn(move || {
                for (r, o) in chunk_rows.iter().zip(chunk_out.iter_mut()) {
                    *o = row_cost(r, &a).0;
                }
            });
        }
    });
    // a weak prior keeping spin decay near a few % per second at 60 m/s (docs/models/flight.md)
    let prior = ((x[5] - 0.0005) / 0.0004).powi(2);
    costs.iter().sum::<f64>() + pen + prior
}

fn nelder_mead(rows: &[FitRow], x0: [f64; N], max_evals: usize) -> ([f64; N], f64, usize) {
    let f = |x: &[f64; N]| total_cost(rows, x);
    let mut simplex: Vec<([f64; N], f64)> = Vec::new();
    simplex.push((x0, f(&x0)));
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

/// `only`: fit just the rows whose id contains it; `dry`: print, don't write the data file
pub fn run(only: Option<&str>, dry: bool) -> Result<(), String> {
    let reference = calib::load_reference()?;
    let rows: Vec<_> = reference.fit_rows.into_iter().filter(|r| only.is_none_or(|o| r.id.contains(o))).collect();
    let start = AeroParams::builtin();
    let t = std::time::Instant::now();
    let c0 = total_cost(&rows, &from_params(&start));
    // two restarts from the previous best: Nelder-Mead stalls on long valleys
    let (mut x, mut cost, mut evals) = nelder_mead(&rows, from_params(&start), 1500);
    for _ in 0..2 {
        let (x2, c2, e2) = nelder_mead(&rows, x, 1500);
        evals += e2;
        if c2 < cost - 1e-9 {
            x = x2;
            cost = c2;
        } else {
            break;
        }
    }
    let a = to_params(&x);
    println!("fit: cost {c0:.2} -> {cost:.2} in {evals} evaluations, {:.1} s", t.elapsed().as_secs_f64());
    for k in 0..N {
        println!("  {:<18} {:.5}", NAMES[k], x[k]);
    }
    let mut table = Vec::new();
    for r in &rows {
        let (_, (c, h, l)) = row_cost(r, &a);
        table.push(serde_json::json!({
            "id": r.id, "carry_yd": [round1(c), r.carry_yd], "apex_yd": [round1(h), r.apex_yd], "land_deg": [round1(l), r.land_deg]
        }));
    }
    let out = serde_json::json!({
        "version": 1,
        "status": "fitted",
        "fitted": "2026-10-12",
        "command": "cargo run --release -p golf-lab -- fit-aero",
        "reference": "data/calibration/reference.v1.json (fit_rows)",
        "objective": "sum over fit rows of ((carry err) / 3 %)^2 + ((apex err) / 10 %)^2 + ((land err) / 3 deg)^2, unverified fields at half weight, plus a weak prior on spin decay; final value below",
        "objective_value": round1(cost * 100.0) / 100.0,
        "model": "Cd = cd0 + cd_s * S + cd_s2 * S^2 + cd_v * (36.5 / |u| - 1); Cl = cl_max * S^p / (cl_s_half^p + S^p); dω/dt = -spin_decay_per_m * |u| * ω  (S = r |ω⊥| / |u|, u = ball velocity relative to the air)",
        "params": {"cd0": x[0], "cd_s": x[1], "cd_s2": x[6], "cd_v": x[2], "cl_max": x[3], "cl_s_half": x[7], "cl_p": x[4], "spin_decay_per_m": x[5]},
        "residuals": table,
        "air": "TrackMan normal: sea level, 23.9 C, 75 % humidity"
    });
    if dry || only.is_some() {
        for r in &table {
            println!("  {r}");
        }
        return Ok(());
    }
    let p = calib::repo_root().join("data/aero/ball.v1.json");
    std::fs::write(&p, serde_json::to_string_pretty(&out).unwrap() + "\n").map_err(|e| e.to_string())?;
    println!("wrote {}", p.display());
    Ok(())
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
