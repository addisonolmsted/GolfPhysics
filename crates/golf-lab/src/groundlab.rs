//! `ground-lab`: the ground model on shaped terrain, scenario by scenario, each with what physics says should happen
//! and whether it did. Writes out/viewer/ground.html ("Ground Lab").

use golf_core::math::G_MPS2;
use golf_core::{Air, LaunchConditions, Vec2, Vec3};
use golf_course::SurfaceClass as C;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, Cup, EventKind, FnTerrain, GroundMaterials, Outcome, RestState, Terrain};
use serde_json::{Value, json};

use crate::calib;

const TEMPLATE: &str = include_str!("../assets/ground-viewer.html");
const YD: f64 = 0.9144;

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn smoothstep(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

fn model<'a, T: Terrain>(t: &'a T, m: &'a GroundMaterials, cup: Option<Cup>) -> BallModel<'a, T> {
    BallModel::new(t, m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, cup)
}

/// a shot's path [x, y, z], time series [t, speed, backspin rpm along +x travel, x], and events
fn shot_json(label: &str, o: &Outcome, verdict: Option<(bool, String)>) -> Value {
    let path: Vec<Value> = o.samples.iter().map(|s| json!([r2(s.p.x), r2(s.p.y), (s.p.z * 1000.0).round() / 1000.0, (s.t * 1000.0).round() / 1000.0])).collect();
    let land_t = o.events.iter().find(|e| matches!(e.kind, EventKind::Land { .. } | EventKind::StartSlide)).map_or(0.0, |e| e.t);
    let series: Vec<Value> = o
        .samples
        .iter()
        .filter(|s| s.t >= land_t - 0.05 && s.v != Vec3::ZERO || s.phase == golf_ground::Phase::Rest)
        .map(|s| {
            let dir = if s.v.xy().len() > 1e-6 { s.v.xy().norm() } else { Vec2::new(1.0, 0.0) };
            // backspin: the spin about the axis to the right of travel (right = travel x up)
            let right = Vec3::new(dir.y, -dir.x, 0.0);
            json!([r2(s.t - land_t), r2(s.v.len()), (s.w.dot(right) * 30.0 / std::f64::consts::PI).round(), r2(s.p.x)])
        })
        .collect();
    let events: Vec<Value> = o
        .events
        .iter()
        .filter(|e| !matches!(e.kind, EventKind::StartSlide))
        .map(|e| {
            let k = match e.kind {
                EventKind::Land { .. } => "land",
                EventKind::Bounce { .. } => "bounce",
                EventKind::StartRoll => "roll",
                EventKind::Liftoff => "airborne",
                EventKind::LipOut => "lip-out",
                EventKind::Holed => "holed",
                EventKind::Water => "water",
                EventKind::Plug => "plug",
                EventKind::Rest => "rest",
                EventKind::OffWorld => "off world",
                EventKind::Timeout => "timeout",
                EventKind::StartSlide => "slide",
            };
            json!({"t": r2(e.t), "x": r2(e.p.x), "y": r2(e.p.y), "z": r2(e.p.z), "kind": k})
        })
        .collect();
    json!({
        "label": label, "path": path, "series": series, "events": events,
        "rest": [r2(o.rest.x), r2(o.rest.y), r2(o.rest.z)], "state": format!("{:?}", o.state),
        "carry": r2(o.carry_m()), "total": r2(o.total_m()),
        "verdict": verdict.map(|(ok, text)| json!({"ok": ok, "text": text}))
    })
}

/// the ground under the x axis, [x, z, class]
fn profile<T: Terrain>(t: &T, x0: f64, x1: f64, step: f64) -> Value {
    let n = ((x1 - x0) / step).ceil() as usize;
    Value::Array(
        (0..=n)
            .map(|k| {
                let x = x0 + k as f64 * step;
                json!([r2(x), (t.surface_at(x, 0.0).map_or(0.0, |s| s.z) * 1000.0).round() / 1000.0, t.class_at(x, 0.0).0])
            })
            .collect(),
    )
}

fn has(o: &Outcome, k: EventKind) -> bool {
    o.events.iter().any(|e| e.kind == k)
}

/// the speed of the last sample before x
fn speed_at_x(o: &Outcome, x: f64) -> f64 {
    o.samples.iter().take_while(|s| s.p.x < x).last().map_or(0.0, |s| s.v.len())
}

fn ridge(m: &GroundMaterials) -> Value {
    let (a, w, xc) = (0.8, 2.5, 15.0);
    let t = FnTerrain { h: move |x: f64, _y: f64| a * golf_core::math::exp(-((x - xc) / w).powi(2)), class: |_, _| C::GREEN };
    let v_crit = (G_MPS2 * w * w / (2.0 * a)).sqrt();
    let shots: Vec<Value> = [4.5, 6.5, 9.0, 12.0]
        .iter()
        .map(|&v0| {
            let o = model(&t, m, None).roll_from(Vec2::ZERO, Vec3::new(v0, 0.0, 0.0));
            let vc = speed_at_x(&o, xc - 0.3);
            let air = has(&o, EventKind::Liftoff);
            let reached = o.samples.iter().any(|s| s.p.x > xc);
            let ok = if !reached { !air } else if air { vc > 0.85 * v_crit } else { vc < 1.15 * v_crit };
            let text = if !reached {
                "didn't reach the crest; rolled back".to_string()
            } else {
                format!("{:.1} m/s at the crest: {}", vc, if air { "went airborne" } else { "stayed on the ground" })
            };
            shot_json(&format!("{v0:.1} m/s"), &o, Some((ok, text)))
        })
        .collect();
    json!({"id": "ridge", "title": "Rolling over a ridge", "units": "m",
        "question": "Does a fast ball leave the ground over a crest, and a slow one stay on it? (A green-speed surface, so friction doesn't hide the curvature.)",
        "expect": format!("A ball leaves the ground where the crest curves away faster than gravity can pull it down: speed² × curvature > g. This ridge (0.8 m high, 2.5 m wide, curvature 0.26 /m at the top) throws a ball moving faster than {v_crit:.1} m/s at the crest."),
        "profile": profile(&t, -1.0, 40.0, 0.25), "shots": shots})
}

fn bowl(m: &GroundMaterials) -> Value {
    let t = FnTerrain { h: |x: f64, _y: f64| 0.6 * (1.0 - golf_core::math::exp(-((x - 10.0) / 3.0).powi(2))), class: |_, _| C::GREEN };
    let shots: Vec<Value> = [(7.5, 0.0), (6.0, 0.0), (10.0, 2.5)]
        .iter()
        .map(|&(x0, v0)| {
            let o = model(&t, m, None).roll_from(Vec2::new(x0, 0.0), Vec3::new(v0, 0.0, 0.0));
            let off = (o.rest.x - 10.0).abs();
            let ok = o.state == RestState::OnGround && off < 0.6;
            shot_json(
                &if v0 == 0.0 { format!("released at rest at x = {x0}") } else { format!("{v0} m/s from the bottom") },
                &o,
                Some((ok, format!("came to rest {off:.2} m from the bottom after {:.1} s", o.time_s))),
            )
        })
        .collect();
    json!({"id": "bowl", "title": "A ball in a hollow", "units": "m",
        "question": "Does a ball rock back and forth in a hollow and settle at the bottom?",
        "expect": "On a green-speed surface, a ball released on the side of a 0.6 m hollow rolls through the bottom, climbs the far side a little less high each time (rolling resistance), and settles at the bottom. It shouldn't stop part-way up a side steeper than the green holds (about 7 %).",
        "profile": profile(&t, 0.0, 20.0, 0.1), "shots": shots})
}

fn cliff(m: &GroundMaterials) -> Value {
    let t = FnTerrain { h: |x: f64, _y: f64| -2.0 * smoothstep((x - 10.0) / 1.0), class: |_, _| C::GREEN };
    let v_crit = (G_MPS2 / 12.0).sqrt();
    let shots: Vec<Value> = [1.5, 2.5, 4.0, 7.0]
        .iter()
        .map(|&v0| {
            let o = model(&t, m, None).roll_from(Vec2::new(8.0, 0.0), Vec3::new(v0, 0.0, 0.0));
            let air = has(&o, EventKind::Liftoff);
            let vlip = speed_at_x(&o, 10.0);
            let ok = air == (vlip > v_crit);
            let land = o.events.iter().filter(|e| matches!(e.kind, EventKind::Bounce { .. })).next().map(|e| e.p.x);
            let text = format!(
                "{:.1} m/s at the lip: {}{}",
                vlip,
                if air { "flew off" } else { "rolled down the face" },
                land.map_or(String::new(), |x| format!(", first touched down at x = {x:.1} m"))
            );
            shot_json(&format!("{v0:.1} m/s"), &o, Some((ok, text)))
        })
        .collect();
    json!({"id": "cliff", "title": "Rolling off a drop", "units": "m",
        "question": "Does a ball go airborne off the lip of a steep drop and land below?",
        "expect": format!("The lip of this 2 m drop curves away at 12 /m, so anything faster than {v_crit:.1} m/s at the lip leaves the ground and lands down the slope; slower than that, it rolls over the edge and down the face."),
        "profile": profile(&t, 7.0, 30.0, 0.1), "shots": shots})
}

fn face(m: &GroundMaterials) -> Value {
    let t = FnTerrain { h: |x: f64, _y: f64| 1.2 * smoothstep((x - 10.0) / 0.8), class: |x, _| if x < 10.0 { C::GREEN } else { C::ROUGH } };
    let need = (2.0 * G_MPS2 * 1.2 * 1.4).sqrt();
    let mut shots: Vec<Value> = [3.0, 5.0, 6.5, 8.0, 11.0]
        .iter()
        .map(|&v0| {
            let o = model(&t, m, None).roll_from(Vec2::new(6.0, 0.0), Vec3::new(v0, 0.0, 0.0));
            let over = o.rest.x > 10.8;
            let ok = if v0 < need { !over } else { true };
            let text = if over { format!("climbed the face and stopped on top at x = {:.1} m", o.rest.x) } else { format!("couldn't climb it; rolled back to x = {:.1} m", o.rest.x) };
            shot_json(&format!("{v0:.1} m/s"), &o, Some((ok, text)))
        })
        .collect();
    // a pitch flown into the face
    let o = model(&t, m, None).play(Vec2::new(-14.0, 0.0), &LaunchConditions::from_monitor(44.0, 18.0, 0.0, 4500.0, 0.0));
    let ok = o.rest.x < 10.8 && o.samples.iter().all(|s| s.p.z > t.surface_at(s.p.x, s.p.y).map_or(-9.0, |g| g.z) - 0.05);
    shot_json_into(&mut shots, "a pitch flown into the face", &o, Some((ok, format!("rests at x = {:.1} m, {}", o.rest.x, if o.rest.x < 10.8 { "at the foot of the face" } else { "on top" }))));
    json!({"id": "face", "title": "Into a steep face", "units": "m",
        "question": "Does a ball rolling at a steep face (1.2 m up over 0.8 m, up to 66°: like a bunker lip or a cliff wall) climb it only when fast enough, and does a pitch into the face rebound without passing through?",
        "expect": format!("Climbing 1.2 m takes at least {need:.1} m/s of rolling speed before any friction (rolling balls need g h (1 + k)); slower balls roll back. (The approach is a green-speed surface; the face is rough.) A ball flown into the face bounces off it and drops at its foot."),
        "profile": profile(&t, -15.0, 20.0, 0.1), "shots": shots})
}

fn shot_json_into(v: &mut Vec<Value>, label: &str, o: &Outcome, verdict: Option<(bool, String)>) {
    v.push(shot_json(label, o, verdict));
}

/// y where the path crosses x (linear between samples)
fn y_at_x(o: &Outcome, x: f64) -> Option<f64> {
    o.samples.windows(2).find(|w| w[0].p.x <= x && w[1].p.x >= x).map(|w| {
        let f = if w[1].p.x > w[0].p.x { (x - w[0].p.x) / (w[1].p.x - w[0].p.x) } else { 0.0 };
        w[0].p.y + f * (w[1].p.y - w[0].p.y)
    })
}

fn cross_slope(m: &GroundMaterials) -> Value {
    let slope = 0.02;
    let d = 6.0;
    let t = FnTerrain { h: move |_x: f64, y: f64| slope * y, class: |_, _| C::GREEN };
    let putt = |mats: &GroundMaterials, aim: f64, cup: Option<Cup>| {
        let a = mats.get(C::GREEN).base_decel();
        let v0 = (2.0 * a * (d + 0.4)).sqrt();
        model(&t, mats, cup).roll_from(Vec2::ZERO, Vec3::new(v0 * golf_core::math::cos(aim), v0 * golf_core::math::sin(aim), 0.0))
    };
    // the aim that passes through the hole's centre, by bisection on the angle
    let solve = |mats: &GroundMaterials| {
        let (mut lo, mut hi) = (0.0, 0.25);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            match y_at_x(&putt(mats, mid, None), d) {
                Some(y) if y < 0.0 => lo = mid,
                Some(_) => hi = mid,
                None => hi = mid,
            }
        }
        0.5 * (lo + hi)
    };
    let mut shots = Vec::new();
    let straight = putt(m, 0.0, Some(Cup::regulation(Vec2::new(d, 0.0), 0.0)));
    let miss = y_at_x(&straight, d).unwrap_or(f64::NAN);
    shots.push(shot_json("aimed straight at the hole (regulation cup)", &straight, Some((miss < -0.02, format!("broke {:.0} cm downhill by the hole: {:?}", -miss * 100.0, straight.state)))));
    let mut breaks = Vec::new();
    for stimp in [8.0, 10.5, 13.0] {
        let mut mm = GroundMaterials::builtin();
        let mut g = mm.get(C::GREEN).clone();
        g.stimp_ft = Some(stimp);
        mm.set(C::GREEN, g);
        let aim = solve(&mm);
        let o = putt(&mm, aim, Some(Cup::regulation(Vec2::new(d, 0.0), 0.0)));
        let above = d * golf_core::math::tan(aim);
        breaks.push(above);
        shots.push(shot_json(&format!("read for stimp {stimp}: aim {:.0} cm above", above * 100.0), &o, Some((o.state == RestState::Holed, format!("{:?}", o.state)))));
    }
    let game = putt(m, 0.0, Some(Cup::game(Vec2::new(d, 0.0), 0.0)));
    shots.push(shot_json("aimed straight, the 10-inch game cup", &game, Some((true, format!("{:?}", game.state)))));
    let monotone = breaks.windows(2).all(|w| w[1] > w[0]);
    json!({"id": "cross", "title": "Break on a cross slope", "units": "m", "topview": true,
        "question": "Does a putt across a slope curve downhill, and does a faster green need more break?",
        "expect": format!("On a 2 % side slope, a 6 m putt struck to finish 0.4 m past the hole curves downhill. Faster greens need more break, because the putt is struck more softly and spends longer on the slope. Here the aim above the hole grows with stimp: {} ({}).",
            breaks.iter().map(|b| format!("{:.0} cm", b * 100.0)).collect::<Vec<_>>().join(", "), if monotone { "as expected" } else { "NOT increasing" }),
        "cup": [d, 0.0], "cup_r": [golf_core::ball::CUP_RADIUS_M, golf_core::ball::GAME_CUP_RADIUS_M],
        "profile": Value::Null, "shots": shots, "slope": {"dir": "falls towards -y", "grade": slope}})
}

fn wedges(m: &GroundMaterials) -> Value {
    let t = FnTerrain { h: |_x: f64, _y: f64| 0.0, class: |_, _| C::GREEN };
    let cases = [
        ("Tour low spinner: 80 mph, 26°, 7,000 rpm", 80.0, 26.0, 7000.0, "grabs, then releases into a slow roll: 1-6 yd"),
        ("Tour check shot: 85 mph, 24°, 9,500 rpm", 85.0, 24.0, 9500.0, "checks hard: within about 1.5 yd of where it lands"),
        ("Tour full PW: 102 mph, 24°, 9,300 rpm", 102.0, 24.2, 9304.0, "checks and spins back"),
        ("Lob wedge: 65 mph, 32°, 10,500 rpm", 65.0, 32.0, 10500.0, "checks: within about 1.5 yd (slow lobs rarely spin back)"),
    ];
    let shots: Vec<Value> = cases
        .iter()
        .map(|&(label, mph, la, rpm, expect)| {
            let o = model(&t, m, None).play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
            let roll = (o.rest.x - o.first_contact.map_or(0.0, |c| c.x)) / YD;
            let ok = match rpm as i64 {
                7000 => (1.0..=6.0).contains(&roll),
                9304 => roll < 0.0,
                _ => roll.abs() <= 1.5,
            };
            shot_json(label, &o, Some((ok, format!("{expect}. Rolled {roll:+.1} yd after landing"))))
        })
        .collect();
    json!({"id": "wedges", "title": "Check, release and spin-back", "units": "m", "series": true,
        "question": "Do tour wedges grab the green the way they do on TV: a low spinner that checks and trickles, a check shot that stops, a full wedge that spins back?",
        "expect": "Watch speed and spin after landing. A check shows as speed collapsing at a bounce while the backspin is still high; the release is the slow roll after the spin has turned into topspin (negative backspin). Spin-back is backspin left over after the ball stops moving forward.",
        "profile": profile(&t, 0.0, 130.0, 1.0), "shots": shots})
}

fn backstop(m: &GroundMaterials) -> Value {
    let flat = FnTerrain { h: |_x: f64, _y: f64| 0.0, class: |_, _| C::GREEN };
    let cases = [("Tour full PW: 102 mph, 24°, 9,300 rpm", 102.0, 24.2, 9304.0), ("Tour check shot: 85 mph, 24°, 9,500 rpm", 85.0, 24.0, 9500.0)];
    let mut shots = Vec::new();
    let mut prof = Value::Null;
    for (k, &(label, mph, la, rpm)) in cases.iter().enumerate() {
        let launch = LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0);
        let land = model(&flat, m, None).play(Vec2::ZERO, &launch).first_contact.map_or(80.0, |c| c.x);
        // the slope starts 3 m short of where the shot lands, rising to 12 %
        let x0 = land - 3.0;
        let t = FnTerrain {
            h: move |x: f64, _y: f64| {
                let w = 1.0;
                0.12 * w * golf_core::math::ln(1.0 + golf_core::math::exp((x - x0) / w))
            },
            class: |_, _| C::GREEN,
        };
        let o = model(&t, m, None).play(Vec2::ZERO, &launch);
        let landed = o.first_contact.map_or(0.0, |c| c.x);
        let back = landed - o.rest.x;
        let ok = back > 0.3;
        shots.push(shot_json(label, &o, Some((ok, format!("landed on the slope, checked, and finished {back:+.1} m back down from where it landed")))));
        if k == 0 {
            prof = profile(&t, land - 15.0, land + 10.0, 0.1);
        }
    }
    json!({"id": "backstop", "title": "A wedge into a backstop", "units": "m", "series": true,
        "question": "Does a high-spinning wedge that lands on a slope behind the pin check and roll back down?",
        "expect": "The green rises to a 12 % slope starting 3 m short of where each shot lands. The ball should check on the slope, then its leftover backspin and the slope bring it back down past where it landed. (Profile shown for the full PW.)",
        "profile": prof, "shots": shots})
}

fn cup_table() -> Value {
    let rows: Vec<Value> = [0.0, 0.25, 0.5, 0.75, 0.9]
        .iter()
        .map(|&f| {
            json!({"offset_frac": f,
                "regulation": r2(golf_ground::capture_speed_for(f * golf_core::ball::CUP_RADIUS_M, golf_core::ball::CUP_RADIUS_M)),
                "game": r2(golf_ground::capture_speed_for(f * golf_core::ball::GAME_CUP_RADIUS_M, golf_core::ball::GAME_CUP_RADIUS_M))})
        })
        .collect();
    json!({"regulation_mm": 108, "game_mm": 254, "rows": rows,
        "note": "The fastest a ball can cross the cup and drop, by how far its line passes from the centre (as a share of the radius). A 10-inch cup accepts any putt on line up to about 3.9 m/s, so pace hardly matters unless the speed is capped. For scale: a putt that would finish 0.4 m past crosses the hole at about 0.65 m/s on a stimp 10.5 green; 2 m past, about 1.45 m/s."})
}

pub fn run() -> Result<(), String> {
    let m = GroundMaterials::builtin();
    let scenarios = vec![ridge(&m), bowl(&m), cliff(&m), face(&m), cross_slope(&m), wedges(&m), backstop(&m)];
    for s in &scenarios {
        println!("== {}", s["title"].as_str().unwrap_or(""));
        for sh in s["shots"].as_array().unwrap() {
            let v = &sh["verdict"];
            println!("   {:<52} {} {}", sh["label"].as_str().unwrap_or(""), if v["ok"].as_bool().unwrap_or(true) { "ok " } else { "XX " }, v["text"].as_str().unwrap_or(""));
        }
    }
    let data = json!({"generated": "2026-10-13", "scenarios": scenarios, "cup": cup_table()});
    let html = TEMPLATE.replacen("/*__DATA__*/null", &data.to_string(), 1);
    let out = calib::repo_root().join("out/viewer");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let p = out.join("ground.html");
    std::fs::write(&p, html).map_err(|e| e.to_string())?;
    println!("wrote {} ({:.0} KB)", p.display(), std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every Ground Lab shot must behave as its scenario predicts: liftoff over a crest and a drop, settling in a
    /// hollow, a face that a pitch can't bounce back off, cross-slope break, checks and spin-back.
    #[test]
    fn every_ground_lab_shot_behaves_as_predicted() {
        let m = GroundMaterials::builtin();
        for s in [ridge(&m), bowl(&m), cliff(&m), face(&m), cross_slope(&m), wedges(&m), backstop(&m)] {
            for sh in s["shots"].as_array().unwrap() {
                let v = &sh["verdict"];
                assert!(v["ok"].as_bool().unwrap_or(true), "{} / {}: {}", s["title"], sh["label"], v["text"]);
            }
        }
    }
}
