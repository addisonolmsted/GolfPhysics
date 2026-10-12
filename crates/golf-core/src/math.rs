//! Deterministic math. The core crates call these, never `f64::sin` and friends: `std`'s versions may go to the
//! platform libm, which differs in the last bit between macOS, Linux and WASM (docs/01-conventions.md,
//! Determinism). `sqrt`, `abs`, `floor` and the basic operators are IEEE-exact everywhere, so they stay on `f64`.

#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}
#[inline]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
#[inline]
pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}
#[inline]
pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}
#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}
#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}
#[inline]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
#[inline]
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// metres per yard (exact, international yard)
pub const M_PER_YD: f64 = 0.9144;
/// rad/s per rpm
pub const RADPS_PER_RPM: f64 = core::f64::consts::PI / 30.0;
/// standard gravity, m/s²
pub const G_MPS2: f64 = 9.80665;

/// an angle wrapped into [0, 2π): a direction (docs/01-conventions.md, "Directions vs axes")
#[inline]
pub fn wrap_dir(a: f64) -> f64 {
    let t = core::f64::consts::TAU;
    let r = a % t;
    if r < 0.0 { r + t } else { r }
}
