//! What the ground model needs from the world: the height function with its derivatives, the surface class, and
//! water. The course provides it; `TestPlane` is a uniform tilted plane for exact tests.

use golf_core::Vec2;
use golf_course::{Course, Surface, SurfaceClass};
use golf_flight::{Ground, Surface as ContactSurface};

pub trait Terrain: Ground {
    fn surface_at(&self, x: f64, y: f64) -> Option<Surface>;
    fn class_at(&self, x: f64, y: f64) -> SurfaceClass;
    /// the water surface at p, when p is wet
    fn water_at(&self, p: Vec2) -> Option<f64>;
}

impl Terrain for Course {
    #[inline]
    fn surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        Course::surface_at(self, x, y)
    }
    #[inline]
    fn class_at(&self, x: f64, y: f64) -> SurfaceClass {
        self.surface(x, y)
    }
    #[inline]
    fn water_at(&self, p: Vec2) -> Option<f64> {
        Course::water_at(self, p).map(|w| w.surface_m)
    }
}

/// z = z0 + gx x + gy y, one surface class everywhere
pub struct TestPlane {
    pub z0: f64,
    pub gx: f64,
    pub gy: f64,
    pub class: SurfaceClass,
}

impl TestPlane {
    pub fn flat(class: SurfaceClass) -> TestPlane {
        TestPlane { z0: 0.0, gx: 0.0, gy: 0.0, class }
    }
}

impl Ground for TestPlane {
    fn contact(&self, x: f64, y: f64) -> Option<(f64, ContactSurface)> {
        let z = self.z0 + self.gx * x + self.gy * y;
        Some((z, if self.class == SurfaceClass::WATER { ContactSurface::Water } else { ContactSurface::Ground }))
    }
}

impl Terrain for TestPlane {
    fn surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        Some(Surface { z: self.z0 + self.gx * x + self.gy * y, dzdx: self.gx, dzdy: self.gy, dxx: 0.0, dxy: 0.0, dyy: 0.0, tier: 0 })
    }
    fn class_at(&self, _x: f64, _y: f64) -> SurfaceClass {
        self.class
    }
    fn water_at(&self, _p: Vec2) -> Option<f64> {
        None
    }
}

/// Any height function z = h(x, y) with a surface class per point: for shaped test terrain (ridges, steps, cross
/// slopes, backstops). Derivatives are central differences, accurate for smooth functions.
pub struct FnTerrain<H: Fn(f64, f64) -> f64, K: Fn(f64, f64) -> SurfaceClass> {
    pub h: H,
    pub class: K,
}

const FD_STEP_M: f64 = 1e-3;

impl<H: Fn(f64, f64) -> f64, K: Fn(f64, f64) -> SurfaceClass> Ground for FnTerrain<H, K> {
    fn contact(&self, x: f64, y: f64) -> Option<(f64, ContactSurface)> {
        Some(((self.h)(x, y), ContactSurface::Ground))
    }
}

impl<H: Fn(f64, f64) -> f64, K: Fn(f64, f64) -> SurfaceClass> Terrain for FnTerrain<H, K> {
    fn surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let e = FD_STEP_M;
        let h = &self.h;
        let z = h(x, y);
        let (xp, xm, yp, ym) = (h(x + e, y), h(x - e, y), h(x, y + e), h(x, y - e));
        let dxy = (h(x + e, y + e) - h(x + e, y - e) - h(x - e, y + e) + h(x - e, y - e)) / (4.0 * e * e);
        Some(Surface {
            z,
            dzdx: (xp - xm) / (2.0 * e),
            dzdy: (yp - ym) / (2.0 * e),
            dxx: (xp - 2.0 * z + xm) / (e * e),
            dxy,
            dyy: (yp - 2.0 * z + ym) / (e * e),
            tier: 0,
        })
    }
    fn class_at(&self, x: f64, y: f64) -> SurfaceClass {
        (self.class)(x, y)
    }
    fn water_at(&self, _p: Vec2) -> Option<f64> {
        None
    }
}
