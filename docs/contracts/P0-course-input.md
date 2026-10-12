# P0: Course input

How `golf-course` reads a C0 bundle, and the queries it answers for the rest of the engine. Status: **current, M1
(2026-10-12)**: implemented in `golf-course`, written against C0 v4.2 bundles (`c0_version: 4`, `bundle_format: 1`).

## What we load

**Only the files in `manifest.json` → `subsets.physics`** (decided, owner, 2026-10-11: D13). The reader never opens
the render files, the phone maps or `outlines.json`, and it works on a bundle that contains only the physics subset.
Per layer:

| Layer | We use it for | Decode notes |
|---|---|---|
| `manifest.json` | versions, tier extents, encodings, hashes, prevailing wind, hole list | Refuse a `c0_version` or `bundle_format` we don't know. |
| `height/play_0p5m_*`, `height/core_2m`, `height/full_8m`, `surround/` heights | the height function | u16 tiles of 256 × 256 with a one-sample overlap; `planar_zigzag_byteplanes` filter (see the layer entry's `filter_def`); zstd frames concatenated per file; value = `(offset_q + q) × scale_m`; nodata = 65535. |
| `surface/class_1m_*` (0.5 m once the generator ships it) | the surface class under the ball | u8 class ids; outside the play tiles, the cover raster. |
| `materials.json` | class ids and names; default physics values | Our override replaces the physics block (below). |
| `holes.json` | tees, greens, pins, line of play, aim targets | Points are `[x, y]` (check `point_order`). |
| `rules.json` | the OB ring(s), penalty areas (red, yellow) and their reference points, drop zones, walks | The OB test: the **whole ball** outside the property ring. |
| `water.json` | water outlines and surface heights | Heights in the tiers under water are the **bed**; the water surface is here. |
| `instances.bin` | trees and bushes: position, asset, scale, crown form, lean, in-play flag | Format `c0in` v4 (C0 §instances.bin). Proxies need the asset table; see [05](../05-generator-requests.md#gaps-we-found-in-the-fixtures). |

`verify()` checks every loaded file's blake3 against the manifest at load. `golf-lab` may read other files for
offline comparisons (e.g. `outlines.json` as a reference for the class raster in M3); the engine never does.

## The height function

C0 makes the interpolation part of the contract, so ours must match the generator's reference reader exactly:

- At a point, use the **finest tier whose 4 × 4 Catmull-Rom nodes around the point are all present**:
  `play_0p5m`, then `core_2m`, then `full_8m`, then the surround's 8 m.
- Weights for `t ∈ [0, 1)`: `[-t³/2 + t² - t/2, 3t³/2 - 5t²/2 + 1, -3t³/2 + 2t² + t/2, t³/2 - t²/2]` (from the
  manifest's `height_function`).
- **Normals come from the analytic derivative of the same bicubic**, not from finite differences. This is exact
  and as cheap as the height itself. GolfEngine used central differences with an 8 cm step.
- Curvature (the second derivative, also analytic) is available for the ground model's liftoff test on crests.

The tiers are blended by the generator at their seams (8 samples), so there is no step to handle.

**Conformance check** (M1 gate): `golf-lab conform <bundle>` compares our `height(x, y)` with
`course_bundle::reader::Heights::z` at 100k random points (including seams) and fails above 1e-9 m. This is the
only place we link the generator's crate.

## In-memory layout and cost (measured, M1, 2026-10-12)

- Height tiles stay **`u16` plus the tile's offset**, converted per node read: one integer add and one multiply,
  exactly the generator's arithmetic, so the values are bit-identical to its reader's. Class tiles stay `u8`.
- Tiles sit in a dense rectangle of slots per tier, so finding a node's tile is arithmetic, not a map lookup. When
  all 16 nodes of a lookup belong to one tile (most of the time), they are read in one pass.
- OB rings, penalty areas and water bodies are even-odd regions on an 8 m grid (4 m for water). Each cell knows
  whether its centre is inside and which edges touch it, so a point test only checks the edges of its own cell.
- The whole `Course` is immutable after load and can be shared with `Arc`.

Measured on the six v1 fixtures (`golf-lab bench`, release build, M-series Mac):

| Measure | Result | Budget ([02](../02-latency-budget.md)) |
|---|---|---|
| Load: read + blake3 + decode | 78-105 ms (decode is ~95 % of it: pure-Rust zstd and the planar unfilter) | < 300 ms |
| Resident rasters | 13-17 MB per course | (none set; fine) |
| `height` | ~21 ns | |
| `surface_at` (height + gradient + second derivatives) | ~39 ns | |
| `surface` (class) | ~4 ns | |
| `in_bounds` + `penalty_area` | 8-44 ns (more rings, more edges per cell) | |
| A worst-case 20k-step roll, one `surface_at` per step | ~0.8 ms | ground 0.5 ms: **M3 adds a patch cache** (a rolling ball stays in one 0.5 m cell for ~100 steps at 1 ms, so the 16 nodes can be reused) |

**Conformance** (`golf-lab conform`, built with `--features conform`): our height function equals the generator's
reference reader **bit for bit** (max difference 0) at 100k points per fixture, two thirds of them at tile seams,
on all six fixtures. Note that the reference reader evaluates only the three C0 tiers, and `full_8m` already covers
the whole 3 km world, so the surround tier answers only outside the world.

## The queries (as built, M1)

```rust
impl Course {
    fn open(dir: &Path) -> Result<Course, CourseError>;          // physics subset only, blake3-verified
    fn height(&self, x: f64, y: f64) -> Option<f64>;             // THE height function; None past the surround
    fn surface_at(&self, x: f64, y: f64) -> Option<Surface>;     // z, gradient, second derivatives, tier
    fn normal(&self, x: f64, y: f64) -> Option<Vec3>;
    fn surface(&self, x: f64, y: f64) -> SurfaceClass;           // play raster, then cover_4m, then 0
    fn material(&self, class: SurfaceClass) -> &Material;        // bundle defaults today; our overrides from M3
    fn water_at(&self, p: Vec2) -> Option<WaterHit>;             // body (if any) + surface height
    fn in_bounds(&self, centre: Vec2, ball_radius_m: f64) -> bool;               // whole-ball rule
    fn penalty_area(&self, centre: Vec2, ball_radius_m: f64) -> Option<&PenaltyArea>; // in or touching
    fn hole(&self, number: u8) -> Option<&Hole>;                 // tees (with sets), green, pin, line of play
    // trees_near_segment: M6, once tree proxies exist (generic models, D20)
}
```

**What is wet** (decided by the data, 2026-10-12): the **class raster** decides; `water.json` only names the body
and gives its surface height. On fixture 1920628, two dry islands in the lake are exported as `pond` bodies, so a
ring test alone would call them water (found by `golf-lab/examples/water_audit.rs`; reported in
[05](../05-generator-requests.md#gaps-we-found-in-the-fixtures)). Water outside every body takes the surround's 8 m
water surface, else the ground height.

**Rule facts.** OB: a ball is out only when the whole ball is out (outside an in-bounds ring, or inside an
out-of-bounds ring such as the clubhouse, and more than one ball radius from the line). Penalty area: any part of
the ball in or touching it. A creek can run on past the property while its penalty area stops at the line.

## Materials

C0 ships per-class defaults (`restitution`, `rolling_resistance`, `sliding_friction`, `spin_retention`, `firmness`,
`lie_penalty`, `stimp` on greens) and says the physics team owns the values. We keep ours in
`data/materials/physics.v1.json`, keyed by C0 class id, and apply them over the bundle's on load (D18; from M3).
The model needs these per class (detail in [ground.md](../models/ground.md) and [impact.md](../models/impact.md)):

| Parameter | Used by | Meaning |
|---|---|---|
| `cor_max`, `cor_decay_spm` | bounce | normal restitution `e = cor_max · exp(-cor_decay · |v_n|)` |
| `bounce_mu` | bounce | tangential friction coefficient at impact |
| `turf_tilt_rad_per_mps`, `turf_tilt_max_rad` | bounce | how far the effective normal leans back towards the incoming ball as it digs in (soft = more) |
| `firmness_pa` | bounce, plugging | resistance to penetration; plugging threshold |
| `slide_mu` | roll | sliding friction until the ball rolls |
| `roll_resistance` | roll | rolling resistance coefficient; for greens, derived from `stimp` and not stored |
| `stimp_ft` | roll (greens only) | green speed |
| `lie` block | impact | speed factor, spin factor, launch change, variability (fliers): see [impact.md](../models/impact.md#lie) |
| `capture` | ground | `water` (stop, penalty), `plug_soft` (sand), none |

Our file starts from C0's defaults and GolfEngine's surface table (both placeholders) and is tuned in M3 and M10.
A retune is a new file version, never an edit.

## Not in the bundle yet (we cope until it is)

| Missing | Until then |
|---|---|
| Tree proxy parameters (asset library) | Generic models per tree kind in `data/trees/generic.v1.json` (D20). See [05](../05-generator-requests.md#tree-proxies-before-the-asset-library-exists). |
| Wind speed | The game picks a mean speed per round. |
| `edges.json` (lips, walls) | Bunkers are pure heightfield; no lip behaviour. |
| `physics/lie_*` | The class alone sets the lie. |
| 0.25 m green patches | 0.5 m heights; M3 decides whether they're needed (D15). |
