//! The ball: one set of constants for every step (D7, docs/models/README.md).

/// kg, the Rules of Golf maximum
pub const MASS_KG: f64 = 0.04593;
/// m, the Rules of Golf minimum diameter 42.67 mm
pub const RADIUS_M: f64 = 0.021335;
/// m, the radius the ball touches things with: the drawn ball, 2x a real one (D49). Flight, spin and roll keep the
/// real RADIUS_M; the ball's height on the ground, touchdown, the cup, the flagstick, trees and boundary edges use this
pub const CONTACT_RADIUS_M: f64 = 2.0 * RADIUS_M;
/// I = k m r²; placeholder until M3 picks one value (a real ball is close to a solid sphere's 0.4)
pub const INERTIA_K: f64 = 0.4;
/// m, the regulation cup's radius (108 mm diameter)
pub const CUP_RADIUS_M: f64 = 0.054;
/// m, the game's cup radius: 10 inches across (254 mm), for forgiving putts with the 2x ball (owner, 2026-10-13: D47, D51)
pub const GAME_CUP_RADIUS_M: f64 = 0.127;
