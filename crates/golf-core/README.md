# golf-core

Shared by every crate: the vocabulary of the engine.

- **Units and frame:** `Vec2`, `Vec3` (f64), azimuth helpers, the shot-local frame (aim, right, up), handedness
  mirroring. See [01-conventions.md](../../docs/01-conventions.md).
- **Deterministic math:** `math::{sin, cos, exp, pow, atan2, …}` wrapping `libm`. The core crates call these, never
  `f64::sin` ([01 §Determinism](../../docs/01-conventions.md#determinism)).
- **Seeded RNG:** a small counter-based generator keyed by `(shot_id, stream, index)`.
- **Hand-off types:** `ClubDelivery`, `LaunchConditions`, `BallState`, `Contact`, `SurfaceClass`, `ShotError`.
  Field definitions: [00 §2](../../docs/00-architecture.md#the-hand-off-types).
- **Ball constants:** mass, radius, inertia factor ([models/README](../../docs/models/README.md#the-ball)).

**Depends on:** `libm` only. **Status:** M1 has `math` (libm wrappers and constants), `Vec2`/`Vec3` and the ball
constants; the hand-off types and the RNG come with M2.
