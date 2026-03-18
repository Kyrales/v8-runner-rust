use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Runtime context for one logical source-set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSetContext {
    /// Logical name (matches `SourceSetConfig.name`).
    pub name: String,
    /// Absolute root directory of the sources.
    pub path: PathBuf,
    /// Key used to name the JSON hash-storage file (`workPath/hash-storages/<key>.json`).
    pub storage_key: String,
}

impl SourceSetContext {
    pub fn new(name: impl Into<String>, path: PathBuf, storage_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path,
            storage_key: storage_key.into(),
        }
    }

    /// Absolute path to the JSON hash-storage file for this context.
    pub fn storage_path(&self, work_path: &std::path::Path) -> PathBuf {
        work_path
            .join("hash-storages")
            .join(format!("{}.json", self.storage_key))
    }
}
