# golf-lab

Offline tools. Not part of the shipped engine.

| Command (planned) | Milestone | What it does |
|---|---|---|
| `smoke <bundle>` | M1 | the physics-starter smoke test, all five steps |
| `tees <fixtures dir>` | M1 | hole lengths per par and tee set, the table behind the baseline carries ([assists](../../docs/gameplay/assists.md)) |
| `conform <bundle>` | M1 | our height function vs the generator's `course-bundle` reader (the only link to the generator's crates) |
| `fit-aero` | M2 | fits the aero coefficients to the launch-monitor table; writes `data/aero/` |
| `table` | M2, M4 | launch or delivery in, carry, apex, landing angle and roll out, as a table against the targets |
| `fuzz <bundle>` | M3 | random shots, checks the invariants (no NaN, no tunnelling, every shot rests) |
| `replay <swings.jsonl>` | M5 | runs recorded swings through the whole engine, raw vs assisted |
| `view` | M2+ | writes an HTML or SVG view of trajectories over a hole for review |

Sweeps use a fixed pool of at most 4 workers (see [HANDOFF](../../docs/HANDOFF.md#machine-safety)).

**Depends on:** `golf-core`, `golf-course`, `golf-shot`. **Status:** scaffold (M0).
