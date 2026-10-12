# golf-flight

`LaunchConditions + Course + Wind → airborne path + first contact`: drag and Magnus lift as functions of the spin
factor, spin decay, wind with a height profile, fixed-step RK4 at 1 ms, ground and water contact refined by
bisection, tree and flagstick contact.

- Models: [flight.md](../../docs/models/flight.md), [wind.md](../../docs/models/wind.md),
  [obstacles.md](../../docs/models/obstacles.md).
- Aero coefficients: `data/aero/ball.v1.json` (from the M2 fit).
- Also runs the short hops between bounces for `golf-ground`.

**API:** `Model::new(aero, air, wind, &ground).fly(start_xy, &launch) -> Flight` (samples, apex, the contact state,
how it ended), or `fly_from(state)` for a bounce hop. `Ground` is implemented for `Course` (heights, water above
the bed) and `Flat` (calibration).

**Tests:** `tests/calibration.rs`: every fit row within 4 %, every sniff band, and the shape checks (spin optimum,
wind asymmetry, curve, altitude, determinism).

**Depends on:** `golf-core`, `golf-course`, `serde`. **Status:** M2 done (2026-10-12); results in
[flight.md](../../docs/models/flight.md#results-2026-10-12).
