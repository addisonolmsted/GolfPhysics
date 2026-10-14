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

**Sizes.** The game draws the ball at 2x a real one (85.3 mm) and the ball touches things at that size (D49):
`CONTACT_RADIUS_M`. Flight, spin, inertia and rolling keep the real 42.67 mm ball. The game cup is **10 inches
(254 mm)** across (D51), 3 balls wide against 2.5 for a real ball in a real cup; `Cup::regulation` (108 mm) stays
for tests and the lab. The liner is 4 inches deep.

**The hole is geometry, not a rule** (D48). Once the ball's centre is over the hole, `cup.rs` takes it: a sphere
meeting the rim (a circle at the ground), the liner wall and the green around the hole, in a local frame tilted with
the green. Contacts are stiff springs with damping set by a restitution (turf edge 0.3, liner 0.45), Coulomb friction
that also turns the spin (turf 0.4, liner 0.25), and the green's rolling resistance on turf. Geometry uses the
contact radius; the contact point's slip and the spin use the real one. 20 µs steps, only at the hole: a putt costs
~0.3 ms in all.

**Holed** is the whole ball below the rim, or any touch of the bottom (D50: no bounce-outs from the bottom, dunks
included). With the 85 mm ball in a 102 mm cup the first always comes first.

Everything else falls out of the contacts:

- dead weight drops; a firm putt hits the back wall below the rim and drops;
- a too-fast putt hasn't fallen far enough by the far side, so the edge throws it up and out;
- off-centre, the rim holds the ball against itself and bends its path round the cup (**rim rides**). A ball that
  slows while hanging over the hole falls in; one still fast where the rim turns away spins out. A fast ball can
  drop part-way and swirl round inside the lip. On a flat green ~1 in 40 edge putts (lines 55-100 % out) go more
  than half-way round, a few drop after 290-500°; on a 2 % side slope it is far more common for balls entering on
  the high side, which gravity holds against the rim, and absent on the low side (`examples/rim_ride.rs`);
- a ball whose centre never crosses the rim is never taken.

**The speed cap** (`GAME_CAPTURE_CAP_MPS`, 2.0 m/s, D51): uncapped, the 10-inch cup takes centred putts up to ~3 m/s
(a putt heading ~9 m past at stimp 10.5). The cap is applied inside the model: a ball that reaches the hole fast
feels less of gravity while over the hole, `(v0/v)^4` above v0, with v0 set so a centred putt drops up to the cap
(`Cup::hole_pull`). It dips less and kicks off the far edge, as a too-fast putt does. Slow putts are untouched.

**The flagstick** (D52) is **in for any shot from off the green** (the collar included) **and out for any shot on
it**; `Cup::flagstick` can force it either way. It is a pole 25.4 mm thick and 2.13 m tall in the middle of the cup.
A flexible fibreglass pole loose in its ferrule soaks up most of a ball's pace: restitution 0.12, friction 0.2 on
the contact point's slip, which also changes the spin (`cup::pin_impulse`, one impulse for both places it is hit):

- in flight (`golf_flight::Hole::pin_hit`, a swept test against the pole each 10 ms step, above the rim);
- at the hole, inside the cup model. The cap's reduced pull is recomputed from the speed the ball leaves the pin
  with, so a firm chip the pin has stopped falls normally.

Measured: a ball rolling into the pin drops up to ~3.5 m/s at the hole (the same pace putted from the green, pin
out, lips out over the 2.0 m/s cap); at 5 m/s it rebounds and stops ~20 cm short. A pitch at the pin in the air
rebounds, and the backspin it still carries turns it back the way it came when it lands.

**Slam dunks**: a ball coming down within reach of the opening (centre within the cup radius plus the contact
radius) ends its flight at rim height as a `Surface::Hole` contact and goes to the cup model, from the air, with full
gravity (the cap is for putts). Dunks onto the pin, beside it, or into the back wall with the pin out are holed:
touching the bottom or going below the rim (D50). The rim can still throw a ball out. Before this, the height
function had no hole and a ball landing on the cup bounced on it.

Make rates (flat putts; start-line error σ 1° for a good phone putter and 2.5° for a casual one, pace error 8 % and
15 %, aiming 0.4 m past; the error sizes are guesses until the pilot measures them):

| Cup | Casual: 1 / 2 / 3 / 5 m | Good: 1 / 2 / 3 / 5 m |
|---|---|---|
| Regulation, real ball | 63 / 28 / 15 / 7 % | 98 / 72 / 49 / 25 % |
| 8 in, 2x ball | 84 / 51 / 32 / 16 % | 98 / 87 / 72 / 47 % |
| **10 in, 2x ball, cap 2.0 (game)** | **87 / 60 / 40 / 21 %** | **98 / 87 / 77 / 56 %** |
| 12 in, 2x ball, cap 2.0 | 87 / 66 / 47 / 26 % | 98 / 88 / 78 / 62 % |

12 inches was considered: it pushes good putters well past tour make rates from 3-5 m, on top of the putting assist.

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
| Cup | regulation cup with the 2x ball: holed through the centre at 0.6 and 1.0 m/s, lips out at 1.7; game cup: 80 mm off line holed, 1.7 m/s holed, 2.2 lips out (cup model, D48-D51) | met |
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
| Cross slope (2 %, 4 m putt) | aimed straight, breaks 65 cm and misses the 10-inch cup; read right it drops at stimp 8, 10.5 and 13 (reads 51, 67, 85 cm) |
| Tour wedges | low spinner +5.6 yd (grabs, then a slow roll), check shot +1.2, full PW −2.0 (spins back), lob +0.1 |
| Backstop (a slope behind the pin) | the full PW comes back 18.1 m down the slope; the check shot 6.4 m |

## Not yet

- The putter's skid at the start of a putt (M4's putter impact); `roll_from` starts a putt already rolling.
- Lips and walls (when `edges.json` exists), wet ground, grain on greens.
- Lie-dependent variation inside a class (the lie layer, after M7).
