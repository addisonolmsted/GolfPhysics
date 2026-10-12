# golf-course

Loads a C0 course bundle (the physics subset) and answers the spatial questions the engine asks: height, normal,
curvature, surface class, material, water, OB, penalty areas, and trees near a segment.

- Contract: [P0-course-input.md](../../docs/contracts/P0-course-input.md).
- Our own reader, without the generator's crates (D13, proposed). Conformance with the generator's
  `course-bundle` reader is checked from `golf-lab`.
- The `Course` is immutable after load and shared across rounds with `Arc`.
- The only core crate that does I/O (file reads at load). Queries are pure.

**Planned modules:** `manifest`, `tiles` (u16 + planar zigzag filter + zstd), `heights` (the bicubic height function
and its analytic derivatives), `classes`, `materials` (bundle defaults + our override file), `holes`, `rules`,
`water`, `instances` (`c0in` v4), `index` (the 4 m grid).

**Depends on:** `golf-core`; later `zstd`, `blake3`, `serde_json`. **Status:** scaffold (M0); M1 is next.
