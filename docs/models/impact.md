# Impact model

`golf-impact`: `ClubDelivery + Club + Lie → LaunchConditions`. Status: **current, M4 (2026-10-13)**. Club table:
`data/clubs/standard.v1.json`; lie table: `data/materials/lie.v1.json`. Fit: `golf-lab fit-impact`. Tests:
`crates/golf-impact/tests/impact.rs`. Shapes: `golf-lab/examples/shapes.rs`. Viewer: **Impact Lab**
(`tools/impact-lab.sh` → `out/viewer/impact.html`): the engine compiled to WebAssembly (`crates/golf-wasm`) runs in
the page, so every slider plays a real shot: delivery, impact, flight, bounce and roll.

## The model (D53)

An **oblique impact of the face on the ball**, in 3D (the D-plane), with a few fitted coefficients per club. The
draft planned purely empirical relations (launch = A + f·SL and so on); the physical form gives the same
relationships from fewer numbers, and handles any delivery (an open-faced flop, a de-lofted punch, a sidehill lie)
the same way.

The club moves along `u` (path P and attack A); the face points along `n` (face angle F and dynamic loft L). The
angle between them is the **spin loft**. With the ball mass `m`, head mass `M` and `k = 0.4` (D7):

- **Along the face normal**, restitution: `v_n = V cos(SL) (1 + e) / (1 + m/M)`, with
  `e = cor_ref + cor_slope (V cos SL − 40)`: the face gives back less at higher impact speeds.
- **Across the face**, the ball is dragged along until it rolls on the face:
  `v_t = V sin(SL) / (1 + 1/k + m/M)`, leaving it moving down the face with backspin `ω = v_t / (k r)` about
  `ŝ × n`. Measured balls leave with more spin than rolling (they are compliant tangentially: "overspin"), so
  `ω` is multiplied by `spin_factor · (v_n / 40)^−spin_speed_exp`.
- **What falls out:** launch ≈ 82 % loft and 18 % attack; start direction ≈ 85 % face for the driver, 75 % for
  irons, the rest path (TrackMan's rules); the spin axis tilts with face to path, more for low spin loft.
- **Spin axis gain** (D55): the axis tilt is scaled per club so each club curves TrackMan's ~0.7 % of carry per
  degree of D-plane axis. The flight model's sideways lift is ~45 % strong for irons (flight.md, M2b), so irons
  and long clubs come out at 0.63-0.78, the driver 0.94, the short wedges ~1.
- **Strike point:** ball speed falls with the offset from the sweet spot (`strike_loss_per_m`); gear effect: a toe
  strike adds draw spin, a heel strike fade (`gear_h`), a high strike less backspin (`gear_v`) and more launch
  (`launch_per_m_high`). Woods strongest, wedges weakest, none for the putter. Placeholders until feel tuning.
- **Lie** (`lie.v1.json`, per C0 class): speed, spin and launch factors. Rough: 0.92 speed, 0.6 spin (fliers),
  +1° launch; fescue 0.78, 0.45, +2°. Placeholders (D21). No per-shot randomness: a variation would have to come
  from the situation (the position), never from a dice roll, to keep same swing + same situation = same shot (D22).
- **Slope:** the ground's normal at the ball. A sidehill tilts the face with the sole (in full): with the ball above
  the feet a lofted face points left, more for a wedge. Uphill or downhill tilts the swing by `slope_follow`
  (0.6) of the slope: players partly swing with the slope.

All angles are relative to the aim line, + = right (open face, in-to-out path); a left-hander is mirrored at the
phone boundary.

## Fit (`golf-lab fit-impact`, 2026-10-13)

TrackMan publishes club speed, attack, ball speed, launch and spin, but not dynamic loft. So for each PGA Tour row
the fit finds, exactly: the **tour shaft lean** (dynamic minus static loft) from the launch angle, `cor_ref` from
the ball speed, and `spin_factor` from the spin. The shared parameters come from the rows **not** fitted, each at
the dynamic loft that gives its own launch (lean is the player's, and the phone measures it):

| Shared parameter | Value | From |
|---|---|---|
| `cor_slope` | −0.004 per m/s | LPGA ball speeds (rms 2.1 %) |
| `spin_speed_exp`, irons and wedges | 0.3 | LPGA irons (spin rms 4.2 %) |
| `spin_speed_exp`, fairway woods and hybrids | 0 | LPGA woods |
| `spin_speed_exp`, driver | 1.5 | LPGA and amateur drivers (D54) |

| Club | Static loft | Tour lean | Tour dynamic loft | `cor_ref` | `spin_factor` | Axis gain |
|---|---:|---:|---:|---:|---:|---:|
| Driver | 10.5 | +3.1 | 13.6 | 0.913 | 0.93 | 0.94 |
| 7 iron | 34 | −12.6 | 21.4 | 0.711 | 1.30 | 0.68 |
| PW | 46 | −14.1 | 31.9 | 0.722 | 1.25 | 0.78 |

(All clubs in the data file. Clubs without a row copy their neighbours: 7 wood from the 5 wood and hybrid, 4 hybrid
from the hybrid, the gap, sand and lob wedges from the PW. The putter's values are placeholders until putting is
tuned.) `cor_ref` above 0.83 (the legal limit) is an **effective** value: it also carries the shaft and hands
behind the head, which the bare head mass leaves out.

**Why the driver has its own speed exponent (D54).** Fitted on the tour row alone, the driver under-spins every
slower player: LPGA −22 %, the average amateur −13 %, and through the flight the LPGA drive loses 25 yd. Tour
players strike the driver high on the face with low-spin heads, so their row spins far less per degree of spin
loft. Most of our players swing at LPGA-to-amateur speeds, so the driver is fitted to them, and the tour row is
still reproduced exactly.

## Results

| Check | Result | Gate |
|---|---|---|
| PGA Tour rows, tour delivery | exact (fitted): ball speed, launch, spin | ±2 %, ±1°, ±10 %: met |
| LPGA rows, same coefficients, own launch | ball speed within 2.5 %; spin within 10 % except the 3 wood (+13 %; the row itself is odd: 2,704 rpm against the 5 wood's 4,501) | (second speed point) |
| Average amateur driver | ball speed +4 % (TrackMan's average includes off-centre strikes; ours is centred), spin +6 % | |
| Face and path | every case in the table goes the right way (below) | met |
| Gear, strike, lie, slope | toe draws, heel fades, high launches higher with less spin, off-centre loses speed; rough cuts spin and speed, uphill launches higher, ball above the feet starts left, more for a wedge | met |
| Determinism | bit-identical | met |

Shapes with the tour delivery (`examples/shapes.rs`), through the flight model: + = right, yards.

| Face, face to path | Driver: start, landing side | 7 iron | PW | Expected |
|---|---|---|---|---|
| 0, 0 | 0, 0 | 0, 0 | 0, 0 | straight |
| +2, +4 | +1.3°, +34 | +1.1°, +13 | +1.0°, +8 | push start, fades right |
| −2, −4 | −1.3°, −34 | −1.1°, −13 | −1.0°, −8 | pull start, draws left |
| 0, +4 | −0.7°, +25 | −0.9°, +7 | −1.0°, +3 | starts nearly straight, fades |
| +6, +10 | +4.1°, +80 | +3.9°, +33 | +3.5°, +21 | slice |
| 1° open to path | curve 7.2 | 2.5 | 1.4 | |

**Iron curve (M2b's open item): resolved** by the axis gain: each club now curves TrackMan's rule per degree of
D-plane axis.

## Not yet

- The delivery from the phone (M5): dynamic loft, face, path and attack from the swing, and the speed curve.
- Putting: the putter's restitution and the skid are placeholders; putts are tuned with the putting assist (M7).
- Bunker shots from greenside sand (sand first, the ball on a cushion) as their own shot type, with the assists.
- Strike location from the phone (sole tilt) and the gear and strike values' feel tuning.
