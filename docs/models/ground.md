# Ground model

`golf-ground`: the first contact → bounces → slide → roll → rest (or holed, water, plug). Status: **draft, M0
(2026-10-11)**. Milestone M3.

## Phases

```
contact ─▶ BOUNCE (impulse) ─▶ airborne hop (golf-flight's integrator, short) ─▶ contact ─▶ …
                 │ when the hop would be tiny
                 ▼
             SLIDE (contact point slipping; friction acts on speed and spin)
                 │ when the slip ≈ 0
                 ▼
             ROLL (rolling resistance + slope gravity, on the height function)
                 │ when speed < v_rest and the slope can't restart it
                 ▼
             REST
```

The cup can capture the ball in any phase near the pin. Water stops it at once. Soft sand can plug it on a bounce.

## Bounce

Following Penner's model of a ball landing on turf (A. R. Penner, *The run of a golf ball*, Can. J. Phys., 2002).
Check its constants against the paper before relying on them:

1. **Effective normal.** The turf gives way under the ball, so the ball sees a surface tilted back towards it. The
   normal from the height function is rotated towards the incoming direction by `θ_c = min(k_t · |v_in|, θ_max)`.
   `k_t` depends on the material's firmness (softer = more tilt). This is what makes a steep, fast wedge stop and a
   shallow driver run.
2. **Normal restitution** `e_n(|v_n|)`, falling with impact speed: `e = cor_max · exp(-cor_decay · |v_n|)` as in
   GolfEngine, or Penner's polynomial. Per material.
3. **Tangential impulse: grip or slip.** Compute the impulse that would bring the contact point to rolling
   (`v_t + ω × r_c = 0`, using the ball's one moment of inertia). If it is more than `μ · (1 + e_n) · |v_n| · m`
   (Coulomb), the ball slips and gets the capped impulse instead. Spin changes by the same impulse's torque. This
   gives check and spin-back naturally: a high-backspin ball reverses its contact velocity and the friction pulls
   it backwards.
4. **Plugging.** On soft material (bunker sand, wet ground later): if the impact energy normal to the surface beats
   `firmness_pa × contact area`, the ball plugs (event `Plug`) and stops, or stops after a short slide.

**What we are not carrying over:** GolfEngine's spin-gated "forward absorption", "skid loss" and per-bounce caps.
They were added to fix specific shots and made the model fragile. We start from the three mechanisms above and add a
term only when a test in [03 §2](../03-success-indicators.md#2-ground-m3) fails, with a note on why.

Bounces end, and the ball goes to SLIDE or ROLL, when the next hop's apex would be under about 1 cm or the normal
speed is below about 0.3 m/s (fixed thresholds, tuned in M3).

## Slide and roll

On the ground the ball follows the height function: its centre sits at `r` above the surface along the normal, and
its velocity is projected onto the tangent plane every step.

- **Slide:** kinetic friction `μ_slide · N` opposes the contact-point slip, and its torque changes the spin. Slide
  ends when the slip is near zero; from then on `ω` is locked to rolling.
- **Roll:** the deceleration along the ground is `a = g_t / (1 + k) + a_rr`, where `g_t` is gravity along the slope,
  `k` the ball's inertia factor (`I = k m r²`), and `a_rr` the rolling resistance opposing motion.
- **Rolling resistance from stimp** on greens: a stimpmeter releases the ball at about 1.83 m/s, and it rolls the
  stimp distance `d` on a flat green, so `a_rr = v₀² / (2 d)`. Stimp 10.5 ft (3.20 m) gives `a_rr ≈ 0.52 m/s²`.
  This replaces `materials.json`'s `rolling_resistance` for greens; one source of truth (D18). Other surfaces have
  a rolling resistance per material. A small extra term at low speed stops endless creeping (the ball settles
  into the grass).
- **Liftoff on crests:** if `|v|² · κ > g cos θ` (the curvature from the bicubic's second derivative), the ball
  leaves the ground and goes back to a short flight.
- **Integration:** semi-implicit Euler at 1 ms with the exact height and normal each step (cheap). We drop
  GolfEngine's ground air drag (Cd 0.45 on a rolling ball), which isn't physical at roll speeds.
- **Rest:** speed below about 2 cm/s and the slope's pull less than static rolling resistance (`g_t/(1 + k) < a_static`).

## Cup and flagstick

- **Cup:** 108 mm diameter (Rules of Golf), centred on the pin `xy`. A ball whose centre passes over the cup is
  captured if its speed is below a capture speed that depends on how far from the centre it crosses: full speed
  limit near the centre, falling to zero at the edge (B. W. Holmes, *Putting: how a golf ball and hole interact*,
  Am. J. Phys., 1991, gives the classic curve; about 1.3-1.6 m/s for a centred putt; check the paper's numbers). Faster balls lip out, with a deflection (event `LipOut`). The putting assist can
  widen the effective cup (logged).
- **Flagstick:** a vertical cylinder in the cup; a hit is an impulse with low restitution. Flagstick in or out is a
  game setting.

## Materials (parameters per class)

The list is in [P0 §Materials](../contracts/P0-course-input.md#materials). First values come from GolfEngine's
surface table (green, fairway, fringe, rough, bunker) and C0's defaults for the rest; all placeholders until the M3
tests and M10 playtests.
