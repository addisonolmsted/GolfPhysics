# golf-impact

`ClubDelivery + Club + Lie → LaunchConditions`: a physical oblique impact in 3D (the D-plane): restitution along
the face normal, the ball rolling off the face across it, fitted per-club restitution and spin, a per-club spin
axis gain, gear effect and strike loss, lie factors and the slope at the ball.

- Model: [impact.md](../../docs/models/impact.md).
- Club table: `data/clubs/standard.v1.json` (lofts, lengths and the fitted coefficients: `golf-lab fit-impact`).
- Lie table: `data/materials/lie.v1.json`.
- Closed form, no course access except the lie passed in.

**Depends on:** `golf-core`. **Status:** current (M4, 2026-10-13).
