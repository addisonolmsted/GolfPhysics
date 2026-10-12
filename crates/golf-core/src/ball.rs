//! The ball: one set of constants for every step (D7, docs/models/README.md).

/// kg, the Rules of Golf maximum
pub const MASS_KG: f64 = 0.04593;
/// m, the Rules of Golf minimum diameter 42.67 mm
pub const RADIUS_M: f64 = 0.021335;
/// I = k m r²; placeholder until M3 picks one value (a real ball is close to a solid sphere's 0.4)
pub const INERTIA_K: f64 = 0.4;
/// m, the cup's radius (108 mm diameter)
pub const CUP_RADIUS_M: f64 = 0.054;
