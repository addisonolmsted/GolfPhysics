# P0: Course input

How `golf-course` reads a C0 bundle, and the queries it answers for the rest of the engine. Status: **draft, M0
(2026-10-11)**, written against C0 v4.2 bundles (`c0_version: 4`, `bundle_format: 1`).

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

## In-memory layout (*proposed*, to measure in M1)

- Height tiles decoded to `f32` metres relative to a per-tile offset, or kept as `u16` plus offset and converted per
  read. u16 is 4× smaller and the conversion is one multiply-add; pick by measurement.
- Class tiles as `u8`.
- A **lookup grid** (first guess: 4 m uniform cells) over the property, each cell listing: tree proxies (in-play
  trees), deep-woods stands, penalty-area polygons, water bodies, and the OB ring edges that cross it. A segment
  query walks the cells along the segment (a DDA), so a flight step only tests what is near it. Who builds it is
  open (D16): the recommendation is that we build it once per course, from geometry the generator owns, and cache
  it.
- The whole `Course` is immutable after load and shared with `Arc`.

## The queries

```rust
impl Course {
    fn height(&self, x: f64, y: f64) -> Option<f64>;                  // None past the surround
    fn height_grad(&self, x: f64, y: f64) -> Option<(f64, f64, f64)>; // z, dz/dx, dz/dy
    fn normal(&self, x: f64, y: f64) -> Option<Vec3>;
    fn surface(&self, x: f64, y: f64) -> SurfaceClass;                // id from materials.json
    fn material(&self, class: SurfaceClass) -> &Material;             // with our overrides applied
    fn water_surface(&self, x: f64, y: f64) -> Option<(WaterId, f64)>;
    fn in_bounds(&self, p: Vec2, ball_radius_m: f64) -> bool;         // whole-ball rule
    fn penalty_area(&self, p: Vec2) -> Option<PenaltyAreaRef>;
    fn trees_near_segment(&self, a: Vec3, b: Vec3, out: &mut Vec<ProxyRef>);
    fn hole(&self, number: u8) -> &Hole;                              // tees, green, pin, aim targets
}
```

## Materials

C0 ships per-class defaults (`restitution`, `rolling_resistance`, `sliding_friction`, `spin_retention`, `firmness`,
`lie_penalty`, `stimp` on greens) and says the physics team owns the values. We keep ours in
`data/materials/physics.v1.json`, keyed by C0 class id, and apply them over the bundle's on load (*proposed*, D18).
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
