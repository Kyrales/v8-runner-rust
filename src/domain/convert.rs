use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConvertDirection {
    EdtToDesigner,
    DesignerToEdt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConvertScope {
    All,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertOutput {
    pub source_set: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResult {
    pub ok: bool,
    pub direction: ConvertDirection,
    pub scope: ConvertScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_set: Option<String>,
    pub workspace_path: PathBuf,
    pub outputs: Vec<ConvertOutput>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
