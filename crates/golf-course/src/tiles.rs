//! The C0 raster tiling and the decoders shared by every raster layer (docs/contracts/P0-course-input.md).
//!
//! Tiles are 256 x 256 samples with a one-sample overlap (stride 255), anchored at the world origin: tile
//! `(tx, ty)` sample row `a`, column `b` is global node `(255 tx + b, 255 ty + a)` at world `(gx res, gy res)`.
//! Rows run south to north. A node on a tile edge is stored in both tiles.

use std::io::Read;

use crate::error::{CourseError, format};

pub const TILE: usize = 256;
pub const STRIDE: i64 = 255;
pub const NODATA_U16: u16 = 65535;
pub const NODATA_U8: u8 = 255;

/// one zstd frame, decompressed; it must come to exactly `raw_len` bytes
pub fn unzstd(path: &str, b: &[u8], raw_len: usize) -> Result<Vec<u8>, CourseError> {
    let mut src = b;
    let mut d = ruzstd::decoding::StreamingDecoder::new(&mut src).map_err(|e| format(path, format!("zstd: {e}")))?;
    let mut out = Vec::with_capacity(raw_len);
    d.read_to_end(&mut out).map_err(|e| format(path, format!("zstd: {e}")))?;
    if out.len() != raw_len {
        return Err(format(path, format!("frame decodes to {} bytes, expected {raw_len}", out.len())));
    }
    Ok(out)
}

/// the slice `[offset, offset + len)` of a file, checked
pub fn slice<'a>(path: &str, b: &'a [u8], offset: usize, len: usize) -> Result<&'a [u8], CourseError> {
    b.get(offset..offset + len).ok_or_else(|| format(path, "tile frame past the end of the file"))
}

/// Inverse of the generator's `planar_zigzag_byteplanes` filter (C0 height tiers): byte planes (all low bytes, then
/// all high bytes) of zigzagged residuals of the planar predictor `p = left + up - upleft` (mod 2^16; row 0: left,
/// column 0: up, sample (0, 0): 0). Returns the u16 samples.
pub fn unfilter_planar(f: &[u8]) -> Vec<u16> {
    let n = TILE;
    let half = f.len() / 2;
    let mut q = vec![0u16; half];
    for a in 0..n {
        for b in 0..n {
            let k = a * n + b;
            let z = f[k] as u16 | ((f[half + k] as u16) << 8);
            let r = ((z >> 1) as i16) ^ -((z & 1) as i16);
            q[k] = pred(&q, a, b).wrapping_add(r as u16);
        }
    }
    q
}

#[inline]
fn pred(q: &[u16], a: usize, b: usize) -> u16 {
    let n = TILE;
    match (a, b) {
        (0, 0) => 0,
        (0, _) => q[b - 1],
        (_, 0) => q[(a - 1) * n],
        _ => q[a * n + b - 1].wrapping_add(q[(a - 1) * n + b]).wrapping_sub(q[(a - 1) * n + b - 1]),
    }
}

/// plain little-endian u16 samples (the surround's tiles, which are not filtered)
pub fn u16_le(b: &[u8]) -> Vec<u16> {
    b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
}

/// Catmull-Rom weights for nodes -1, 0, 1, 2 at fraction t in [0, 1), and their first and second derivatives in t.
/// The weights are C0's (the manifest's `height_function`), written exactly as the generator's reader writes them.
#[inline]
pub fn catmull(t: f64) -> ([f64; 4], [f64; 4], [f64; 4]) {
    let (t2, t3) = (t * t, t * t * t);
    let w = [-0.5 * t3 + t2 - 0.5 * t, 1.5 * t3 - 2.5 * t2 + 1.0, -1.5 * t3 + 2.0 * t2 + 0.5 * t, 0.5 * t3 - 0.5 * t2];
    let d = [-1.5 * t2 + 2.0 * t - 0.5, 4.5 * t2 - 5.0 * t, -4.5 * t2 + 4.0 * t + 0.5, 1.5 * t2 - t];
    let dd = [-3.0 * t + 2.0, 9.0 * t - 5.0, -9.0 * t + 4.0, 3.0 * t - 1.0];
    (w, d, dd)
}

/// the tile holding global node g along one axis, and the node's index in it
#[inline]
pub fn split(g: i64) -> (i64, usize) {
    (g.div_euclid(STRIDE), g.rem_euclid(STRIDE) as usize)
}

/// A rectangle of tile slots, so a node lookup is arithmetic, not a map search.
pub struct TileGrid<T> {
    tx0: i64,
    ty0: i64,
    ntx: i64,
    nty: i64,
    slots: Vec<Option<T>>,
}

impl<T> TileGrid<T> {
    pub fn new(tiles: Vec<((i64, i64), T)>) -> TileGrid<T> {
        if tiles.is_empty() {
            return TileGrid { tx0: 0, ty0: 0, ntx: 0, nty: 0, slots: Vec::new() };
        }
        let tx0 = tiles.iter().map(|t| t.0.0).min().unwrap();
        let ty0 = tiles.iter().map(|t| t.0.1).min().unwrap();
        let ntx = tiles.iter().map(|t| t.0.0).max().unwrap() - tx0 + 1;
        let nty = tiles.iter().map(|t| t.0.1).max().unwrap() - ty0 + 1;
        let mut slots: Vec<Option<T>> = (0..ntx * nty).map(|_| None).collect();
        for ((tx, ty), t) in tiles {
            slots[((ty - ty0) * ntx + (tx - tx0)) as usize] = Some(t);
        }
        TileGrid { tx0, ty0, ntx, nty, slots }
    }
    #[inline]
    pub fn get(&self, tx: i64, ty: i64) -> Option<&T> {
        let (i, j) = (tx - self.tx0, ty - self.ty0);
        if i < 0 || j < 0 || i >= self.ntx || j >= self.nty {
            return None;
        }
        self.slots[(j * self.ntx + i) as usize].as_ref()
    }
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the generator's filter (course-bundle tiles.rs), to test the inverse against
    fn filter_planar(q: &[u16]) -> Vec<u8> {
        let n = TILE;
        let mut out = vec![0u8; q.len() * 2];
        let half = q.len();
        for a in 0..n {
            for b in 0..n {
                let k = a * n + b;
                let r = q[k].wrapping_sub(pred(q, a, b)) as i16;
                let z = ((r << 1) ^ (r >> 15)) as u16;
                out[k] = z as u8;
                out[half + k] = (z >> 8) as u8;
            }
        }
        out
    }

    #[test]
    fn planar_filter_round_trips() {
        let mut q: Vec<u16> = (0..TILE * TILE).map(|k| (30000 + (k % 977) * 7 + (k / TILE) * 3) as u16).collect();
        for k in (0..q.len()).step_by(97) {
            q[k] = NODATA_U16;
        }
        q[0] = 0;
        q[1] = 65534;
        assert_eq!(unfilter_planar(&filter_planar(&q)), q);
    }

    #[test]
    fn catmull_weights_partition_and_derivatives() {
        let (w0, _, _) = catmull(0.0);
        assert_eq!(w0, [0.0, 1.0, 0.0, 0.0]);
        for k in 0..20 {
            let t = k as f64 / 20.0;
            let (w, d, dd) = catmull(t);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            assert!(d.iter().sum::<f64>().abs() < 1e-12);
            assert!(dd.iter().sum::<f64>().abs() < 1e-12);
            let h = 1e-6;
            let (wp, dp, _) = catmull(t + h);
            let (wm, dm, _) = catmull(t - h);
            for i in 0..4 {
                assert!(((wp[i] - wm[i]) / (2.0 * h) - d[i]).abs() < 1e-6);
                assert!(((dp[i] - dm[i]) / (2.0 * h) - dd[i]).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn split_handles_negative_nodes() {
        assert_eq!(split(0), (0, 0));
        assert_eq!(split(255), (1, 0));
        assert_eq!(split(-1), (-1, 254));
    }
}
