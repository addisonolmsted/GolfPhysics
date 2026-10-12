# Wind

The wind the ball flies through. Status: **draft, M0 (2026-10-11)**. Milestone M2 (mean wind and height profile);
gusts in M2 or M7.

## Source

- **Direction:** C0's `prevailing_wind.azimuth_rad` is the **downwind** azimuth: the direction the wind blows
  *towards*, from +x towards +y. (Launch-monitor and weather conventions usually name where the wind comes *from*;
  convert only for display.)
- **Speed:** `speed_mps` is null in the v1 bundles. Until the generator ships one (see
  [05](../05-generator-requests.md) Q9), the game layer chooses a mean speed per round.
- **Per round:** the day's wind is drawn from the course mean using `variability_seed` + the round id: a speed
  around the mean, a direction within a band around the prevailing one.

## Height profile

The wind near the ground is slower than at 10 m. A log profile:

```
w(z) = w_10 · ln(z_agl / z0) / ln(10 / z0)     for z_agl ≥ z_min
```

`z_agl` is the height above ground (from the height function), `z0` the roughness length (about 0.03 m for open
grass; larger near trees later), `z_min` about 1 m to avoid the singularity. A driver's apex at 30 m sees about 1.2×
the 10 m wind; a putt sees nearly none. Putts ignore wind in v1.

## Gusts

A slowly varying multiplier on speed and a small direction wobble, from smooth seeded noise over time (periods of
a few seconds). Within one shot the wind at time `t` is a deterministic function of the round seed and the shot's
start time on the round clock, so a replay gets the same wind. The display shows the wind the shot used (the
result carries it).

## Not in v1

- Shelter behind trees, dunes and hills (the terrain could reduce `w` downwind of a ridge).
- Wind on a rolling ball.
- Rain and wet ground (would change materials, not the wind).
