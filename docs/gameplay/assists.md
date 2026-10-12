# Assists: raise the floor, keep the ceiling

`golf-assist`. Status: **draft, rewritten 2026-10-12** around the owner's intent. The principles are decided; the
mechanisms and every number are *proposed* until M5 (the swing corpus) and M7 (playtests).

## The intent (owner, 2026-10-12)

- **Easy to pick up.** A normal, smooth swing gives playable carries, so casual players can still make strategic
  decisions: which club, where to aim, whether to go for the green. Their phone speeds are far below what a real
  club head would need, and the game accounts for that.
- **A high skill ceiling.** Good players can produce high club head speeds and can draw or cut the ball when they
  mean to, with no extra input beyond the swing.
- **Fair.** A good player who chooses to make a slower swing gets the same shot as a casual player making that same
  swing.
- **Smart correction.** The game corrects towards the ideal shot, using what it knows about the situation: ball
  position, the target zone, the aim line, the lie and the club. This raises the floor without lowering the
  ceiling.

## Principles

1. **Same swing + same situation = same shot.** The assist reads the swing and the situation, never who is
   swinging. No per-player calibration, no skill rating, no history (this replaces the earlier per-player
   calibration idea, D10). The fairness rule above follows directly. Test: `simulate` takes no player identity,
   and two requests that differ only in the player give identical bytes.
2. **The floor is about distance; the ceiling is about speed and shape.** A smooth swing earns a playable, gapped
   carry with every club. Swinging faster always goes further, all the way to tour speeds.
3. **Correct towards the target, not towards straight.** A shot that finishes where the player was aiming is left
   alone, however much it curved. This is how a draw or a cut survives without any extra input: **the target zone
   tells intent from error.** A curve that brings the ball to the target is intent; a curve that takes it away is
   error.
4. **Help fades as speed rises.** More power means less forgiveness (decided, owner 2026-10-12; the shape of the
   fade is tuned in playtests). A player
   who wants precision can swing smoother and get more help. A player who wants distance takes the risk. This is
   also what makes a slow swing from a good player match a casual one: the help depends on the swing, not on who
   made it.
5. **Hidden from players, open to us.** The player never sees that an assist acted (decided, owner 2026-10-12).
   Every change is still logged with before and after values, server-side only, for the lab and for tuning
   (invariant 5). The display can show the raw shot next to the assisted one in the lab, never in the game.
6. **One assist configuration, always** (decided, owner 2026-10-12). No difficulty levels, no modes, no per-player
   settings. Players learn the game, and every shot they take behaves by the same rules. The configuration is one
   versioned data file; it changes only with a release, like any other physics tuning.

## Where assists act

```
phone payload ─▶ golf-swing ─▶ raw delivery ──▶ S  speed curve: phone speed → head speed (universal)
                                     │
                                     ▼
                                 golf-impact ─▶ raw launch
                                     │
                                     ▼
                     T  target-aware correction: simulates the raw shot; if it misses the zone,
                        adjusts the delivery (face + path together, speed) and re-runs impact
                                     │
                                     ▼
                         golf-flight / ground (pure physics)
```

## S: The speed curve (replaces per-player calibration)

One mapping from the phone's swing to club head speed, the same for every player (per club). It takes **two
inputs**: the speed index, and **how full the swing was** (backswing length and tempo; see
[Touch shot or slow full swing?](#touch-shot-or-slow-full-swing)). The curve is S-shaped: lifted at the bottom,
honest in the middle, tapered at the top (owner, 2026-10-12).

| Region | Swing | Mapping |
|---|---|---|
| **Partial** | A short backswing: pitches, chips, touch shots | **No lift**, about proportional to speed. A half swing gives about half the full-swing speed, for everyone. |
| **Casual** | A full backswing at the speed most people make without trying hard | **Lifted** to the club's **baseline speed**, the head speed that gives the club's baseline carry (below). The slope is shallow, so a slightly harder or softer swing changes distance a little. |
| **Honest** | Faster full swings, up to about tour speed for the club | The lift fades out; output follows input about one-for-one. |
| **Tapered** | Faster than tour speed for the club | **Slowly growing** (decided, owner 2026-10-12): above a knee (about the club's tour-average speed: driver ~113 mph, 7 iron ~90 mph), the output grows like a logarithm of the excess. More speed always gives more distance, but a phone flick that would map to a 150 mph driver gives a long-drive speed, not a world record. Here the curve *lowers* the head speed below a straight-line mapping. |

A shape for the taper (constants from the M5 swing corpus and M7 playtests):

```
above the knee K:  V_out = K + T ln(1 + (V_in - K) / T)
```

`T` sets how fast it flattens. With K = 113 mph and T = 15 mph, an input of 125 maps to about 122, 140 to about 128,
and 160 to about 134. The slope is 1 at the knee (no kink) and falls towards 0, but never reaches it.

- **Monotonic, never flat.** Swinging harder always goes further, in every region.
- **Gapping stays.** The curve gives a head speed and the club's loft does the rest, so a casual 7-iron still goes
  shorter than a casual 5-iron, and club choice stays a real decision.
- **Baseline carry per club:** see the next section.
- **Device normalisation only.** If phone models measure differently (sensor ranges, weight), a per-model
  correction is applied. That corrects the device, not the person.
- **Where the region edges come from:** the M5 swing corpus, which must include casual and skilled swingers. The
  casual region is centred on what casual players produce when told "make a normal swing".

## Touch shot or slow full swing?

The same phone speed can mean two different things (owner's question, 2026-10-12): a casual player's **full** swing
(boost it to the club's baseline) or a good player's **deliberate soft** swing (leave it soft). Speed alone can't
tell them apart, and the fairness rule forbids asking who the player is. The swing itself has to say.

**The signal is how full the swing is, not how fast.** The phone already measures the backswing (the swing angle
at the top, `top_angle_rad` in P1) and the tempo (address → top → impact times):

| Swing | Backswing | Speed | Region |
|---|---|---|---|
| Casual player, normal full swing | full | slow | **Casual**: lifted to the baseline |
| Good player, soft full swing | full | slow | **Casual**: the same as the casual player (fair) |
| Good player, half-swing pitch | short | slow-ish, but brisk *for its length* | **Partial**: no lift |
| Casual player, half-swing pitch | short | slow | **Partial**: no lift, so it comes up short |

The boost scales with **fullness**, a 0-1 number from backswing length (and, if the corpus shows it helps, the ratio
of speed to backswing length). A full backswing gets the full lift and a half backswing almost none, with a smooth
ramp between them so there's no cliff.

**Club choice does the rest.** Two players 80 yd out, one with a 60° wedge and one with a pitching wedge:

| Swing | 60° wedge (baseline ~60-65 yd) | Pitching wedge (baseline ~105 yd) |
|---|---|---|
| Full, slow (casual) | lifted to ~62 yd: **18 yd short**. Target correction helps only part way (a big miss keeps most of its size), so it's short. | lifted to ~105 yd: **25 yd long**, the same. |
| Half backswing, crisp | ~45-55 yd, short | ~75-85 yd: **on the number**, untouched |
| Full, hard | ~75-80 yd, high and soft | 115+ yd, long |

So at 80 yd a casual player's right play is a full swing with a gap or sand wedge, and a good player's options
include a crisp half pitching wedge. Both work, which is the strategic game we want. The 60° and the PW never act the
same, because physics gives them different launch, spin and carry from the same head speed.

**If the corpus can't separate them** (casual full swings and skilled half swings overlapping in backswing length),
the fallback is the phone's shot-type selector (full / pitch / chip, already in P1), defaulting to "full". That's an
extra input, so only if needed.

## Baseline carry (called "floor carry" in the first draft)

**Definition:** the carry (the distance in the air, to where the ball first lands) a club gives from a **normal,
smooth full swing** (the middle of the casual band), from a good lie (tee or fairway), on flat ground, at sea
level, with no wind. One number per club.

- **It isn't a minimum.** A softer or partial swing still goes shorter, and the control band handles those. It is
  where an ordinary swing lands, so "baseline" describes it better than "floor".
- **It isn't a cap.** Faster swings go past it, into the power band.
- **Roll comes on top.** It depends on the landing (firm fairway, soft green), so it isn't part of the number.
- **It's the dial that sets how long the course plays for a casual player.** For example, it decides whether a
  normal swinger can reach a typical par 4 in two.

**Tuned for the middle tees** (owner, 2026-10-12). The baseline is set so a casual player has a fair game from the
**middle** tee set. The back tees (and tips) are for people making real, full-speed swings and are meant to be long
for a casual player. Their length is what the power band is for.

**What the middle tees ask of it.** On the v1 fixtures (54 holes), using each hole's `middle` tee set, or the
average of `middle_back` and `middle_forward` where it has those instead:

| Par | Holes | 25th pct | Median | 75th pct | Range | Back-tee median, for comparison |
|---|---|---|---|---|---|---|
| 3 | 12 | 148 yd | 157 yd | 159 yd | 116-170 yd | 174 yd |
| 4 | 30 | 322 yd | 372 yd | 386 yd | 244-504 yd | 400 yd |
| 5 | 12 | 467 yd | 487 yd | 515 yd | 418-548 yd | 517 yd |

Nine-hole totals from the middle tees are 2,800-3,330 yd. (Computed from `holes.json`, `tees[].set` and
`tees[].yardage_m`; `golf-lab tees` will reproduce this table in M1.)

**Starting values (to tune):**

| Club | Baseline carry | Why |
|---|---|---|
| Driver (from a tee) | ~210 yd (~230 yd with roll) | Leaves ~140 yd on the median middle-tee par 4: a 7-iron into the green. The longest par 4s (to ~500 yd) become three-shotters. |
| 7-iron | ~140 yd | That second shot. |
| Gaps | ~10-12 yd per club | The median par 3 (157 yd) is then a 6-iron or 5-iron, and most par 3s sit between 7-iron and 5-iron. |
| Pitching wedge | ~105-110 yd | Follows from the gaps. |

From the back tees, the median par 4 (400 yd) needs about a 250 yd drive to leave the same 7-iron: a real, fast
swing, as intended.

**The tee boost** from the original brief stays as an option: the driver's baseline from a tee can sit higher than
from the fairway.

## T: Target-aware correction (replaces straight-line direction correction and scoring-zone help)

Per shot, using the course and the round state:

1. **Build the target zone.** Start from the player's **aim direction**; the engine infers the aim point (see
   [Inferring the aim point](#inferring-the-aim-point)). The zone is the good ground around that point:
   - fairway, or the hole's landing zone from `holes.json`, from the tee;
   - the green or the area around the pin when it is in range (the old scoring-zone help folds in here: the target
     becomes a distance as well as a direction);
   - clipped away from water, OB and bunkers, so a correction never pulls a ball *into* trouble;
   - the player's aim always wins. If they aim to lay up short or bail out left, the zone is built there, not where
     the generator's ideal line runs.
2. **Simulate the raw shot** (flight + ground) and see where it finishes.
3. **Inside the zone: no correction.** An intended draw or cut that finishes in the zone is untouched. That is the
   ceiling.
4. **Outside: find the smallest fix.** In this order:
   - **Rotate the start line:** face and path move together, so face-to-path, and therefore the curve, stays as
     swung. A draw stays a draw; it just starts where it needed to.
   - **Then distance:** ball speed, by a secant search on the simulated finish.
   - **Only then reduce the curve,** for large hooks and slices, and only partly.
5. **Apply a fraction** `s` of that fix, where `s` depends on:

| Factor | Effect on `s` |
|---|---|
| Lie | Highest from the tee and fairway; lower from the rough; lowest from fescue and sand. A bad lie stays a bad lie. |
| Error size | A soft knee: near misses get most of the fix; big misses keep most of their size, so a real mistake still looks like one. |
| Swing speed | Full in the control and casual bands, fading through the power band (principle 4). |
| Swing quality | A clean, smooth swing gets more than a jerky one with low `quality`. |
| Club | Putter and wedges near the green have their own curves (pace matters most); the driver from the tee is the most forgiving. |

**How it plays out:**

- A casual player makes a smooth, slightly open swing at the fairway. The ball would have finished in the right
  rough; the start line is rotated so it finishes on the fairway's right side.
- A good player aims at the right edge and draws the ball back to the middle. It finishes in the zone, so nothing is
  touched.
- A good player swings flat out and hooks it 40 yards. The error is large and the swing is in the power band, so
  there is very little help. It's a hook.
- The same good player makes a smooth swing. They get the same help as the casual player.

**Cost:** 2-6 simulations per shot, about 1-2 ms. The CPU budget rises from 2 ms to about 3 ms p99, which is still
nothing next to the network ([02](../02-latency-budget.md)).

## Putting

The same machinery with the cup as the target: pace first, then line. The effective cup can be a little wider on
the one assist configuration. Same principles: help depends on the stroke and the situation, not on the player.

## What the phone should send for this

- **The aim direction only** (decided, owner 2026-10-12). The point the player taps on the hole map only sets a
  direction. A player in the trees taps around themselves until the line threads a gap, without meaning that exact
  spot, so the tapped point says nothing reliable about distance. The engine infers the distance itself (below).
- `top_angle_rad` and tempo, already in P1, to tell partial swings from slow full ones.

## Inferring the aim point

From the ball position, the aim direction and the course:

1. **Cast the aim line** from the ball across the course, out to the club's longest reach (its power-band carry
   plus roll).
2. **Collect the candidate targets** where the line crosses good ground: each stretch of fairway (or the hole's
   landing zone), the green, and any other short grass. Each candidate is a stretch of the line, from where it
   enters that ground to where it leaves.
3. **Keep the candidates the club can reach**, between the club's control-band and power-band distances.
4. **Pick the candidate the raw shot is heading for:** the one whose stretch contains the raw shot's distance along
   the line, otherwise the nearest one. The green wins a near tie, because going for the green is the most likely
   intent when it is in range.
5. **The aim point** is the point on that stretch nearest the raw shot's distance. The target zone is the good
   ground around it, clipped away from hazards (step 1 of T).
6. **No candidate in reach** (a punch out of the trees, a line that only crosses rough): there is no distance target.
   Only the direction is corrected, towards the aim line, so a player threading a gap gets help through the gap and
   the speed is left as swung.

The swing still decides the distance: the inference only chooses which stretch of good ground the player was
heading for, then corrects towards it. This is cheap: a line walk on the course's class raster and the lookup grid,
well under 0.1 ms.

## Risks to watch in playtests

- **The game playing itself.** If casual shots always land in the zone, decisions stop mattering. The soft knee and
  the lie factor must leave room to miss.
- **A magnet feeling.** Heavy correction can look like the ball is pulled mid-air. It isn't, since only the launch
  changes, but a start line rotated by a lot can still look odd next to the swing. Cap the rotation.
- **A wrong inference.** Aiming down a line that crosses two fairways (a dogleg, a neighbouring hole) could pick
  the wrong one. Log every inferred target in the lab and review the odd ones.
- **Exploits.** For example, swinging deliberately in the casual band to get maximum help, then relying on club
  choice for distance. That is probably fine: it is the strategic game working as intended. Watch for anything
  that beats a skilled swing.

## Decided 2026-10-12 (owner)

- Help fades with speed (principle 4); its shape is tuned in playtests.
- One assist configuration, always: no levels, modes or per-player settings (principle 6).
- The phone sends an aim direction; the engine infers the aim point ([above](#inferring-the-aim-point)).
- Assists are always hidden from players (principle 5).

## Still open

- Baseline carries are set for the middle tees (decided); the exact values are tuned in M5 and M7 playtests from
  the starting values above.
