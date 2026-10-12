//! The surface class under a point: the play class raster (nearest node, C0 `lookup: nearest node`), then the
//! surround's 4 m cover raster, then 0 ("out of property / no data").

use serde_json::Value;

use crate::bundle::{Files, int, num, text};
use crate::error::CourseError;
use crate::tiles::{self, NODATA_U8, TILE, TileGrid};

/// a C0 surface class id (`materials.json`); ids are never renumbered within a C0 major version
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SurfaceClass(pub u8);

impl SurfaceClass {
    pub const NO_DATA: SurfaceClass = SurfaceClass(0);
    pub const GREEN: SurfaceClass = SurfaceClass(1);
    pub const COLLAR: SurfaceClass = SurfaceClass(2);
    pub const FAIRWAY: SurfaceClass = SurfaceClass(3);
    pub const TEE: SurfaceClass = SurfaceClass(4);
    pub const FIRST_CUT: SurfaceClass = SurfaceClass(5);
    pub const ROUGH: SurfaceClass = SurfaceClass(6);
    pub const FESCUE: SurfaceClass = SurfaceClass(7);
    pub const WASTE_SAND: SurfaceClass = SurfaceClass(8);
    pub const WATER: SurfaceClass = SurfaceClass(9);
    pub const PATH: SurfaceClass = SurfaceClass(10);
    pub const WOODS_FLOOR: SurfaceClass = SurfaceClass(11);
    pub const NATIVE: SurfaceClass = SurfaceClass(12);
    pub const STRUCTURE: SurfaceClass = SurfaceClass(13);
    pub const BUNKER_SAND: SurfaceClass = SurfaceClass(15);
    pub const FIELD: SurfaceClass = SurfaceClass(16);
}

/// one u8 raster on the C0 tiling
pub struct ClassRaster {
    pub name: String,
    pub res_m: f64,
    tiles: TileGrid<Box<[u8]>>,
}

impl ClassRaster {
    /// the class at the nearest node, None off the raster or at nodata
    #[inline]
    pub fn at(&self, x: f64, y: f64) -> Option<u8> {
        let (gx, gy) = ((x / self.res_m).round() as i64, (y / self.res_m).round() as i64);
        let (tx, b) = tiles::split(gx);
        let (ty, a) = tiles::split(gy);
        let c = self.tiles.get(tx, ty)?[a * TILE + b];
        (c != NODATA_U8).then_some(c)
    }

    pub fn n_tiles(&self) -> usize {
        self.tiles.len()
    }
}

pub struct Classes {
    /// the play raster (`surface/class_1m` today; 0.5 m once the generator ships it)
    pub play: ClassRaster,
    /// the surround's `cover_4m`
    pub cover: ClassRaster,
}

impl Classes {
    pub fn load(f: &Files) -> Result<Classes, CourseError> {
        let l = f
            .manifest
            .layers
            .iter()
            .find(|l| l["name"].as_str().is_some_and(|n| n.starts_with("surface/class_")))
            .ok_or_else(|| CourseError::Missing { what: "layer surface/class_*".into() })?;
        let name = l["name"].as_str().unwrap_or_default().to_string();
        let res_m = l["resolution_m"].as_f64().unwrap_or(1.0);
        let mut out = Vec::new();
        for t in l["tiles"].as_array().ok_or_else(|| CourseError::Missing { what: format!("{name}.tiles") })? {
            let path = text(&t["file"], "tile.file")?;
            let frame = tiles::slice(path, f.get(path)?, int(&t["offset"], "tile.offset")? as usize, int(&t["len"], "tile.len")? as usize)?;
            let raw = tiles::unzstd(path, frame, TILE * TILE)?;
            out.push(((int(&t["tx"], "tile.tx")?, int(&t["ty"], "tile.ty")?), raw.into_boxed_slice()));
        }
        let play = ClassRaster { name, res_m, tiles: TileGrid::new(out) };

        let s: Value = f.json("surround/surround.json")?;
        let l = s["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"].as_str() == Some("surround/cover_4m")))
            .ok_or_else(|| CourseError::Missing { what: "surround/cover_4m".into() })?;
        let res_m = num(&l["tiling"]["resolution_m"], "surround/cover_4m.tiling.resolution_m")?;
        let mut out = Vec::new();
        for t in l["tiles"].as_array().ok_or_else(|| CourseError::Missing { what: "surround/cover_4m.tiles".into() })? {
            let path = format!("surround/{}", text(&t["path"], "tile.path")?);
            let raw = tiles::unzstd(&path, f.get(&path)?, TILE * TILE)?;
            out.push(((int(&t["tx"], "tile.tx")?, int(&t["ty"], "tile.ty")?), raw.into_boxed_slice()));
        }
        let cover = ClassRaster { name: "surround/cover_4m".into(), res_m, tiles: TileGrid::new(out) };
        Ok(Classes { play, cover })
    }

    /// the class at (x, y): play raster, then cover, then 0
    #[inline]
    pub fn at(&self, x: f64, y: f64) -> SurfaceClass {
        SurfaceClass(self.play.at(x, y).or_else(|| self.cover.at(x, y)).unwrap_or(0))
    }
}
