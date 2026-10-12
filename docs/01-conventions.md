# 01: Conventions

Rules that apply across all crates. A model doc may narrow one, but may not contradict one.

## Units

| Quantity | Unit | Suffix | Notes |
|---|---|---|---|
| Length, height | metres (f64) | `_m` | Yards and feet only in display strings, converted at the edge. |
| Time | seconds | `_s` | Budgets in milliseconds: `_ms`. |
| Speed | m/s | `_mps` | mph only for comparing with published tables, in `golf-lab`. |
| Angle | radians | `_rad` | Degrees only in data files meant for people (club lofts), converted on load. |
| Spin rate | rad/s | `_radps` | rpm only at the edges (tables, logs). 1 rpm = 2π/60 rad/s. |
| Mass | kg | `_kg` | |
| Acceleration | m/s² | `_mps2` | The phone reports g; `golf-swing` converts on input. |
| Fractions, dials | [0, 1] | none | Clamped, never wrapped. |

The units of anything crossing a crate boundary are in its name. A bare `f64` is fine inside a function.

## Frame

**The C0 frame, unchanged** (*proposed* answer to generator question 2):

- x east, y north, z up, metres;
- origin at the south-west corner of the 3 km world;
- elevations in the generator's local datum.

The display swaps to y-up when it loads, as C0 already expects. The engine never does.

**Azimuths** are measured from +x towards +y (counter-clockwise from east), in `[0, 2π)`, the same as C0's wind and
tree yaw. A *direction* is in `[0, 2π)`; an *axis* (no head or tail) is in `[0, π)`. This follows the generator's
rule; mixing the two is a classic bug.

**Shot-local frame** (for club and launch numbers only): `aim` = the unit horizontal vector of the aim azimuth,
`up` = +z, `right` = `aim × up`. Horizontal angles (`path_rad`, `face_rad`, `direction_rad` relative to aim) are
positive to the **right** of the aim line, as launch monitors report them for a right-hander. Left-handed players
are mirrored at the `golf-swing` boundary, so everything after it sees a right-hander's conventions.

**Spin axis tilt** `spin_axis_rad`: positive tilts the axis so the ball curves right (fade or slice for a
right-hander), the launch-monitor convention. Inside the integrator the spin is a `Vec3` in world coordinates.

## Naming

- Crates `golf-<step>`; modules by noun (`aero`, `bounce`, `roll`, `cup`).
- Data files: `data/<kind>/<name>.v<N>.json`, e.g. `data/clubs/standard.v1.json`,
  `data/assist/casual.v1.json`, `data/materials/physics.v1.json`. A data file states its own `version` and is never
  edited after a release; a change is a new version.
- Model constants carry a provenance comment (a source, a fit run, or "placeholder").

## Determinism

**The same `ShotRequest` and the same data versions give the same `ShotResult` bytes on every platform.**

- f64 throughout the core. No f32 in the physics state.
- `sin`, `cos`, `exp`, `pow`, `atan2`, `sqrt` (and so on) go through `golf_core::math`, which wraps the `libm` crate.
  No `f64::sin` in the core crates. `std`'s versions may call platform libm, which differs in the last bit between
  macOS, Linux and WASM. A clippy `disallowed_methods` rule will enforce this from M1.
- No fused multiply-add unless written explicitly. No `-ffast-math`-style flags.
- Fixed time steps. Event times (ground contact, tree contact) are refined by bisection to a fixed iteration count,
  never to a tolerance that depends on the data.
- Randomness (tree deflection, gusts) comes only from `golf_core::rng`, a small counter-based generator seeded from
  `(shot_id, stream_name)`. **No retry-forked draws:** the number of draws must not depend on earlier outcomes.
  Where a variable number of events can happen (branch hits), each event draws from its own sub-stream, keyed by
  its index.
- No wall clock, no `HashMap` iteration order, no thread-order dependence inside one shot. The server may run many
  shots in parallel, but never splits one.

## Errors

- The core never panics on input. Bad payloads give `ShotError` values (`PayloadInvalid`, `CourseQueryOutside`,
  `NoContact`…), and the server turns them into a "swing not read" response.
- `golf-course` validates a bundle on load (hashes, versions, tier extents) and refuses a bundle it cannot fully
  read. A shot never discovers a broken course.
- A ball that leaves every height tier (past the surround) ends the shot with an `OffWorld` rest state. That is a
  fact, and the game treats it as OB.

## Tests

- Unit tests sit next to the code. Fixture tests read `GOLF_FIXTURES` (default
  `/Volumes/GolfOut/out/physics_fixtures/v1`) and **skip with a message** when it is missing. They never fail
  silently or fake data.
- Golden tests: a corpus of requests with their result hashes. A model change that moves a hash updates the golden
  set in the same commit, with the reason in the commit message.
