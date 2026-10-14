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

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
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
    let path: Vec<Value> = o.samples.iter().map(|s| json!([r3(s.p.x), r3(s.p.y), (s.p.z * 1000.0).round() / 1000.0, (s.t * 1000.0).round() / 1000.0])).collect();
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
                EventKind::Flagstick => "flagstick",
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
        "cup": [d, 0.0], "cups": [{"r": golf_core::ball::CUP_RADIUS_M, "fill": true}, {"r": golf_core::ball::GAME_CUP_RADIUS_M, "fill": false}],
        "cup_label": "10-inch game cup (dashed), regulation cup (solid)",
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

/// a putt along +x on a flat green, its line `b` from the centre of a cup 1 m ahead, reaching the hole at `v`
fn putt_at_cup(m: &GroundMaterials, cup_r: f64, cap: Option<f64>, b: f64, v: f64) -> Outcome {
    let green = golf_ground::TestPlane::flat(C::GREEN);
    let d = 1.0;
    let a = m.get(C::GREEN).base_decel();
    let cup = Cup { xy: Vec2::new(d, 0.0), z: 0.0, radius_m: cup_r, capture_cap_mps: cap, flagstick: None };
    model(&green, m, Some(cup)).roll_from(Vec2::new(0.0, b), Vec3::new((v * v + 2.0 * a * (d - cup_r)).sqrt(), 0.0, 0.0))
}

fn lip_outs(m: &GroundMaterials) -> Value {
    let r = golf_core::ball::GAME_CUP_RADIUS_M;
    let cap = golf_ground::GAME_CAPTURE_CAP_MPS;
    let d = 1.0;
    // (label, line offset as a share of the radius, speed at the hole, should it drop, what to expect)
    let cases: [(&str, f64, f64, bool, &str); 7] = [
        ("dead weight, centre, 0.4 m/s", 0.0, 0.4, true, "drops in"),
        ("firm, centre, 1.5 m/s", 0.0, 1.5, true, "hits the back wall and drops"),
        ("too firm, centre, 2.4 m/s", 0.0, 2.4, false, "dips, kicks off the far edge and hops out"),
        ("dying at the edge, 90 % out, 0.35 m/s", 0.9, 0.35, true, "topples in the side door"),
        ("firm at the edge, 92 % out, 1.4 m/s", 0.92, 1.4, false, "rides the rim and is turned away"),
        ("half the cup, 50 % out, 1.9 m/s", 0.5, 1.9, false, "catches the far side of the rim and spins out"),
        ("just past the edge, 105 % out, 0.6 m/s", 1.05, 0.6, false, "misses: the rim never takes the ball's weight"),
    ];
    let mut shots = Vec::new();
    for (label, f, v, drop, what) in cases {
        let o = putt_at_cup(m, r, cap, f * r, v);
        let holed = o.state == RestState::Holed;
        let after = o.samples.iter().rev().find(|s| s.v.xy().len() > 0.05 && s.p.x > d);
        let turn = after.map_or(0.0, |s| s.v.y.atan2(s.v.x).to_degrees());
        let text = if holed {
            format!("holed: {what}")
        } else {
            format!("{} the ball left heading {turn:+.0}° off its line and stopped {:.2} m from the hole ({what})", if has(&o, EventKind::LipOut) { "lipped out:" } else { "missed:" }, (o.rest.xy() - Vec2::new(d, 0.0)).len())
        };
        shots.push(shot_json(label, &o, Some((holed == drop, text))));
    }
    let (x0, x1) = (d - 0.3, d + 0.5);
    let depth = golf_ground::cup::CUP_DEPTH_M;
    let profile = json!([[x0, 0.0, 1], [d - r, 0.0, 0], [d - r, -depth, 0], [d + r, -depth, 0], [d + r, 0.0, 1], [x1, 0.0, 1]]);
    json!({"id": "lip", "title": "Lip-outs", "units": "m", "topview": true, "slowmo": true,
        "question": "What does the cup do with putts of different pace and line: dead weight, firm, too firm, and catching the edge?",
        "expect": "The ball is a sphere meeting the rim, the liner wall and the bottom. A slow ball over the hole falls in, even off the edge; a firm one falls far enough to strike the back wall below the rim and drops; a too-fast one hasn't fallen far enough by the far side, so the edge throws it up and out. Off-centre, the rim pushes the ball sideways: a firm putt is turned away (the classic lip-out), a dying one topples in. The game's 10-inch cup with the 2x ball lets a putt on line drop up to about 2.0 m/s.",
        "cup": [d, 0.0], "cups": [{"r": r, "fill": true}], "cup_label": "10-inch game cup",
        "window": {"x": [x0, x1], "y": [-0.16, 0.16], "z": [-0.115, 0.08]},
        "profile": profile, "profile_note": "Section through the cup's centre; off-centre balls cross a narrower part of the hole.",
        "shots": shots})
}

/// H holed, L lipped out, . missed, by speed at the hole and line offset, for three cups
/// the angle a ball's centre sweeps about the cup centre (degrees) while it is near the rim
fn swept_deg(o: &Outcome, cup: Vec2, near: f64) -> f64 {
    let mut swept = 0.0;
    let mut last: Option<f64> = None;
    for s in &o.samples {
        let p = s.p.xy() - cup;
        if p.len() < near {
            let a = p.y.atan2(p.x);
            if let Some(l) = last {
                let mut da = a - l;
                while da > std::f64::consts::PI {
                    da -= 2.0 * std::f64::consts::PI;
                }
                while da < -std::f64::consts::PI {
                    da += 2.0 * std::f64::consts::PI;
                }
                swept += da;
            }
            last = Some(a);
        } else {
            last = None;
        }
    }
    swept.abs().to_degrees()
}

fn rim_rides(m: &GroundMaterials) -> Value {
    let r = golf_core::ball::GAME_CUP_RADIUS_M;
    let cap = golf_ground::GAME_CAPTURE_CAP_MPS;
    let d = 1.0;
    // found by examples/rim_ride.rs on a flat green: (label, line offset as a share of the radius, speed at the hole, drops)
    let cases: [(&str, f64, f64, bool); 4] = [
        ("all the way round and in: 89 % out, 0.80 m/s", 0.8875, 0.80, true),
        ("swirls inside the lip and drops: 61 % out, 1.80 m/s", 0.61, 1.80, true),
        ("nearly a full lap, then out: 70 % out, 1.55 m/s", 0.70, 1.55, false),
        ("three-quarters round and out: 85 % out, 0.90 m/s", 0.85, 0.90, false),
    ];
    let mut shots = Vec::new();
    for (label, f, v, drop) in cases {
        let o = putt_at_cup(m, r, cap, f * r, v);
        let holed = o.state == RestState::Holed;
        let sw = swept_deg(&o, Vec2::new(d, 0.0), r + golf_core::ball::CONTACT_RADIUS_M);
        let text = format!("went {sw:.0}° around the cup, then {}", if holed { "dropped".to_string() } else { format!("spun out and stopped {:.2} m away", (o.rest.xy() - Vec2::new(d, 0.0)).len()) });
        shots.push(shot_json(label, &o, Some((holed == drop && sw > 180.0, text))));
    }
    json!({"id": "rim", "title": "Around the rim", "units": "m", "topview": true, "slowmo": true,
        "question": "Do putts ride the rim, sometimes most of the way round, and then drop or spin out?",
        "expect": "Nothing here is scripted. A ball that catches the rim on an edge line is held against it: the edge pushes it towards the centre, which bends its path round the cup while friction slows it. If it slows enough while hanging over the hole it falls in; if it is still fast where the rim turns away, it spins out. A fast ball can also drop part-way and swirl round inside the lip. On a flat green about 1 in 40 edge putts (lines 55-100 % out) go more than half-way round; on a side slope it is far more common for balls entering on the high side, which gravity holds against the rim.",
        "cup": [d, 0.0], "cups": [{"r": r, "fill": true}], "cup_label": "10-inch game cup",
        "window": {"x": [d - 0.3, d + 0.35], "y": [-0.24, 0.24], "z": [-0.115, 0.08]},
        "profile": Value::Null, "shots": shots})
}

fn flagstick(m: &GroundMaterials) -> Value {
    let d = 6.0;
    let r = golf_core::ball::GAME_CUP_RADIUS_M;
    // a green of radius 4 m round the hole, fairway (the fringe) outside it
    let t = FnTerrain { h: |_x: f64, _y: f64| 0.0, class: move |x: f64, y: f64| if (x - d).hypot(y) < 4.0 { C::GREEN } else { C::FAIRWAY } };
    let game = Cup::game(Vec2::new(d, 0.0), 0.0);
    let forced_out = Cup { flagstick: Some(false), ..game };
    // a roll from x0 reaching the hole's x at v (searched on the same ground with no hole)
    let roll_to = |x0: f64, v: f64, cup: Cup| {
        let (mut lo, mut hi) = (0.1, 8.0);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if speed_at_x(&model(&t, m, None).roll_from(Vec2::new(x0, 0.0), Vec3::new(mid, 0.0, 0.0)), d) < v { lo = mid } else { hi = mid }
        }
        model(&t, m, Some(cup)).roll_from(Vec2::new(x0, 0.0), Vec3::new(0.5 * (lo + hi), 0.0, 0.0))
    };
    // a pitch from x0 whose first touchdown (no hole) is at x_land, found by its speed
    let pitch_to = |x0: f64, y: f64, la: f64, rpm: f64, x_land: f64, cup: Cup| {
        let (mut lo, mut hi) = (5.0, 90.0);
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            let o = model(&t, m, None).play(Vec2::new(x0, y), &LaunchConditions::from_monitor(mid, la, 0.0, rpm, 0.0));
            if o.first_contact.map_or(0.0, |c| c.x) < x_land { lo = mid } else { hi = mid }
        }
        model(&t, m, Some(cup)).play(Vec2::new(x0, y), &LaunchConditions::from_monitor(0.5 * (lo + hi), la, 0.0, rpm, 0.0))
    };
    let hit_pin = |o: &Outcome| has(o, EventKind::Flagstick);
    let mut shots = Vec::new();
    let mut add = |label: &str, o: Outcome, ok: bool, what: &str| {
        let end = match o.state {
            RestState::Holed => "holed".to_string(),
            _ => format!("stopped {:.2} m from the hole ({})", (o.rest.xy() - Vec2::new(d, 0.0)).len(), if o.rest.x < d { "short" } else { "past" }),
        };
        let pin = if hit_pin(&o) { "hit the flagstick, " } else { "" };
        shots.push(shot_json(label, &o, Some((ok, format!("{pin}{end}. {what}")))));
    };
    let o = roll_to(0.0, 2.6, game);
    let ok = o.state == RestState::Holed && hit_pin(&o);
    add("chip from the fringe, firm: 2.6 m/s at the hole, flagstick in", o, ok, "The pin takes the pace off and the ball drops.");
    let o = roll_to(3.0, 2.6, game);
    let ok = o.state != RestState::Holed && !hit_pin(&o);
    add("the same pace putted from the green, flagstick out", o, ok, "Over the 2.0 m/s cap with no pin to stop it: lips out.");
    let o = pitch_to(d - 30.0, 0.0, 34.0, 7000.0, d, game);
    let ok = o.state == RestState::Holed;
    add("pitch from 30 m dunked onto the pin", o, ok, "Comes down on the flagstick and falls into the cup.");
    let o = pitch_to(d - 30.0, 0.07, 34.0, 7000.0, d, game);
    let ok = o.state == RestState::Holed;
    add("pitch dunked 7 cm beside the pin", o, ok, "Straight into the gap between the pin and the liner: holed when it goes below the rim (or touches the bottom).");
    let o = pitch_to(d - 30.0, 0.0, 34.0, 7000.0, d, forced_out);
    let ok = o.state == RestState::Holed;
    add("the same dunk with the flagstick out (forced, for comparison)", o, ok, "Into the back wall below the rim: holed, no bounce-out.");
    let o = pitch_to(d - 30.0, 0.0, 34.0, 7000.0, d + r * 0.95, game);
    add("pitch aimed to land on the far lip", o, true, "Coming down at about 45°, it reaches the flagstick before the far lip; physics decides from there.");
    let o = pitch_to(d - 8.0, 0.0, 12.0, 3000.0, d + 3.0, game);
    let ok = hit_pin(&o);
    add("low chip flying at the pin, would land 3 m past", o, ok, "Hits the flagstick in the air and rebounds; the backspin it still carries now turns it back the way it came when it lands.");
    json!({"id": "pin", "title": "Flagstick and dunks", "units": "m", "topview": true, "slowmo": true,
        "question": "Does the flagstick stop a firm chip, do dunks drop, and is the flagstick in or out by the game's rule?",
        "expect": "The flagstick is in for any shot from off the green and out for any shot on it (D52). It is a 25 mm pole, 2.13 m tall, flexible and loose in its ferrule, so it soaks up most of the pace (restitution 0.12), so a firm chip that hits it loses most of its pace and drops. A ball coming down over the hole goes straight to the cup model: dunks onto the pin or beside it fall in, and touching the bottom is holed (D50). A ball can still be thrown out by the rim itself.",
        "cup": [d, 0.0], "cups": [{"r": r, "fill": true}], "cup_label": "10-inch game cup, flagstick in the middle", "pin": true,
        "window": {"x": [d - 1.6, d + 1.6], "y": [-0.6, 0.6], "z": [-0.12, 2.3]},
        "profile": json!([[d - 1.6, 0.0, 1], [d - r, 0.0, 1], [d - r, -golf_ground::cup::CUP_DEPTH_M, 0], [d + r, -golf_ground::cup::CUP_DEPTH_M, 0], [d + r, 0.0, 1], [d + 1.6, 0.0, 1]]),
        "profile_note": "Section through the cup's centre; the flagstick stands 2.13 m.",
        "shots": shots})
}

fn capture_map(m: &GroundMaterials) -> Value {
    use golf_core::ball::{CUP_RADIUS_M, GAME_CUP_RADIUS_M};
    let cups = [("Regulation 4.25 in", CUP_RADIUS_M, None), ("10 in, no cap", GAME_CUP_RADIUS_M, None), ("Game: 10 in, 2.0 m/s cap", GAME_CUP_RADIUS_M, golf_ground::GAME_CAPTURE_CAP_MPS)];
    let speeds: Vec<f64> = (0..16).map(|k| r2(0.2 + 0.2 * k as f64)).collect();
    let offsets: Vec<f64> = (0..20).map(|j| r2(0.05 * j as f64)).collect();
    let maps: Vec<Value> = cups
        .iter()
        .map(|&(name, r, cap)| {
            let rows: Vec<String> = speeds
                .iter()
                .map(|&v| {
                    offsets
                        .iter()
                        .map(|&f| {
                            let o = putt_at_cup(m, r, cap, f * r, v);
                            if o.state == RestState::Holed { 'H' } else if has(&o, EventKind::LipOut) { 'L' } else { '.' }
                        })
                        .collect()
                })
                .collect();
            json!({"name": name, "radius_mm": (r * 2000.0).round(), "rows": rows})
        })
        .collect();
    json!({"speeds": speeds, "offsets": offsets, "maps": maps})
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
    json!({"regulation_mm": 108, "game_mm": 254, "game_cap": golf_ground::GAME_CAPTURE_CAP_MPS, "rows": rows,
        "note": "The game draws the ball at 2x a real one, and the ball touches the cup, the ground and everything else at that size (D49), so what players see is what drops. The cup is 10 inches (254 mm), 3 balls across, against 2.5 for a real ball in a real cup (D51). Line forgiveness is the cup's radius: a putt drops when its centre crosses within 127 mm of the hole's centre at a dying pace. Pace is capped at 2.0 m/s through the centre, a putt heading about 3.8 m past on a stimp 10.5 green. Touching the bottom of the cup is holed (D50)."})
}

pub fn run() -> Result<(), String> {
    let m = GroundMaterials::builtin();
    let scenarios = vec![ridge(&m), bowl(&m), cliff(&m), face(&m), cross_slope(&m), lip_outs(&m), rim_rides(&m), flagstick(&m), wedges(&m), backstop(&m)];
    for s in &scenarios {
        println!("== {}", s["title"].as_str().unwrap_or(""));
        for sh in s["shots"].as_array().unwrap() {
            let v = &sh["verdict"];
            println!("   {:<52} {} {}", sh["label"].as_str().unwrap_or(""), if v["ok"].as_bool().unwrap_or(true) { "ok " } else { "XX " }, v["text"].as_str().unwrap_or(""));
        }
    }
    let mut cup = cup_table();
    cup["map"] = capture_map(&m);
    let data = json!({"generated": "2026-10-13", "scenarios": scenarios, "cup": cup});
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
        for s in [ridge(&m), bowl(&m), cliff(&m), face(&m), cross_slope(&m), lip_outs(&m), rim_rides(&m), flagstick(&m), wedges(&m), backstop(&m)] {
            for sh in s["shots"].as_array().unwrap() {
                let v = &sh["verdict"];
                assert!(v["ok"].as_bool().unwrap_or(true), "{} / {}: {}", s["title"], sh["label"], v["text"]);
            }
        }
    }
}
