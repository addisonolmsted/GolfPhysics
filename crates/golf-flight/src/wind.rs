//! The wind the ball flies through (docs/models/wind.md): a mean wind at 10 m with a log height profile. Gusts come
//! later; the mean is enough for M2.

use golf_core::{Vec2, Vec3, math};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    /// speed at 10 m above the ground
    pub speed_10m_mps: f64,
    /// the direction the wind blows towards (C0's downwind convention), from +x towards +y
    pub downwind_rad: f64,
    /// roughness length, m (open grass ~0.03)
    pub z0_m: f64,
}

/// below this height the profile is held (the log law has no meaning at the surface)
pub(crate) const Z_MIN_M: f64 = 1.0;

impl Wind {
    pub const CALM: Wind = Wind { speed_10m_mps: 0.0, downwind_rad: 0.0, z0_m: 0.03 };

    pub fn new(speed_10m_mps: f64, downwind_rad: f64) -> Wind {
        Wind { speed_10m_mps, downwind_rad, z0_m: 0.03 }
    }

    /// the wind vector at a height above the ground
    #[inline]
    pub fn at(&self, z_agl_m: f64) -> Vec3 {
        if self.speed_10m_mps == 0.0 {
            return Vec3::ZERO;
        }
        let z = z_agl_m.max(Z_MIN_M);
        let k = math::ln(z / self.z0_m) / math::ln(10.0 / self.z0_m);
        Vec2::from_azimuth(self.downwind_rad).extend(0.0) * (self.speed_10m_mps * k)
    }
}
