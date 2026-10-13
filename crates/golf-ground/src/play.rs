//! A whole shot: flight, then bounces and hops, then slide and roll, to rest, the cup, water, a plug or off the
//! world.

use golf_core::{Air, LaunchConditions, Vec2, Vec3};
use golf_flight::{AeroParams, End, Model, State, Surface as ContactSurface, Wind};

use crate::bounce::bounce;
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

impl<'a, T: Terrain> BallModel<'a, T> {
    pub fn new(terrain: &'a T, mats: &'a GroundMaterials, aero: AeroParams, air: Air, wind: Wind, cup: Option<Cup>) -> BallModel<'a, T> {
        BallModel { terrain, mats, flight: Model::new(aero, air, wind, terrain), cup }
    }

    /// a full shot from a ball resting at `start`
    pub fn play(&self, start: Vec2, launch: &LaunchConditions) -> Outcome {
        let f = self.flight.fly(start, launch);
        let start3 = f.launch.p;
        let mut rec = Recorder::default();
        self.continue_from(f, start3, &mut rec, None)
    }

    /// a ball already rolling on the ground (a putt, or a test): velocity along the ground, rolling spin
    pub fn roll_from(&self, start: Vec2, v: Vec3) -> Outcome {
        let z = self.terrain.surface_at(start.x, start.y).map_or(0.0, |s| s.z);
        let p = start.extend(z + golf_core::ball::RADIUS_M);
        let n = self.terrain.surface_at(start.x, start.y).map_or(Vec3::Z, |s| Vec3::new(-s.dzdx, -s.dzdy, 1.0).norm());
        let s0 = State { t: 0.0, p, v, w: n.cross(v) / golf_core::ball::RADIUS_M };
        let mut rec = Recorder::default();
        let (end, s) = roll(self.terrain, self.mats, s0, self.cup, &mut rec);
        self.after_roll(end, s, p, &mut rec, Some(p))
    }

    fn after_roll(&self, end: RollEnd, s: State, start: Vec3, rec: &mut Recorder, first_contact: Option<Vec3>) -> Outcome {
        match end {
            RollEnd::Liftoff => {
                let f = self.flight.fly_from(s);
                self.continue_from(f, start, rec, first_contact)
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

    fn continue_from(&self, mut f: golf_flight::Flight, start: Vec3, rec: &mut Recorder, mut first: Option<Vec3>) -> Outcome {
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
                        let (end, r) = roll(self.terrain, self.mats, next, self.cup, rec);
                        if end != RollEnd::Liftoff {
                            return self.after_roll(end, r, start, rec, first);
                        }
                        f = self.flight.fly_from(r);
                    } else {
                        f = self.flight.fly_from(next);
                    }
                }
            }
        }
        let s = f.last;
        rec.event(&s, EventKind::Timeout);
        self.done(rec, start, first, s, RestState::Timeout)
    }
}
