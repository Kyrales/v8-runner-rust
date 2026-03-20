use std::time::Duration;

use crate::config::model::AppConfig;
use crate::domain::launch::{LaunchMode, LaunchResult};
use crate::platform::locator::UtilityType;
use crate::platform::process::ProcessRequest;
use crate::platform::utilities::PlatformUtilities;
use crate::support::error::AppError;
use crate::use_cases::context::ExecutionContext;
use crate::use_cases::request::{LaunchModeRequest, LaunchRequest as LaunchArgs};
use crate::use_cases::result::{UseCaseFailure, UseCaseResult};
use tracing::info;

const LAUNCH_STARTUP_PROBE: Duration = Duration::from_millis(250);

pub fn execute(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &LaunchArgs,
) -> UseCaseResult<LaunchResult> {
    info!(
        command = context.command().as_str(),
        transport = ?context.transport(),
        mode = ?args.mode,
        "executing launch use case"
    );
    let (mode, utility, command_mode) = match args.mode {
        LaunchModeRequest::Designer => (LaunchMode::Designer, UtilityType::V8, "DESIGNER"),
        LaunchModeRequest::Thin => (LaunchMode::Thin, UtilityType::V8C, "ENTERPRISE"),
        LaunchModeRequest::Thick => (LaunchMode::Thick, UtilityType::V8, "ENTERPRISE"),
    };

    let mut utilities = PlatformUtilities::from_config(config);
    let location = utilities
        .locate(utility)
        .map_err(|e| UseCaseFailure::without_payload(AppError::Platform(e.to_string())))?;

    let mut process_args = vec![command_mode.to_owned()];
    process_args.extend(config.v8_connection().args());

    let spawned = utilities
        .runner_for(utility)
        .spawn(&ProcessRequest {
            program: location.path.clone(),
            args: process_args,
            workdir: None,
            stdout_log_path: None,
            stderr_log_path: None,
            startup_probe: Some(LAUNCH_STARTUP_PROBE),
        })
        .map_err(|e| UseCaseFailure::without_payload(AppError::Platform(e.to_string())))?;

    let result = LaunchResult {
        ok: true,
        mode,
        pid: Some(spawned.pid),
        binary: spawned.binary.clone(),
        message: Some(format!(
            "Launched {} via {} (pid {})",
            mode_label(args.mode),
            spawned.binary.display(),
            spawned.pid
        )),
    };
    Ok(result)
}

fn mode_label(mode: LaunchModeRequest) -> &'static str {
    match mode {
        LaunchModeRequest::Designer => "designer",
        LaunchModeRequest::Thin => "thin",
        LaunchModeRequest::Thick => "thick",
    }
}
