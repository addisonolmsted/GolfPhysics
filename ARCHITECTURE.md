# Architecture

The map of this repo. **Update this file every time a folder or file is added.** The detailed documentation is in
[docs/](docs/README.md). Milestones are in [PLAN.md](PLAN.md).

## The engine in one paragraph

A shot is a **pure function**: `simulate(course, ShotRequest) -> ShotResult`. The request carries the phone's
swing payload and the server's round state (ball position, lie, wind). The function goes through
six steps, each in its own crate and each with a typed hand-off:

1. **swing:** phone kinematics become a club delivery (head speed, path, attack angle, face, dynamic loft, strike
   point).
2. **impact:** delivery, club and lie become launch conditions (ball speed, launch angle, direction, spin rate and
   spin axis).
3. **assist:** target-aware correction: the raw shot is simulated, and corrected only if it misses the target zone.
4. **flight:** airborne integration with drag, lift, spin decay, wind and tree contact.
5. **ground:** bounce, check, roll, putting and cup capture on the course's height function and surfaces.
6. **rest:** the final lie plus the rule facts (OB, penalty area, where the ball crossed into it).

The same inputs give byte-identical output on every platform, so a shot can be replayed, audited and, later,
re-simulated on the client. A full shot should take under 3 ms of CPU (p99), assists included. The latency the player feels is set by
the network.

Full treatment: [docs/00-architecture.md](docs/00-architecture.md).

## Repo layout

```
README.md              the front page: what this is, where it sits, the reading order
ARCHITECTURE.md        this file: the repo map and the invariants
PLAN.md                milestones, status, the queue
Cargo.toml             Rust workspace (resolver 2, edition 2024)

docs/                  the authoritative documentation (index: docs/README.md)
  HANDOFF.md           where the work stands; read first in a new session
  00-architecture.md   the shot pipeline, the hand-off types, server/client split, why each choice
  01-conventions.md    units, frame, naming, determinism, error handling
  02-latency-budget.md the end-to-end budget, phone to display, and the per-step CPU budget
  03-success-indicators.md  calibration targets (launch monitor tables, stimp), test tiers, gates
  04-decision-log.md   every decision with its date, who made it, and its status
  05-generator-requests.md  our answers to the generator's ten questions; what we ask it to build
  contracts/           P0 course input (C0 as we read it), P1 swing input, P2 shot result
  models/              impact, flight, ground, obstacles, wind: the physics in each step
  gameplay/            assists: the universal speed curve, target-aware correction, fairness
  reference/           the earlier GolfEngine and SwingApp v0: what we keep and what we drop

crates/                one crate per step; each has a README
  golf-core            units, frame, vectors, deterministic math (libm), seeded RNG, shared types
  golf-course          C0 bundle reader + spatial queries (height, normal, surface, trees, OB, water)
  golf-swing           P1 payload -> ClubDelivery
  golf-impact          ClubDelivery + Club + Lie -> LaunchConditions
  golf-flight          LaunchConditions -> airborne trajectory and the first ground contact
  golf-ground          bounce, roll, putt, cup, rest
  golf-assist          raw shot + target zone -> corrected delivery (one fixed configuration)
  golf-shot            ShotRequest -> ShotResult; the only API the server calls
  golf-server          network service: course cache, sessions, phone and display connections
  golf-lab             offline tools: fixture smoke tests, calibration sweeps, plots, swing replay

data/                  versioned tuning data: calibration targets, aero fit, materials, clubs, tree
                       proxies, the assist configuration (empty at M0; README lists what's planned)
fixtures/              where the test courses live (not in git; README points at them)
```

## Invariants

Every crate keeps these. A change that breaks one needs a decision log entry first.

1. **Units and frame are C0's.** Metres, seconds, radians, kilograms. x east, y north, z up, origin at the world's
   south-west corner. Fields carry unit suffixes (`_m`, `_mps`, `_rad`, `_rpm` only at the edges for display).
   See [01-conventions.md](docs/01-conventions.md).
2. **Deterministic.** f64 throughout. Transcendental functions come from `libm` and never from platform intrinsics.
   Fixed time steps. Every random draw comes from a stream seeded by the shot id. No wall clock, no `HashMap`
   iteration order, no thread-order dependence inside a shot.
3. **Pure core.** The crates from `golf-core` through `golf-shot` do no I/O except `golf-course`'s loader. They
   would build for `wasm32`, so the display could re-run a shot itself later.
4. **One height function.** Heights and normals come from C0's bicubic Catmull-Rom over the finest tier, the same
   function the renderer uses. The ball and the drawn ground never disagree.
5. **Assists are logged, and hidden from players.** The assist step records the raw and the assisted values side
   by side in the server-side result, for the lab and for tuning. Nothing that reaches a player's device shows them.
   It never edits the trajectory mid-flight. Gameplay changes the inputs to the physics, never the physics itself.
6. **The engine states facts; the game applies the rules.** The engine reports where the ball is and what it
   crossed. Stroke counting and relief belong to the game layer.
7. **Data, not code, for tuning.** Club tables, material overrides and the assist configuration are versioned data files
   loaded at start-up. Adding a club or retuning rough is not a code change.
8. **Fair.** The same swing in the same situation gives the same shot, whoever makes it. Nothing in the engine
   reads a player's identity, skill or history (D22).
