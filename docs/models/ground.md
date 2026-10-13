# Ground model

`golf-ground`: the first contact → bounces → slide → roll → rest (or holed, water, plugged). Status: **current, M3
(2026-10-13)**. Parameters: `data/materials/physics.v1.json`. Tests: `crates/golf-ground/tests/ground.rs`. Viewer:
"Ball Flight Lab", tab "Bounce and roll".

## Phases

```
flight ─▶ contact ─▶ BOUNCE ─┬─ normal speed out ≥ 0.5 m/s ─▶ a hop (golf-flight) ─▶ contact ─▶ …
                             └─ below that ─▶ SLIDE (contact point slipping) ─▶ ROLL ─▶ REST
                                                     │ a crest drops away faster than gravity ─▶ a hop
```

`BallModel::play` runs it all; `BallModel::roll_from` starts a ball already rolling (putts, tests). A shot ends:
- **at rest** on the ground;
- **holed**;
- **in water**: the ball crosses a water surface, where the class raster says water;
- **plugged**;
- **off the world** (past the surround);
- or, as a safety stop, **timed out**.

## Bounce (`bounce.rs`)

After Penner's model of a ball landing on turf (A. R. Penner, *The run of a golf ball*, Can. J. Phys., 2002; the
constants below are as remembered, to check against the paper):

1. **The turf gives.** The normal from the height function leans back towards the incoming ball by
   `θc = tilt_deg · (v / 18.6) · (landing angle / 44.4°)`, capped at `tilt_max_deg`. Penner's green value is 15.4°.
   This is what makes a steep, fast wedge stop and a shallow drive run.
2. **Restitution falls with impact speed:** `e = cor_scale · (0.510 − 0.0375 v + 0.000903 v²)` (Penner's green
   curve, to 20 m/s), clamped to `[cor_min, 0.8]`.
3. **Grip or slip.** The tangential impulse that stops the contact point slipping is `−m v_c / (1 + 1/k)`. If it
   exceeds `bounce_mu · J_n` (Coulomb), the ball slips with the capped impulse instead. The same impulse changes the
   spin through `r_c × J / I`. Check and spin-back fall out of this, with no special case.
4. **Plugging** goes by how hard the ball meets the actual ground, before the turf's give: a normal speed above
   `plug_speed` (bunker 22 m/s, waste sand 24 m/s). So only steep, fast landings plug, not every iron into sand.

One moment of inertia everywhere, `I = 0.4 m r²` (D7).

## Slide and roll (`roll.rs`)

Semi-implicit Euler at 1 ms. Each step the ball's centre sits at `r / n_z` above the height function and its
velocity is projected onto the tangent plane.

- **Slide:** kinetic friction `slide_mu · g · n_z` against the contact point's slip, on the centre and as a torque on
  the spin, until the slip reverses or falls under 2 cm/s.
- **Roll:** acceleration `g_t / (1 + k) − (roll_decel + roll_c1 · v) · v̂`. On greens the constant comes from stimp,
  `1.83² / (2 · stimp)`, and `roll_c1 = 0`: 0.52 m/s² at stimp 10.5. Rolling resistance never pushes a ball
  backwards.
- **Rest:** slower than 1 cm/s with the slope's pull `g_t / (1 + k)` at most `static_factor · roll_decel`.
  `static_factor` is at least 1: if a ball at rest were held less firmly than a moving one is slowed, it would stop
  and restart on a slope forever (found on a 6 % slope).
- **Liftoff:** if the ground's curvature along the motion drops away faster than gravity (`h_ss v² < −g n_z`), the
  ball goes back to a hop.

## The cup

The cup is 108 mm across, centred on the pin. A ball can drop only if it falls about its own radius while it crosses
the hole. Crossing on a line `b` from the centre, the chord is `2√(R² − b²)`, so the fastest capture is

```
v_cap(b) = 2 √(R² − b²) · √(g / (2 r))       (1.64 m/s through the centre; Holmes measured 1.63 m/s)
```

What decides capture is the **line's** distance from the centre. The first version used the distance at the moment
the ball met the rim, which is always ~R, so every putt lipped out. Faster than `v_cap` is a **lip-out**: the ball
turns away from the centre (up to 0.6 rad, more the nearer the edge) and slows by 20 %. That rule is a placeholder.
The flagstick is out for now.

## Materials

`data/materials/physics.v1.json`, per C0 class id (D18; it replaces the bundle's `materials.json` physics block for
the engine): `cor_scale`, `cor_min`, `tilt_deg`, `tilt_max_deg`, `bounce_mu`, `slide_mu`, `roll_decel` or `stimp_ft`,
`roll_c1`, `static_factor`, `plug_speed`. Where the values come from:

| Surface | Source |
|---|---|
| Green | Penner's curve (tilt 15.4°, μ 0.4) and stimp 10.5 |
| Fairway, tee | **fitted** to TrackMan's average amateur driver: carry 204, total 226 yd, so 22 yd of roll. Tilt 18°, roll `1.2 + 0.2 v` m/s². Found by `golf-lab/examples/roll_sweep.rs`: rolling resistance alone couldn't get below 25 yd, because most of the excess was hops after a first bounce that took too little speed out (`bounce_trace.rs`). |
| Everything else | scaled from those: rough tilt 24°, `2.8 + 0.4 v`; fescue 28°, `4 + 0.6 v`; bunker cor ×0.3, tilt 25-35°, `6 + 0.3 v` |

All but the green and fairway are placeholders until feel tuning (D21).

## Results (2026-10-13)

| Check | Result | Gate ([03 §2](../03-success-indicators.md#2-ground-m3)) |
|---|---|---|
| Stimp | 1.83 m/s rolls 3.199 m (stimp 10.5 = 3.200 m) | ±3 %: met |
| Holding on a green slope | holds at 3 %, 5 % (up to ~7 % at stimp 10.5); rolls off 10 % | met |
| Amateur driver on fairway | carry 199, **roll 22.4 yd** (TrackMan 22) | met (fitted) |
| Other drivers on fairway | PGA 19, LPGA 28, 70 mph 37 yd of roll; into rough the amateur driver rolls 9 yd | 15-40 yd: met |
| Wedges on a green | PGA PW −0.1 yd (checks), amateur PW +1.7, LPGA PW +3.6; tour 7 iron +4.1, amateur 7 iron +14.9 | met |
| Bunker | steep iron and wedge landings stop within a yard; a shallow driver landing runs 5.5 yd (a feel item); a near-vertical fast landing plugs | met |
| Cup | holed through the centre up to 1.64 m/s and 30 mm off-centre at 1 m/s; lips out at 1.9 m/s; misses past the rim | met |
| Settling | a ball dropped 0.5 m from every pin of the six fixtures (4 directions) rests within 3 m | met |
| Fuzz | 900 random shots (speeds 40-170 mph, launch 5-35°, wind to 10 m/s, any direction) on three fixtures: no timeout, no NaN, no ball resting below the ground | met |
| Determinism | bit-identical repeats | met |

**Speed** (`golf-lab bench`): a whole shot (flight + bounces + roll) 0.52-0.59 ms over real terrain; a 10 m putt
0.47 ms. Within the shot budget. The putting assist runs 6-10 putts per stroke, so a patch cache or a longer roll
step on smooth greens is an M7 item.

**Found on the way:** the M3 fuzz found a flight bug. A hop off a slope could start a hair below the surface, never
rise 1 cm, and so never switch on contact testing, falling through the ground. golf-flight now tests for contact
after 1 cm of clearance **or** 50 ms, and always catches a ball 2 cm underground.

## Not yet

- The flagstick (in or out is a game setting) and slam-dunks (a ball landing in the cup on the fly).
- The putter's skid at the start of a putt (M4's putter impact); `roll_from` starts a putt already rolling.
- Lips and walls (when `edges.json` exists), wet ground, grain on greens.
- Lie-dependent variation inside a class (the lie layer, after M7).
