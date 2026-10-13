//! What a shot leaves behind: samples (~30 Hz plus every event) and events, the material for P2.

use golf_core::Vec3;
use golf_course::SurfaceClass;
use golf_flight::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Air,
    Slide,
    Roll,
    Rest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub t: f64,
    pub p: Vec3,
    pub v: Vec3,
    pub phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
    Land { class: SurfaceClass },
    Bounce { class: SurfaceClass, cor: f64, gripped: bool },
    StartSlide,
    StartRoll,
    Liftoff,
    LipOut,
    Holed,
    Water,
    Plug,
    Rest,
    OffWorld,
    Timeout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub t: f64,
    pub p: Vec3,
    pub kind: EventKind,
}

#[derive(Default, Debug, Clone)]
pub struct Recorder {
    pub samples: Vec<Sample>,
    pub events: Vec<Event>,
    next_t: f64,
}

pub const SAMPLE_DT_S: f64 = 1.0 / 30.0;

impl Recorder {
    /// a ground sample, kept when 1/30 s has passed since the last one
    pub fn sample(&mut self, s: State, phase: Phase) {
        if s.t + 1e-9 >= self.next_t || phase == Phase::Rest {
            self.samples.push(Sample { t: s.t, p: s.p, v: s.v, phase });
            self.next_t = s.t + SAMPLE_DT_S;
        }
    }

    /// the flight's own samples (already ~30 Hz)
    pub fn flight(&mut self, f: &golf_flight::Flight) {
        for s in &f.samples {
            self.samples.push(Sample { t: s.t, p: s.p, v: s.v, phase: Phase::Air });
        }
        self.next_t = f.last.t;
    }

    pub fn event(&mut self, t: f64, p: Vec3, kind: EventKind) {
        self.events.push(Event { t, p, kind });
        // every event is also a sample, so a drawn path goes through it
        if self.samples.last().is_none_or(|s| s.t < t) {
            self.samples.push(Sample { t, p, v: Vec3::ZERO, phase: Phase::Roll });
        }
    }
}
