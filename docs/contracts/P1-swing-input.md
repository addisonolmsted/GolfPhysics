# P1: Swing input

What the phone sends for each swing, and how `golf-swing` turns it into a `ClubDelivery`. Status: **draft, M0
(2026-10-11)**. The owner noted the phone side "will be remade to what we need for the physics", so this is our
proposal for that remake. What SwingApp does today is in [swingapp-v0.md](../reference/swingapp-v0.md).

## 1. The swing model we assume

The current idea (owner, 2026-10-11), which this contract keeps:

1. **Address.** The player holds the phone like a club at address and confirms. The phone records the address
   orientation; velocity is taken as zero.
2. **Backswing and top.** The phone swings back. When the downswing starts, the top of the swing is taken as
   another zero-velocity point.
3. **Downswing.** The acceleration is integrated from the top to get velocity.
4. **Impact.** The moment the phone passes back through the address orientation. The velocity, angular velocity
   and orientation there, relative to address, give the delivery.

Two zero-velocity points (address and top) are worth more than one: integrating from the top alone accumulates
drift over about 0.3 s, but the backswing integration (address → top) must also end near zero. The error at the top
measures the accelerometer bias, which can then be removed from the downswing. That is a cheap improvement to
test in M5.

## 2. The payload

One message per swing, phone → server, on an open WebSocket (*proposed*; JSON in development, a compact binary
encoding later with the same fields).

```jsonc
{
  "p1_version": 1,
  "session_id": "…", "player_id": "…",
  "swing_id": 1234,                    // increments per swing; the server rejects repeats
  "device": { "model": "iPhone15,2", "os": "iOS 19.1", "app": "0.4.0" },

  // What the player chose. The server checks these against its own state.
  "context": {
    "club_id": "7i",                   // from the club table the server publishes
    "shot_type": "full",              // full | punch | chip | pitch | putt | flop (the phone UI offers these)
    "aim_azimuth_rad": 1.92,           // C0 frame: only the direction the player dialled in; the engine infers the
                                       // aim point from the course (assists.md, "Inferring the aim point"; D26)
    "handedness": "right",
    "ball_pos_echo_m": [1504.0, 1112.0]// what the phone thinks; the server's own position wins
  },

  // Timing, all from the phone's monotonic clock, in seconds.
  "timing": {
    "address_t": 0.000, "top_t": 0.912, "impact_t": 1.218,
    "sent_t": 1.226                    // to measure segment 1 of the latency budget
  },

  // The phone's own summary at impact (what SwingApp computes today, cleaned up).
  // Frame: the ADDRESS frame (see §3), not the device frame at impact.
  "impact": {
    "q_address": [w, x, y, z],         // world (CoreMotion xArbitraryZVertical) orientation at address
    "q_impact":  [w, x, y, z],         // same frame, at the interpolated crossing
    "vel_mps":   [x, y, z],            // phone velocity at impact, address frame
    "omega_radps": [x, y, z],          // angular velocity at impact, address frame
    "top_angle_rad": 2.4,              // backswing length: the swing angle at the top
    "peak_omega_radps": 21.0
  },

  // Quality flags: the server may refuse or down-weight a swing.
  "quality": {
    "clipped": false, "clip_repaired": false,
    "top_residual_mps": 0.21,          // the velocity left at the top from the address→top integration
    "samples_dropped": 0
  },

  // No raw data here: the raw window travels in its own message (§4).
}
```

The server never trusts `context` alone: the club must be one the player is allowed to use here, and
`ball_pos_echo_m` must match the server's position, or the server sends back a resync.

## 3. Frames

- Everything is expressed in the **address frame**: the phone's orientation when the player confirmed address.
  The heading of CoreMotion's world frame is arbitrary per session, so world quaternions can only be compared
  through the address one.
- The address frame is tied to the aim on the server: at address, the phone's "shaft" axis and the player's facing
  define which way is the target line, and the aim azimuth maps that line into the C0 frame.
- How the phone is held (screen facing the player or away, portrait up or down) is a calibration step, done once
  per grip, not a sign flip in the algorithm.

## 4. The raw window (D11, revised 2026-10-11)

**Production swings send the summary only. The raw sensor window is a development and monitoring tool, sent
outside the latency path.**

### What it is and what it costs

The raw window is the fused device motion from address through the follow-through, at 120 Hz: the attitude
quaternion, user acceleration and rotation rate. About 2 s is 240 samples × 10 values. That is about 9.6 KB as f32,
or about 5 KB quantised to i16. The summary in §2 is about 30 numbers: about 150 bytes in binary, about 1 KB as JSON.

| Per… | Summary only (binary) | Summary + raw window |
|---|---|---|
| Swing | ~0.15 KB | ~5 KB |
| Round (~40 swings, 9 holes, putts included) | ~6 KB | ~200 KB |
| 1 M rounds per month | ~6 GB | ~200 GB |

Bandwidth is not the main cost. Cloud providers typically don't charge for inbound traffic, and 200 KB per round
is nothing on a phone plan. The costs that matter are these:

- **Latency.** On a weak mobile uplink (around 1 Mbit/s), 5 KB takes about 40 ms longer to send than 150 bytes, and
  that time falls directly in the budget the player feels ([02](../02-latency-budget.md)). This alone rules the raw
  window out of the hot path.
- **Storage and processing**, if we keep it: 200 GB a month that grows, plus the server time to re-derive each
  swing.

### What it buys, honestly

| Reason | Holds up? |
|---|---|
| **Building the swing corpus** (M5): tagged swings we can re-process every time the algorithm changes | **Yes, essential.** Without raw data, every algorithm change needs the swings recorded again. |
| **Debugging odd shots** during development and playtests | **Yes**, for testers. |
| **Monitoring in production:** checking that the algorithm still holds on new phone models and new kinds of players | **Yes, but a small sample is enough.** |
| Changing the algorithm without an app release | **Mostly no.** Most of this benefit comes from the summary being physical quantities (velocity, angular velocity and orientations in the address frame) rather than golf numbers. The step from the summary to a delivery (`golf-swing`) stays on the server and can change at any time. Only changes to impact detection or integration need an app release, and those can ship with one. |
| Anti-cheat (checking the summary against the raw data) | **No.** A modified client can fake the raw data as easily as the summary. |

### Policy (*proposed*)

| Build | Raw window |
|---|---|
| Development, internal playtests | Always, sent **after** the shot result, never ahead of it |
| Production | Off by default. A sample (for example 1 % of swings, chosen by the server per session), plus any swing the summary marks as odd (`quality` flags), sent in the background after the shot. Kept for a fixed period, then deleted. |

The raw window goes in its own message, `swing_raw { swing_id, rate_hz, t0, fields, data }`, sent after the
phone has the shot result. Because it never travels ahead of the summary, it can't add latency. The production sample rate is a
dial on the server, set from what monitoring actually needs.

## 5. Payload → ClubDelivery

`golf-swing` maps the impact summary to a delivery (in development it can also re-derive from the raw window). First mapping,
to calibrate on the swing corpus in M5:

| Delivery field | From the phone | Notes |
|---|---|---|
| `path_rad` | the horizontal direction of `vel_mps` relative to the address target line | in-to-out = positive (right) |
| `attack_rad` | the vertical angle of `vel_mps` | down = negative |
| `face_rad` | the phone's rotation about the shaft axis at impact relative to address, mapped to face angle | open = positive; gain fitted per grip |
| `dynamic_loft_rad` | the club's static loft + shaft lean (the phone's pitch at impact relative to address) | clamped per club |
| `head_speed_mps` | a speed index from `|vel_mps|` and `omega_radps`, through the universal speed curve (below) | |
| `strike_offset_m` | none at first (center strike); later from how far the impact orientation is from address (sole tilt) | a source of believable mishits |
| `quality` | from `quality` flags and consistency checks | low quality → the server can refuse the swing ("swing not read") |

### Speed: one curve for everyone (D22, D23)

There is no per-player calibration. The speed index goes through the **universal speed curve** in
[assists §S](../gameplay/assists.md#s-the-speed-curve-replaces-per-player-calibration): the same mapping for every
player, so the same swing gives the same shot whoever makes it. Only the phone model is normalised (sensor ranges,
weight), measured per model.

## 6. Shot types

The phone's `shot_type` tells the engine what the player meant, and changes the mapping:

- **putt:** speed maps to a putter delivery with a much smaller range; the face sets the start line. Assists use
  the putting rules of the assist configuration.
- **chip / pitch / flop:** a shorter mapping range for the wedges; flop opens the face and adds loft.
- **punch:** de-lofted, lower launch.
- **full:** the default.

## 7. Open items

- Impact detection stays on the phone (the haptic must fire at the moment of impact, and production has no raw
  window to re-detect from).
- How to detect "top" robustly (the v0 detector resets on noise).
- Whether to sample faster than 120 Hz where the hardware allows (the downswing lasts about 0.25-0.35 s, so 120 Hz
  is 30-40 samples).
- Left-handed: mirror at this boundary (see [01](../01-conventions.md#frame)).
