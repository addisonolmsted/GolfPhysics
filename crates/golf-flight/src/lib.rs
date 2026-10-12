//! Launch conditions -> airborne trajectory. See README.md and docs/models/flight.md.
//!
//! Drag and Magnus lift as functions of the spin factor, spin decay, a mean wind with a log height profile,
//! classic RK4 at a fixed 10 ms step (4th order: the wind is taken at each stage's height), and the ground (or water) contact found by bisection. Deterministic: f64,
//! libm, fixed steps, fixed iteration counts.

pub mod aero;
pub mod flight;
pub mod ground;
pub mod wind;

pub use aero::AeroParams;
pub use flight::{End, Flight, Model, State};
pub use ground::{Flat, Ground, Surface};
pub use wind::Wind;
