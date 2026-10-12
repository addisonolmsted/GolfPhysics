//! Opening a bundle: the manifest, the physics subset's files, and their blake3 check (D13: physics subset only).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::CourseError;

/// C0 document versions this reader understands
pub const C0_VERSIONS: &[u32] = &[4];
/// on-disk bundle layouts this reader understands
pub const BUNDLE_FORMATS: &[u32] = &[1];

#[derive(Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub bytes: u64,
    pub blake3: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Subset {
    pub files: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Subsets {
    pub physics: Subset,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrevailingWind {
    /// downwind azimuth (the direction the wind blows towards), from +x towards +y
    pub azimuth_rad: f64,
    pub speed_mps: Option<f64>,
    pub variability_seed: u64,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Rect {
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Manifest {
    pub c0_version: u32,
    pub bundle_format: u32,
    pub course_id: String,
    pub seed: u64,
    pub archetype: String,
    pub par: u32,
    pub n_holes: u32,
    pub bounds: Rect,
    pub prevailing_wind: PrevailingWind,
    pub files: Vec<FileEntry>,
    pub layers: Vec<Value>,
    pub subsets: Subsets,
}

impl Manifest {
    pub fn layer(&self, name: &str) -> Option<&Value> {
        self.layers.iter().find(|l| l["name"].as_str() == Some(name))
    }
}

/// The manifest plus the bytes of every physics-subset file, keyed by bundle path.
pub struct Files {
    pub manifest: Manifest,
    files: BTreeMap<String, Vec<u8>>,
}

impl Files {
    /// Read the manifest and only the files `subsets.physics` lists. With `verify`, check each file's size and
    /// blake3 against the manifest.
    pub fn open(dir: &Path, verify: bool) -> Result<Files, CourseError> {
        let mpath = dir.join("manifest.json");
        let raw = std::fs::read(&mpath).map_err(|err| CourseError::Io { path: "manifest.json".into(), err })?;
        let manifest: Manifest =
            serde_json::from_slice(&raw).map_err(|e| CourseError::Json { path: "manifest.json".into(), err: e.to_string() })?;
        if !C0_VERSIONS.contains(&manifest.c0_version) {
            return Err(CourseError::Version { what: "c0_version".into(), found: manifest.c0_version.to_string() });
        }
        if !BUNDLE_FORMATS.contains(&manifest.bundle_format) {
            return Err(CourseError::Version { what: "bundle_format".into(), found: manifest.bundle_format.to_string() });
        }
        let index: BTreeMap<&str, &FileEntry> = manifest.files.iter().map(|f| (f.path.as_str(), f)).collect();
        let mut files = BTreeMap::new();
        for p in &manifest.subsets.physics.files {
            let entry = index.get(p.as_str()).ok_or_else(|| CourseError::Missing { what: format!("{p} in manifest files[]") })?;
            let b = std::fs::read(dir.join(p)).map_err(|err| CourseError::Io { path: p.clone(), err })?;
            if verify {
                if b.len() as u64 != entry.bytes {
                    return Err(CourseError::Integrity { path: p.clone(), what: format!("{} bytes, manifest says {}", b.len(), entry.bytes) });
                }
                let h = blake3::hash(&b).to_hex();
                if h.as_str() != entry.blake3 {
                    return Err(CourseError::Integrity { path: p.clone(), what: format!("blake3 {h} != manifest {}", entry.blake3) });
                }
            }
            files.insert(p.clone(), b);
        }
        Ok(Files { manifest, files })
    }

    pub fn get(&self, p: &str) -> Result<&[u8], CourseError> {
        self.files.get(p).map(|v| v.as_slice()).ok_or_else(|| CourseError::Missing { what: p.to_string() })
    }

    pub fn json<T: for<'de> Deserialize<'de>>(&self, p: &str) -> Result<T, CourseError> {
        serde_json::from_slice(self.get(p)?).map_err(|e| CourseError::Json { path: p.to_string(), err: e.to_string() })
    }

    pub fn total_bytes(&self) -> usize {
        self.files.values().map(|v| v.len()).sum()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// a JSON number field, or a Missing error naming it
pub(crate) fn num(v: &Value, what: &str) -> Result<f64, CourseError> {
    v.as_f64().ok_or_else(|| CourseError::Missing { what: what.to_string() })
}

pub(crate) fn int(v: &Value, what: &str) -> Result<i64, CourseError> {
    v.as_i64().ok_or_else(|| CourseError::Missing { what: what.to_string() })
}

pub(crate) fn text<'a>(v: &'a Value, what: &str) -> Result<&'a str, CourseError> {
    v.as_str().ok_or_else(|| CourseError::Missing { what: what.to_string() })
}
