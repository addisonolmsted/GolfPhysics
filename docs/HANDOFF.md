# Handoff: where the work stands

Updated 2026-10-11. Read this first when picking the work up in a new session.

## State

- **M0 done:** repo cloned to `~/Documents/GitHub/GolfPhysics`, workspace with 10 crate scaffolds (`cargo check`
  passes; no code yet), and the full documentation set. Branch: `main`; the initial commit is pushed.
- **In progress: M1, the course reader** (see [PLAN.md](../PLAN.md#m1-course-reader-next)).

## Owner decisions waiting

Everything marked `proposed` in the [decision log](04-decision-log.md). The ones that block work soonest:

No owner decisions are blocking. Baseline carries are tuned for the **middle tees** (starting values in
[assists.md](gameplay/assists.md#baseline-carry-called-floor-carry-in-the-first-draft)); the exact values come from
the M5 and M7 playtests. A new generator ask: consistent course-wide tee sets ([05](05-generator-requests.md#gaps-we-found-in-the-fixtures)).

Settled: everything recommended on 2026-10-11 (owner approval 2026-10-12), including D13 (own reader, physics subset
only), D16 (the generator owns collision geometry, the engine the lookup grid), D19 (raw window policy), D20
(generic tree models → asset library bounds) and D21 (feel overrides published data). D15 is deferred to M3. On
2026-10-12 the owner set the assist intent: D22 (fairness: no per-player calibration), with D23 (universal speed
curve) and D24 (target-aware correction) decided in principle. Also 2026-10-12: help fades with speed (tuned in
playtests), D26 (aim direction only; the engine infers the aim point), D27 (one assist configuration, always), D28
(assists always hidden).

## Key references

- **Fixtures:** `/Volumes/GolfOut/out/physics_fixtures/v1/` (six bundles; see [fixtures/README.md](../fixtures/README.md)).
  Mount with `hdiutil attach "/Volumes/T7 Shield/GolfOut.sparsebundle"` if missing.
- **Generator:** `~/Documents/GitHub/GolfProceduralGenerator`, especially `docs/physics-starter.md`,
  `docs/contracts/C0-delivery.md` and `crates/course-bundle/src/{reader,tiles,heights,classes}.rs`.
- **Phone app:** `~/Documents/GitHub/SwingApp` (notes: [swingapp-v0.md](reference/swingapp-v0.md)).
- **Old engine:** `~/Documents/GitHub/GolfEngine` (notes: [legacy-golfengine.md](reference/legacy-golfengine.md)).

## Machine safety

Same rule as the generator repo: one heavy job at a time, at most 4 worker processes. The Mac kernel-panicked on
2026-10-10 from an unbounded parallel batch. Use `xargs -P 4` or rayon with a fixed pool for calibration sweeps,
never a shell `jobs` loop.
