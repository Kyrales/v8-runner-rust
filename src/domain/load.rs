use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::artifacts::ArtifactBuildMode;
use crate::domain::execution::ExecutionOutcome;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadMode {
    Load,
    Merge,
    Update,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadTargetKind {
    Unknown,
    Configuration,
    Extension,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityState {
    Supported,
    NotSupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadExecutionMetadata {
    pub applied: bool,
    pub target_kind: LoadTargetKind,
    pub compatibility_state: CompatibilityState,
    pub update_db_cfg_ran: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    pub ok: bool,
    pub mode: LoadMode,
    pub artifact_path: PathBuf,
    pub artifact_type: ArtifactBuildMode,
    pub target_kind: LoadTargetKind,
    pub compatibility_state: CompatibilityState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_log_path: Option<PathBuf>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub execution: ExecutionOutcome<LoadExecutionMetadata>,
}
