# golf-assist

Gameplay assists that raise the floor and keep the ceiling: **target-aware correction**. It simulates the raw shot.
If the shot finishes in the target zone (inferred from the player's aim direction and the hole, clipped away from hazards),
nothing changes, so intended draws and cuts survive. Otherwise it applies part of the smallest fix (start line
first, then speed, then curve), scaled by lie, error size, swing speed, quality and club. Never reads who the player
is (invariant 8). Each change is logged with its before and after values.

- Design: [assists.md](../../docs/gameplay/assists.md).
- Configuration: one file, `data/assist/assist.v<N>.json`, the same for every shot and every player. No levels or
  modes.
- The correction runs flight and ground simulations, so `golf-shot` passes it a simulate callback rather than this
  crate depending on the physics crates.
- The universal speed curve itself is applied in `golf-swing`; its data lives in the assist configuration.

**Depends on:** `golf-core`. **Status:** scaffold (M0); M7.
