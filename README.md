# GolfPhysics

The physics engine for the golf game. A phone acts as the club. When a swing ends, the phone sends what it
measured. The engine turns that into a ball trajectory (flight, bounces, roll and where the ball comes to rest)
on a course built by the procedural generator. It should run fast enough that the only delay a player notices
is the network.

Written in Rust. Started 2026-10-11 to replace the earlier `GolfEngine` attempt. We now know the generator's
course data (the C0 bundle) well enough to design around it.

## Where it sits

```
 phone (SwingApp)                server (this repo)                      display (web app)
 ────────────────               ─────────────────────────────           ─────────────────
 address, swing,     payload    golf-server                              draws the course
 impact detection  ─────────▶   └ golf-shot: ShotRequest ─▶ ShotResult ─▶ from the same C0
 club, aim, shot type             swing ▸ assist ▸ impact ▸ flight ▸      bundle, animates
                                  ground ▸ rest + rule facts             the trajectory
                                        ▲
                                        │ C0 bundle (physics subset)
                                 GolfProceduralGenerator
```

- **Inputs:** the phone payload ([P1](docs/contracts/P1-swing-input.md)) plus the server's own round state (ball
  position, lie, wind).
- **Course:** a C0 bundle from the generator, physics subset only ([P0](docs/contracts/P0-course-input.md)).
- **Output:** a `ShotResult` ([P2](docs/contracts/P2-shot-result.md)). It holds the sampled trajectory, events
  (bounces, tree hits, water, holed) and the facts about where the ball finished (lie, OB, penalty area), so the
  game layer can apply the rules.

## Read in this order

1. [ARCHITECTURE.md](ARCHITECTURE.md): the repo map and the invariants every crate keeps.
2. [PLAN.md](PLAN.md): milestones, status and what comes next.
3. [docs/HANDOFF.md](docs/HANDOFF.md): where the work stands right now, and the owner decisions waiting.
4. [docs/](docs/README.md): architecture, conventions, latency budget, success indicators, contracts, models,
   gameplay assists and the decision log.

## Related repos

| Repo | Role here |
|---|---|
| `GolfProceduralGenerator` | Builds the courses. Its [physics-starter](../GolfProceduralGenerator/docs/physics-starter.md) and [C0 contract](../GolfProceduralGenerator/docs/contracts/C0-delivery.md) define what we read. Our answers to its open questions are in [docs/05-generator-requests.md](docs/05-generator-requests.md). |
| `SwingApp` | The iOS phone controller. What it does today and what it should send are in [docs/reference/swingapp-v0.md](docs/reference/swingapp-v0.md) and [P1](docs/contracts/P1-swing-input.md). |
| `GolfEngine` | The earlier engine. Only a loose reference: what to reuse and what to avoid is in [docs/reference/legacy-golfengine.md](docs/reference/legacy-golfengine.md). |

## Building

```sh
cargo check            # the whole workspace
cargo test             # unit tests; fixture tests skip themselves when the fixtures are not mounted
```

The test courses are not in git. See [fixtures/README.md](fixtures/README.md).
