//! golf-lab: offline tools. See README.md.
//!
//! ```text
//! golf-lab smoke <bundle dir>      the physics-starter smoke test (steps 1-5)
//! golf-lab tees  <fixtures dir>    hole lengths per par and tee set (the baseline-carry table)
//! golf-lab bench <bundle dir>      load time, raster memory, height-query cost
//! golf-lab conform <bundle dir>    our height function vs the generator's reader (--features conform)
//! ```

mod bench;
#[cfg(feature = "conform")]
mod conform;
mod smoke;
mod tees;

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, path) = match (args.first(), args.get(1)) {
        (Some(c), Some(p)) => (c.as_str(), PathBuf::from(p)),
        _ => {
            eprintln!("usage: golf-lab <smoke|tees|bench|conform> <path>");
            return ExitCode::from(2);
        }
    };
    let r = match cmd {
        "smoke" => smoke::run(&path),
        "tees" => tees::run(&path),
        "bench" => bench::run(&path),
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
