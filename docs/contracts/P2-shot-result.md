# P2: Shot result

What `golf-shot` returns, what the server sends to the display, and what the game layer reads. Status: **draft, M0
(2026-10-11)**.

## Shape

```rust
pub struct ShotResult {
    pub p2_version: u16,
    pub request_hash: u64,          // hash of the ShotRequest + data versions: replays must match
    pub result_hash: u64,           // hash of everything below

    pub delivery_raw: ClubDelivery, // from the phone
    pub delivery: ClubDelivery,     // after assists
    pub launch_raw: LaunchConditions,
    pub launch: LaunchConditions,   // what was flown
    pub assist_log: Vec<AssistEntry>, // one line per assist that changed something, with before/after;
                                      // includes the inferred aim point and target zone

    pub samples: Samples,           // the path, for drawing
    pub events: Vec<Event>,         // what happened, with times
    pub rest: Rest,                 // where it ended and the facts there
    pub stats: ShotStats,           // carry, total, apex, flight time, curve, landing angle (for the HUD)
}
```

## What leaves the server (D28: assists are hidden)

The full `ShotResult` stays on the server: it is logged for replay, the lab and tuning. Devices get a
`ShotView` built from it, with only the samples, events, rest and stats. The `*_raw` values, the assisted delivery
and launch, and the assist log are never sent, so no client (the display, the phone, or a modified one) can show
or reveal an assist. `stats` are computed from the flown shot, as the player saw it.

## Samples

- Fixed rate, **30 Hz** (*proposed*), from impact to rest: position (m) and velocity (m/s) per sample. The display
  interpolates with cubic Hermite curves between samples, which is smooth at 60 or 120 Hz because it has the
  velocities.
- **Every event is also a sample** at its exact time (bounces, tree hits), so the drawn path goes through the exact
  contact points even between the 30 Hz ticks.
- Spin is sent as a few numbers per sample (rate and axis) only if the display wants to draw the ball's rotation.
- Wire encoding (M8): positions as offsets from the start point in mm (i32), velocities in cm/s (i16), times in
  ms. A 20 s shot is about 600 samples × 18 bytes ≈ 11 KB raw, compressing well; trimmed by sending roll at 15 Hz
  if needed.

## Events

| Event | Fields |
|---|---|
| `Launch` | t = 0, launch conditions |
| `Apex` | t, position |
| `TreeHit` | t, position, kind (trunk, crown, stand), asset id |
| `FlagstickHit` | t, position |
| `Land` | t, position, surface class, landing angle, speed (the first ground contact) |
| `Bounce` | t, position, surface class |
| `StartRoll` | t, position |
| `EnterWater` | t, position, water id (the ball stops there) |
| `Plug` | t, position (sand or soft ground) |
| `CrossOB` | t, position (the first time the whole ball leaves the OB ring) |
| `CrossPenaltyMargin` | t, position, area id, colour (the last crossing into the area, for relief) |
| `Holed` | t |
| `LipOut` | t, position |
| `Rest` | t, position |

## Rest

```rust
pub struct Rest {
    pub pos_m: Vec3,
    pub surface: SurfaceClass,   // the lie
    pub state: RestState,        // OnGround | InWater | Plugged | Holed | OffWorld
    pub in_bounds: bool,         // the whole-ball OB test
    pub penalty_area: Option<PenaltyAreaRef>, // id + colour, if inside one
    pub last_penalty_crossing_m: Option<Vec2>, // where it last crossed into that area
    pub dist_to_pin_m: f64,
    pub slope: Vec2,             // the ground slope at rest, for the next shot's lie
}
```

The game layer uses these facts and `rules.json` to apply the rules (D17, decided). The engine never moves the
ball for relief.

## Who reads what

| Reader | Uses |
|---|---|
| Display | `ShotView`: samples, events (for sounds, splashes, camera cuts), stats (HUD) |
| Game layer (server-side) | rest, events (OB, water, holed), stats |
| Phone | stats and rest for its summary screen (via the game layer) |
| Server logs, `golf-lab` | the full `ShotResult`, including raw vs assisted |
