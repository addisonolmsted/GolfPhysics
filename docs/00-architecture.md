# 00: Architecture

How a swing becomes a shot, and why it is built this way. Status: **draft, M0 (2026-10-11)**. The parts marked
*proposed* are open in the [decision log](04-decision-log.md).

## 1. The shape of the problem

Three machines are involved, and only one of them may decide where the ball goes.

| Machine | Knows | Does |
|---|---|---|
| Phone (SwingApp) | the IMU stream, the club the player picked, the aim they set, the shot type | detects address, the downswing and impact; sends one payload per swing |
| Server (this repo) | the course, every ball position, the wind, the one assist configuration | runs the shot; is the authority on where the ball is |
| Display (web app) | the same C0 bundle, for drawing | animates the result; never decides it |

The server is the authority (*decided*, owner, 2026-10-11: "the phone controller gives output data… we get the
needed inputs and produce a trajectory"). The phone does not tell the server where the ball is. It may echo the
position it believes in, and the server uses that only as a check.

## 2. The pipeline

```
ShotRequest
 ├ payload: SwingPayload (P1)        ┐
 ├ ball:    BallState (server-owned) │
 ├ club:    ClubId                   │  everything the shot depends on,
 ├ aim:     aim azimuth, shot type   │  so a stored request replays exactly
 ├ wind:    WindState (round seed)   │
 └ data:    versions of the data files┘
      │
      ▼  golf-swing      SwingPayload ──────────────▶ ClubDelivery (raw; speed via the universal curve)
      ▼  golf-impact     ClubDelivery + Club + Lie ─▶ LaunchConditions (raw)
      ▼  golf-assist     raw shot + target zone ────▶ ClubDelivery (corrected) + AssistLog
                         (simulates the raw shot; if it misses the zone, adjusts face+path and speed,
                          then golf-impact runs again on the corrected delivery)
      ▼  golf-flight     LaunchConditions + Course ─▶ airborne samples + first contact
      ▼  golf-ground     contact + Course ──────────▶ bounces, roll, rest
      ▼  golf-shot       ───────────────────────────▶ ShotResult (P2)
```

### The hand-off types

These are the seams. Each is a plain struct in `golf-core`, so a step can be tested on its own with typed-in values.

| Type | Fields (SI, C0 frame) | Notes |
|---|---|---|
| `ClubDelivery` | `head_speed_mps`, `path_rad` (horizontal, relative to the aim line), `attack_rad`, `face_rad` (relative to the aim line), `dynamic_loft_rad`, `strike_offset_m: [heel-toe, low-high]`, `quality` (0-1 confidence) | What a launch monitor measures about the club. The phone's job is to estimate this. |
| `LaunchConditions` | `ball_speed_mps`, `launch_rad` (vertical), `direction_rad` (horizontal, world azimuth), `spin_rpm` stored as `spin_radps`, `spin_axis_rad` (tilt; + = fade for a right-hander) | What a launch monitor measures about the ball. The flight model's only input apart from the course and the wind. |
| `BallState` | `pos_m`, `vel_mps`, `spin_radps: Vec3` (the spin vector), `phase` | The integrator's state. The spin is a vector, so the axis can precess and change at a bounce. |
| `Contact` | point, normal, surface class, material, incoming state | Returned by flight; it starts ground. |
| `ShotResult` | samples, events, rest state, final lie, rule facts, raw vs assisted log, request hash | [P2](contracts/P2-shot-result.md). |

### Why split delivery from launch

GolfEngine went straight from phone quaternions to a ball impulse, then rotated the result to hit a target launch
angle (see [legacy-golfengine.md](reference/legacy-golfengine.md)). That mixed three questions into one function:
what the phone measured, what the club did, and what the ball did. Here, `ClubDelivery` is the line between them:

- The swing algorithm can change (it will) without touching the impact or flight models.
- Assists act on numbers that a golfer understands ("face 4° open → 1.5° open"), not on quaternions.
- Impact and flight are calibrated against published launch-monitor data, which is stated in exactly these terms.

## 3. The steps

| Step | Crate | Model doc | Core idea |
|---|---|---|---|
| Swing | `golf-swing` | [P1](contracts/P1-swing-input.md) | Phone kinematics at the bottom of the arc, relative to address, give path, attack, face and loft. Speed goes through one universal speed curve (control, casual, power bands), the same for every player. |
| Assist | `golf-assist` | [assists](gameplay/assists.md) | Target-aware: simulate the raw shot; if it finishes in the target zone, leave it (draws and cuts survive); otherwise apply part of the smallest fix. Never reads who the player is. |
| Impact | `golf-impact` | [impact](models/impact.md) | D-plane: the start direction follows the face; curvature follows face-to-path. Ball speed is head speed times a smash factor that falls off with the strike offset. Spin follows spin loft. |
| Flight | `golf-flight` | [flight](models/flight.md) | Drag and Magnus lift that depend on the spin factor, spin decay, wind relative to the ball with a height profile, RK4 at 1 ms. |
| Obstacles | `golf-flight` and `golf-ground` | [obstacles](models/obstacles.md) | Trunk capsules and crown spheres from `instances.bin`; porous stands for deep woods; the flagstick; lips and walls later. |
| Ground | `golf-ground` | [ground](models/ground.md) | Bounce impulse with turf tilt and grip/slip, then sliding and rolling with slope gravity, then cup capture and rest. |
| Rest | `golf-shot` | [P2](contracts/P2-shot-result.md) | The lie class, the OB test (the whole ball outside the ring), penalty-area membership, the entry crossing. |

## 4. Time and speed

A whole shot is small: about 8 s of flight and up to 20 s of roll. At 1 ms steps that is 30k steps of a few dozen
flops each, plus a height lookup (16 node reads) per ground step. That is well under a millisecond in release
builds. The latency budget ([02](02-latency-budget.md)) is therefore all network, and we spend CPU freely where it
buys accuracy:

- **f64 and a fine fixed step** instead of f32 and adaptive tricks.
- **Simulation inside the assist.** Target-aware correction runs the shot 2-6 times (the raw shot, then a search
  for the smallest fix). That costs about 1-2 ms extra.
- **The whole trajectory in one reply.** No streaming of partial flights is needed at first. If the network
  budget gets tight, the reply can send the launch conditions first, and the display starts the ball while the
  rest arrives (section 6).

## 5. Determinism and replay

Every `ShotResult` carries a hash of its request and of its own bytes. Because the core is pure and deterministic
(the [conventions](01-conventions.md#determinism)):

- A disputed shot replays exactly from its stored request.
- A model change can be checked across thousands of recorded requests to see which shots changed.
- Tuning is A/B testable offline: run the same corpus through two versions of the data files and compare.

## 6. Later option: the display re-simulates (*proposed*, not for M1-M8)

Because the core crates have no I/O and would build for `wasm32`, the display could run the same engine:

1. The server sends only the `LaunchConditions` and the wind state, a few dozen bytes.
2. The display simulates locally and starts drawing immediately.
3. The server's authoritative result follows. If the hashes match, which they will when determinism holds, nothing
   changes on screen.

This saves bandwidth and hides one round trip. It depends on cross-platform determinism holding between native and
WASM in practice. We keep the option open by keeping invariant 3 ([ARCHITECTURE](../ARCHITECTURE.md#invariants)),
and we decide after M8 measures the real latency.

## 7. What lives outside the engine

- **Rules:** stroke counting, relief, drops, provisional balls and lost balls. The engine reports facts; the game
  applies the rules (*proposed*, generator question 8).
- **Club choice and aim UI:** on the phone.
- **Rendering:** the display, from the same C0 bundle and the trajectory samples.
- **Matchmaking, accounts and scorecards:** the game server around `golf-server`.
