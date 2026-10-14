//! golf-lab: offline tools. See README.md.
//!
//! ```text
//! golf-lab smoke <bundle dir>      the physics-starter smoke test (steps 1-5)
//! golf-lab tees  <fixtures dir>    hole lengths per par and tee set (the baseline-carry table)
//! golf-lab bench <bundle dir>      load time, raster memory, height-query cost
//! golf-lab fit-aero               fit data/aero/ball.v1.json to the calibration reference
//! golf-lab fit-impact             fit data/clubs/standard.v1.json to the TrackMan tour rows, check the rest
//! golf-lab table                  the model against every reference row and sniff band
//! golf-lab ground-lab             the ground model on shaped terrain: out/viewer/ground.html
//! golf-lab view                   write the flight viewer to out/viewer/flight.html
//! golf-lab conform <bundle dir>    our height function vs the generator's reader (--features conform)
//! ```

mod bench;
mod calib;
mod fit;
mod groundlab;
mod impactfit;
mod table;
mod view;
#[cfg(feature = "conform")]
mod conform;
mod smoke;
mod tees;

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("");
    let path = PathBuf::from(args.get(1).cloned().unwrap_or_default());
    let needs_path = matches!(cmd, "smoke" | "tees" | "bench" | "conform");
    if cmd.is_empty() || (needs_path && args.len() < 2) {
        eprintln!("usage: golf-lab <smoke|tees|bench|conform> <path> | golf-lab <fit-aero|fit-impact|table|view|ground-lab>");
        return ExitCode::from(2);
    }
    let r = match cmd {
        "smoke" => smoke::run(&path),
        "tees" => tees::run(&path),
        "bench" => bench::run(&path),
        "fit-aero" => {
            let only = args.iter().position(|a| a == "--only").and_then(|k| args.get(k + 1)).map(|s| s.as_str());
            fit::run(only, args.iter().any(|a| a == "--dry"))
        }
        "fit-impact" => impactfit::run(),
        "table" => table::run(),
        "view" => view::run(),
        "ground-lab" => groundlab::run(),
        #[cfg(feature = "conform")]
        "conform" => conform::run(&path),
        #[cfg(not(feature = "conform"))]
        "conform" => Err("built without the `conform` feature: cargo run --release -p golf-lab --features conform -- conform <dir>".into()),
        _ => Err(format!("unknown command {cmd}")),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
