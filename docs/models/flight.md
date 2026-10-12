# Flight model

`golf-flight`: `LaunchConditions + Course + Wind → airborne trajectory + first contact`. Status: **draft, M0
(2026-10-11)**. Milestone M2.

## State and forces

State: position `p`, velocity `v`, spin vector `ω` (rad/s), all world frame, f64.

With `u = v - w(p, t)` the velocity relative to the air (`w` from [wind.md](wind.md)), `A = π r²`, and `ρ` the air
density:

- **Gravity:** `m g`, `g = 9.80665 m/s²` downwards.
- **Drag:** `F_D = -½ ρ A C_D(S, Re) |u| u`.
- **Magnus lift:** `F_L = ½ ρ A C_L(S, Re) |u|² · normalize(ω × u)`.
- **Spin factor** `S = r |ω_⊥| / |u|`, where `ω_⊥` is the part of the spin perpendicular to `u` (spin along the
  velocity, rifle spin, makes no lift).
- **Spin decay:** `dω/dt = -c_ω · (|u| / r) · ω` *or* a simple exponential `dω/dt = -ω / τ`; pick by fit. Either
  way, the target is a few % per second at typical speeds. GolfEngine's 0.3 %/s is too low and gives balloon-y
  descents.

## Coefficients (*to fit in M2*)

Start with smooth parametric forms and fit their few parameters to the launch-monitor table (carry, apex, landing
angle together):

- `C_D(S, Re) = C_D0(Re) + c_1 · S` (plus a small `S²` term if needed). `C_D0` near 0.2-0.25 at driver speeds, higher
  at low speed (below the dimpled ball's drag crisis, around 15-20 m/s). The low-speed rise matters at the end of a
  wedge's flight and for chips.
- `C_L(S) = c_2 · S^p` with `p` near 0.4-0.8, capped. Typical values: `C_L` about 0.15-0.3 over `S` 0.05-0.3.
- `ρ` from the round's temperature, pressure (altitude) and humidity, default 1.225 kg/m³. A per-course altitude is
  a possible later generator field.

GolfEngine's fit (`C_D = 0.135 + 0.072 e^{-0.025 v} + rpm/30000`, clamped to [0.18, 0.5]; `C_L = 0.072 + 0.65 S^0.8`)
is a reference point, not a starting assumption: its drag clamp and constants were there to stop an optimizer from
finding degenerate fits, and it overshot driver carry by 9 %.

## Integration

- **Classic RK4, fixed `dt = 1 ms`** for the whole flight. About 8000 steps for a driver, well inside the budget
  ([02](../02-latency-budget.md)). No adaptive stepping, so results are deterministic and identical across
  platforms.
- **Ground test every step:** `clearance = p.z - r - height(p.x, p.y)`. When it changes sign, bisect the step
  (fixed 24 iterations on the RK4 interpolant) to find the contact time, then hand over to `golf-ground` with the
  exact state.
- **Height lookups only where needed:** while the ball is far above any ground (more than the tile's max height plus
  margin, kept per tile), skip the lookup. This keeps the flight cheap.
- **Water first:** if the ball crosses a water surface (from `water.json`) before the bed, the contact is water.
- **Trees:** each step's segment queries the spatial index for proxies; see [obstacles.md](obstacles.md).
- **The flagstick** is tested in the last metres near the pin.

## Outputs

Samples at 30 Hz plus the exact event points (apex, tree hits, the contact), and flight stats: carry (ground
distance from launch to first contact along the ground), apex height above the launch point, landing angle (from
the local ground plane, and from horizontal), flight time, and lateral curve (from the start line).

## Calibration (M2)

1. Put the launch-monitor table in `data/calibration/`.
2. Fit `C_D0`, `c_1`, `c_2`, `p` and the decay rate to all clubs at once (least squares on carry, apex and landing
   angle, weighted per [03](../03-success-indicators.md)). The fit tool lives in `golf-lab` and uses a fixed-size
   worker pool (≤ 4).
3. Check the shape tests (spin, wind and curve) without refitting.
4. Freeze the result as `data/aero/ball.v1.json` with the fit command and its residuals.
