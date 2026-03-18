use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSetContext {
    pub name: String,
    pub path: std::path::PathBuf,
    pub hash_storage_key: String,
}
