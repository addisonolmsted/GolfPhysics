# 06: The beta plan

What it takes to reach a beta that plays a full round with every component working, where every tester first does a
short swing test, so the beta also yields a 100-200 player swing sample. Status: **draft, 2026-10-13**, for the owner
and every team. Items marked *proposed* need the owner's decision; time estimates are estimates.

## What "beta-ready" means

A tester installs the app from TestFlight, does the swing test, and plays nine holes on a generated course:

1. **Onboarding:** consent, then a swing test of about 6 minutes, uploaded automatically.
2. **A round:**
   - the phone shows the hole map; the player picks a club and an aim and swings;
   - the display shows the avatar following the live swing, then the ball flying, bouncing, rolling and
     coming to rest;
   - the game applies the rules (OB, penalty areas, holed out) and keeps the score.
3. **Behind it:** every shot is logged (its request and result, and the raw swing for beta builds), so the speed
   curve and the assists can be tuned between builds without an app release.

**Gates before inviting testers:**
- a closed run by the team: 3 people × 3 rounds;
- no crash, and no shot that doesn't resolve;
- end-to-end latency measured (impact to first frame on the display, [02](02-latency-budget.md));
- the swing test's upload checked against the data on the server.

## Where things stand (2026-10-13)

| Component | State |
|---|---|
| Courses (generator) | Validated C0 bundles; a pilot pool of about 1,000 courses is next on the generator's queue. Open: the lake-island export fix, consistent tee sets. |
| Course reader (M1) | Done: bit-identical to the generator's reader, 78-105 ms per load. |
| Flight (M2, M2b) | Done: tour and amateur carries, sniff tests, wind, 0.29 ms per flight. Open: driver hang time ~10 % long; iron curve goes to M4. |
| Ground, impact, swing mapping, obstacles, assists, server (M3-M8) | Designed, not built. |
| Swing data app (SwingLab) | Built (SwingApp repo, branch `swinglab`); the 6-10 person pilot over the next 2 weeks. |
| Phone controller (the game app) | SwingApp v0: swing detection only; no transport, club or aim UI. |
| Display (web app) | Front-end team. Needs from us: the live swing stream and the shot result format. |
| Game layer (rules, score, rounds) | Not started; owner to decide where it lives (below). |

## The work, by area

Each item is cut to its beta size: what the beta needs, not the finished version.

### 1. Physics engine (this repo)

| Milestone | Beta scope | Estimate |
|---|---|---|
| **M3 Ground** | Bounce (turf tilt, grip/slip), slide → roll, slope gravity, greens at their stimp, cup capture and lip-outs, water and bunker plugging, our material file; the height patch cache (the roll budget). | 4-6 days |
| **M4 Impact and clubs** | The D-plane model, a 14-club table, lie effects from the material file, the per-club face/path → curve calibration (fixes the iron curve). | 3-4 days |
| **M5 Swing → delivery** | The payload → delivery mapping; the **v0 speed curve** from the pilot; the takeaway direction check and handedness; the "swing not read" rules. The curve is **server-side data**, refitted as beta data arrives. | 4-5 days, then refits |
| **M6 Obstacles (beta size)** | Trees with generic proxies (a trunk capsule and a porous crown); the flagstick; OB and penalty-area crossing facts. Deep-woods stands as porous volumes. No lips or walls (no `edges.json` yet). | 3-4 days |
| **M7 Assists (beta size)** | The speed curve (S), target-aware correction (T) with aim inference, putting pace and line (P), with first-guess dials in the one configuration file. | 4-6 days |
| **M8 Service** | `golf-server`: course cache, sessions, the phone and display WebSockets, the live swing stream, shot logging, and the swing test's upload endpoint. | 5-7 days |

About **5-6 weeks of engine work**, partly in parallel. M3 and M4 need nothing from anyone else and start now. M5
needs the pilot data; M8 needs the decisions below.

### 2. Phone controller app (the beta app)

SwingApp becomes the beta app, or the beta app is built from SwingLab, which already has the detector, the recording,
the protocol screens and the export.

- **Onboarding:** consent, the swing test (below), handedness once in the profile.
- **Swing capture:**
  - the SwingLab detector plus the takeaway direction check;
  - the **live swing stream** to the display (state and the "+ = back" angle every ~5°);
  - the summary payload (P1) on impact;
  - the raw window in beta builds.
- **Round UI:** the hole map (the generator already makes per-hole phone maps), club selection, aim direction, the
  shot result.
- **Distribution:** TestFlight. This **needs the paid Apple Developer Program**: enroll now, because external testing
  needs a beta app review, and organization enrollment can take weeks.

### 3. Display (front-end team)

Draws the course from the C0 bundle, animates the avatar from the live swing stream and the ball from the shot
result. What we owe them, early, so they can build against fixtures:
- **The live swing stream format:** states, angle, timing (P1 addendum, M5).
- **The display's shot result (`ShotView`, P2):** a sample file per shot type (drive, approach, chip, putt, water,
  OB) generated from the fixtures, so they can build before the server exists.

### 4. Game layer

Turns, strokes, OB and penalty-area relief (from `rules.json`'s drop zones and reference points), holed out,
scorecard. Small for a beta, but somebody has to own it. *Proposed:* a module in `golf-server` next to the shot
service, since the server already holds the ball positions.

### 5. Courses (generator)

- **A beta pool:** 20-50 validated courses is plenty for 100-200 testers.
- **The two fixes in progress:** lake islands exported as holes; all 5 tee positions.
- **Course-wide tee sets,** so a round is played from one set (the middle tees for casual players).
- **Nothing new needed for physics:** M3-M6 work from today's bundles (trees use generic proxies).

### 6. Swing data (the 100-200 sample)

- **The pilot (now, 2 weeks, 6-10 people):** SwingLab as built. It tests the detector and the protocol, and gives the
  speed curve's first numbers. Include at least two golfers with launch-monitor numbers if the range has one.
- **The beta's onboarding swing test:** the pilot protocol shortened to about 20 swings (about 6 minutes), because
  every tester has to sit through it:

| Block | Swings |
|---|---|
| Practice | 2 |
| Full: normal | 6 |
| Full: hard | 3 |
| Three-quarter | 3 |
| Chips (15 and 35 yd) | 2 + 2 |
| Putts (2 and 8 m) | 2 + 2 |

  The answers to handedness and hold are kept as **ground truth** for the automatic detection; gameplay doesn't ask.
- **Gameplay is the larger dataset.** In beta builds every swing's raw window goes to the server after its shot
  (D19's development policy). 150 testers × a few rounds is tens of thousands of labelled-by-context swings
  (club, lie, distance to the pin), against about 3,000 from onboarding.
- **Storage:** the server's upload endpoint into object storage, not CloudKit (*proposed*). One pipeline for the
  test and gameplay, with no iCloud dependency.
- **Analysis tools** in `golf-lab`: read `.slab` and session files; distributions by experience and block;
  refit the speed curve; detection accuracy against the ground truth.

## Order of work

```
Weeks 1-2    pilot (6-10 people, SwingLab)      | M3 ground, M4 impact            | enroll in the paid program
             decisions below                      | ShotView samples to front end    | generator: beta pool, fixes
Weeks 3-4    M5 v0 curve from the pilot          | M6 obstacles, M7 assists         | phone app: onboarding, capture, round UI
             live swing stream spec to front end | M8 server: sessions, logging     | display: course, avatar, ball
Weeks 5-6    integration: phone → server → display, the game layer
             closed run by the team (3 × 3 rounds), latency measured, fixes
Week 6-7     TestFlight beta review; invite testers in waves (20, then 50, then the rest)
             refit the speed curve after the first ~30 testers' swing tests
```

Earliest beta: **about 6-7 weeks**, if the phone app and the display are built in parallel with the engine. The
engine isn't the long pole; the phone app and the display are, and they depend on the two formats above being
delivered early.

## Decisions needed (owner)

1. **Paid Apple Developer Program, individual or organization.** Enroll now: TestFlight needs it.
2. **The beta app:** extend SwingApp, or grow the controller from SwingLab (*recommended:* from SwingLab; it has the
   detector, recording, protocols and upload).
3. **Where the game layer lives** (*recommended:* in `golf-server`).
4. **Swing data storage:** our server and object storage (*recommended*) or CloudKit.
5. **Hosting** for `golf-server` and the display: one region near the testers to start.
6. **The beta's onboarding protocol** (the table above).
7. **Privacy:** a privacy policy and consent text for motion data from 100-200 people (TestFlight external testing
   asks for a privacy policy). Plus the data retention period.
8. **Merge the engine branches** (`m1-course-reader` → `m2-flight` → `m2b-flight-refit`) into `main` before M3
   builds on them.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The pilot's 6-10 people don't represent beta testers | v0 speed curve off | The curve is server-side data; refit after the first ~30 testers, without an app release |
| Phone app or display slip | beta slips | Deliver the stream and `ShotView` formats in week 2, with sample files, so both can build against fixtures |
| Detection misreads (holds, handedness, putts) | wrong avatar, false shots | The takeaway check and the downswing safety check; pilot and onboarding measure accuracy; "swing not read" rather than a wrong shot |
| Free-team limits until enrollment | 7-day installs, no TestFlight | Enroll now |
| Latency over mobile networks | the shot feels late | The local haptic at impact, the "swing received" event (02); measure in the closed run |
| Feel is wrong (carries, roll, assists) | testers bounce off | One configuration file for assists and materials, changed between waves; shot logs show what testers actually hit |
