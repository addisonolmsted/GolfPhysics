# Project Plan

**Goal:** a Rust engine that takes the phone's swing payload and the round state and returns a full shot (flight,
bounces, roll, rest and the rule facts) on a generated C0 course. It must be:

- **Fast.** Under 3 ms of CPU per shot (p99, assists included), so the delay the player feels is only the network round trip.
  See [02-latency-budget.md](docs/02-latency-budget.md).
- **Believable.** Launch-to-carry numbers match launch-monitor tables, greens roll at their stimp, and bounce and
  roll look right on every surface class. See [03-success-indicators.md](docs/03-success-indicators.md).
- **Fun and fair.** Easy to pick up, with a high skill ceiling: a universal speed curve gives smooth swings
  playable carries, and target-aware correction raises the floor while leaving intended draws and cuts alone. The
  same swing gives the same shot whoever makes it. See [gameplay/assists.md](docs/gameplay/assists.md).
- **Deterministic.** The same request gives the same bytes on every platform.

Cross-cutting rules and the repo map are in [ARCHITECTURE.md](ARCHITECTURE.md). The design is in
[docs/00-architecture.md](docs/00-architecture.md).

## Status legend

`done` · `next` · `queued` · `blocked (on what)`

## Milestones

The order is set by dependencies. Flight can be tested with launch numbers typed in by hand, so flight and ground
come before impact and swing. The phone work (M5) can run in parallel from M1 on, because recording swings takes
calendar time.

### M0: Scaffold and docs (`done` 2026-10-11)

- Repo cloned, workspace with 10 crate scaffolds, a README per crate and per docs section.
- Architecture, conventions, latency budget, success indicators, contracts P0-P2 (drafts), model docs, assists
  design, decision log, generator answers (proposed), reference notes on GolfEngine and SwingApp.

### M1: Course reader (`done` 2026-10-12, branch `m1-course-reader`)

`golf-course` reads the six v1 fixtures without the generator's crates, opening only the physics subset (D13,
decided).

- Manifest, blake3 verify, height tiers (u16 + planar zigzag filter + zstd), surface classes, `materials.json`,
  `holes.json`, `rules.json`, `water.json`, `instances.bin`, surround.
- Queries: `height(x, y)`, `normal(x, y)`, `surface(x, y)`, `material(class)`, `in_bounds(p, r)`,
  `penalty_area(p)`, `water_at(p)`, trees near a segment.
- A spatial index (4 m uniform grid of tree proxies and polygon edges) built on load.
- **Result:** all gates met. The smoke test passes on all six fixtures (rolling the ball in step 3 waits for M3).
  Our height function is bit-identical to the generator reader's at 600k points. Load 78-105 ms, 13-17 MB per
  course, height queries 21-39 ns. Numbers and findings: [P0](docs/contracts/P0-course-input.md#in-memory-layout-and-cost-measured-m1-2026-10-12).
  Found two generator issues: the lake islands exported as ponds, and inconsistent tee sets
  ([05](docs/05-generator-requests.md#gaps-we-found-in-the-fixtures)).
- **Gate:** the physics-starter smoke test (its five steps) passes on all six fixtures. Our height function equals
  `course-bundle`'s to 1e-9 m at 100k random points (a `golf-lab` conformance check, the only place we link the
  generator's crate). Load time and memory per course are measured and recorded.

### M2: Flight (`done` 2026-10-12, branch `m2-flight`; apex and landing columns to verify)

`golf-flight` with drag, Magnus lift, spin decay, spin-axis handling, wind with a height profile and air density.

- Fixed-step RK4 at 1 ms, with ground-crossing refinement against the real height function.
- **Result:**
  - Carry: 24/24 fit rows within ±3 %.
  - Sniff tests: 11/11 handicap rows in band, from scratch to a 70 mph driver.
  - Apex: 19/23 within ±10 %. Landing angle: 18/24 within ±3°. All misses but one are against unverified tour
    figures. The exception: the amateur driver lands 3.8° too steep.
  - Shape checks pass (spin, wind, curve, altitude, determinism).
  - Speed: 0.29 ms per flight.
  - Viewer: "Ball Flight Lab" (`golf-lab view`).
  - Details: [flight.md](docs/models/flight.md#results-2026-10-12).
- **Gate:** typed-in launch conditions reproduce the launch-monitor table (carry, apex, landing angle, flight time)
  within the tolerances in [03](docs/03-success-indicators.md), across driver to wedge. Carry, apex and landing
  angle are fitted together, never carry alone (a lesson from GolfEngine).

### M2b: Flight follow-up (`next`)

From the owner's review: add driver hang time (6.1-6.3 s on tour), curve per degree of spin axis (~0.7 % of carry),
the published 7-iron wind pair and "a tailwind never shortens a full shot" as fit targets and tests; bound the drag
speed term below the fitted range; refit. Details: [flight.md](docs/models/flight.md#follow-up-review-2026-10-12-time-curve-and-wind).

### M3: Ground (`queued`)

`golf-ground`: bounce (velocity-dependent restitution, turf tilt, grip/slip impulse with spin), the slide-to-roll
switch, rolling resistance, slope gravity, putting, cup capture, water entry, plugging in sand.

- Material parameters move into our own `materials.physics.json` override, keyed by C0 class id.
- **Deferred decisions settled here (D15):** bicubic vs bilinear, 0.5 m vs 0.25 m on greens and bunker faces, and the
  class raster resolution, each by comparing feel and cost on the fixtures.
- **Gate:** a stimp test (1.83 m/s on a flat green rolls the stimp distance ±3 %). Landing roll-outs per club and
  surface fall within the target bands. A ball dropped on every fixture green settles. No ball tunnels through the
  ground or escapes the world in a 100k random-shot fuzz.

### M4: Impact and clubs (`queued`)

`golf-impact` and the club table (data file): delivery to launch through the D-plane model, a smash factor per
club, a strike-point gear effect, and lie effects from the material and later from the lie layer.

- **Gate:** tour-average deliveries give tour-average launch conditions per club, and face/path combinations give
  the expected curve directions (fade, draw, push, pull, slice, hook).

### M5: Swing input (`queued`, can start in parallel after M1)

- Agree on the P1 payload with the SwingApp side. Give SwingApp a transport, a swing id, timestamps and the raw
  IMU window.
- Record a **swing corpus**: tagged swings (club, intent: full/half/chip/putt, a known miss) across several people.
- `golf-swing` maps the payload to a delivery. The universal speed curve maps phone speed to head speed, with its
  band edges set from the corpus (which must include casual and skilled swingers).
- **Gate:** corpus replay in `golf-lab`; tagged intents land in the right delivery bands, and repeat swings are
  consistent.

### M6: Obstacles and rule facts (`queued`)

Trees (a trunk capsule plus porous crown spheres for in-play trees, porous stand volumes for deep woods), the
flagstick, lips and walls (once `edges.json` exists), the OB test, penalty-area crossings, and water.

- Trees use **generic models** until the asset library exists (D20). A **cost sweep** over proxy detail (spheres
  per crown, branch-hit density, shrub proxies) measures CPU per shot and the change in outcomes.
- **Deliverable:** bounds for the asset library (max spheres per crown, max proxies per tree, minimum trunk radius),
  handed to the front-end team for the asset specification.
- **Gate:** the physics-starter step 5 shots behave. A ball into a lake reports its entry point and its last
  crossing of the penalty margin. A straight shot through a stand is slowed and dropped at a measured rate.

### M7: Assists (`queued`)

`golf-assist` with one fixed, versioned assist configuration: target-aware correction (target zone inferred from the
aim direction and the hole, smallest fix applied in part, scaled by lie, error size, swing speed, quality and club),
the baseline-carry table, putting help. See [assists.md](docs/gameplay/assists.md).

- **Gate:** [03 §5](docs/03-success-indicators.md#5-assists-m7): fairness, floor, ceiling, fading help, no pull into
  trouble. Raw and assisted values are logged and visible in the lab viewer.

### M8: Service (`queued`)

`golf-server`: course cache (one loaded course shared by every round on it), sessions, the phone uplink and the
display downlink, and a compact binary `ShotResult`.

- **Gate:** end-to-end latency is measured from phone impact to the first trajectory frame on the display, on
  Wi-Fi and LTE. The server step is under budget at p99 under load.

### M9: Generator feedback round (`queued`, starts during M1)

Turn the answers in [05-generator-requests.md](docs/05-generator-requests.md) into confirmed requests: the M3
resolution results, the spatial index split (D16), resolved world-space tree proxies, the lie layer, `edges.json`
and wind speed.

### M10: Feel tuning (`queued`)

Playtest loops on the fixtures and on new pool courses: material retunes, the baseline carries, and the shape of the
assist fade with speed.

## Claiming work

A milestone is claimed by putting its branch name next to it here. One milestone per branch. A change to a contract
(P0, P1, P2) or an invariant goes in the [decision log](docs/04-decision-log.md) first.
