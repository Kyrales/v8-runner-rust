use tracing::info;

/// Emits a text-mode live progress timeline event before a blocking stage starts.
///
/// Callers must pass fixed, sanitized vocabulary only: no rendered commands, raw
/// arguments, stdout/stderr, environment values, connection strings, or secrets.
pub(crate) fn log_live_stage(label: &str, detail: &str) {
    info!(
        target: "v8_runner::live_progress",
        timeline_status = "running",
        timeline_label = label,
        timeline_detail = detail
    );
}
