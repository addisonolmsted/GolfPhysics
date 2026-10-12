//! `instances.bin` (C0 format `c0in` v4): every tree, shrub, grass clump and snag, with position, asset, scale,
//! crown form, lean and the in-play flag. No z: the trunk stands on the height function.
//!
//! Collision proxies need each asset's size table, which lives in the asset library (not built yet). Until then
//! physics uses generic models per kind (D20, M6); this module only decodes the records.

use golf_core::math;

use crate::bundle::Files;
use crate::error::{CourseError, format};
use crate::tiles;

const MAGIC: &[u8; 4] = b"C0IN";
const VERSION: u16 = 4;
const HEADER_BYTES: usize = 32;
const TILE_DIR_BYTES: usize = 28;
const REC_BYTES: usize = 8;
const Q_PER_M: f64 = 256.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Tree,
    Shrub,
    Clump,
    Snag,
    Other(u8),
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub kind: AssetKind,
    /// the asset library id, e.g. `tree_pine_longleaf_v1`
    pub id: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub x: f64,
    pub y: f64,
    /// index into `Instances::palette`
    pub asset: u8,
    /// within 30 m of play or a walk: gets full collision proxies (C0)
    pub in_play: bool,
    /// lean class k (0 = upright); the trunk tilts 1.5 k degrees towards `yaw_rad`
    pub lean: u8,
    /// crown form f: radius factor FORM_K[f] = [0.8, 1.0, 1.25, 1.55]
    pub crown_form: u8,
    pub mesh: u8,
    /// about +z from +x towards +y
    pub yaw_rad: f64,
    /// uniform scale s = 2^((q - 128) / 64): height = h0 s
    pub scale: f64,
}

pub const FORM_K: [f64; 4] = [0.8, 1.0, 1.25, 1.55];

pub struct Instances {
    pub palette: Vec<Asset>,
    pub records: Vec<Instance>,
}

fn rd16(b: &[u8], at: usize) -> Result<u16, CourseError> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| format("instances.bin", "truncated"))
}

fn rd32(b: &[u8], at: usize) -> Result<u32, CourseError> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| format("instances.bin", "truncated"))
}

impl Instances {
    pub fn load(f: &Files) -> Result<Instances, CourseError> {
        Instances::decode(f.get("instances.bin")?)
    }

    pub fn decode(b: &[u8]) -> Result<Instances, CourseError> {
        let p = "instances.bin";
        if b.len() < HEADER_BYTES || &b[0..4] != MAGIC {
            return Err(format(p, "not a C0 instances file"));
        }
        let version = rd16(b, 4)?;
        if version != VERSION {
            return Err(CourseError::Version { what: "instances.bin version".into(), found: version.to_string() });
        }
        let flags = rd16(b, 6)?;
        let tile_m = rd16(b, 8)? as f64;
        if rd16(b, 10)? as usize != REC_BYTES {
            return Err(format(p, "unexpected record size"));
        }
        let n_assets = rd16(b, 12)? as usize;
        let n_tiles = rd16(b, 14)? as usize;
        let n_records = rd32(b, 16)? as usize;

        let mut at = HEADER_BYTES;
        let mut palette = Vec::with_capacity(n_assets);
        for _ in 0..n_assets {
            let kind = *b.get(at).ok_or_else(|| format(p, "truncated palette"))?;
            let l = *b.get(at + 1).ok_or_else(|| format(p, "truncated palette"))? as usize;
            let id = std::str::from_utf8(b.get(at + 2..at + 2 + l).ok_or_else(|| format(p, "truncated palette"))?)
                .map_err(|e| format(p, e.to_string()))?
                .to_string();
            let kind = match kind {
                0 => AssetKind::Tree,
                1 => AssetKind::Shrub,
                2 => AssetKind::Clump,
                3 => AssetKind::Snag,
                k => AssetKind::Other(k),
            };
            palette.push(Asset { kind, id });
            at += 2 + l;
        }

        let mut records = Vec::with_capacity(n_records);
        for k in 0..n_tiles {
            let d = at + k * TILE_DIR_BYTES;
            let (tx, ty) = (rd16(b, d)? as f64, rd16(b, d + 2)? as f64);
            let n = rd32(b, d + 4)? as usize;
            let (offset, clen, raw_len) = (rd32(b, d + 12)? as usize, rd32(b, d + 16)? as usize, rd32(b, d + 20)? as usize);
            let blk = tiles::slice(p, b, offset, clen)?;
            let raw = if flags & 1 != 0 { tiles::unzstd(p, blk, raw_len)? } else { blk.to_vec() };
            if raw.len() != n * REC_BYTES {
                return Err(format(p, "block length"));
            }
            for r in raw.chunks_exact(REC_BYTES) {
                let (qx, qy) = (u16::from_le_bytes([r[0], r[1]]), u16::from_le_bytes([r[2], r[3]]));
                let v = r[5];
                records.push(Instance {
                    x: tx * tile_m + qx as f64 / Q_PER_M,
                    y: ty * tile_m + qy as f64 / Q_PER_M,
                    asset: r[4],
                    in_play: v & 0x80 != 0,
                    lean: (v >> 4) & 0x07,
                    crown_form: (v >> 2) & 0x03,
                    mesh: v & 0x03,
                    yaw_rad: std::f64::consts::TAU * r[6] as f64 / 256.0,
                    scale: math::pow(2.0, (r[7] as f64 - 128.0) / 64.0),
                });
            }
        }
        if records.len() != n_records {
            return Err(format(p, "record count"));
        }
        if records.iter().any(|r| r.asset as usize >= palette.len()) {
            return Err(format(p, "asset index past the palette"));
        }
        Ok(Instances { palette, records })
    }

    pub fn n_in_play(&self) -> usize {
        self.records.iter().filter(|r| r.in_play).count()
    }
}
