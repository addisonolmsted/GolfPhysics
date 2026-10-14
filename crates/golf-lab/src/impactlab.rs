//! `impact-lab`: the Impact Lab page, with the engine compiled to WebAssembly (crates/golf-wasm) embedded in it.
//! Build the wasm first (tools/impact-lab.sh does both). Writes out/viewer/impact.html.

use golf_impact::{ClubKind, ClubTable};
use serde_json::{Value, json};

use crate::calib;
use crate::impactfit::club_id;

const TEMPLATE: &str = include_str!("../assets/impact-viewer.html");

fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

pub fn run() -> Result<(), String> {
    let root = calib::repo_root();
    let wasm_path = root.join("target/wasm32-unknown-unknown/release/golf_wasm.wasm");
    let wasm = std::fs::read(&wasm_path).map_err(|e| format!("{}: {e} (build it first: tools/impact-lab.sh)", wasm_path.display()))?;
    let reference = calib::load_reference()?;
    let table = ClubTable::builtin();
    // the tour delivery per club: the PGA Tour row where there is one, else by loft
    let clubs: Vec<Value> = table
        .clubs
        .iter()
        .map(|c| {
            let row = reference.fit_rows.iter().find(|r| r.group == "PGA Tour" && club_id(&r.club) == Some(c.id.as_str()));
            let (mph, attack) = match (c.kind, row) {
                (ClubKind::Putter, _) => (4.5, 2.0),
                (_, Some(r)) => (r.club_mph, r.attack_deg.unwrap_or(0.0)),
                _ => ((118.0 - 0.75 * c.loft_deg).clamp(70.0, 115.0), -1.0 - 0.09 * c.loft_deg),
            };
            let ama_attack = if c.id == "dr" { -1.6 } else if c.kind == ClubKind::Putter { 2.0 } else { (attack * 0.7 * 10.0_f64).round() / 10.0 };
            json!({"id": c.id, "name": c.name, "kind": c.kind, "loft": c.loft_deg, "tour_lean": c.tour_lean_deg,
                "tour_mph": mph, "tour_attack": (attack * 10.0_f64).round() / 10.0, "ama_mph": (mph * 0.82 * 2.0_f64).round() / 2.0, "ama_attack": ama_attack})
        })
        .collect();
    let lie: Value = serde_json::from_str(&std::fs::read_to_string(root.join("data/materials/lie.v1.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let order = [4, 3, 2, 5, 6, 7, 15, 8, 11, 12, 16, 10, 1];
    let lies: Vec<Value> = order.iter().filter_map(|id| lie["classes"][id.to_string()]["name"].as_str().map(|n| json!({"id": id, "name": n}))).collect();
    let data = json!({"clubs": clubs, "lies": lies});
    let html = TEMPLATE.replacen("/*__DATA__*/null", &data.to_string(), 1).replacen("__WASM__", &base64(&wasm), 1);
    let out = root.join("out/viewer");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let p = out.join("impact.html");
    std::fs::write(&p, html).map_err(|e| e.to_string())?;
    println!("wrote {} ({:.0} KB, engine {:.0} KB)", p.display(), std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0, wasm.len() as f64 / 1024.0);
    Ok(())
}
