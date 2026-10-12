//! Reads a C0 course bundle (the physics subset only) and answers the spatial queries the engine asks.
//! Contract: docs/contracts/P0-course-input.md. Status: M1.
//!
//! ```no_run
//! let course = golf_course::Course::open(std::path::Path::new("/path/to/course_4_7667961")).unwrap();
//! let z = course.height(1192.0, 1186.0);
//! ```
//!
//! A `Course` is immutable after load; share it across rounds with `Arc`.

pub mod bundle;
pub mod classes;
pub mod error;
pub mod heights;
pub mod holes;
pub mod instances;
pub mod materials;
pub mod polygon;
pub mod rules;
pub mod tiles;
pub mod water;

use std::path::Path;
use std::time::Instant;

use golf_core::{Vec2, Vec3};

pub use bundle::{Manifest, PrevailingWind};
pub use classes::SurfaceClass;
pub use error::CourseError;
pub use heights::Surface;
pub use holes::Hole;
pub use instances::Instances;
pub use materials::{Material, Materials};
pub use rules::{PenaltyArea, PenaltyColour, Rules};
pub use water::{Water, WaterHit};

#[derive(Clone, Copy, Debug)]
pub struct LoadOptions {
    /// check every file's size and blake3 against the manifest
    pub verify: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions { verify: true }
    }
}

/// what loading cost (not part of the course; for the budget in docs/02-latency-budget.md)
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadStats {
    pub files: usize,
    pub bytes_read: usize,
    pub read_ms: f64,
    pub decode_ms: f64,
}

pub struct Course {
    pub manifest: Manifest,
    pub heights: heights::Heights,
    pub classes: classes::Classes,
    pub materials: Materials,
    pub holes: Vec<Hole>,
    pub rules: Rules,
    pub water: Water,
    pub instances: Instances,
    pub load_stats: LoadStats,
}

impl Course {
    pub fn open(dir: &Path) -> Result<Course, CourseError> {
        Course::open_with(dir, LoadOptions::default())
    }

    pub fn open_with(dir: &Path, opt: LoadOptions) -> Result<Course, CourseError> {
        let t0 = Instant::now();
        let f = bundle::Files::open(dir, opt.verify)?;
        let t1 = Instant::now();
        let course = Course {
            heights: heights::Heights::load(&f)?,
            classes: classes::Classes::load(&f)?,
            materials: Materials::load(&f)?,
            holes: holes::load(&f)?,
            rules: Rules::load(&f)?,
            water: Water::load(&f)?,
            instances: Instances::load(&f)?,
            load_stats: LoadStats { files: f.len(), bytes_read: f.total_bytes(), ..Default::default() },
            manifest: f.manifest.clone(),
        };
        let t2 = Instant::now();
        Ok(Course {
            load_stats: LoadStats { read_ms: (t1 - t0).as_secs_f64() * 1e3, decode_ms: (t2 - t1).as_secs_f64() * 1e3, ..course.load_stats },
            ..course
        })
    }

    /// THE height function; None past the surround (the ball has left the world)
    #[inline]
    pub fn height(&self, x: f64, y: f64) -> Option<f64> {
        self.heights.z(x, y)
    }

    /// height, gradient and second derivatives, analytic from the same bicubic
    #[inline]
    pub fn surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        self.heights.surface(x, y)
    }

    /// the upward unit normal of the ground
    pub fn normal(&self, x: f64, y: f64) -> Option<Vec3> {
        let s = self.surface_at(x, y)?;
        Some(Vec3::new(-s.dzdx, -s.dzdy, 1.0).norm())
    }

    /// the surface class (the lie)
    #[inline]
    pub fn surface(&self, x: f64, y: f64) -> SurfaceClass {
        self.classes.at(x, y)
    }

    pub fn material(&self, c: SurfaceClass) -> &Material {
        self.materials.get(c)
    }

    /// The water at p. **The class raster decides what is wet** (P0: physics reads the raster); `water.json`'s
    /// bodies only name the water and give its surface. Outside every body, the surround's 8 m surface gives the
    /// height, else the ground. (On fixture 1920628 two dry islands in the lake are exported as "pond" bodies, so a
    /// ring alone would call them wet: docs/05-generator-requests.md.)
    pub fn water_at(&self, p: Vec2) -> Option<WaterHit> {
        if self.surface(p.x, p.y) != SurfaceClass::WATER {
            return None;
        }
        if let Some(h) = self.water.body_at(p) {
            return Some(h);
        }
        let z = self.water.surface_8m_at(p).or_else(|| self.height(p.x, p.y))?;
        Some(WaterHit { body: None, surface_m: z })
    }

    /// the whole-ball OB test (C0): false only when every part of the ball is out of bounds
    #[inline]
    pub fn in_bounds(&self, centre: Vec2, ball_radius_m: f64) -> bool {
        self.rules.in_bounds(centre, ball_radius_m)
    }

    /// the penalty area the ball is in or touching
    #[inline]
    pub fn penalty_area(&self, centre: Vec2, ball_radius_m: f64) -> Option<&PenaltyArea> {
        self.rules.penalty_area(centre, ball_radius_m)
    }

    /// approximate resident size of the decoded rasters, in bytes (heights as u16, classes as u8)
    pub fn raster_bytes(&self) -> usize {
        let t = tiles::TILE * tiles::TILE;
        let h: usize = self.heights.tiers.iter().map(|k| k.n_tiles()).sum::<usize>()
            + self.water.surface_8m.as_ref().map_or(0, |k| k.n_tiles());
        h * t * 2 + (self.classes.play.n_tiles() + self.classes.cover.n_tiles()) * t
    }

    /// hole by number (1-based)
    pub fn hole(&self, number: u8) -> Option<&Hole> {
        self.holes.iter().find(|h| h.number == number)
    }
}
