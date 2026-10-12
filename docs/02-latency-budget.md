# 02: Latency budget

The goal (owner, 2026-10-11): the shot should feel **as close to real time as the internet connection allows**.
This page splits that into budgets each part can be held to. The numbers are **targets to measure against in M8**,
not measurements.

## End to end: impact to the ball moving on the display

| # | Segment | Target | Notes |
|---|---|---|---|
| 1 | Phone: impact detected → payload ready | ≤ 15 ms | Today's detector fires about 5 samples (42 ms at 120 Hz) after the zero crossing. Firing on the crossing itself, then sending, cuts this. See [swingapp-v0](reference/swingapp-v0.md). |
| 2 | Uplink phone → server | 20-80 ms | Wi-Fi to a nearby region, about 20-40 ms; LTE, 40-80 ms. One small message on an already-open socket, so no handshake. |
| 3 | Server: decode, simulate, encode | **≤ 3 ms p99** | The only segment this repo controls directly. |
| 4 | Downlink server → display | 20-80 ms | Same reasoning as 2. |
| 5 | Display: decode → first frame | ≤ 17 ms | One frame at 60 Hz. |
| | **Total** | **~60-200 ms** | |

Real golf has no delay between impact and ball flight, but we can hide 100-200 ms:

- **The phone plays the impact itself:** a haptic and a sound at the moment of detection, with no network involved.
  The player feels the strike on time.
- **The display reacts to the payload, not the result.** The server forwards a tiny "swing received" event as soon
  as the payload arrives (before simulating). The display starts the club or impact animation, and the ball leaves
  when the result lands, about 2 ms later.
- **The first part of the flight is predictable.** If the gap still shows in M8, the server can send the launch
  conditions in the "swing received" event, and the display starts the ball on a launch-only curve, then blends to
  the real trajectory. With the WASM option ([00 §6](00-architecture.md#6-later-option-the-display-re-simulates-proposed-not-for-m1-m8)), the display would just simulate it.

## Server CPU budget (segment 3)

| Step | Budget (release build, one core) | Why it is cheap |
|---|---|---|
| Decode payload, look up session state | 0.05 ms | One small message. |
| Swing → delivery | 0.05 ms | A few hundred IMU samples at most. |
| Assist (target-aware correction) | 1.5 ms | The raw shot plus up to 5 more flight + ground runs to find the smallest fix (~0.3 ms of flight each, measured M2). |
| Impact | 0.01 ms | Closed form. |
| Flight | 0.3 ms (**measured 0.29 ms**, M2) | ~620 RK4 steps at 10 ms, each with 4 force evaluations (~125 ns each, latency-bound arithmetic); height lookups only near the ground. It was 4.1 ms at 1 ms steps with `libm::pow` before the M2 speed-up. |
| Ground (bounce + roll + putt) | 0.5 ms | Up to 20k steps, each with one bicubic height and normal lookup. |
| Rule facts, encode the result | 0.1 ms | Point-in-polygon against a pre-built grid. |
| **Total** | **≤ 2.5 ms typical, ≤ 3 ms p99** | |

**Load is not per shot.** A course loads once per server process: about 2-3 MB of physics subset decoded to
perhaps 30-60 MB of f32 or f64 grids (to be measured in M1). One `Arc<Course>` is shared by every round on that
course (*proposed* answer to generator question 10). Target load time: under 300 ms from local disk, so the first
shot of a new course does not wait on it. The server preloads a course when a round is created.

## Payload sizes

| Message | Size target | Notes |
|---|---|---|
| Phone → server swing payload | ≤ 0.25 KB binary (≤ 1.5 KB as JSON in development) | Summary only. The raw window (~5 KB) is a separate message sent after the result, in development and for a small production sample, so it never adds latency. See [P1 §4](contracts/P1-swing-input.md#4-the-raw-window-d11-revised-2026-10-11). |
| Server → display shot result | ≤ 8 KB | 30 Hz samples of position and velocity, quantised, plus events. See [P2](contracts/P2-shot-result.md). |

## How we will measure (M8)

- Every message carries the phone's monotonic impact timestamp and the server's receive and send timestamps. The
  display logs when it receives the result and when it draws the first frame.
- Clock offsets between the devices are estimated with NTP-style ping exchanges on the open socket.
- We report p50 and p95 per segment, on Wi-Fi and LTE, and keep the numbers in `docs/measurements/` (created in
  M8).
