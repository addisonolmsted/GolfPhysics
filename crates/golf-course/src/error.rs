//! The loader's one error type. Queries never fail; loading either gives a whole course or an error.

use std::fmt;

#[derive(Debug)]
pub enum CourseError {
    /// a file could not be read
    Io { path: String, err: std::io::Error },
    /// a file's size or blake3 differs from the manifest
    Integrity { path: String, what: String },
    /// a JSON layer did not parse into what P0 expects
    Json { path: String, err: String },
    /// a binary layer is malformed (bad magic, truncated, bad zstd frame, wrong length)
    Format { path: String, what: String },
    /// a version this reader does not know
    Version { what: String, found: String },
    /// something P0 requires is not in the bundle
    Missing { what: String },
}

impl fmt::Display for CourseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CourseError::Io { path, err } => write!(f, "{path}: {err}"),
            CourseError::Integrity { path, what } => write!(f, "{path}: {what}"),
            CourseError::Json { path, err } => write!(f, "{path}: {err}"),
            CourseError::Format { path, what } => write!(f, "{path}: {what}"),
            CourseError::Version { what, found } => write!(f, "unsupported {what}: {found}"),
            CourseError::Missing { what } => write!(f, "missing: {what}"),
        }
    }
}

impl std::error::Error for CourseError {}

pub(crate) fn format(path: &str, what: impl Into<String>) -> CourseError {
    CourseError::Format { path: path.to_string(), what: what.into() }
}
