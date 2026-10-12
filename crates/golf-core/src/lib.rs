//! Shared types, units, the course frame and deterministic math. See README.md and docs/01-conventions.md.
//!
//! Everything in the engine that crosses a crate boundary uses these types. The frame is C0's: metres, x east,
//! y north, z up, origin at the world's south-west corner.

pub mod ball;
pub mod math;
pub mod vec;

pub use vec::{Vec2, Vec3};
