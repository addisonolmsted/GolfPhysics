# 03: Success indicators

How we know each step works. Each milestone gate in [PLAN.md](../PLAN.md) points here.

**Feel overrides the data** (owner, 2026-10-11; D21). Launch-monitor tables and constants from the literature are
the starting point: they make the first version believable, and they catch bugs (a driver carrying 400 yards is a
bug, not a feel choice). Once the game is playable, if a shot feels wrong, playtesting wins. Every such deviation
goes in the [decision log](04-decision-log.md) with what felt wrong, what changed and by how much, so we always know
how far the game sits from the reference and why. The tolerances below are the **M2-M4 gates**; after M10 they
become reference columns in the lab, not pass/fail.

## 1. Launch-monitor targets (flight, M2; impact, M4)

The calibration reference lives in **`data/calibration/reference.v1.json`** (M2, 2026-10-12), with its sources and
which fields are verified:
- **Fit rows:** the TrackMan PGA Tour (12 clubs) and LPGA Tour (11 clubs) tables, and TrackMan's average male
  amateur driver.
- **Sniff rows:** 11 never-fitted handicap cases, scratch to a 70 mph driver, each with a carry band and its
  basis.

The three rows below are a sample of the fit rows.

The model must work for **every golfer, not just tour players** (owner, 2026-10-12). Amateur rows and the sniff
tests are part of the gate, and a fit that pleases the tour table but misses the sniff bands fails.

| Club | Club speed (mph) | Ball speed (mph) | Launch (°) | Spin (rpm) | Apex (yd) | Landing angle (°) | Carry (yd) |
|---|---:|---:|---:|---:|---:|---:|---:|
| Driver | 113 | 167 | 10.9 | 2686 | 32 | 38 | 275 |
| 7-iron | 90 | 120 | 16.3 | 7097 | 32 | 50 | 172 |
| Pitching wedge | 83 | 102 | 24.2 | 9304 | 29 | 52 | 136 |

Club speed, ball speed, launch, spin and carry are verified against a text copy for PGA driver to 6 iron. The apex
and landing-angle columns (TrackMan publishes them as an image) are still unverified, and count half in the fit.
**Results:** [flight.md, Results](models/flight.md#results-2026-10-12).

**Tolerances for flight (M2):** given the ball speed, launch and spin from the table, at sea level with no wind:

| Output | Tolerance |
|---|---|
| Carry | ±3 % |
| Apex height | ±10 % |
| Landing angle | ±3° |
| Flight time | ±10 % (where published) |

Fit carry, apex and landing angle **together**. GolfEngine's notes found that fitting carry alone is degenerate:
many drag and lift pairs give the same carry with the wrong shape.

**Shape checks:** a driver at 2000 rpm flies lower and rolls further than one at 3500 rpm. Raising spin past about
3500 rpm on a driver loses carry (balloons). A 10 m/s headwind costs a 7-iron more carry than a 10 m/s tailwind
adds. A pure 3° spin axis tilt curves a 7-iron a believable amount (3-10 yd). A driver gains 4-10 % carry at
Denver's altitude (TrackMan measures ~2.5 yd per 1,000 ft at 155 mph ball speed; rules of thumb say 1.2-2 % per
1,000 ft). Repeated flights are bit-identical. All of these are tests in `crates/golf-flight/tests/calibration.rs`.

**Sniff tests:** each sniff row's carry falls in its band. A band is set from published data for that golfer
(TrackMan Combine club speeds, TrackMan ball-speed rows, Shot Scope by handicap), never moved to make a row pass
without a recorded reason in the reference file.

**Impact (M4):** the tour-average delivery per club (speed, attack, dynamic loft) reproduces that club's ball speed
±2 %, launch ±1° and spin ±10 %. Face and path combinations give the expected shot shapes:

| Face to target | Face to path | Expected |
|---|---|---|
| 0 | 0 | straight |
| +2° | +4° | push start, fades right |
| -2° | -4° | pull start, draws left |
| 0 | +4° (path left) | starts straight, fades |
| +6° | +10° | slice |

## 2. Ground (M3)

| Test | Pass |
|---|---|
| Stimp | Ball at 1.83 m/s on a flat green of stimp S (feet) rolls S × 0.3048 m ±3 % |
| Putt on a slope | A 2 % side slope breaks a 6 m putt by the band given by green-reading references (to be set in M3) |
| Driver roll-out | Tour-average driver landing on a firm, flat fairway rolls 15-35 m (wide band at first, tightened by playtesting) |
| Wedge check | PW at 9000+ rpm landing on a green stops within 0-3 m, or spins back on a firm green |
| Bunker | A ball landing in bunker sand at landing-angle 45°+ stops within 1 m, and plugs at steep angles and high speeds |
| Settling | A ball dropped next to every fixture pin comes to rest. No endless creeping. |
| Fuzz | 100k random shots per fixture: no NaN, no ball below the ground by more than 1 mm, no ball off the world without an `OffWorld` rest |

## 3. Course reader (M1)

- The physics-starter smoke test, all five steps, on all six fixtures.
- **Conformance:** our `height(x, y)` equals the generator `course-bundle` reader's to 1e-9 m at 100k random
  points per fixture, including tier seams.
- Load time and resident memory per fixture recorded in [HANDOFF.md](HANDOFF.md).

## 4. Swing (M5)

- Corpus replay: for each tagged swing (club, intent, known miss), the delivery falls in the intended band.
- Repeatability: five "same" swings from one person give head speed within ±5 % and face within ±3°.
- No stale values: a swing the phone could not read produces `PayloadInvalid`, never the previous swing's numbers
  (a known SwingApp v0 bug).

## 5. Assists (M7)

- With assists off, the shot equals the raw physics bit for bit.
- **Fairness (D22):** two requests that differ only in the player give identical result bytes. `simulate` has no
  player-identity input.
- **Baseline:** each casual-band corpus swing, made smoothly, carries within ±10 % of its club's baseline carry, and
  club gapping holds (each club longer than the next shorter one).
- **Ceiling:** skilled corpus swings reach tour head speeds; an intended draw or cut that finishes in the target
  zone is left untouched (the assist log is empty for it).
- **Fading help:** the same face/path error gets less correction in the power band than in the casual band.
- **Hidden:** the result sent to devices carries no assist log and no raw values.
- **Aim inference:** on a set of hand-labelled shots per fixture (tee shots, layups, doglegs, punch-outs from trees),
  the inferred target matches the label.
- **No pull into trouble:** across a fuzz of shots, no correction moves a finish into water, OB or a bunker that
  the raw shot avoided.
- Playtest protocol: a fixed swing corpus (casual and skilled swingers) on a fixed set of holes. Targets such as
  "a casual player averages about bogey golf" are set by the owner.

## 6. Performance (all milestones)

`cargo bench` in `golf-shot` with a fixed set of shots (driver, iron, wedge, chip, long putt, tree hit, water):
p99 under 3 ms per shot (assists included) on the reference machine. A regression of more than 20 % fails review.
