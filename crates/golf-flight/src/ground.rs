//! What the flight lands on. The course (heights, plus water surfaces over their beds), or flat ground for
//! calibration against launch-monitor data.

use golf_core::Vec2;
use golf_course::Course;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Ground,
    Water,
    /// the cup's opening (only with a hole in play): golf-ground's cup model takes the ball
    Hole,
}

pub trait Ground {
    /// the height the ball would touch at (x, y) and what it is; None past the world
    fn contact(&self, x: f64, y: f64) -> Option<(f64, Surface)>;
}

/// level ground at a fixed height
pub struct Flat(pub f64);

impl Ground for Flat {
    #[inline]
    fn contact(&self, _x: f64, _y: f64) -> Option<(f64, Surface)> {
        Some((self.0, Surface::Ground))
    }
}

impl Ground for Course {
    #[inline]
    fn contact(&self, x: f64, y: f64) -> Option<(f64, Surface)> {
        let z = self.height(x, y)?;
        match self.water_at(Vec2::new(x, y)) {
            Some(w) if w.surface_m > z => Some((w.surface_m, Surface::Water)),
            _ => Some((z, Surface::Ground)),
        }
    }
}
