use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

/// Error converting a `SystemTime` to nanoseconds since UNIX epoch.
#[derive(Debug, Error)]
#[error("cannot convert mtime to nanoseconds for '{path}': {reason}")]
pub struct MtimeError {
    pub path: PathBuf,
    pub reason: &'static str,
}

/// Recorded state of a single file at scan time.
#[derive(Debug, Clone)]
pub struct FileState {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Last-modified timestamp as nanoseconds since UNIX epoch.
    pub mtime_ns: u64,
    /// SHA-256 lowercase hex digest of file contents.
    pub hash: String,
}

impl FileState {
    /// Construct a captured file state from absolute path, mtime, and content hash.
    pub fn new(path: PathBuf, mtime_ns: u64, hash: String) -> Self {
        Self {
            path,
            mtime_ns,
            hash,
        }
    }
}

/// Convert a `SystemTime` to nanoseconds since UNIX epoch.
///
/// Returns `Err` for pre-epoch times or values that overflow `u64`.
pub fn mtime_nanos(t: SystemTime, path: &Path) -> Result<u64, MtimeError> {
    let dur = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| MtimeError {
            path: path.to_path_buf(),
            reason: "pre-epoch mtime",
        })?;
    dur.as_nanos().try_into().map_err(|_| MtimeError {
        path: path.to_path_buf(),
        reason: "mtime nanoseconds overflow u64",
    })
}
