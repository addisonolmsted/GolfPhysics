# gameplay

How the engine bends real physics to make the game fun, and the rules that keep the bending honest.

| Doc | What it covers | Status |
|---|---|---|
| [assists.md](assists.md) | Raise the floor, keep the ceiling: fairness, the universal speed curve, target-aware correction, putting | principles decided 2026-10-12; mechanisms draft |

The owner's brief (2026-10-11): *"We will also be doing some ease of play and other gameplay-related edits to the
physics based on club selected, map position, etc. (e.g. in rough we may have less correction to the shot; if
someone is hitting a driver from a tee we may boost the shot if the input speeds are slower than we expect; at 80
yards from the hole we may adjust how we boost and correct.)"*

The owner's intent (2026-10-12) sharpened this: easy to pick up, a high skill ceiling where good players can shape
the ball with no extra input, and fair, so the same swing gives the same shot whoever makes it. Correction aims at
the target, not at straight.

The ground rules are invariants 5 and 8 in [ARCHITECTURE.md](../../ARCHITECTURE.md#invariants): assists change inputs
(the delivery and the launch), never the flight itself; every change is logged with before and after values; and nothing reads who the player is.
