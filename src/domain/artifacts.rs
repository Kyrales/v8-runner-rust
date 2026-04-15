use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::artifact::ArtifactSet;
use crate::domain::execution::ExecutionOutcome;

pub const CF_RUNNER_ID: &str = "designer-cf";
pub const CFE_RUNNER_ID: &str = "designer-cfe";
pub const EPF_RUNNER_ID: &str = "designer-epf";
pub const ERF_RUNNER_ID: &str = "designer-erf";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBuildMode {
    Unknown,
    ConfigurationCf,
    ExtensionCfe,
    ExternalDataProcessorEpf,
    ExternalReportErf,
}

impl ArtifactBuildMode {
    pub const fn file_extension(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::ConfigurationCf => "cf",
            Self::ExtensionCfe => "cfe",
            Self::ExternalDataProcessorEpf => "epf",
            Self::ExternalReportErf => "erf",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactBuildMetadata {
    pub artifact_type: ArtifactBuildMode,
    pub output_path: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_names: Vec<String>,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactsResult {
    pub ok: bool,
    pub mode: ArtifactBuildMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    pub output_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_log_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "ArtifactSet::is_empty")]
    pub artifacts: ArtifactSet,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub execution: ExecutionOutcome<ArtifactBuildMetadata>,
}

impl ArtifactSet {
    pub fn is_empty(&self) -> bool {
        self.root_dir.is_none() && self.items.is_empty()
    }
}
