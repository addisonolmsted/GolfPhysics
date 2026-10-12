# models

The physics in each step of the shot. Each doc gives the equations, the parameters (with provenance) and the
calibration plan. Status of all: **draft, M0 (2026-10-11)**; they become `current` as their milestone lands.

| Doc | Crate | Milestone | In one line |
|---|---|---|---|
| [impact.md](impact.md) | `golf-impact` | M4 | Delivery → launch: D-plane, smash factor, spin loft, gear effect, lie |
| [flight.md](flight.md) | `golf-flight` | M2 | Drag and Magnus lift as functions of the spin factor, spin decay, RK4 at 1 ms |
| [ground.md](ground.md) | `golf-ground` | M3 | Bounce impulse with turf tilt, slide → roll, slope gravity, stimp, cup capture |
| [obstacles.md](obstacles.md) | `golf-flight`, `golf-ground` | M6 | Trunks, crowns, porous stands, the flagstick, water, OB and penalty crossings |
| [wind.md](wind.md) | `golf-core`, `golf-flight` | M2 | Mean wind, height profile, seeded gusts |

## The ball

One set of constants, used by every step (D7):

| Quantity | Value | Source |
|---|---|---|
| Mass | 0.04593 kg | Rules of Golf maximum |
| Radius | 0.021335 m (42.67 mm diameter) | Rules of Golf minimum |
| Moment of inertia | `k m r²`, with `k` ≈ 0.38-0.40 (a real ball is close to a solid sphere's 0.4) | pick one value in M3 and use it everywhere; GolfEngine used 0.4 at impact and 0.33 on the ground |

A larger "gameplay" radius for forgiveness (GolfEngine doubled it) is **not** a physics constant. If we want it, it
is an assist (e.g. a slightly bigger effective cup, or tree proxies made slightly smaller): part of the one assist
configuration, recorded in the assist log and hidden from players.
