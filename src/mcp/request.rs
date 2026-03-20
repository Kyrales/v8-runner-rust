/// MCP request for `build_project`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpBuildProjectRequest {
    /// Optional full-rebuild flag from the MCP tool surface.
    pub full_rebuild: Option<bool>,
}

/// MCP request for `run_all_tests`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpRunAllTestsRequest {
    /// Optional full-report flag from the MCP tool surface.
    pub full: Option<bool>,
}

/// MCP request for `run_module_tests`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRunModuleTestsRequest {
    /// Module name to execute.
    pub module_name: String,
    /// Optional full-report flag from the MCP tool surface.
    pub full: Option<bool>,
}

/// MCP request for `dump_config`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpDumpConfigRequest {
    /// Optional raw dump mode. Null/blank defaults to `INCREMENTAL` in service mappers.
    pub mode: Option<String>,
    /// Optional extension name.
    pub extension: Option<String>,
    /// Requested object list for partial dump.
    pub objects: Vec<String>,
}

/// MCP request for `launch_app`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpLaunchAppRequest {
    /// Raw utility alias from the MCP tool surface.
    pub utility_type: String,
}

/// MCP request for `check_syntax_edt`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpCheckSyntaxEdtRequest {
    /// Optional project name; when absent, all EDT projects are checked.
    pub project_name: Option<String>,
}

/// MCP request for `check_syntax_designer_config`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpCheckSyntaxDesignerConfigRequest {
    pub config_log_integrity: Option<bool>,
    pub incorrect_references: Option<bool>,
    pub thin_client: Option<bool>,
    pub web_client: Option<bool>,
    pub mobile_client: Option<bool>,
    pub server: Option<bool>,
    pub external_connection: Option<bool>,
    pub external_connection_server: Option<bool>,
    pub mobile_app_client: Option<bool>,
    pub mobile_app_server: Option<bool>,
    pub thick_client_managed_application: Option<bool>,
    pub thick_client_server_managed_application: Option<bool>,
    pub thick_client_ordinary_application: Option<bool>,
    pub thick_client_server_ordinary_application: Option<bool>,
    pub mobile_client_digi_sign: Option<bool>,
    pub distributive_modules: Option<bool>,
    pub unreference_procedures: Option<bool>,
    pub handlers_existence: Option<bool>,
    pub empty_handlers: Option<bool>,
    pub extended_modules_check: Option<bool>,
    pub check_use_synchronous_calls: Option<bool>,
    pub check_use_modality: Option<bool>,
    pub unsupported_functional: Option<bool>,
    /// Optional extension selector. Blank values are treated as absent.
    pub extension: Option<String>,
    /// Optional tri-state all-extensions flag. When omitted, service mappers infer the default
    /// from `extension`.
    pub all_extensions: Option<bool>,
}

/// MCP request for `check_syntax_designer_modules`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpCheckSyntaxDesignerModulesRequest {
    pub thin_client: Option<bool>,
    pub web_client: Option<bool>,
    pub server: Option<bool>,
    pub external_connection: Option<bool>,
    pub thick_client_ordinary_application: Option<bool>,
    pub mobile_app_client: Option<bool>,
    pub mobile_app_server: Option<bool>,
    pub mobile_client: Option<bool>,
    pub extended_modules_check: Option<bool>,
    /// Optional extension selector. Blank values are treated as absent.
    pub extension: Option<String>,
    /// Optional tri-state all-extensions flag. When omitted, service mappers infer the default
    /// from `extension`.
    pub all_extensions: Option<bool>,
}
