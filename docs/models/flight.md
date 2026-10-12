# Flight model

`golf-flight`: `LaunchConditions + Course + Wind → airborne trajectory + first contact`. Status: **current, M2
(2026-10-12)**. Coefficients: `data/aero/ball.v1.json`. Reference data: `data/calibration/reference.v1.json`. Viewer:
`golf-lab view` (published as "Ball Flight Lab").

## State and forces

State: position `p` (the ball's centre), velocity `v`, spin vector `ω` (rad/s), all world frame, f64.

With `u = v - w(z)` the velocity relative to the air, `A = π r²`, `ρ` the air density:

- **Gravity:** `g = 9.80665 m/s²` downwards.
- **Drag:** `a_D = -(½ ρ A / m) C_D |u| u`.
- **Magnus lift:** `a_L = (½ ρ A / m) C_L |u|² · normalize(ω⊥ × u)`.
- **Spin factor** `S = r |ω⊥| / |u|`, where `ω⊥` is the spin perpendicular to `u` (rifle spin makes no lift).
- **Spin decay:** `dω/dt = -k_ω |u| ω`.

## Coefficients (fitted, M2)

```
C_D = cd0 + cd_s S + cd_s2 S² + cd_v (36.5 / |u| - 1)
C_L = cl_max S^p / (cl_s_half^p + S^p)
```

| Parameter | Value | What it does |
|---|---|---|
| `cd0` | 0.233 | drag at zero spin, at 36.5 m/s |
| `cd_s`, `cd_s2` | 0.462, -0.345 | drag rises with spin, flattening at high spin |
| `cd_v` | 0.131 | drag falls as the ball goes faster (below) |
| `cl_max`, `cl_s_half`, `p` | 0.476, 0.170, 1.20 | lift grows as S^1.2 at low spin and saturates for wedges |
| `spin_decay_per_m` | 0.000496 | about 3 %/s at 60 m/s |

How the form was chosen (each step is in the git history of `crates/golf-flight/src/aero.rs`):

1. **Linear drag, saturating exponential lift:** drivers short and wedges short. One drag curve can't fit slow and
   fast balls.
2. **A Reynolds-number drag term** fixed the speed trend, but **failed the altitude check**: thinner air also lowers
   the Reynolds number, which raised drag and cancelled half the gain (Denver +2.3 % against the ~5 % TrackMan
   measures). The term is now a function of **air speed only**. It is empirical: whatever makes fast balls fly
   relatively further, it doesn't scale with density.
3. **Power-law lift** (the Smits & Smith 1994 shape, `C_L ∝ S^0.4`): drivers still flew too high and landed too
   steeply. A **drivers-only fit was near perfect**, which showed the model's parts were right and the lift curve's
   shape was the compromise. Low-spin drivers want lift to grow faster than linearly with spin; wedges want it to
   level off.
4. **Saturating lift** (`S^p / (s_half^p + S^p)`) plus a **quadratic spin term in drag**: the current fit.

GolfEngine's fit (`C_D = 0.135 + 0.072 e^{-0.025 v} + rpm/30000`; `C_L = 0.072 + 0.65 S^0.8`) was not used.

## Integration

- **Classic RK4 at a fixed 10 ms step.** The wind is evaluated at each stage's own height. With the wind frozen
  across a step the error grew linearly with the step size; with per-stage wind it is 4th order. Against a 0.25 ms
  reference, every test shot (tour driver, amateur 7 iron, tour PW, a chip, with wind and spin axis tilt) lands
  within 0.03 mm at 10 ms (`golf-lab/examples/dt_study.rs`).
- **Ground and water contact:** the ground is queried only when it could be within reach (clearance over a
  closing speed that assumes slopes up to 2:1), then a fixed 24 halvings of the step find the contact. A grazing
  pass over a sharp crest within a single 0.7 m step can be missed; trees and lips get swept tests in M6.
- **Water first:** the course's contact height is the water surface where the class raster says water and the
  surface is above the bed.
- **Samples** for drawing: the first step past each 1/30 s, plus the exact contact.
- **Speed:** ~620 steps for a driver, ~0.29 ms per flight over real terrain with wind
  (`golf-lab bench`). `S^p` is computed as `exp(p ln S)` (a third of `libm::pow`'s cost), and the constants are
  precomputed once per `Model`.

## Calibration

### What it is fitted to

`golf-lab fit-aero`: Nelder-Mead over the 8 parameters, against all 24 fit rows at once. The objective weights carry
(±3 %), apex (±10 %) and landing angle (±3°) together; unverified fields count half; a weak prior keeps spin decay
near a few % per second.

| Rows | Source | Verified |
|---|---|---|
| 12 PGA Tour clubs, driver to PW | TrackMan tour averages | speeds, launch, spin, carry for driver to 6 iron; the rest, and every apex and landing angle, from the table as remembered |
| 11 LPGA Tour clubs | the same table | driver to 8 iron as above |
| The average male amateur's driver (14.5 hcp, 93.4 mph) | TrackMan Combine | carry and landing angle |

### Sniff tests (never fitted)

The fit leans on tour players, so the model is also checked against golfers it wasn't fitted to: 11 rows from
scratch to a 70 mph driver, in `reference.v1.json` → `sniff_rows`. Each row gives a club speed (TrackMan Combine
by handicap where published), an assumed ball speed, launch and spin, standing in for the impact model, and a carry
band. Two corrections were made on 2026-10-12, both recorded in the file:

- **7-iron smash 1.33.** A source's amateur 7 iron (116 mph ball from 80 mph club) is a smash of 1.45, higher
  than either tour's and inconsistent with its own 145 yd carry.
- **The three slowest drivers' bands** were re-anchored from a club-speed rule of thumb to TrackMan ball-speed data.
  The rule asked a 112 mph ball to carry more than TrackMan's own LPGA 5 iron gets at that ball speed and launch.
  The old bands stay in the file and on the viewer as dashed outlines.

### Results (2026-10-12)

| Check | Result | Gate ([03](../03-success-indicators.md)) |
|---|---|---|
| Carry, 24 fit rows | all within ±3 % (worst 3.0 %) | ±3 %: **met** |
| Apex, 23 rows with one | 19 within ±10 %; misses: PGA hybrid, 3 iron, PW (model 3-4 yd high), LPGA 3 wood (4 yd low) | ±10 %: 4 misses, all against unverified figures |
| Landing angle, 24 rows | 18 within ±3°; misses: PGA 4 and 5 iron, LPGA 3 wood, 7 wood, 5 iron (model shallower), **the amateur driver: 38.6° against a verified 34.8°** | ±3°: 6 misses, 5 against unverified figures |
| Sniff rows | 11 / 11 in band | **met** |
| Shape checks (`cargo test -p golf-flight`) | spin optimum ~2,700-3,500 rpm on a tour driver with ballooning above; 10 m/s into a 7 iron costs 52 yd, downwind adds 19; a 3° spin axis curves a 7 iron 4.9 yd; Denver +8 %; bit-identical repeats | **met** |

**Known residual:** drivers land 3-4° steeper than TrackMan says, while their carry is right. The fix is either a
richer spin decay or a lift that depends on speed. It isn't worth the parameters until the apex and landing columns
are verified. A steeper landing also means a little less roll (M3), which feel tuning will catch first (D21).

### Re-running

```sh
cargo run --release -p golf-lab -- fit-aero    # writes data/aero/ball.v1.json (a full refit takes ~10-35 s)
cargo run --release -p golf-lab -- table       # every row against its target or band; fails on a sniff miss
cargo test --release -p golf-flight            # the gates as tests
cargo run --release -p golf-lab -- view        # out/viewer/flight.html
```
