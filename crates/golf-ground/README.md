# golf-ground

From the first contact to rest: bounce impulse (turf tilt, restitution, grip/slip with spin), sliding, rolling on the
height function with slope gravity and stimp-derived rolling resistance, crest liftoff, cup capture and lip-outs,
water entry, plugging.

- Model: [ground.md](../../docs/models/ground.md).
- Material parameters: our `data/materials/physics.v1.json` over the bundle's `materials.json`.

**API:** `BallModel::new(&terrain, &materials, aero, air, wind, cup).play(start, &launch) -> Outcome` (samples at
~30 Hz plus every event, events, the rest position and state, carry, total); `roll_from(start, velocity)` for a ball
already rolling. `Terrain` is implemented for `Course` and for `TestPlane` (a uniform tilted plane, for exact tests).

**Modules:** `bounce` (one impact), `roll` (slide, roll, the cup), `play` (the whole shot), `materials`, `terrain`,
`record` (samples and events).

**Tests:** `tests/ground.rs`: stimp, slopes, driver roll against TrackMan, wedges, bunkers and plugging, the cup,
settling at every fixture pin, a 900-shot fuzz on three fixtures, determinism.

**Depends on:** `golf-core`, `golf-course`, `golf-flight` (hops), `serde`. **Status:** M3 done (2026-10-13); results in
[ground.md](../../docs/models/ground.md#results-2026-10-13).
