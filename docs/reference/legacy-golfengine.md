# GolfEngine (the earlier attempt)

`~/Documents/GitHub/GolfEngine`. A Rust library (edition 2024, nalgebra, f32), 16 commits, last on 2026-06-07 ("fixed
bounce and roll phys"). It ran a whole shot: sensor data → club angles → impulse launch → RK4 flight → bounce →
slide/roll. Most constants were fitted by an optimizer against tour averages. Surveyed 2026-10-11.

Its frame was +x right of target, +y towards the target, +z up. Ours is the C0 world frame; see
[01](../01-conventions.md#frame).

## Where things were

| Topic | File |
|---|---|
| Ball and aero constants | `src/ball_constants.rs` |
| Flight (RK4) | `src/aerial_trajectory.rs` |
| The shot state machine (flight, bounce, ground) | `src/trajectory.rs` (2190 lines) |
| Bounce | `src/bounce.rs` |
| Surface table | `src/surface_props.rs` |
| Impact | `src/ball_initial_conditions.rs`, `src/clubhead_velocity.rs`, `src/club.rs` |
| Trees, flagstick, cup | `src/objects/` |
| Optimizers and carry tests | `src/bin/` |

## Keep (as ideas, rewritten)

- **The aero structure:** drag and lift as functions of the spin factor, velocity relative to the wind, Magnus
  along `ω × v`. Fitting carry, apex and landing angle together, never carry alone; its own notes found carry-only
  fits degenerate.
- **The bounce structure:** a turf-tilted normal (Penner) plus grip/slip tangential impulse coupled to spin.
- **Slide → roll from the contact-point slip:** spin-back falls out of it with no special case.
- **A surface table, blended in softness space** (1/firmness) at boundaries.
- **Putting friction from stimp.**
- **Terrain and material as interfaces** (closures there; `Course` queries here).
- **A "laxity" dial** (slerp the raw inputs towards an ideal swing): the seed of our direction assist, now split by
  context and logged.

## Drop

| Problem | What it did | What we do |
|---|---|---|
| Inconsistent inertia | 2/7 and 5/2 (solid sphere) at impact, `MOI_RATIO = 0.33` on the ground | one `k` everywhere (D7) |
| Launch angle forced | rotated the impulse result to `dynamic_loft - k · spin_loft`, plus a 7° floor that left the spin axis behind | an empirical D-plane model fitted directly ([impact](../models/impact.md)) |
| Spin hardly decays, axis fixed | 0.3 %/s decay, constant axis | a few %/s fitted; spin as a vector (D8) |
| Optimizer artefacts | `0.13500001`-style constants, a hard Cd clamp at 0.18 | smooth parametric forms with few parameters and a recorded fit |
| Tuning hacks on the ground | forward absorption, skid loss, liftoff margins, churn fixes | start from the three mechanisms; add a term only for a failing test, with its reason |
| f32 and Euler on the ground | precision and stability | f64, RK4 in flight, semi-implicit 1 ms on the ground |
| Ground air drag | Cd 0.45 applied while rolling | dropped |
| Overshooting carries | driver 273 m vs a 250 m target; 3-wood 245.5 vs 222 | calibrate to the full launch-monitor table, with tolerances ([03](../03-success-indicators.md)) |
| Doubled ball radius | collision radius 2× for an arcade feel | real radius; forgiveness is an assist |

## Numbers worth a look (placeholders only)

- Club table (`club.rs`): driver loft 10°, impact mass 0.21 kg, COR 0.786; 7-iron 34°, 0.308 kg, COR 0.712; snap
  factors 1.5 (driver) to 1.2 (wedges).
- Surface table (`surface_props.rs`): e.g. green `en_max` 0.43, tilt 10°, roll `c0` 0.035; fairway 0.53, 5°,
  0.10; rough 0.31, 7°, 0.24; bunker 0.10, firmness 50 kPa.
- Lie factors: rough 0.875 speed / 0.65 spin; bunker 0.45 / 0.15; green 1.0 / 1.1.
- The old cup radius was 0.1524 m (a 1 ft cup), about 2.8× the real one (108 mm diameter → 0.054 m radius). We use
  the real cup and make any enlargement an assist.
