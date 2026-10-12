//! THE height function (C0): bicubic Catmull-Rom over the finest tier whose 4 x 4 nodes around the point are all
//! present -- play 0.5 m, core 2 m, full 8 m, then the surround's 8 m -- plus its analytic derivatives.
//!
//! Node lookup and the summation order follow the generator's reference reader
//! (`course-bundle/src/reader.rs`) exactly, so the two agree to rounding (the M1 conformance check).

use serde_json::Value;

use crate::bundle::{Files, int, num, text};
use crate::error::CourseError;
use crate::tiles::{self, NODATA_U16, STRIDE, TILE, TileGrid};

/// how a tile's u16 samples become metres
#[derive(Clone, Copy, Debug)]
enum Enc {
    /// the C0 tiers: `(offset_q + q) * scale_m`
    Grid { offset_q: i64, scale_m: f64 },
    /// the surround: `offset_m + q * scale_m`
    Offset { offset_m: f64, scale_m: f64 },
}

struct Tile {
    q: Box<[u16]>,
    enc: Enc,
}

impl Tile {
    #[inline]
    fn value(&self, a: usize, b: usize) -> Option<f64> {
        let q = self.q[a * TILE + b];
        if q == NODATA_U16 {
            return None;
        }
        Some(match self.enc {
            Enc::Grid { offset_q, scale_m } => (offset_q + q as i64) as f64 * scale_m,
            Enc::Offset { offset_m, scale_m } => offset_m + q as f64 * scale_m,
        })
    }
}

/// one height tier
pub struct Tier {
    pub name: String,
    pub res_m: f64,
    tiles: TileGrid<Tile>,
}

/// the height, its gradient and its second derivatives at a point, and which tier answered
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub z: f64,
    pub dzdx: f64,
    pub dzdy: f64,
    pub dxx: f64,
    pub dxy: f64,
    pub dyy: f64,
    /// index into `Heights::tiers` (0 = finest)
    pub tier: usize,
}

impl Tier {
    /// The node at global (gx, gy): from its tile, or from the neighbour that shares it when the owning tile is
    /// absent or holds nodata there (the reference reader's rule).
    #[inline]
    pub fn node(&self, gx: i64, gy: i64) -> Option<f64> {
        let (tx, b) = tiles::split(gx);
        let (ty, a) = tiles::split(gy);
        if let Some(v) = self.tiles.get(tx, ty).and_then(|t| t.value(a, b)) {
            return Some(v);
        }
        for (dx, dy) in [(1, 0), (0, 1), (1, 1)] {
            if (dx == 1 && b != 0) || (dy == 1 && a != 0) {
                continue;
            }
            let (bb, aa) = (if dx == 1 { 255 } else { b }, if dy == 1 { 255 } else { a });
            if let Some(v) = self.tiles.get(tx - dx, ty - dy).and_then(|t| t.value(aa, bb)) {
                return Some(v);
            }
        }
        None
    }

    /// the 4 x 4 nodes around (x, y) and the fractions, or None unless all 16 are present
    #[inline]
    fn patch(&self, x: f64, y: f64) -> Option<([[f64; 4]; 4], f64, f64)> {
        let (fx, fy) = (x / self.res_m, y / self.res_m);
        let (gx, gy) = (fx.floor() as i64, fy.floor() as i64);
        let v = self.patch_fast(gx, gy).or_else(|| self.patch_slow(gx, gy))?;
        Some((v, fx - gx as f64, fy - gy as f64))
    }

    /// Fast path: all 16 nodes are owned by one tile (columns and rows 0..=254 of it) and none is nodata. Exactly
    /// what `node` returns for each, without the per-node tile search.
    #[inline]
    fn patch_fast(&self, gx: i64, gy: i64) -> Option<[[f64; 4]; 4]> {
        let (tx, b0) = tiles::split(gx - 1);
        let (ty, a0) = tiles::split(gy - 1);
        if b0 > 251 || a0 > 251 {
            return None;
        }
        let t = self.tiles.get(tx, ty)?;
        let mut q = [[0u16; 4]; 4];
        for (a, row) in q.iter_mut().enumerate() {
            let base = (a0 + a) * TILE + b0;
            row.copy_from_slice(&t.q[base..base + 4]);
            if row.contains(&NODATA_U16) {
                return None;
            }
        }
        let mut v = [[0.0; 4]; 4];
        match t.enc {
            Enc::Grid { offset_q, scale_m } => {
                for a in 0..4 {
                    for c in 0..4 {
                        v[a][c] = (offset_q + q[a][c] as i64) as f64 * scale_m;
                    }
                }
            }
            Enc::Offset { offset_m, scale_m } => {
                for a in 0..4 {
                    for c in 0..4 {
                        v[a][c] = offset_m + q[a][c] as f64 * scale_m;
                    }
                }
            }
        }
        Some(v)
    }

    fn patch_slow(&self, gx: i64, gy: i64) -> Option<[[f64; 4]; 4]> {
        let mut v = [[0.0; 4]; 4];
        for (a, row) in v.iter_mut().enumerate() {
            for (c, n) in row.iter_mut().enumerate() {
                *n = self.node(gx - 1 + c as i64, gy - 1 + a as i64)?;
            }
        }
        Some(v)
    }

    /// the height at (x, y), None unless the 16 nodes are present
    #[inline]
    pub fn z(&self, x: f64, y: f64) -> Option<f64> {
        let (v, tx, ty) = self.patch(x, y)?;
        let (wx, _, _) = tiles::catmull(tx);
        let (wy, _, _) = tiles::catmull(ty);
        let mut s = 0.0;
        for (a, wa) in wy.iter().enumerate() {
            let mut r = 0.0;
            for (c, wb) in wx.iter().enumerate() {
                r += wb * v[a][c];
            }
            s += wa * r;
        }
        Some(s)
    }

    /// the height and its first and second derivatives at (x, y)
    pub fn surface(&self, x: f64, y: f64, tier: usize) -> Option<Surface> {
        let (v, tx, ty) = self.patch(x, y)?;
        let (wx, dx, ddx) = tiles::catmull(tx);
        let (wy, dy, ddy) = tiles::catmull(ty);
        let (mut z, mut zx, mut zy, mut zxx, mut zxy, mut zyy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for a in 0..4 {
            let (mut r, mut rx, mut rxx) = (0.0, 0.0, 0.0);
            for c in 0..4 {
                r += wx[c] * v[a][c];
                rx += dx[c] * v[a][c];
                rxx += ddx[c] * v[a][c];
            }
            z += wy[a] * r;
            zx += wy[a] * rx;
            zxx += wy[a] * rxx;
            zy += dy[a] * r;
            zxy += dy[a] * rx;
            zyy += ddy[a] * r;
        }
        let h = self.res_m;
        Some(Surface { z, dzdx: zx / h, dzdy: zy / h, dxx: zxx / (h * h), dxy: zxy / (h * h), dyy: zyy / (h * h), tier })
    }

    /// the value at the nearest node, None at nodata (for layers where nodata has a meaning, e.g. dry water)
    #[inline]
    pub fn nearest(&self, x: f64, y: f64) -> Option<f64> {
        self.node((x / self.res_m).round() as i64, (y / self.res_m).round() as i64)
    }

    pub fn n_tiles(&self) -> usize {
        self.tiles.len()
    }
}

/// every tier, finest first
pub struct Heights {
    pub tiers: Vec<Tier>,
}

impl Heights {
    pub fn load(f: &Files) -> Result<Heights, CourseError> {
        let mut tiers = Vec::new();
        for name in ["height/play_0p5m", "height/core_2m", "height/full_8m"] {
            let l = f.manifest.layer(name).ok_or_else(|| CourseError::Missing { what: format!("layer {name}") })?;
            tiers.push(c0_tier(f, name, l)?);
        }
        tiers.push(surround_tier(f, "surround/height_8m")?.ok_or_else(|| CourseError::Missing { what: "surround/height_8m".into() })?);
        Ok(Heights { tiers })
    }

    /// THE height function: the height at (x, y) and the tier that answered; None past every tier
    #[inline]
    pub fn at(&self, x: f64, y: f64) -> Option<(f64, usize)> {
        self.tiers.iter().enumerate().find_map(|(k, t)| t.z(x, y).map(|z| (z, k)))
    }

    #[inline]
    pub fn z(&self, x: f64, y: f64) -> Option<f64> {
        self.at(x, y).map(|v| v.0)
    }

    /// the height with its derivatives, from the same tier `at` would use
    pub fn surface(&self, x: f64, y: f64) -> Option<Surface> {
        self.tiers.iter().enumerate().find_map(|(k, t)| t.surface(x, y, k))
    }
}

fn c0_tier(f: &Files, name: &str, l: &Value) -> Result<Tier, CourseError> {
    let res_m = num(&l["resolution_m"], &format!("{name}.resolution_m"))?;
    let scale_m = num(&l["encoding"]["scale_m"], &format!("{name}.encoding.scale_m"))?;
    let filter = text(&l["encoding"]["filter"], &format!("{name}.encoding.filter"))?;
    if filter != "planar_zigzag_byteplanes" {
        return Err(CourseError::Version { what: format!("{name} filter"), found: filter.into() });
    }
    let mut out = Vec::new();
    for t in l["tiles"].as_array().ok_or_else(|| CourseError::Missing { what: format!("{name}.tiles") })? {
        let path = text(&t["file"], "tile.file")?;
        let b = f.get(path)?;
        let frame = tiles::slice(path, b, int(&t["offset"], "tile.offset")? as usize, int(&t["len"], "tile.len")? as usize)?;
        let q = tiles::unfilter_planar(&tiles::unzstd(path, frame, TILE * TILE * 2)?);
        let enc = Enc::Grid { offset_q: int(&t["offset_q"], "tile.offset_q")?, scale_m };
        out.push(((int(&t["tx"], "tile.tx")?, int(&t["ty"], "tile.ty")?), Tile { q: q.into_boxed_slice(), enc }));
    }
    Ok(Tier { name: name.to_string(), res_m, tiles: TileGrid::new(out) })
}

/// A u16 layer of the surround (`surround/surround.json`): `surround/height_8m`, or `surround/water_8m` (the water
/// surface; nodata = dry). None when the layer isn't listed (a course without water has no water tiles).
pub(crate) fn surround_tier(f: &Files, name: &str) -> Result<Option<Tier>, CourseError> {
    let s: Value = f.json("surround/surround.json")?;
    let Some(l) = s["layers"].as_array().and_then(|ls| ls.iter().find(|l| l["name"].as_str() == Some(name))) else {
        return Ok(None);
    };
    let res_m = num(&l["tiling"]["resolution_m"], &format!("{name}.tiling.resolution_m"))?;
    if l["tiling"]["stride_samples"].as_i64() != Some(STRIDE) {
        return Err(CourseError::Version { what: format!("{name} tiling"), found: l["tiling"].to_string() });
    }
    let mut out = Vec::new();
    for t in l["tiles"].as_array().map(|v| v.as_slice()).unwrap_or_default() {
        let path = format!("surround/{}", text(&t["path"], "tile.path")?);
        let raw = tiles::unzstd(&path, f.get(&path)?, TILE * TILE * 2)?;
        let enc = Enc::Offset { offset_m: num(&t["offset_m"], "tile.offset_m")?, scale_m: num(&t["scale_m"], "tile.scale_m")? };
        out.push(((int(&t["tx"], "tile.tx")?, int(&t["ty"], "tile.ty")?), Tile { q: tiles::u16_le(&raw).into_boxed_slice(), enc }));
    }
    Ok(Some(Tier { name: name.to_string(), res_m, tiles: TileGrid::new(out) }))
}
