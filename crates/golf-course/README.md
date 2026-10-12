# golf-course

Loads a C0 course bundle (the physics subset) and answers the spatial questions the engine asks: height, normal,
curvature, surface class, material, water, OB, penalty areas, and trees near a segment.

- Contract: [P0-course-input.md](../../docs/contracts/P0-course-input.md).
- Our own reader, without the generator's crates (D13, proposed). Conformance with the generator's
  `course-bundle` reader is checked from `golf-lab`.
- The `Course` is immutable after load and shared across rounds with `Arc`.
- The only core crate that does I/O (file reads at load). Queries are pure.

**Modules:**

| Module | What |
|---|---|
| `bundle` | the manifest, the physics-subset files, the blake3 check |
| `tiles` | the C0 tiling, the planar-filter inverse, zstd frames, Catmull-Rom weights and their derivatives |
| `heights` | THE height function over four tiers, with analytic gradient and second derivatives |
| `classes` | the surface class: play raster, then the surround's `cover_4m` |
| `materials` | `materials.json` (our override file comes in M3) |
| `holes` | tees (with sets), greens, pins, line of play |
| `polygon` | even-odd regions with an O(1) grid point test and boundary distance |
| `rules` | OB rings (whole-ball rule) and penalty areas (in or touching) |
| `water` | water bodies and the surround's water surface; the class raster decides what is wet |
| `instances` | `instances.bin` (`c0in` v4) records |

**Tests:** unit tests in the modules, plus `tests/fixtures.rs` on the six v1 fixtures (skipped when they aren't
mounted; see [fixtures/README.md](../../fixtures/README.md)).

**Depends on:** `golf-core`, `serde`, `serde_json`, `ruzstd` (pure Rust, keeps the wasm32 option open), `blake3`.
**Status:** M1 done (2026-10-12). Costs and conformance results are in
[P0](../../docs/contracts/P0-course-input.md#in-memory-layout-and-cost-measured-m1-2026-10-12).
