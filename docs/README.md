# docs

The authoritative documentation. When code and a doc disagree, one of them is a bug; fix it in the same change.

## Numbered docs (read in order)

| Doc | What it covers |
|---|---|
| [HANDOFF.md](HANDOFF.md) | Where the work stands and what's waiting on the owner. Read first. |
| [00-architecture.md](00-architecture.md) | The shot pipeline, the hand-off types, who is authoritative, why |
| [01-conventions.md](01-conventions.md) | Units, frame, naming, determinism, errors, tests |
| [02-latency-budget.md](02-latency-budget.md) | Phone-to-display budget and the server CPU budget |
| [03-success-indicators.md](03-success-indicators.md) | Calibration targets and the gate for each milestone |
| [04-decision-log.md](04-decision-log.md) | Every decision, its date, owner and status |
| [05-generator-requests.md](05-generator-requests.md) | Our answers to the generator's questions and what we ask it for |

## Sections

| Folder | What it holds |
|---|---|
| [contracts/](contracts/README.md) | The three seams with other systems: P0 course input, P1 swing input, P2 shot result |
| [models/](models/README.md) | The physics of each step: impact, flight, ground, obstacles, wind |
| [gameplay/](gameplay/README.md) | Assists: the speed curve, target-aware correction, fairness |
| [reference/](reference/README.md) | Notes on the earlier repos (GolfEngine, SwingApp v0) |

## Writing rules

- Every doc says its status and date at the top (`draft`, `current`, `superseded by …`).
- Decisions are marked inline as `decided` (owner, date) or `proposed`, and listed in the decision log.
- Numbers carry their source: a reference, a measurement (with the command that produced it), or "placeholder".
- A new folder gets a README and a line in [ARCHITECTURE.md](../ARCHITECTURE.md).
