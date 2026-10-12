# Flight model

`golf-flight`: `LaunchConditions + Course + Wind → airborne trajectory + first contact`. Status: **current, M2b
(2026-10-12)**. Coefficients: `data/aero/ball.v1.json`. Reference data: `data/calibration/reference.v1.json`. Viewer:
`golf-lab view` (published as "Ball Flight Lab").

## State and forces

State: position `p` (the ball's centre), velocity `v`, spin vector `ω` (rad/s), all world frame, f64.

With `u = v - w(z)` the velocity relative to the air, `A = π r²`, `ρ` the air density:

- **Gravity:** `g = 9.80665 m/s²` downwards.
- **Drag:** `a_D = -(½ ρ A / m) C_D |u| u`.
- **Magnus lift:** `a_L = (½ ρ A / m) C_L |u|² · normalize(ω⊥ × u)`.
- **Spin factor** `S = r |ω⊥| / |u|`, where `ω⊥` is the spin perpendicular to `u` (rifle spin makes no lift).
- **Spin decay:** `dω/dt = -(k_t + k_u |u|) ω`.

## Coefficients (fitted, M2b)

```
C_D = cd0 + cd_s S + cd_s2 S² + cd_v · min(0, cd_v_ref / |u| - 1)
C_L = cl_max S^p / (cl_s_half^p + S^p)
```

| Parameter | Value | What it does |
|---|---|---|
| `cd0` | 0.256 | drag at zero spin, below ~39 m/s |
| `cd_s`, `cd_s2` | 0.433, -0.260 | drag rises with spin, flattening at high spin |
| `cd_v`, `cd_v_ref` | 0.196, 38.9 m/s | drag falls as the ball goes faster than ~39 m/s; flat below |
| `cl_max`, `cl_s_half`, `p` | 0.440, 0.143, 1.47 | lift grows as S^1.5 at low spin and saturates for wedges |
| `spin_decay_per_s`, `spin_decay_per_m` | 0.034 /s, ~0 | spin falls ~3.4 % per second, at any speed |

How the form was chosen (each step is in the git history of `crates/golf-flight/src/aero.rs`):

1. **Linear drag, saturating exponential lift:** drivers short and wedges short. One drag curve can't fit slow and
   fast balls.
2. **A Reynolds-number drag term** fixed the speed trend, but **failed the altitude check**: thinner air also lowers
   the Reynolds number, which raised drag and cancelled half the gain (Denver +2.3 % against the ~5 % TrackMan
   measures). The term became a function of **air speed only**.
3. **Power-law lift** (the Smits & Smith 1994 shape, `C_L ∝ S^0.4`): drivers still flew too high and landed too
   steeply. A **drivers-only fit was near perfect**, which showed the lift curve's shape was the compromise.
4. **Saturating lift** (`S^p / (s_half^p + S^p)`) plus a **quadratic spin term in drag**: the M2 fit.
5. **M2b:** the speed term made **one-sided** (it only lowers drag above ~39 m/s). Two-sided, it raised drag on slow
   balls as they lost airspeed, so a tailwind shortened a 70 mph drive. Spin decay gained a time-based term (the fit
   then chose it over the speed-based one). A **speed-dependent lift** was tried and rejected: the fit improved only
   by drifting to drag at its upper bound and more lift at low speed, and the two verified targets got worse.

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

### M2b: time, curve and wind (2026-10-12)

The owner asked whether flight time, curve and wind are right, beyond carry and shape. Published references added as
fit targets (`reference.v1.json` → `extra_targets`, with sources):

- **Hang time:** the PGA Tour driver average, 6.1-6.3 s for years (ShotLink, via Golf Digest), ±0.4 s, since
  that's a real-conditions average, not a normalised one.
- **Curve:** TrackMan's ~0.7 % of carry per degree of spin axis, ±25 %, on six rows.
- **Wind:** Golf Digest's tour 7 iron (166 yd), -17 yd into 10 mph and +13 down, ±3 yd.
- **The tailwind rule:** no fit or sniff row may lose carry downwind, at 10 or 20 mph.

Measured with `golf-lab/examples/flight_checks.rs`, before (M2) and after (M2b):

| Shot | Carry | Time | Curve per 1° | 10 mph into / down | 20 mph into / down | 10 mph cross |
|---|---|---|---|---|---|---|
| PGA driver | 270 → 267 | 6.9 → 6.8 s | 2.0 → 2.0 yd | -18/+13 → -17/+12 | -41/+21 → -38/+21 | 17.7 → 17.3 yd |
| PGA 7 iron | 167 → 166 | 6.3 → 6.2 s | 1.6 → 1.6 yd | -19/+12 → -19/+12 | -45/+18 → -45/+19 | 15.6 → 15.2 yd |
| Amateur driver | 200 → 199 | 5.9 → 5.8 s | 1.5 → 1.5 yd | -11/+6 → -11/+7 | -28/+8 → -28/+10 | 12.7 → 12.7 yd |
| 70 mph driver | 123 → 123 | 4.0 → 4.1 s | 0.6 → 0.6 yd | -2/**-1** → -3/+1 | -7/**-3** → -9/0 | 5.8 → 5.8 yd |

What it shows:

- **Into and down the wind: good.** The 7 iron matches Golf Digest within 2 yd; the amateur rows match Andrew Rice's
  rule (about 1 yd per mph into, half that down; about 20 % + 5 yd into 20 mph).
- **The tailwind bug is fixed.** No row loses carry downwind. A slow drive in a strong tailwind now gains nothing,
  for a physical reason (below).
- **Hang time stays ~10 % long for drivers** (6.8 s against 6.1-6.3 s). Pushing it down costs carry: weighted
  hard, the fit reached 6.4 s but the tour driver lost 6 % of its carry. The launch-monitor sessions of the swing
  study will measure it directly. Crosswind drift comes from the time in the air, so it's ~10 % high too.
- **Iron curve stays ~30-40 % above TrackMan's rule; drivers match.** This is structural: sideways lift is the
  vertical lift tilted by the spin axis, and the vertical lift is fixed by the iron's apex and carry. **M4 handles
  it:** players never set a spin axis, they produce a face-to-path difference, and the face/path → spin-axis step is
  calibrated per club against measured curve.
- **Accepted deviations (D37):** the PGA driver carries 3.6 % short (gate ±3 %), and the 10 handicap sniff row carries
  1.5 yd under its band (an accepted miss in the reference file; the band is unchanged).

**Wind height:** golfers' crosswind rules (5-10 yd per 10 mph) refer to the wind felt near the ground, about 60 % of
the 10 m wind the model takes. The game should state which wind it shows; the recommendation is the 10 m wind, like
weather reports, with the height profile doing the rest.

### Why a strong tailwind gives a slow drive nothing

From `golf-lab/examples/tailwind_why.rs`, at launch and over the whole flight:

| Shot | Wind | Airspeed | Lift | Drag | Carry | Time | Apex | Mean ground speed |
|---|---|---|---|---|---|---|---|---|
| 70 mph driver | calm | 43.4 m/s | 0.85 g | 1.06 g | 123.3 yd | 4.06 s | 13.1 yd | 27.8 m/s |
| 70 mph driver | 20 mph down | 34.7 m/s | 0.63 g | 0.75 g | 123.3 yd | 3.47 s | 10.8 yd | 32.5 m/s |
| Tour driver | calm | 74.7 m/s | 1.38 g | 2.04 g | 266.6 yd | 6.79 s | 32.4 yd | 35.9 m/s |
| Tour driver | 20 mph down | 65.9 m/s | 1.21 g | 1.73 g | 287.3 yd | 6.06 s | 27.1 yd | 43.4 m/s |

A tailwind does two things. It **helps** by cutting drag (less air going past the ball) and carrying the ball along.
It **hurts** by cutting lift, because lift grows with the square of airspeed too. Which wins depends on how much of
the airspeed the wind takes away:

- A 20 mph wind is **20 % of a slow drive's airspeed**, so it cuts lift by about a quarter: from 0.85 g, nearly
  holding the ball up, to 0.63 g. The ball peaks lower and lands 0.6 s sooner. It moves 17 % faster over the ground
  but stays up 15 % shorter, and the two cancel.
- The same wind is only **12 % of a tour drive's airspeed**. Its lift drops less (1.38 g to 1.21 g, still more than
  the ball's weight), it loses less time in the air (11 %), and its ground speed rises more (21 %): +21 yd.

This matches what golfers do: downwind, you want a higher launch and more spin, to keep the ball up while the wind
carries it. A slow, mid-launch drive has little lift to spare, so a strong tailwind mostly brings it down sooner.
The model's prediction is physical, but unverified for slow swingers; the swing study's launch-monitor sessions
can't check it indoors, so it stays a feel item (D21).

### Re-running

```sh
cargo run --release -p golf-lab -- fit-aero    # writes data/aero/ball.v1.json (a full refit takes ~10-35 s)
cargo run --release -p golf-lab -- table       # every row against its target or band; fails on a sniff miss
cargo test --release -p golf-flight            # the gates as tests
cargo run --release -p golf-lab -- view        # out/viewer/flight.html
```
