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
| `strike_m` | heel-toe from the toe-up change at impact; high-low centred (below) | believable mishits, never a miss |
| `quality` | from `quality` flags and consistency checks | low quality → the server can refuse the swing ("swing not read") |

### Orientation and rotation at impact → face, loft, path, attack, strike (D56-D58, M5)

Everything is **relative to address**: the phone has no reliable heading indoors. At address (pointing down, still
for 1.5 s) the face is taken as square to the target at the club's static loft. A **virtual clubhead** is fixed to
the phone (the phone is the grip, its long axis the shaft). Which side the target is on, and so handedness, comes
from the takeaway: the head always moves away from the target first. A left-hander is mirrored here.

**Interpolate to the impact instant.** The face closes at ~1,000-2,000°/s through impact: 10-20° between samples at
100 Hz. The orientation is slerped to the sub-sample moment the swing angle crosses the address position; the
nearest sample would make the face useless.

With `R = R_impact · R_address⁻¹`:

| Delivery | From | How |
|---|---|---|
| `face_rad` | orientation | the address face normal rotated by `R`: its horizontal angle to the target line. Picks up the twist about the shaft, and a toe-up or toe-down shaft turning a lofted face (as on a sidehill lie) |
| `dynamic_loft_rad` | orientation | the same rotated normal: its elevation (static loft, shaft lean, the face opening or closing) |
| `path_rad`, `attack_rad` | rotation rate | the head's velocity ≈ `ω × r` (the gyro's rate crossed with the shaft vector, hands → head): its horizontal and vertical angles. First version; a body-pivot (hub) term with a fixed arm length is calibrated with the launch-monitor sessions |
| `head_speed_mps` | rotation rate | a speed index from `|ω|` and the radius, through the speed curve (below) |
| `strike_m.x` (heel-toe) | orientation | the toe-up change, below |
| `strike_m.y` (high-low) | none | **centred** (D57) |

Face and loft come from rotating one vector, so they can't disagree with each other, and they go straight into the
impact model's D-plane.

**Heel-toe strike from the toe-up change (D56).** Not a lie angle read off the attitude: Euler-style angles depend
on rotation order and mix in shaft lean and face rotation, which are both large and fast at impact (the likely
reason the raw lie angle varied so much in the SwingApp tests). Instead, rotate the address **toe vector** `t0`
(along the face towards the toe, horizontal at address) by `R`:

```
toe_up = asin(t · up)                                   (lean and face rotation don't lift it)
Δ = toe_up_impact − toe_up_address − μ                  (μ: the population's typical droop, one constant)
strike_x = −cap · tanh(softshrink(Δ, d) · g / cap)     (toe up → towards the heel; cap ≈ 15 mm)
```

- `μ` makes an ordinary swing read centred (real swings come in slightly toe-up); `softshrink` (zero inside ±d,
  linear beyond) means everyday wobble gives no mishit, with no jump at the edge; `tanh` caps the worst swings at a
  clear heel or toe strike, never the hosel or a miss.
- One mapping for everyone (D22). `d` and `g` are set from the pilot's population, aiming for ~70-80 % of casual
  full swings within ±4 mm and ~5 % beyond 10 mm (a feel choice for the owner), fitted by a `golf-lab` tool from
  the exported raw swings.
- The impact model makes these mild (M4): a 15 mm toe strike on a driver loses a few % of ball speed and adds
  gear-effect draw. A toe-up shaft also turns a lofted face left, through `face_rad`: both are real, and for woods
  they partly cancel (as in real golf). Tune with both in.

**The floor: every read swing makes a shot (D58).** No whiffs, no shanks, no hosel strikes: the phone can't tell
them from noise, so a model that produced them would mostly punish good swings with noisy readings, and they teach
nothing. Bad swings still make bad shots: a slice, a hook, a pull, a toe-struck drive that comes up short.
- The strike is heel-toe only and capped (above); high-low, fat and thin stay centred (D57).
- `face_rad` and `path_rad` are clamped to about ±15°: the worst miss is a big curve, never a ball going sideways.
- A swing whose data fails the quality checks (badly clipped, interrupted, no proper address) is **"swing not
  read"**: no shot, no stroke, swing again. Never a whiff made out of bad data.
- Practice swings need no special case: a swing counts only after the address pose, the player's signal of intent.

**Sensitivity and gains.** With real physics, 1° of face is several yards off line, and a phone with no clubhead
weight is noisier than a club. The raw angles get **fixed gains below 1** (one mapping for everyone), set from the
pilot so a casual player's spread gives believable misses; the target-aware assist trims the rest.

**Saturation.** Older phones clip the gyro around 2,000°/s, and fast swings pass it near impact: the orientation
drifts and the speed reads low. Clipped samples are detected and bridged from both sides; the speed curve treats a
clipped swing as "at least this fast". The pilot shows how often it happens on which phones.

**Measured in the first pilot session (2026-10-16, iPhone 12, one golfer):** the accelerometer (±8 g) clips for
0.2-0.27 s around impact on **every** full swing, and on 25-40 yd pitches; the gyro peaks at 31 rad/s on normal full
swings and touches its ±2,000°/s ceiling (34.8 rad/s) on the hardest. **The clipping corrupts CoreMotion's fused
attitude:** for many seconds afterwards its gravity estimate is off by roughly 4-5° (a false 0.06-0.15 g of "user
acceleration" on a phone held still). Face, loft and toe-up are angles of a few degrees, so `golf-swing` should not
take CoreMotion's attitude at face value: integrate the raw gyro over the swing (address to impact is ~1-3 s, where
gyro drift is tiny), anchored at address by the raw accelerometer's gravity (the phone is still there, so it is true
gravity). Gyro clipping on the hardest swings still needs the bridging above.

**What the pilot must answer:** the noise of face, path, loft and attack on swings meant to be straight (it sets the
gains); whether skilled players can produce a fade or a draw on purpose (an optional block for experienced
players); the toe-up distribution (it sets `μ`, `d`, `g`); how often fast swings clip.

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
