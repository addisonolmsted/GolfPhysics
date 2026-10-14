//! A whole shot: flight, then bounces and hops, then slide and roll, to rest, the cup, water, a plug or off the
//! world.

use golf_core::{Air, LaunchConditions, Vec2, Vec3};
use golf_flight::{AeroParams, End, Model, State, Surface as ContactSurface, Wind};

use crate::bounce::bounce;
use crate::cup::CupEnd;
use crate::materials::GroundMaterials;
use crate::record::{Event, EventKind, Recorder, Sample};
use crate::roll::{Cup, RollEnd, roll};
use crate::terrain::Terrain;

/// below this normal speed after a bounce the ball stays on the ground (it would rise less than ~1 cm)
pub const HOP_MIN_MPS: f64 = 0.5;
const MAX_SEGMENTS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestState {
    OnGround,
    Holed,
    InWater,
    Plugged,
    OffWorld,
    Timeout,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub samples: Vec<Sample>,
    pub events: Vec<Event>,
    pub start: Vec3,
    pub first_contact: Option<Vec3>,
    pub rest: Vec3,
    pub state: RestState,
    pub time_s: f64,
}

impl Outcome {
    /// horizontal distance from the start to the first contact
    pub fn carry_m(&self) -> f64 {
        self.first_contact.map_or(0.0, |c| (c.xy() - self.start.xy()).len())
    }
    /// horizontal distance from the start to where the ball finished
    pub fn total_m(&self) -> f64 {
        (self.rest.xy() - self.start.xy()).len()
    }
    pub fn bounces(&self) -> usize {
        self.events.iter().filter(|e| matches!(e.kind, EventKind::Bounce { .. })).count()
    }
}

pub struct BallModel<'a, T: Terrain> {
    pub terrain: &'a T,
    pub mats: &'a GroundMaterials,
    pub flight: Model<'a>,
    pub cup: Option<Cup>,
}

/// what one shot plays against: the cup with its flagstick settled for this shot, and the flight's view of it
#[derive(Clone, Copy)]
struct Shot {
    cup: Option<Cup>,
    hole: Option<golf_flight::Hole>,
}


impl<'a, T: Terrain> BallModel<'a, T> {
    pub fn new(terrain: &'a T, mats: &'a GroundMaterials, aero: AeroParams, air: Air, wind: Wind, cup: Option<Cup>) -> BallModel<'a, T> {
        BallModel { terrain, mats, flight: Model::new(aero, air, wind, terrain), cup }
    }

    /// the cup for a shot from `start`, with the flagstick in or out by the game's rule (D52)
    fn shot(&self, start: Vec2) -> Shot {
        let cup = self.cup.map(|c| c.for_shot(self.terrain.class_at(start.x, start.y)));
        let hole = cup.map(|c| c.hole(self.terrain.surface_at(c.xy.x, c.xy.y).map_or(c.z, |s| s.z)));
        Shot { cup, hole }
    }

    /// a full shot from a ball resting at `start`
    pub fn play(&self, start: Vec2, launch: &LaunchConditions) -> Outcome {
        let sh = self.shot(start);
        let f = self.flight.fly_at(start, launch, sh.hole.as_ref());
        let start3 = f.launch.p;
        let mut rec = Recorder::default();
        self.continue_from(&sh, f, start3, &mut rec, None)
    }

    /// a ball already rolling on the ground (a putt, or a test): velocity along the ground, rolling spin
    pub fn roll_from(&self, start: Vec2, v: Vec3) -> Outcome {
        let sh = self.shot(start);
        let z = self.terrain.surface_at(start.x, start.y).map_or(0.0, |s| s.z);
        let p = start.extend(z + golf_core::ball::CONTACT_RADIUS_M);
        let n = self.terrain.surface_at(start.x, start.y).map_or(Vec3::Z, |s| Vec3::new(-s.dzdx, -s.dzdy, 1.0).norm());
        let s0 = State { t: 0.0, p, v, w: n.cross(v) / golf_core::ball::RADIUS_M };
        let mut rec = Recorder::default();
        let (end, s) = roll(self.terrain, self.mats, s0, sh.cup, &mut rec);
        self.after_roll(&sh, end, s, p, &mut rec, Some(p))
    }

    fn after_roll(&self, sh: &Shot, end: RollEnd, s: State, start: Vec3, rec: &mut Recorder, first_contact: Option<Vec3>) -> Outcome {
        match end {
            RollEnd::Liftoff => {
                let f = self.flight.fly_from_at(s, sh.hole.as_ref());
                self.continue_from(sh, f, start, rec, first_contact)
            }
            _ => {
                let state = match end {
                    RollEnd::Rest => RestState::OnGround,
                    RollEnd::Holed => RestState::Holed,
                    RollEnd::InWater => RestState::InWater,
                    RollEnd::OffWorld => RestState::OffWorld,
                    _ => RestState::Timeout,
                };
                Outcome { samples: std::mem::take(&mut rec.samples), events: std::mem::take(&mut rec.events), start, first_contact, rest: s.p, state, time_s: s.t }
            }
        }
    }

    fn done(&self, rec: &mut Recorder, start: Vec3, first: Option<Vec3>, s: State, state: RestState) -> Outcome {
        Outcome { samples: std::mem::take(&mut rec.samples), events: std::mem::take(&mut rec.events), start, first_contact: first, rest: s.p, state, time_s: s.t }
    }

    fn continue_from(&self, sh: &Shot, mut f: golf_flight::Flight, start: Vec3, rec: &mut Recorder, mut first: Option<Vec3>) -> Outcome {
        let hole = sh.hole.as_ref();
        for _ in 0..MAX_SEGMENTS {
            rec.flight(&f);
            let s = f.last;
            match f.end {
                End::OffWorld => {
                    rec.event(&s, EventKind::OffWorld);
                    return self.done(rec, start, first, s, RestState::OffWorld);
                }
                End::Timeout => {
                    rec.event(&s, EventKind::Timeout);
                    return self.done(rec, start, first, s, RestState::Timeout);
                }
                End::Flagstick { normal } => {
                    // off the pole: an impulse with its restitution, and Coulomb friction on the contact point's
                    // slip, which also changes the spin (as in cup.rs)
                    rec.event(&s, EventKind::Flagstick);
                    let r = crate::cup::pin_impulse(s, normal);
                    let mut r = r;
                    r.p += normal * 1e-3;
                    f = self.flight.fly_from_at(r, hole);
                }
                End::Contact { surface, .. } => {
                    let class = self.terrain.class_at(s.p.x, s.p.y);
                    if first.is_none() {
                        first = Some(s.p);
                        rec.event(&s, EventKind::Land { class });
                    }
                    if surface == ContactSurface::Water {
                        rec.event(&s, EventKind::Water);
                        return self.done(rec, start, first, s, RestState::InWater);
                    }
                    if surface == ContactSurface::Hole
                        && let (Some(c), Some(h)) = (sh.cup.as_ref(), hole)
                    {
                        // coming down at the hole: the cup model takes it (dunks, and bounces next to the hole)
                        let cn = self.terrain.surface_at(c.xy.x, c.xy.y).map_or(Vec3::Z, |sf| Vec3::new(-sf.dzdx, -sf.dzdy, 1.0).norm());
                        let rr = self.mats.get(self.terrain.class_at(c.xy.x, c.xy.y)).base_decel();
                        let (end, r) = crate::cup::at_the_cup(c, c.xy.extend(h.z), cn, rr, false, s, rec);
                        match end {
                            CupEnd::Holed => return self.done(rec, start, first, r, RestState::Holed),
                            CupEnd::Airborne => f = self.flight.fly_from_at(r, hole),
                            CupEnd::Rolling => {
                                let (end, r2) = roll(self.terrain, self.mats, r, sh.cup, rec);
                                if end != RollEnd::Liftoff {
                                    return self.after_roll(sh, end, r2, start, rec, first);
                                }
                                f = self.flight.fly_from_at(r2, hole);
                            }
                        }
                        continue;
                    }
                    let Some(sf) = self.terrain.surface_at(s.p.x, s.p.y) else {
                        return self.done(rec, start, first, s, RestState::OffWorld);
                    };
                    let n = Vec3::new(-sf.dzdx, -sf.dzdy, 1.0).norm();
                    let b = bounce(s.v, s.w, n, self.mats.get(class));
                    if b.plugged {
                        rec.event(&s, EventKind::Plug);
                        let mut r = s;
                        r.v = Vec3::ZERO;
                        r.w = Vec3::ZERO;
                        return self.done(rec, start, first, r, RestState::Plugged);
                    }
                    rec.event(&s, EventKind::Bounce { class, cor: b.cor, gripped: b.gripped });
                    let next = State { t: s.t, p: s.p, v: b.v, w: b.w };
                    if b.v.dot(n) < HOP_MIN_MPS {
                        let (end, r) = roll(self.terrain, self.mats, next, sh.cup, rec);
                        if end != RollEnd::Liftoff {
                            return self.after_roll(sh, end, r, start, rec, first);
                        }
                        f = self.flight.fly_from_at(r, hole);
                    } else {
                        f = self.flight.fly_from_at(next, hole);
                    }
                }
            }
        }
        let s = f.last;
        rec.event(&s, EventKind::Timeout);
        self.done(rec, start, first, s, RestState::Timeout)
    }
}
