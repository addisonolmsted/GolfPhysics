//! `view`: write the flight viewer (an HTML page with the data inlined) to out/viewer/flight.html. The page shows
//! every reference row and sniff row in the air (side and top views), the shape checks (spin, wind, curve,
//! altitude), and, when the fixtures are mounted, real shots over a fixture hole's terrain.

use std::path::Path;

use golf_core::{Air, LaunchConditions, Vec2};
use golf_course::Course;
use golf_flight::{AeroParams, Flight, Model, Wind};
use serde_json::{Value, json};

use crate::calib::{self, M_PER_YD};

const TEMPLATE: &str = include_str!("../assets/flight-viewer.html");

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// the flight as [downrange, lateral (+ right), height] in yards, relative to the launch point (samples at ~30 Hz)
fn path(f: &Flight) -> Value {
    let d = f.launch.v.xy().norm();
    let o = f.launch.p;
    let pts: Vec<Value> = f
        .samples
        .iter()
        .map(|s| {
            let off = s.p.xy() - o.xy();
            json!([r1(off.dot(d) / M_PER_YD), r1(-d.cross(off) / M_PER_YD), r1((s.p.z - o.z) / M_PER_YD)])
        })
        .collect();
    Value::Array(pts)
}

fn summary(f: &Flight) -> Value {
    json!({
        "carry": r1(f.carry_m() / M_PER_YD), "apex": r1(f.apex_m() / M_PER_YD), "land": r1(f.land_angle_rad().to_degrees()),
        "lateral": r1(f.lateral_m() / M_PER_YD), "time": r1(f.time_s()), "path": path(f)
    })
}

fn shapes(a: &AeroParams) -> Value {
    let spin: Vec<Value> = [1500.0, 2000.0, 2700.0, 3500.0, 4500.0, 6000.0]
        .iter()
        .map(|&s| json!({"label": format!("{s:.0} rpm"), "model": summary(&calib::fly(a, 167.0, 10.9, s, 0.0, Wind::CALM))}))
        .collect();
    let wind: Vec<Value> = [("10 m/s into", 10.0, std::f64::consts::PI), ("5 m/s into", 5.0, std::f64::consts::PI), ("calm", 0.0, 0.0), ("5 m/s down", 5.0, 0.0), ("10 m/s down", 10.0, 0.0), ("10 m/s from the left", 10.0, -std::f64::consts::FRAC_PI_2)]
        .iter()
        .map(|&(l, sp, dir)| json!({"label": l, "model": summary(&calib::fly(a, 120.0, 16.3, 7097.0, 0.0, Wind::new(sp, dir)))}))
        .collect();
    let curve: Vec<Value> = [-10.0, -5.0, -3.0, 0.0, 3.0, 5.0, 10.0]
        .iter()
        .map(|&ax| json!({"label": format!("{ax:+.0}°"), "model": summary(&calib::fly(a, 120.0, 16.3, 7097.0, ax, Wind::CALM))}))
        .collect();
    let alt: Vec<Value> = [("Sea level", 0.0), ("1,000 m", 1000.0), ("Denver, 1,600 m", 1600.0), ("2,400 m", 2400.0)]
        .iter()
        .map(|&(l, h)| {
            let g = golf_flight::Flat(0.0);
            let m = Model::new(*a, Air::at_altitude(23.9, h, 0.75), Wind::CALM, &g);
            json!({"label": l, "model": summary(&m.fly(Vec2::ZERO, &LaunchConditions::from_monitor(167.0, 10.9, 0.0, 2686.0, 0.0)))})
        })
        .collect();
    json!({
        "spin": {"title": "Spin on a tour driver", "note": "167 mph, 10.9° launch. Too little spin falls out of the sky; too much balloons and loses carry.", "rows": spin},
        "wind": {"title": "Wind on a tour 7 iron", "note": "120 mph, 16.3°, 7,097 rpm; wind at 10 m with a log height profile. Into the wind costs more than downwind gives.", "rows": wind},
        "curve": {"title": "Spin axis on a tour 7 iron", "note": "Positive tilt curves right (fade for a right-hander). No wind.", "rows": curve},
        "altitude": {"title": "Altitude on a tour driver", "note": "TrackMan measures about 2.5 yd per 1,000 ft for a 155 mph ball: roughly 5 % in Denver.", "rows": alt}
    })
}

/// the samples of a whole shot (air and ground) as [downrange, lateral (+ right), height] yards from the start
fn outcome_path(o: &golf_ground::Outcome, dir: Vec2) -> Value {
    let pts: Vec<Value> = o
        .samples
        .iter()
        .map(|s| {
            let off = s.p.xy() - o.start.xy();
            json!([r1(off.dot(dir) / M_PER_YD), r1(-dir.cross(off) / M_PER_YD), r1((s.p.z - o.start.z) / M_PER_YD)])
        })
        .collect();
    Value::Array(pts)
}

fn outcome_summary(o: &golf_ground::Outcome, dir: Vec2) -> Value {
    json!({
        "carry": r1(o.carry_m() / M_PER_YD), "total": r1(o.total_m() / M_PER_YD), "roll": r1((o.total_m() - o.carry_m()) / M_PER_YD),
        "bounces": o.bounces(), "state": format!("{:?}", o.state), "time": r1(o.time_s), "path": outcome_path(o, dir)
    })
}

/// landing, bounce and roll on uniform surfaces (golf-ground's test planes)
fn ground_demo(a: &AeroParams) -> Value {
    use golf_course::SurfaceClass as C;
    let mats = golf_ground::GroundMaterials::builtin();
    let cases: [(&str, C, f64, f64, f64); 12] = [
        ("PGA driver", C::FAIRWAY, 167.0, 10.9, 2686.0),
        ("Amateur driver", C::FAIRWAY, 132.6, 12.6, 3275.0),
        ("LPGA driver", C::FAIRWAY, 140.0, 13.2, 2611.0),
        ("70 mph driver", C::FAIRWAY, 97.0, 15.0, 3200.0),
        ("Amateur driver", C::ROUGH, 132.6, 12.6, 3275.0),
        ("PGA 7 iron", C::GREEN, 120.0, 16.3, 7097.0),
        ("Amateur 7 iron", C::GREEN, 106.0, 17.5, 6500.0),
        ("PGA PW", C::GREEN, 102.0, 24.2, 9304.0),
        ("Amateur PW", C::GREEN, 90.0, 26.0, 8400.0),
        ("Chip, 34 mph at 30°", C::GREEN, 34.0, 30.0, 3000.0),
        ("PGA 7 iron", C::BUNKER_SAND, 120.0, 16.3, 7097.0),
        ("Amateur driver", C::BUNKER_SAND, 132.6, 12.6, 3275.0),
    ];
    let rows: Vec<Value> = cases
        .iter()
        .map(|&(label, class, mph, la, rpm)| {
            let plane = golf_ground::TestPlane::flat(class);
            let b = golf_ground::BallModel::new(&plane, &mats, *a, Air::trackman_normal(), Wind::CALM, None);
            let o = b.play(Vec2::ZERO, &LaunchConditions::from_monitor(mph, la, 0.0, rpm, 0.0));
            json!({"label": label, "surface": mats.get(class).name, "model": outcome_summary(&o, Vec2::new(1.0, 0.0))})
        })
        .collect();
    json!({"title": "Bounce and roll", "note": "Each shot lands on a flat, uniform surface: the first contact, the bounces (turf give, restitution falling with impact speed, grip or slip with spin), then slide and roll to rest. The fairway is fitted to TrackMan's amateur driver (carry 204, total 226 yd); the green rolls at stimp 10.5.", "rows": rows})
}

/// a few shots from hole 1's back tee of a fixture, flown over the real ground
fn course_demo(a: &AeroParams) -> Option<Value> {
    let dir = std::env::var("GOLF_FIXTURES").unwrap_or_else(|_| "/Volumes/GolfOut/out/physics_fixtures/v1".into());
    let c = Course::open(&Path::new(&dir).join("course_4_7667961")).ok()?;
    let h = c.hole(1)?;
    let t = &h.tees[0];
    let start = Vec2::new(t.plate[0], t.plate[1]);
    let aim = Vec2::new(t.aim[0], t.aim[1]);
    let az = golf_core::math::atan2(aim.y - start.y, aim.x - start.x);
    let d = (aim - start).norm();
    let z0 = c.height(start.x, start.y)?;
    let mats = golf_ground::GroundMaterials::builtin();
    let cup = golf_ground::Cup { xy: Vec2::new(h.green.pin.xy[0], h.green.pin.xy[1]), z: h.green.pin.z };
    let ball = golf_ground::BallModel::new(&c, &mats, *a, Air::trackman_normal(), Wind::CALM, Some(cup));
    let shots = [("PGA Tour driver", 167.0, 10.9, 2686.0), ("Average amateur driver", 132.6, 12.6, 3275.0), ("Slow swinger driver", 112.0, 14.0, 3200.0), ("Average amateur 7 iron", 106.0, 17.5, 6500.0)];
    let mut out = Vec::new();
    for (label, mph, launch, spin) in shots {
        let o = ball.play(start, &LaunchConditions::from_monitor(mph, launch, az, spin, 0.0));
        let pts: Vec<Value> = o
            .samples
            .iter()
            .map(|s| {
                let off = s.p.xy() - start;
                json!([r1(off.dot(d) / M_PER_YD), r1(-d.cross(off) / M_PER_YD), r1((s.p.z - golf_core::ball::RADIUS_M - z0) / M_PER_YD)])
            })
            .collect();
        let land = o.first_contact.unwrap_or(o.rest);
        let rest_class = c.surface(o.rest.x, o.rest.y);
        out.push(json!({
            "label": label,
            "carry": r1(o.carry_m() / M_PER_YD),
            "total": r1(o.total_m() / M_PER_YD),
            "lands_on": c.material(c.surface(land.x, land.y)).name,
            "rests_on": c.material(rest_class).name,
            "state": format!("{:?}", o.state),
            "drop": r1((land.z - golf_core::ball::RADIUS_M - z0) / M_PER_YD),
            "path": pts
        }));
    }
    // the ground along the aim line, every 2 yd, out to 320 yd
    let profile: Vec<Value> = (0..=160)
        .filter_map(|k| {
            let s = k as f64 * 2.0 * M_PER_YD;
            let p = start + d * s;
            Some(json!([r1(s / M_PER_YD), r1((c.height(p.x, p.y)? - z0) / M_PER_YD), c.surface(p.x, p.y).0]))
        })
        .collect();
    let classes: Value = c.materials.iter().map(|m| (m.id.to_string(), json!(m.name))).collect::<serde_json::Map<_, _>>().into();
    Some(json!({"course": c.manifest.course_id, "hole": h.number, "par": h.par, "length_yd": r1(h.length_m / M_PER_YD), "tee": t.set, "shots": out, "profile": profile, "classes": classes}))
}

pub fn run() -> Result<(), String> {
    let r = calib::load_reference()?;
    let a = AeroParams::builtin();
    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    let mut worst: f64 = 0.0;
    let mut within3 = 0;
    for row in &r.fit_rows {
        let f = calib::fly(&a, row.ball_mph, row.launch_deg, row.spin_rpm, 0.0, Wind::CALM);
        let err = (f.carry_m() / M_PER_YD - row.carry_yd) / row.carry_yd * 100.0;
        worst = worst.max(err.abs());
        within3 += (err.abs() <= 3.0) as u32;
        let v = json!({
            "id": row.id, "kind": "fit", "club": row.club, "club_mph": row.club_mph, "ball_mph": row.ball_mph,
            "launch_deg": row.launch_deg, "spin_rpm": row.spin_rpm,
            "ref": {"carry": row.carry_yd, "apex": row.apex_yd, "land": row.land_deg},
            "verified": row.verified, "model": summary(&f)
        });
        match groups.iter_mut().find(|g| g.0 == row.group) {
            Some(g) => g.1.push(v),
            None => groups.push((row.group.clone(), vec![v])),
        }
    }
    let mut sniff_pass = 0;
    for row in &r.sniff_rows {
        let f = calib::fly(&a, row.ball_mph, row.launch_deg, row.spin_rpm, 0.0, Wind::CALM);
        let c = f.carry_m() / M_PER_YD;
        sniff_pass += (c >= row.carry_band_yd[0] - row.accepted_miss_yd && c <= row.carry_band_yd[1] + row.accepted_miss_yd) as u32;
        let v = json!({
            "id": row.id, "kind": "sniff", "club": row.club, "club_mph": row.club_mph, "ball_mph": row.ball_mph,
            "launch_deg": row.launch_deg, "spin_rpm": row.spin_rpm,
            "ref": {"band": row.carry_band_yd, "accepted_miss": row.accepted_miss_yd}, "basis": row.basis, "model": summary(&f), "group": row.group
        });
        let gname = "Handicap sniff tests".to_string();
        match groups.iter_mut().find(|g| g.0 == gname) {
            Some(g) => g.1.push(v),
            None => groups.push((gname, vec![v])),
        }
    }
    // the raw reference file, for the rule-of-thumb bands kept beside the revised ones
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(calib::repo_root().join("data/calibration/reference.v1.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    for g in groups.iter_mut() {
        for v in g.1.iter_mut() {
            if let Some(b) = raw["sniff_rows"].as_array().and_then(|a| a.iter().find(|x| x["id"] == v["id"])).and_then(|rr| rr.get("rule_of_thumb_band_yd")) {
                v["ref"]["rule_band"] = b.clone();
            }
        }
    }
    let data = json!({
        "generated": "2026-10-12",
        "aero": serde_json::to_value(a).unwrap(),
        "summary": {"fit_rows": r.fit_rows.len(), "within3": within3, "worst_fit_err": r1(worst), "sniff_rows": r.sniff_rows.len(), "sniff_pass": sniff_pass},
        "groups": groups.into_iter().map(|(n, rows)| json!({"name": n, "rows": rows})).collect::<Vec<_>>(),
        "shapes": shapes(&a),
        "ground": ground_demo(&a),
        "course": course_demo(&a)
    });
    let html = TEMPLATE.replacen("/*__DATA__*/null", &data.to_string(), 1);
    let out = calib::repo_root().join("out/viewer");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let p = out.join("flight.html");
    std::fs::write(&p, html).map_err(|e| e.to_string())?;
    println!("wrote {} ({:.0} KB)", p.display(), std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0);
    Ok(())
}
