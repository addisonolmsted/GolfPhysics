# Ground model

`golf-ground`: the first contact → bounces → slide → roll → rest (or holed, water, plugged). Status: **current, M3
(2026-10-13)**. Parameters: `data/materials/physics.v1.json`. Tests: `crates/golf-ground/tests/ground.rs`. Viewers:
"Ball Flight Lab", tab "Bounce and roll"; **Ground Lab** (`golf-lab ground-lab`), shaped terrain scenarios.

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
   `θc = tilt_deg · (v / 18.6) · (landing angle / 44.4°)`, capped at `tilt_max_deg` and at `0.9 · (90° − landing angle)` (D45: without that cap a pitch into a steep face had its normal turned past the incoming ball and was thrown 67 m back). Penner's green value is 15.4°; ours is 18° (D44).
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
- **Liftoff:** if the ground's curvature along the motion drops away faster than gravity (`h_ss v_h² < −g`, D46), the
  ball goes back to a hop.

## The cup (`cup.rs`)

The regulation cup is 108 mm across; **the game's cup is 8 inches (203.2 mm)** (D47), `Cup::game`. `Cup::regulation`
stays for tests and the lab. Both are centred on the pin, with a 4-inch-deep liner.

**The hole is geometry, not a rule** (D48). Once the ball's centre is over the hole, `cup.rs` takes it: a sphere
meeting the rim (a circle at the ground), the liner wall, the bottom and the green around the hole, in a local frame
tilted with the green. Contacts are stiff springs with damping set by a restitution (turf edge 0.3, liner 0.45,
bottom 0.3), Coulomb friction that also turns the spin (turf 0.4, liner 0.25), and the green's rolling resistance on
turf. 20 µs steps, only at the hole: a putt costs 0.16-0.27 ms in all. Holed means the whole ball is below the rim.
Everything else falls out:

- dead weight drops; a firm putt hits the back wall below the rim and drops;
- a too-fast putt hasn't fallen far enough by the far side, so the edge throws it up and out;
- off-centre, the rim pushes the ball around the back of the cup: firm putts horseshoe out (a putt half a cup
  out at 1.9 m/s leaves turned 40°), dying ones topple in the side door;
- a ball whose centre never crosses the rim is never taken (the green holds its weight).

Measured on a flat green (Ground Lab, "The 8-inch cup" tab): the regulation cup takes a centred putt up to
~1.9 m/s. Holmes' estimate, `v = 2√(R² − b²)·√(g/2r)` (1.64 m/s), assumes the ball must fall its full radius; in the
model a ball that has fallen most of it meets the far edge near its equator and is turned back in. The uncapped
8-inch cup takes ~2.9 m/s.

**The game's speed cap** (`GAME_CAPTURE_CAP_MPS`, 1.8 m/s, D47) is applied inside the model, not as a rule: a ball
that reaches the hole fast feels less of gravity while it is over the hole, `(v0/v)^4` for v > v0, with v0 set so a
centred putt drops up to the cap (`Cup::hole_pull`). It dips less and kicks off the far edge, which is what a
too-fast putt looks like; putts below ~1.5 m/s are untouched. The fall is quick (well under a frame at real speed),
so the display can't tell.

Why 8 inches and not 10: a make-rate model (flat putts, start-line error σ 1° for a good phone putter and 2.5° for a
casual one, pace error 8 % and 15 %, aiming 0.4 m past; the error sizes are guesses until the pilot measures them):

| Cup | Casual: 1 / 2 / 3 / 5 m | Good: 1 / 2 / 3 / 5 m |
|---|---|---|
| 4.25" regulation | 63 / 28 / 15 / 7 % | 98 / 72 / 49 / 25 % |
| 6" | 78 / 42 / 26 / 13 % | 98 / 84 / 65 / 39 % |
| **8", cap 1.8 m/s** | **85 / 53 / 34 / 17 %** | **98 / 87 / 74 / 50 %** |
| 10", cap 2.0 m/s | 87 / 62 / 42 / 23 % | 98 / 87 / 77 / 58 % |

Past 8 inches most misses are pace misses, which a wider cup doesn't help; and the putting assist adds forgiveness
on top.

## Materials

`data/materials/physics.v1.json`, per C0 class id (D18; it replaces the bundle's `materials.json` physics block for
the engine): `cor_scale`, `cor_min`, `tilt_deg`, `tilt_max_deg`, `bounce_mu`, `slide_mu`, `roll_decel` or `stimp_ft`,
`roll_c1`, `static_factor`, `plug_speed`. Where the values come from:

| Surface | Source |
|---|---|
| Green | Penner's curve and μ 0.4, stimp 10.5; **tilt 18° (max 30°), fitted** with `green_sweep.rs` so tour wedges check and spin back (D44). Penner's 15.4° let a tour PW release ~2 yd. |
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
| Cup | holed through the centre at 1.0 and 1.4 m/s and 30 mm off-centre at 1 m/s; lips out at 2.2 m/s; misses past the rim (cup model, D48) | met |
| Settling | a ball dropped 0.5 m from every pin of the six fixtures (4 directions) rests within 3 m | met |
| Fuzz | 900 random shots (speeds 40-170 mph, launch 5-35°, wind to 10 m/s, any direction) on three fixtures: no timeout, no NaN, no ball resting below the ground | met |
| Determinism | bit-identical repeats | met |

**Speed** (`golf-lab bench`): a whole shot (flight + bounces + roll) 0.52-0.59 ms over real terrain; a 10 m putt
0.47 ms. Within the shot budget. The putting assist runs 6-10 putts per stroke, so a patch cache or a longer roll
step on smooth greens is an M7 item.

**Found on the way:** the M3 fuzz found a flight bug. A hop off a slope could start a hair below the surface, never
rise 1 cm, and so never switch on contact testing, falling through the ground. golf-flight now tests for contact
after 1 cm of clearance **or** 50 ms, and always catches a ball 2 cm underground.

## Ground Lab (2026-10-13)

`golf-lab ground-lab` plays the engine on analytic terrains (`FnTerrain`: a height function and a class function)
and writes `out/viewer/ground.html`: a side view per scenario, a top view for the cross slope, speed and spin after
landing for the wedges. Each shot carries its prediction; the test `every_ground_lab_shot_behaves_as_predicted`
fails if one stops behaving.

| Scenario | Result |
|---|---|
| Crest (green, 0.5 m rise, curvature fits a 6.2 m/s liftoff) | 4.0 m/s at the crest stays down; 7.4 m/s goes airborne |
| Hollow | rolls back and forth, settles at the bottom |
| Drop (a rounded lip onto a lower level) | below ~0.9 m/s rolls over the lip; faster flies off it |
| Steep face | a roll climbs it only above ~6 m/s; a pitch into it stops at the foot (4.5 m) |
| Cross slope (2 %, 4 m putt) | aimed straight, breaks 65 cm and misses the 8-inch cup; read right it drops at stimp 8, 10.5 and 13 (reads 51, 67, 85 cm) |
| Tour wedges | low spinner +5.6 yd (grabs, then a slow roll), check shot +1.2, full PW −2.0 (spins back), lob +0.1 |
| Backstop (a slope behind the pin) | the full PW comes back 18.1 m down the slope; the check shot 6.4 m |

## Not yet

- The flagstick (in or out is a game setting) and slam-dunks: a ball arriving in the air bounces on the height
  function over the hole and is taken only once it rolls (`cup.rs` starts from rolling).
- The putter's skid at the start of a putt (M4's putter impact); `roll_from` starts a putt already rolling.
- Lips and walls (when `edges.json` exists), wet ground, grain on greens.
- Lie-dependent variation inside a class (the lie layer, after M7).
