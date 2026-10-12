# Impact model

`golf-impact`: `ClubDelivery + Club + Lie → LaunchConditions`. Status: **draft, M0 (2026-10-11)**. Milestone M4.

## Approach

An **empirical D-plane model**, not a rigid-body impulse. GolfEngine used an impulse model and then rotated its
answer to hit a target launch angle (see [legacy notes](../reference/legacy-golfengine.md)), which is the
empirical model with extra steps. Launch monitors measure exactly the relationships below, so we fit them directly
and get the published behaviour with few parameters.

## Equations (first form; coefficients fitted in M4)

Let `F` = face angle and `P` = path (both relative to the aim line, + = right), `L` = dynamic loft, `A` = attack
angle, `V` = head speed.

- **Spin loft** `SL = L - A` (in the vertical plane; the full D-plane form uses the 3D angle between the face
  normal and the club path, which also folds in face-to-path).
- **Ball speed** `v = V · smash(club, SL) · strike(club, offset)`. `smash` falls as spin loft rises (driver about
  1.48-1.50 at its best, 7-iron about 1.33, wedges lower). `strike` falls off with the strike offset (a heel or toe
  strike loses a few % per cm, a thin or fat strike more).
- **Launch angle** `launch = A + f_v(club) · SL`, with `f_v` about 0.75-0.85 (more face-dominated for the driver).
- **Start direction** `dir = P + f_h(club) · (F - P)`, with `f_h` about 0.75-0.85 (driver higher, irons lower).
  Equivalently: start ≈ mostly face, a little path.
- **Spin rate** `spin = k_s(club) · v · SL` to first order, then reduced by the lie's spin factor. Saturates at
  very high spin loft.
- **Spin axis tilt** `tilt = atan2(k_t · (F - P), SL)`: face open to path tilts the axis right (fade). A strike
  toward the toe of a wood adds draw tilt (**gear effect**), towards the heel adds fade; irons have much less.

All per-club coefficients live in the club table (`data/clubs/standard.v1.json`): static loft, length, `smash`
curve, `f_v`, `f_h`, `k_s`, `k_t`, gear-effect strength, head-speed range used by the assists.

## Lie

The surface under the ball changes the strike. Per class, in our materials file:

| Field | Meaning | Example (placeholder) |
|---|---|---|
| `speed_factor` | ball speed multiplier | rough 0.9, fescue 0.75, bunker (full shot) 0.85 |
| `spin_factor` | spin multiplier: grass between face and ball | first cut 0.85, rough 0.6, fescue 0.4 |
| `launch_add_rad` | launch change | rough +1°, fescue +2° |
| `variability` | spread of the above per shot (seeded): fliers, heavy lies | rough 0.1, fescue 0.25 |
| `max_club` | e.g. no driver from fescue without a big penalty | |

GolfEngine's table (rough 0.875 speed / 0.65 spin, bunker 0.45 / 0.15, green 1.0 / 1.1) is a starting point.
Its bunker numbers are for an explosion shot. A **greenside bunker shot** (sand first, the ball comes out on a
cushion of sand) is a separate `shot_type`-dependent case, to be designed with the assists.

The ball's own position also matters: from the tee it sits up (a positive attack angle is fine); on a slope the
launch rotates with the ground (a sidehill lie aims the ball, an uphill lie adds loft). GolfEngine applied slope
as rotations of the result; we do the same, from the normal at the ball.

## Calibration (M4)

1. Fit the coefficients so that the tour-average delivery per club gives that club's launch-monitor row
   ([03 §1](../03-success-indicators.md#1-launch-monitor-targets-flight-m2-impact-m4)).
2. Check the face/path table there.
3. Check the slower (LPGA) row with the same coefficients, as a second speed point.
