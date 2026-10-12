# golf-flight

`LaunchConditions + Course + Wind → airborne path + first contact`: drag and Magnus lift as functions of the spin
factor, spin decay, wind with a height profile, fixed-step RK4 at 1 ms, ground and water contact refined by
bisection, tree and flagstick contact.

- Models: [flight.md](../../docs/models/flight.md), [wind.md](../../docs/models/wind.md),
  [obstacles.md](../../docs/models/obstacles.md).
- Aero coefficients: `data/aero/ball.v1.json` (from the M2 fit).
- Also runs the short hops between bounces for `golf-ground`.

**Depends on:** `golf-core`, `golf-course`. **Status:** scaffold (M0); M2.
