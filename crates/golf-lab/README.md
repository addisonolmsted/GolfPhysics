# golf-lab

Offline tools. Not part of the shipped engine.

| Command | Milestone | What it does |
|---|---|---|
| `smoke <bundle>` | M1 ✓ | the physics-starter smoke test, all five steps (step 3's roll waits for M3) |
| `tees <fixtures dir>` | M1 ✓ | hole lengths per par and tee set, the table behind the baseline carries ([assists](../../docs/gameplay/assists.md)) |
| `conform <bundle>` | M1 ✓ | our height function vs the generator's `course-bundle` reader; needs `--features conform` and `../GolfProceduralGenerator` (the only link to the generator's crates) |
| `bench <bundle>` | M1 ✓ | load time, resident rasters, query costs (release builds only) |
| `fit-aero [--only <id part>] [--dry]` | M2 ✓ | fits the aero coefficients to every fit row at once; writes `data/aero/ball.v1.json` (`--only dr --dry`: fit a subset, print only) |
| `table` | M2 ✓ | every fit row against TrackMan and every sniff row against its band; exits non-zero on a sniff miss |
| `fuzz` | M3 ✓ | as a test instead: `golf-ground/tests/ground.rs` plays 900 random shots on three fixtures |
| `replay <swings.jsonl>` | M5 | runs recorded swings through the whole engine, raw vs assisted |
| `view` | M2 ✓, M3 ✓ | writes `out/viewer/flight.html` ("Ball Flight Lab"): every row in the air, shape checks, bounce and roll, whole shots over a fixture hole |

Run them with `cargo run --release -p golf-lab -- <command> <path>`. Investigations live in `examples/`
(`water_audit`: water rings against the class raster; `dt_study`: flight error against step size; `math_cost`:
what the libm calls cost; `fly_loop`: a loop to profile; `flight_checks`, `tailwind_why`: M2b; `ground_checks`,
`roll_sweep`, `bounce_trace`: M3).

Sweeps use a fixed pool of at most 4 workers (see [HANDOFF](../../docs/HANDOFF.md#machine-safety)).

**Depends on:** `golf-core`, `golf-course`, `golf-shot`. **Status:** scaffold (M0).
