use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpResult {
    pub ok: bool,
    pub mode: DumpMode,
    pub target_path: std::path::PathBuf,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DumpMode {
    Full,
    Incremental,
    Partial,
}
