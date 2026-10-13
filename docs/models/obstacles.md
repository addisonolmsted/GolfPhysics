# Obstacles, water and rule facts

Trees, stands, the flagstick, lips and walls, water, and the geometric tests behind OB and penalty areas. Status:
**draft, M0 (2026-10-11)**. Milestone M6.

## Trees with full proxies (in-play instances)

C0 marks an instance as in play when it is within 30 m of play or a walk. Those get explicit proxies:

- **Trunk:** a capsule from the base (the height function at `(x, y)` minus `sink_m · s`) along the lean (tilted
  `1.5 k°` towards the yaw for lean class `k`) up to the crown base, with the asset's trunk radius × scale. A snag
  is its trunk alone.
- **Crown:** a set of spheres filling the crown from `h (1 - LCR)` to `h`, following the asset's profile (cone for
  pines, egg for broadleaves), radius `h0 · k0 · FORM_K[f] · s`, shifted downwind by the lean offset.

The real numbers will come from the asset library, which doesn't exist yet. Until then (D20):

- **Generic models** per kind (pine, broadleaf, snag, shrub, grass clump) in `data/trees/generic.v1.json`: `h0`,
  `k0`, trunk radius, live crown ratio and crown profile, all placeholders, applied to each instance's own scale,
  crown form and lean.
- **The level of detail is a dial:** the number of spheres per crown, the branch-hit density, whether shrubs and
  clumps get proxies at all.
- **Cost sweep (M6):** raise each dial step by step on the fixtures and measure the CPU per shot against the 3 ms
  budget and how much the outcomes change. The point where more detail stops changing outcomes, or starts costing
  too much, becomes a **bound for the asset library** (max spheres per crown, max proxies per tree, minimum trunk
  radius worth modelling). Those bounds go to the front-end team with the asset specification.
- **Ownership later:** the generator places the assets and should ship their resolved, world-space proxies; see
  [05 §The spatial index](../05-generator-requests.md#the-spatial-index-who-owns-what).

**Trunk hit:** a solid impulse (restitution about 0.4-0.6 for bark, friction), plus a small seeded jitter in the
normal so two identical shots off a tree don't look scripted.

**Crown pass (porous):** inside a crown, a ball meets twigs and leaves at a rate `λ = density × σ` per metre, where
`density` is per species. Over a step of length `ds`, the chance of a hit is `1 - exp(-λ ds)`. A hit:

- multiplies the speed by a seeded factor (e.g. 0.5-0.9),
- deflects the direction within a seeded cone,
- cuts the spin.

Many small hits give the familiar results without special cases: a ball through the top of a tree loses some
distance; a ball into the heart of a crown drops nearly straight down.

**Determinism:** the n-th crown-hit test of a shot draws from sub-stream `("crown", n)`, so the number of draws is
fixed per step regardless of earlier outcomes ([01](../01-conventions.md#determinism)).

## Deep woods (porous stands)

Instances that are not in play are render-only. Physics treats each stand as a volume, from C0's `stands[]`: the
outline, the mean canopy height, the crown share. Inside the outline:

- below the crown base, a **trunk-hit rate** from the stand's stem density (a hit is a trunk impulse in a random
  horizontal direction);
- between the crown base and the canopy top, the crown-pass model above with the stand's density.

The ball usually ends up on the woods floor (class 11) not far inside the edge, which is what players expect.

## Flagstick

See [ground.md](ground.md#the-cup). In flight, it is tested in the last few metres around the pin.

## Lips and walls (when `edges.json` exists)

- **Lip:** a strip along the bunker edge with a crest height and overhang. A ball rolling or bouncing into it from
  the sand side is stopped or plugged under the overhang with some probability; from the grass side it is just
  ground.
- **Wall:** a vertical segment with the style's restitution.

Until `edges.json` exists, bunkers are pure heightfield.

## Water

- The ball crosses a water surface (`water.json` gives the outline and surface height): it stops there (event
  `EnterWater`, rest state `InWater`). No skipping off water in v1.
- Heights under water are the bed, so the ground test must check the water surface first.

## OB and penalty areas (facts only)

- **In bounds:** C0's rule is that a ball is OB when **the whole ball** is outside the property ring. With the ball
  centre `c`, OB means `c` is outside the ring **and** its distance to the ring is more than `r`.
- `CrossOB` event: the first time in the shot that the whole ball is outside (it may come back in on a bounce; the
  rest position decides the fact, the event is for the camera and commentary).
- **Penalty areas** (red and yellow polygons in `rules.json`): the rest position's area, and the **last point where
  the ball crossed into that area's margin** (`CrossPenaltyMargin`). The rules need that point for relief, and only
  the engine knows the path.
- All polygon tests go through the 4 m grid; each cell holds only the edges that cross it.
