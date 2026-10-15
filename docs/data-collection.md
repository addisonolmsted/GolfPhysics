# Swing data collection (SwingLab)

The first swing dataset: real people swinging a phone, labelled by what they were asked to do. It feeds M5 (the swing
input contract and the universal speed curve), and checks two M2b open items: hang time, and how often phones
saturate. Status: **v1 built, 2026-10-13** (SwingApp repo, branch `swinglab`, target `SwingLab`). It compiles for the
Simulator and the phone, and every screen and the export were checked in the Simulator; recording needs a phone. Owner decisions: D38 (a small app and a 6-10 person sample, in
parallel with M3), a new target inside SwingApp, the highest sample rate the phone gives. CloudKit was chosen, then
set aside: the team is a free personal team, which can't use iCloud (D41). Sessions stay on the phone and leave by
Export.

## What the data must answer

| Question | Used by | From |
|---|---|---|
| What phone speed does a "normal, comfortable full swing" give, and how much does it vary between people? | the casual region of the speed curve ([assists §S](gameplay/assists.md#s-the-speed-curve-replaces-per-player-calibration)) | the "normal" blocks |
| How fast is a hard swing, so where does the taper start? | the taper knee | the "hard" blocks |
| Does backswing length separate a slow full swing from a deliberate partial one? | the fullness signal (D36) | full vs three-quarter vs chip blocks, everyone |
| Does the swing change with the club the player imagines? | whether the speed curve needs a club input | later blocks (v2) |
| How often does the accelerometer saturate (±8 g on older phones, ±16 g on newer)? Is the old clip repair still needed? | P1 quality flags | the raw accelerometer, every swing |
| What rate does the phone actually deliver? | P1 `rate_hz` | every swing |
| Real club speed, carry and hang time for golfers who also hit on a launch monitor | the speed curve's honest region; M2b hang time | the optional launch-monitor fields |

## The app

A new target, **SwingLab**, in the SwingApp project (`SwingApp/SwingLab/`). The swing-angle calculation and the
pointing-down check are shared with SwingApp through `SwingApp/Shared/`, so detection fixes flow both ways.

### Flow

1. **Home:** statistics for this phone, **Export** (new sessions, all sessions, or one session), and **New
   session**.
2. **New session** (changed 2026-10-14): the anonymous participant code, handedness (it sets the swing direction),
   **golf ability** (required: never, casual, regular, competitive), handicap, and optional gender, age band and
   height. No agreement questions: the pilot is run in person with friends and family, on the team's phones, and
   consent is given in person (recorded as given). Experienced golfers (plays regularly or competitively, or a
   handicap of 20 or better) get the **fade and draw blocks**.
3. **How to hold it:** like a club grip, the top of the phone pointing at the ground, the screen facing the target.
   The hold is saved with every swing.
4. **The protocol**, block by block, with a workflow strip across the top (Practice → Full → Hard → ¾ → (Fade →
   Draw) → Chips → Putts) and a 3D target picture for the current block. For each swing:
   - **Address:** point the phone down and hold still. After **1.0 s of stillness** (1.5 s before 2026-10-15) the phone vibrates and the
     screen flashes green: armed.
   - **Swing.** The swing ends at the **impact crossing** (below), the phone vibrates again, and recording continues
     for 1 s of follow-through.
   - **Result:** the swing is kept, with **Delete previous** available. A deleted swing is kept as *discarded*, never
     erased, so false triggers can be studied.
5. **Last details:** optional launch-monitor numbers (if real balls were hit) and notes.
6. **Review and save:** confirm and save on the phone, then **Export now** or later from Home.

### Detection

All angles are in the **address frame**: the phone's orientation at the moment it armed.

| Step | Rule (v1 values, tunable) |
|---|---|
| Pointing down | from CoreMotion's gravity vector: the phone's +y within 45° of straight down to start settling, within 55° to stay (hysteresis) |
| Still | for **1.0 s**. The count starts on the **first** sample that is down and still (under 0.35 rad/s and 0.06 g), with no lag; during it, **smoothed** rotation rate and acceleration (0.15 s time constant) break it only above 0.7 rad/s or 0.12 g, so one twitch doesn't restart it. (2026-10-14's version also smoothed the start, which hid a 0.3-0.5 s delay after the hand stopped, and used a 35° cone, which a leaning address could miss.) |
| Armed | haptic + green flash; the address orientation `q0` is the **average attitude over the last 0.3 s** of the still window, not one sample: a steadier reference for the face and toe angles at impact (P1, D56) |
| Re-address | armed, if the player settles again (still 0.6 s, under 1° of turn across all of it) turned at least 4° from `q0` (a grip adjustment after the flash), `q0` is taken again with a light tick. A slow putting takeaway turns too much to count as still. The count goes in the swing header. |
| Swing angle | SwingApp's angle: the phone's +y axis in the address frame, projected on the plane normal to address +x, `atan2(z, y)`, **unwrapped**, so a full swing past vertical doesn't wrap. That's why a plain dot-product test would fire at the top. |
| Backswing | the side the angle first leaves by 8°; the top is the largest excursion on that side |
| Impact | the angle crosses back through 0 after a backswing of at least the block's minimum (full 60°, ¾ 45°, chip 15°, putt 4°), with the rotation rate about the swing axis at least the block's minimum (full 3 rad/s, ¾ 2.5, chip 0.8, putt 0.25). The crossing time is interpolated between samples. |
| Not a swing | crossing back too slowly: disarm (the player re-addressed). A wobble under the minimum backswing: stay armed. Nothing within 12 s of arming: disarm. |

### What a swing records

Everything from 1 s before arming to 1 s after impact, at the phone's top rate (requested 200 Hz; iPhones usually
deliver 100 Hz):

- **Device motion:** time, attitude quaternion, user acceleration (g), gravity (g), rotation rate (rad/s), all in
  the device frame, plus the world frame `xArbitraryZVertical`.
- **Raw accelerometer:** time, x, y, z (g). Only this stream shows saturation.
- **Markers:** still-from, armed, backswing start, top, impact.
- **Quality:**
  - measured rates for both streams;
  - the raw accelerometer's largest component;
  - the number of samples at or beyond 7.8 g and 15.6 g;
  - peak rotation rate, backswing extent, backswing and downswing times.

**File format** (`.slab`, one per swing, little-endian):

```
"SLB1" | u32 header length | header JSON (versions, columns, rates, markers, quality, labels)
| motion rows: f64 x 17 (t, qw, qx, qy, qz, ax, ay, az, gx, gy, gz, rx, ry, rz, swing_angle_deg, reserved x2)
| raw rows: f64 x 4 (t, ax, ay, az)
```

The header gives both row counts, so a reader needs no other context. A 4 s swing is about 60 KB.

### Getting the data off the phone (D41)

**Export** builds one `.zip`:
- `<session id>/session.json`: the session, the survey, and every swing's quality and markers;
- `<session id>/swings/*.slab`: one file per swing, discarded swings included.

The share sheet sends it by AirDrop to the Mac, to Files or by Mail; exported sessions get a tick on Home. The app's
folder is also visible in the Files app (On My iPhone › SwingLab) and in Finder when the phone is plugged in.

**Free-team limits:** an app installed from a free team stops opening after 7 days; re-run it from Xcode to renew it
(the sessions on the phone are kept). There's no TestFlight, so participants use the team's phone.

**CloudKit, for later (needs a paid Apple Developer team):** the code is in the app behind the `SWINGLAB_CLOUDKIT`
compilation condition, with the entitlements in `SwingLabSupport/SwingLab.cloudkit.entitlements`. The plan for it:

| Record type | Fields |
|---|---|
| `Session` | sessionId, participantCode, createdAt, protocolId, protocolVersion, appVersion, deviceModel, osVersion, handedness, hold, survey (JSON), nKept, nDiscarded, nPractice |
| `Swing` | session (reference), sessionId, blockId, kind, attempt, status (kept, discarded, practice), header (JSON), data (asset: the `.slab` file) |

Its setup:
1. Enable iCloud (CloudKit) on the SwingLab target with the container `iCloud.Davis-Test-Group.SwingLab`.
2. Add queryable indexes on `recordName` for `Session` and `Swing` in the CloudKit Console.
3. Deploy the schema to production before TestFlight.

No names or contact details are stored either way; the participant code is random.

## Protocol v2 (about 15 minutes, 41 swings; 49 for experienced golfers)

| Block | Swings | Prompt | Target picture |
|---|---|---|---|
| Practice | 3 | Three normal swings to get the feel (flagged practice) | tee, 220 yd |
| Full: normal | 10 | Your normal, comfortable full swing | tee, 220 yd |
| Full: hard | 5 | As hard as you comfortably can, staying balanced | tee, 250 yd |
| Three-quarter | 5 | A controlled three-quarter swing | fairway, 140 yd |
| Fade (experienced golfers) | 4 | Swing to hit a fade: the ball curving away from you | fairway, 150 yd |
| Draw (experienced golfers) | 4 | Swing to hit a draw: the ball curving towards you | fairway, 150 yd |
| Chip 10 yd | 3 | A short chip | chip, 10 yd |
| Chip 25 yd | 3 | A pitch | chip, 25 yd |
| Chip 40 yd | 3 | A longer pitch | chip, 40 yd |
| Putt 1.5 m | 3 | A short putt | green, 1.5 m |
| Putt 4 m | 3 | A medium putt | green, 4 m |
| Putt 10 m | 3 | A long putt | green, 10 m |

Protocols are data (`SwingLab/Protocols/*.json`), so a block can be added or reordered without changing code. A block
with `"requires": "skilled"` is only swung by experienced golfers. The fade and draw blocks test whether a phone
swing carries intent at all (P1: do face to path and the start line separate between the two?).

## The pilot (D38)

- **6-10 people:** a mix of non-golfers, casual golfers and at least two regular golfers. Golfers with a launch
  monitor fill in its numbers.
- **One phone**, the team's, first. The older phone's ±8 g limit is itself a test case: compare its clip counts with
  a newer phone's.
- **After the pilot:** check that detection worked (false triggers and misses in the discarded list), look at the
  distributions, then decide the size of the full study (~40-60 people, then 200-300).

## Safety

A wrist strap or a firm grip on a case is mandatory, and whoever runs the session checks for clear space (the app
no longer asks, 2026-10-14). The hard block says "comfortably" on purpose.
