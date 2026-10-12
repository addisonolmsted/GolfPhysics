# 05: Generator requests

Our answers to the ten questions at the end of the generator's
[physics-starter.md](../../GolfProceduralGenerator/docs/physics-starter.md), plus the gaps we found reading the
fixtures. Each answer decides what the generator builds next, so each is marked:

- **firm:** we are confident; the generator can act on it.
- **proposed:** our best guess now; it will be confirmed by a measurement in the milestone named.
- **not needed:** the generator can drop it.

Status of the whole page: **proposed, M0 (2026-10-11).** Nothing here has been sent to the generator side yet.

## The ten questions

| # | Question | Answer | Status |
|---|---|---|---|
| 1 | Heights: is 0.5 m enough? Bicubic or bilinear? | **Deferred until shots are modelled** (owner, 2026-10-11). We use what ships (0.5 m, bicubic) and compare in M3: putts and bounces on fixture greens and bunker faces, bicubic against bilinear, 0.5 m against an upsampled 0.25 m, looking at both feel and cost. The result decides the answer. | deferred (M3) |
| 2 | Frame: z-up or y-up? | z-up, as shipped. | firm |
| 3 | Surfaces: which parameters per class? Is 1 m fine enough? | The parameter list is in [P0 §Materials](contracts/P0-course-input.md#materials). We own the values and keep them in our own data file. **The resolution is deferred until shots are modelled** (owner, 2026-10-11). In M3 we'll measure how often a 1 m raster gets the lie wrong next to bunker and green edges, compared with the polygons in `outlines.json` (used only as an offline reference). | params: firm; resolution: deferred (M3) |
| 4 | Lie: a grass height and density layer, or is the class enough? | The class is enough for M1-M4. We want the lie layer (grass height in cm, density 0-1, at 0.5 m) later, for variation inside a class: thin and thick rough, and fescue patches. | proposed (after M7) |
| 5 | Trees: are trunk capsule plus crown spheres right? How should a ball move through porous woods? | Yes, capsule plus spheres. Inside a crown or a deep-woods stand, a ball takes branch hits at a rate proportional to path length × foliage density. Each hit costs speed and deflects the ball randomly (seeded). Details in [obstacles.md](models/obstacles.md). For deep woods we need each stand's outline, canopy base and top heights and its density (C0's `stands[]`). | firm |
| 6 | Lips and walls: geometry or a flag? | **Both, as C0 already drafts:** lips as a flag on the heightfield plus `crest_h_m` and `overhang_m` per vertex (we'll treat a lip as a steep strip that can stop or plug the ball); walls as segments with a style restitution. Not needed until M6. | proposed (M6) |
| 7 | Spatial index: ship a per-hole grid, or does the engine build its own? | **Open, owner to decide.** The recommendation is to split it: the generator owns the collision geometry, the engine owns the lookup grid. See [§The spatial index](#the-spatial-index-who-owns-what) below. | open |
| 8 | Rules: does the engine apply OB and penalty relief? | The engine reports the facts (rest point, inside or outside the OB ring, which penalty area, the last point where the ball crossed into it). The game layer applies relief, using `rules.json`'s reference points and drop zones. | proposed |
| 9 | Wind: mean speed per course? Gusts from `variability_seed`? | Yes to both. Please ship a **mean speed per course** (or a distribution: mean and spread, from which the round seed draws the day's wind). We define the gust model on our side, seeded from `variability_seed` + the round. If the generator can't pick a speed yet, the game will. | proposed |
| 10 | Loading: memory, load time, sharing? | Target under 300 ms load from local disk; memory measured in M1 (we expect 30-60 MB decoded). One loaded course is shared by every round on it (`Arc<Course>`). | proposed (M1) |

## The spatial index: who owns what

Owner's position (2026-10-11): the generator should own it, because it places the assets and builds the height
map, and courses are prebuilt into a pool, so nothing should be rebuilt per session.

The index has two parts, and they have different natural owners:

1. **The collision geometry:** what is where and what shape it has. Each in-play tree's trunk capsule and crown
   spheres in world coordinates, each deep-woods stand's volume and density, the lips and walls. **The generator
   should own this**, for the reasons the owner gives: it places the assets, it knows the ground they stand on, and
   its validation step (S11) can check the geometry before a course enters the pool (a crown through the ground, a
   trunk in a bunker). Today C0 makes the engine assemble proxies from instance records plus an asset library; a
   resolved, world-space proxy layer (`physics/proxies.bin`) shipped with the bundle is cleaner. The engine then
   only reads shapes, never computes where they are.
2. **The lookup grid:** which shapes and polygon edges fall in each cell, so a flight step only tests what is near
   it. This is a derived, performance-only structure, and **the engine should own it**:
   - **It's a function of part 1 alone.** It holds no information the bundle doesn't already have, so shipping it
     stores the same facts twice, and the two copies can disagree.
   - **Its best shape depends on the integrator,** not on the course: cell size, per-hole or global, how the
     entries are packed. We'll change it while optimizing in M2-M6. If it lives in the bundle, each change is a
     contract change plus a backfill of every course in the pool.
   - **It's cheap and isn't rebuilt per session.** A course has about 1.5k in-play trees and a few thousand polygon
     edges; the grid builds in about a millisecond (to be measured in M1). The server builds it once, when it first
     loads a course, and shares it across every round on that course. It can also be cached on the server's disk,
     keyed by the bundle hash and the index version, so it is built once per course, much like the pool itself.

If the owner still prefers the generator to ship the grid too, it works on two conditions: the physics side writes
its format into P0, and the format is versioned so the engine can fall back to building its own when the versions
differ.

## Tree proxies before the asset library exists

Owner's direction (2026-10-11): the front-end team decides what variety the asset library needs, the generator
builds the assets to that specification, and physics reads them. Until then, physics tests on **generic models**,
and uses them to set **bounds for the asset library**.

- **Generic models** (`data/trees/generic.v1.json`): one per kind (pine, broadleaf, snag, shrub, grass clump), built
  from what `instances.bin` already carries (height scale, crown form, lean) plus our own placeholder `h0`, `k0`,
  trunk radius and live crown ratio per kind. Crown sphere count and foliage density are parameters, not constants.
- **Cost sweep** (M6, in `golf-lab`): shots through and near trees on the fixtures, with the number of crown spheres,
  branch-hit density and proxies per tree raised step by step. We measure the CPU per shot against the 3 ms budget
  ([02](02-latency-budget.md)), and how much the extra detail changes outcomes.
- **Bounds for the front end** (an M6 deliverable): for example, the maximum spheres per crown, the maximum proxies
  per tree, the minimum trunk radius worth modelling, and the in-play radius. These limits go to the front-end team
  as part of the asset specification.

## Gaps we found in the fixtures

| Gap | Why it matters | Ask |
|---|---|---|
| **Tree proxy parameters.** `h0`, `k0`, `sink_m`, the trunk radius and the live crown ratio live in the asset library, which isn't built (`asset_library.version: null`). | We can't place real proxies yet. | Nothing for now: we use generic models (above). Later, the resolved proxy layer (part 1 of the spatial index section). |
| **Wind speed.** `prevailing_wind.speed_mps` is null. | See Q9. | See Q9. |
| **Class raster resolution.** C0 says classes at 0.5 m; the bundle ships `class_1m_*`. | See Q3. | See Q3. |
| **Cup position precision.** Pins are given at 1 m granularity in the fixtures (e.g. `[1192.0, 1186.0]`). | Fine for flight, but putting and the flagstick need the exact spot. The 1 m value is OK only if it is the true spot by definition. | Confirm that the pin `xy` is the true cup centre, or ship it at cm precision. |
| **Flagstick.** No height or radius. | Flagstick hits need them. | A constant is fine (2.13 m tall, 12.7 mm radius are standard values; to confirm). The game can own this. |
| **Tee sets differ between holes.** Holes have tee sets named `tips/back/middle/forward/front`, `back/middle/forward`, `back/middle_back/middle_forward/forward` or just `back/forward`. 4 of 54 fixture holes have no middle tee at all. | The game (and our baseline-carry tuning) wants a player to play one tee set for a whole round, and the casual baseline is tuned to the **middle** set. | Ship course-wide tee sets: the same named sets on every hole (at least back, middle, forward), with a course total per set. Holes may share a ground between sets. |
| **Islands exported as water bodies** (fixture 1920628). The lake is `water_0` (labelled `creek`; 98.7 % of its ring is class water). `water_1` and `water_2` (labelled `pond`, `in_property: null`, `penalty_areas: [null]`) are dry islands inside it: 2-5 % class water. They should be holes in the lake's polygon. The physics-starter's "3 lakes" for this seed doesn't match `water.json` either. Found by `golf-lab/examples/water_audit.rs`, 2026-10-12. | A ring-based wet test calls the islands water and puts them in the lake. We now let the class raster decide what is wet, so we're not blocked. | Export lakes with their islands as holes (a body with several rings, even-odd), drop the island bodies, check the `creek`/`pond`/`lake` labels, and write `[]` rather than `[null]`. |
| **Water surface vs bed.** Heights under water are the bed; the surface is in `water.json`. | Fine: we test against the water surface first. | None. Noted so nobody "fixes" it. |

## What we don't need

- The render layers (`render/`, phone maps, `surround/render`): already excluded by `subsets.physics`.
