# SwingApp v0 (the phone controller today)

`~/Documents/GitHub/SwingApp`. An iOS app in Swift and SwiftUI with CoreMotion; 13 Swift files, about 1,870 lines,
two commits. Surveyed 2026-10-11. The owner expects it to be reworked for what physics needs; the proposal for that
is [P1](../contracts/P1-swing-input.md). Paths below are under `SwingApp/`.

## What it does

| Step | How | Where |
|---|---|---|
| Sensors | Fused device motion (attitude quaternion, user acceleration in g, rotation rate) at 120 Hz, frame `xArbitraryZVertical` (z up, heading arbitrary). Raw accelerometer at 120 Hz, only for clip detection. 240-sample ring buffers (2 s). | `Motion/SensorManager.swift`, `Motion/MotionProcessor.swift:48-54` |
| Address | The Initialize button is enabled while the phone's +y axis is within 45° of straight down; pressing it stores the orientation. | `Detectors/PointingDownDetector.swift`, `MotionProcessor.swift:183-190` |
| Swing angle | The device +y axis rotated by `init⁻¹ · q`, x component zeroed, `atan2(z, y)`: the shaft's rotation about the address x axis. Unwrapped. Sign flipped when the screen faces the player. | `Calculator/SwingAngleCalculator.swift:20-53` |
| Downswing | The newest of the last 5 angles is less than the oldest. | `Detectors/DownswingDetector.swift:14-27` |
| Impact | The angle 5 samples ago was above 0, now ≤ 0, and `|ω| ≥ 0.65 rad/s`. The crossing is then interpolated (time, nlerp quaternion, ω). | `Detectors/BottomCrossDetector.swift`, `Calculator/BottomOrientationCalculator.swift:28-101` |
| Velocity | Trapezoidal integration of user acceleration from the downswing start to the crossing, each sample rotated into the device frame at the crossing; 0.15 g per-axis deadband; result negated. | `Calculator/BottomSpeedCalculator.swift:17-98` |
| Clip repair | Raw accel within 0.3 g of ±8 or ±16 g for 5+ samples counts as clipped; fused acceleration rebuilt with Hermite splines; peak guessed by an empirical fit. | `Detectors/ClipDetector.swift`, `BottomSpeedCalculator.swift:166-317` |
| Output | A `SwingRecord` in memory: impact and address quaternions, angular velocity (device frame), velocity (device frame), screen-facing flag. Exported as CSV through the share sheet. | `ViewModel.swift:14-20, 234-263` |

There is **no network transport, no club selection, no player position, no swing id and no timestamps** in the
output.

## Issues that matter for physics

| Issue | Effect | P1 fix |
|---|---|---|
| Stale impact values: `deinitialize()` doesn't clear the bottom values; a failed calculator reuses the last swing's | a misread swing silently becomes a copy of the last one | a swing id, explicit failure, the server rejects repeats |
| Velocity and ω in the **device frame at impact** | hard to turn into path and attack without the orientation | send them in the address frame |
| 0.15 g deadband | biases the integrated speed low, more for gentle swings | drop it; remove bias with the address→top zero-velocity check |
| Downswing start resets on one noisy window | integration may start up to 2 s early | a robust top detector (P1 open item) |
| Impact fires about 5 samples (~42 ms) after the crossing | eats latency budget | fire on the crossing ([02](../02-latency-budget.md)) |
| 2 s buffer caps the window | a slow downswing gets cut | longer buffer; send the raw window |
| No explicit velocity-sign convention (the final negation is uncommented) | easy to get path backwards | the address frame defined in P1 §3 |
| Angle wrap not handled in the bottom search (`prevAngle: nil`) | a false crossing on ±180° | fix in the app |

## Keep

The overall idea (address and top as zero-velocity points, impact at the address crossing), the interpolated
crossing, and clip detection with repair, which matters for hard swings that saturate the accelerometer.
