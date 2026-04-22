use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable MCP-facing build mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpBuildMode {
    EdtExport,
    Full,
    Partial { file_count: usize },
    Skipped,
}

/// MCP build step result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpBuildStep {
    pub source_set: String,
    pub mode: McpBuildMode,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// MCP execution step result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpStepStatus {
    Succeeded,
    Failed,
    Skipped,
    Degraded,
}

/// MCP execution step kind.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpStepKind {
    Validation,
    ResolveTarget,
    PrepareWorkspace,
    PlatformCommand,
    ParseOutput,
    Publish,
    Cleanup,
    Diagnostics,
    Other,
}

/// MCP execution step result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpStepResult {
    pub name: String,
    pub ok: bool,
    pub status: McpStepStatus,
    pub kind: McpStepKind,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
}

/// Stable MCP-facing test status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpTestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// MCP test case.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpTestCase {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    pub status: McpTestStatus,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

/// MCP test suite.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpTestSuite {
    pub name: String,
    pub cases: Vec<McpTestCase>,
    pub duration_ms: u64,
}

/// Stable MCP-facing issue severity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpIssueSeverity {
    Error,
    Warning,
    Info,
}

/// MCP module issue.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpModuleIssue {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub severity: McpIssueSeverity,
}

/// MCP object issue.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpObjectIssue {
    pub object: String,
    pub message: String,
    pub severity: McpIssueSeverity,
}

/// MCP EDT issue.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpEdtIssue {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub severity: McpIssueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
}

/// MCP issue payload.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpIssue {
    Module(McpModuleIssue),
    Object(McpObjectIssue),
    Edt(McpEdtIssue),
}

/// MCP response for `build_project`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpBuildResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<McpBuildStep>>,
}

/// MCP response for `run_all_tests` and `run_module_tests`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpTestResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tests: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passed_tests: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_tests: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_detail: Option<Vec<McpTestSuite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<McpStepResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

/// MCP response for `dump_config`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpDumpResponse {
    pub success: bool,
    pub message: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dump_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dumped_objects: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<McpStepResult>>,
}

/// MCP response for `launch_app`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpLaunchResponse {
    pub success: bool,
    pub message: String,
}

/// MCP response for syntax-check tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct McpSyntaxCheckResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<McpIssue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}
