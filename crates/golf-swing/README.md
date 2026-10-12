# golf-swing

Turns the phone's swing payload into a `ClubDelivery` (head speed, path, attack, face, dynamic loft, strike offset,
quality).

- Contract: [P1-swing-input.md](../../docs/contracts/P1-swing-input.md).
- Validates the payload (version, swing id, quality flags) and refuses unreadable swings with `PayloadInvalid`.
- Applies the universal speed curve (the same for every player; per-device normalisation only) and mirrors
  left-handed swings.
- Can re-derive the delivery from the raw IMU window, so swing algorithm changes don't need an app release.

**Depends on:** `golf-core`. **Status:** scaffold (M0); work starts in M5.
