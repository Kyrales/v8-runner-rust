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
