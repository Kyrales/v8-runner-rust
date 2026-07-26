use super::*;
use crate::use_cases::progress::log_live_stage;

pub(super) fn run_tests(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &TestArgs,
) -> UseCaseResult<TestRunResult> {
    let started = Instant::now();
    let runner_kind = args.execution.profile.kind.clone();
    debug!(
        full = args.full,
        scope = ?args.scope,
        runner = ?runner_kind,
        "starting test run"
    );
    let mode = if args.full {
        TestOutputMode::Full
    } else {
        TestOutputMode::Compact
    };
    let export_prepare_started = Instant::now();
    let junit_export = match args.junit_output.clone() {
        Some(path) => match JunitExport::prepare(path.clone()) {
            Ok(export) => Some(export),
            Err(error) => {
                let message = error.to_string();
                let export_error = test_execution_error(TestErrorKind::JunitExportFailed, &message);
                let step = failed_step(
                    "export_junit",
                    ExecutionStepKind::Publish,
                    export_prepare_started.elapsed().as_millis() as u64,
                    &message,
                )
                .with_target(path.display().to_string())
                .with_errors(vec![export_error.clone()]);
                let outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                    .with_diagnostics(vec![message])
                    .with_errors(vec![export_error]);
                let result = make_test_result(
                    TestTarget::All,
                    mode,
                    outcome,
                    Vec::new(),
                    vec![step],
                    started.elapsed().as_millis() as u64,
                );
                return Err(TestExecutionFailure::with_payload(error, result));
            }
        },
        None => None,
    };
    let target = match validate_target(&runner_kind, &args.scope) {
        Ok(target) => target,
        Err(error) => {
            let outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                .with_diagnostics(vec![error.to_string()]);
            let result = make_test_result(
                TestTarget::All,
                mode,
                outcome,
                Vec::new(),
                Vec::new(),
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(error, result));
        }
    };

    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    if let Some(failure) =
        interrupted_test_failure(context, &target, &mode, &warnings, &steps, started)
    {
        return Err(failure);
    }
    let runner_id = match validate_runner_profile_id(&args.execution.profile.id) {
        Ok(runner_id) => runner_id,
        Err(error) => {
            let outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                .with_diagnostics(vec![error.to_string()])
                .with_errors(vec![test_execution_error(
                    TestErrorKind::TestSetupFailed,
                    error.to_string(),
                )]);
            let result = make_test_result(
                target,
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(error, result));
        }
    };

    debug!("running build prerequisite for tests");
    log_live_stage(
        "test: build prerequisite",
        "[Build] preparing test infobase",
    );
    let build_started = Instant::now();
    let build_result = match build_project::execute(
        context,
        config,
        &BuildArgs {
            full_rebuild: false,
            source_set: None,
        },
    ) {
        Ok(result) => result,
        Err(failure) => {
            let summary = failure
                .payload
                .as_ref()
                .map(build_summary)
                .unwrap_or_else(|| failure.error.to_string());
            steps.push(
                failed_step(
                    "build",
                    ExecutionStepKind::PlatformCommand,
                    build_started.elapsed().as_millis() as u64,
                    summary.clone(),
                )
                .with_errors(vec![test_execution_error(
                    TestErrorKind::BuildFailed,
                    summary.clone(),
                )]),
            );
            let outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                .with_diagnostics(vec![summary.clone()])
                .with_errors(vec![test_execution_error(
                    TestErrorKind::BuildFailed,
                    summary.clone(),
                )]);
            let result = make_test_result(
                target,
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(failure.error, result));
        }
    };
    steps.push(succeeded_step(
        "build",
        ExecutionStepKind::PlatformCommand,
        build_started.elapsed().as_millis() as u64,
        build_summary(&build_result),
    ));

    debug!("preparing test run artifacts");
    let prepare_artifacts_started = Instant::now();
    let mut artifacts = match create_run_artifacts(config, &runner_id) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            let app_error =
                AppError::Runtime(format!("failed to prepare test run directory: {error}"));
            steps.push(
                failed_step(
                    "prepare_artifacts",
                    ExecutionStepKind::PrepareWorkspace,
                    prepare_artifacts_started.elapsed().as_millis() as u64,
                    app_error.to_string(),
                )
                .with_errors(vec![test_execution_error(
                    TestErrorKind::TestSetupFailed,
                    app_error.to_string(),
                )]),
            );
            let outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                .with_diagnostics(vec![app_error.to_string()])
                .with_errors(vec![test_execution_error(
                    TestErrorKind::TestSetupFailed,
                    app_error.to_string(),
                )]);
            let result = make_test_result(
                target,
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(app_error, result));
        }
    };
    steps.push(
        succeeded_step(
            "prepare_artifacts",
            ExecutionStepKind::PrepareWorkspace,
            prepare_artifacts_started.elapsed().as_millis() as u64,
            format!("created {}", artifacts.run_dir.display()),
        )
        .with_target(artifacts.run_dir.display().to_string()),
    );

    let prepare_runner_started = Instant::now();
    if let Some(failure) =
        interrupted_test_failure(context, &target, &mode, &warnings, &steps, started)
    {
        return Err(failure);
    }
    let prepared_run = match prepare_runner_artifacts(config, args, &target, &mut artifacts) {
        Ok(prepared_run) => {
            steps.push(
                succeeded_step(
                    "prepare_runner",
                    ExecutionStepKind::PrepareWorkspace,
                    prepare_runner_started.elapsed().as_millis() as u64,
                    prepared_run_summary(&prepared_run),
                )
                .with_target(artifacts.config_json.display().to_string()),
            );
            prepared_run
        }
        Err(error) => {
            steps.push(
                failed_step(
                    "prepare_runner",
                    ExecutionStepKind::PrepareWorkspace,
                    prepare_runner_started.elapsed().as_millis() as u64,
                    error.to_string(),
                )
                .with_target(artifacts.config_json.display().to_string())
                .with_errors(vec![test_execution_error(
                    TestErrorKind::TestSetupFailed,
                    error.to_string(),
                )]),
            );
            let retained_paths = retain_run_artifacts(config, &artifacts).ok();
            let outcome = with_retained_artifacts(
                ExecutionOutcome::new(ExecutionStatus::Failed)
                    .with_diagnostics(vec![error.to_string()])
                    .with_errors(vec![test_execution_error(
                        TestErrorKind::TestSetupFailed,
                        error.to_string(),
                    )]),
                retained_paths,
            );
            let result = make_test_result(
                target.clone(),
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(error, result));
        }
    };

    debug!(path = %artifacts.run_dir.display(), "launching enterprise test run");
    log_live_stage("test: enterprise run", "[Enterprise] running test runner");
    let run_started = Instant::now();
    let enterprise_runner = crate::platform::process::ProcessExecutor;
    let platform_launch = build_platform_launch(&args.execution.launch, &prepared_run, &artifacts);
    let enterprise = match build_enterprise_dsl(
        context,
        config,
        &artifacts,
        &prepared_run,
        &platform_launch,
        &enterprise_runner,
        args.execution
            .client_mode
            .unwrap_or(LaunchClientModeRequest::Thin),
        capped_timeout_ms(args.execution.timeouts.total_ms, context),
    ) {
        Ok(dsl) => dsl,
        Err(error) => {
            steps.push(
                failed_step(
                    "run",
                    ExecutionStepKind::PlatformCommand,
                    run_started.elapsed().as_millis() as u64,
                    error.to_string(),
                )
                .with_target(artifacts.platform_log.display().to_string())
                .with_errors(vec![test_execution_error(
                    TestErrorKind::TestSetupFailed,
                    error.to_string(),
                )]),
            );
            let retained_paths = retain_run_artifacts(config, &artifacts).ok();
            let outcome = with_retained_artifacts(
                ExecutionOutcome::new(ExecutionStatus::Failed)
                    .with_diagnostics(vec![error.to_string()])
                    .with_errors(vec![test_execution_error(
                        TestErrorKind::TestSetupFailed,
                        error.to_string(),
                    )]),
                retained_paths,
            );
            let result = make_test_result(
                target,
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(error, result));
        }
    };

    if let Some(failure) =
        interrupted_test_failure(context, &target, &mode, &warnings, &steps, started)
    {
        return Err(failure);
    }
    let enterprise_completion = match enterprise.run_launch(&platform_launch) {
        Ok(result) => {
            steps.push(
                if result.process.exit_code == 0 {
                    succeeded_step(
                        "run",
                        ExecutionStepKind::PlatformCommand,
                        run_started.elapsed().as_millis() as u64,
                        format!("enterprise exit code {}", result.process.exit_code),
                    )
                } else {
                    failed_step(
                        "run",
                        ExecutionStepKind::PlatformCommand,
                        run_started.elapsed().as_millis() as u64,
                        format!("enterprise exit code {}", result.process.exit_code),
                    )
                }
                .with_target(artifacts.platform_log.display().to_string()),
            );
            EnterpriseCompletion::Completed(result)
        }
        Err(error) => {
            let (kind, app_error, interruption, status) = enterprise_error_kind(error);
            let mut step = failed_step(
                "run",
                ExecutionStepKind::PlatformCommand,
                run_started.elapsed().as_millis() as u64,
                app_error.to_string(),
            )
            .with_target(artifacts.platform_log.display().to_string());
            if let Some(kind) = kind.clone() {
                step = step.with_errors(vec![test_execution_error(kind, app_error.to_string())]);
            }
            steps.push(step);
            if junit_export.is_none() {
                let retained_paths = retain_run_artifacts(config, &artifacts).ok();
                let mut outcome =
                    ExecutionOutcome::new(status).with_diagnostics(vec![app_error.to_string()]);
                if let Some(kind) = kind.clone() {
                    outcome = outcome
                        .with_errors(vec![test_execution_error(kind, app_error.to_string())]);
                }
                if let Some(interruption) = interruption.clone() {
                    outcome = outcome.with_interruptions(vec![interruption]);
                }
                let outcome = with_retained_artifacts(outcome, retained_paths);
                let result = make_test_result(
                    target,
                    mode,
                    outcome,
                    warnings,
                    steps,
                    started.elapsed().as_millis() as u64,
                );
                return Err(TestExecutionFailure::with_payload(app_error, result));
            }
            EnterpriseCompletion::Failed {
                kind,
                error: app_error,
                interruption,
                status,
            }
        }
    };

    if matches!(prepared_run, PreparedRun::Vanessa { .. }) {
        resolve_vanessa_junit_path(&mut artifacts);
        if let Err(warning) = materialize_vanessa_runner_log(&artifacts) {
            warnings.push(warning);
        }
    }

    debug!(path = %artifacts.junit_xml.display(), "parsing JUnit report");
    let parse_junit_started = Instant::now();
    let junit_parse = parse_junit_report(&artifacts);
    let validated_junit = match junit_parse.payload {
        Some(validated) => {
            steps.push(
                succeeded_step(
                    "parse_junit",
                    ExecutionStepKind::ParseOutput,
                    parse_junit_started.elapsed().as_millis() as u64,
                    format!("parsed {} test cases", validated.report.summary.total),
                )
                .with_target(artifacts.junit_xml.display().to_string()),
            );
            validated
        }
        None => {
            let error = junit_parse
                .errors
                .first()
                .cloned()
                .expect("junit parse error");
            let kind =
                TestErrorKind::from_code(&error.code).unwrap_or(TestErrorKind::JunitMalformed);
            let message = error.message.clone();
            steps.push(
                failed_step(
                    "parse_junit",
                    ExecutionStepKind::ParseOutput,
                    parse_junit_started.elapsed().as_millis() as u64,
                    message.clone(),
                )
                .with_target(artifacts.junit_xml.display().to_string())
                .with_errors(vec![error
                    .clone()
                    .with_details(junit_parse.diagnostics.clone())]),
            );
            let retained_paths = retain_run_artifacts(config, &artifacts).ok();
            let mut diagnostics = enterprise_completion.diagnostics(vec![message.clone()], config);
            let mut errors = vec![error.with_details(junit_parse.diagnostics)];
            if junit_export.is_some()
                || matches!(enterprise_completion, EnterpriseCompletion::Failed { .. })
            {
                enterprise_completion.append_enterprise_error(&mut errors);
                if let Some(exit_code) = enterprise_completion
                    .process_exit_code()
                    .filter(|exit_code| *exit_code != 0)
                {
                    diagnostics.push(format!("enterprise test run exited with code {exit_code}"));
                }
            }
            let mut failed_outcome =
                ExecutionOutcome::new(test_execution_status(Some(kind.clone()), false))
                    .with_diagnostics(diagnostics)
                    .with_errors(errors);
            if let Some(interruption) = enterprise_completion.interruption() {
                failed_outcome = failed_outcome.with_interruptions(vec![interruption.clone()]);
            }
            let outcome = with_retained_artifacts(failed_outcome, retained_paths);
            let result = make_test_result(
                target,
                mode,
                outcome,
                warnings,
                steps,
                started.elapsed().as_millis() as u64,
            );
            return Err(TestExecutionFailure::with_payload(
                AppError::Runtime(message),
                result,
            ));
        }
    };

    let mut report = validated_junit.report;
    let mut deferred_interruptions = Vec::new();
    let mut export_diagnostics = Vec::new();
    let failed_exit_code = enterprise_completion
        .process_exit_code()
        .filter(|exit_code| *exit_code != 0);
    if let Some(export) = junit_export {
        let export_started = Instant::now();
        let export_result = match &enterprise_completion {
            EnterpriseCompletion::Completed(_) => export.publish(context, &validated_junit.bytes),
            EnterpriseCompletion::Failed { .. } => {
                export.publish_after_enterprise_failure(context, &validated_junit.bytes)
            }
        };
        match export_result {
            Ok(outcome) => {
                let path = outcome.path.display().to_string();
                steps.push(
                    succeeded_step(
                        "export_junit",
                        ExecutionStepKind::Publish,
                        export_started.elapsed().as_millis() as u64,
                        format!("JUnit report exported to {path}"),
                    )
                    .with_target(path.clone()),
                );
                export_diagnostics.push(format!("JUnit report exported to {path}"));
                if let Some(warning) = outcome.cleanup_warning {
                    warnings.push(warning);
                }
                if let Some(interruption) = outcome.deferred_interruption {
                    if !enterprise_completion.represents_publication_interruption(interruption) {
                        let message = crate::use_cases::interruption::deferred_interruption_warning(
                            "JUnit report publication completed",
                            interruption,
                        );
                        warnings.push(message.clone());
                        deferred_interruptions.push(
                            crate::use_cases::interruption::deferred_command_interruption_details(
                                interruption,
                                "export_junit",
                                "JUnit report publication completed",
                            ),
                        );
                    }
                }
            }
            Err(error) => {
                let message = error.to_string();
                let export_error = test_execution_error(TestErrorKind::JunitExportFailed, &message);
                steps.push(
                    failed_step(
                        "export_junit",
                        ExecutionStepKind::Publish,
                        export_started.elapsed().as_millis() as u64,
                        &message,
                    )
                    .with_target(export.target().display().to_string())
                    .with_errors(vec![export_error.clone()]),
                );
                let has_test_failures = report.summary.failed > 0 || report.summary.errors > 0;
                let mut errors = vec![export_error];
                enterprise_completion.append_enterprise_error(&mut errors);
                if has_test_failures {
                    errors.push(test_execution_error(
                        TestErrorKind::TestFailures,
                        "test run reported failures",
                    ));
                }
                let mut diagnostics =
                    enterprise_completion.diagnostics(vec![message.clone()], config);
                if has_test_failures {
                    diagnostics.push("test run reported failures".to_owned());
                }
                if let Some(exit_code) = failed_exit_code {
                    diagnostics.push(format!(
                        "enterprise test run exited with code {}",
                        exit_code
                    ));
                }
                let rendered_report = match mode {
                    TestOutputMode::Full => report.clone(),
                    TestOutputMode::Compact => compact_report(&report),
                };
                let retained_paths = retain_run_artifacts(config, &artifacts).ok();
                let outcome = with_retained_artifacts(
                    {
                        let mut export_outcome = ExecutionOutcome::new(ExecutionStatus::Failed)
                            .with_diagnostics(diagnostics)
                            .with_errors(errors)
                            .with_metrics(ExecutionMetrics::from(&report.summary))
                            .with_payload(rendered_report);
                        if let Some(interruption) = enterprise_completion.interruption() {
                            export_outcome =
                                export_outcome.with_interruptions(vec![interruption.clone()]);
                        }
                        export_outcome
                    },
                    retained_paths,
                );
                let result = make_test_result(
                    target,
                    mode,
                    outcome,
                    warnings,
                    steps,
                    started.elapsed().as_millis() as u64,
                );
                return Err(TestExecutionFailure::with_payload(error, result));
            }
        }
    }

    parse_runner_log(
        &prepared_run,
        &artifacts.runner_log,
        &mut report,
        &mut warnings,
        &mut steps,
    );

    let rendered_report = match mode {
        TestOutputMode::Full => report.clone(),
        TestOutputMode::Compact => compact_report(&report),
    };

    let has_test_failures = report.summary.failed > 0 || report.summary.errors > 0;
    let process_failed = failed_exit_code.is_some();
    let diagnostics = enterprise_completion.diagnostics(export_diagnostics, config);
    let deferred_enterprise_error = enterprise_completion.enterprise_error();

    if let EnterpriseCompletion::Failed {
        kind: _,
        error,
        interruption,
        status,
    } = enterprise_completion
    {
        let mut errors = deferred_enterprise_error.into_iter().collect::<Vec<_>>();
        if has_test_failures {
            errors.push(test_execution_error(
                TestErrorKind::TestFailures,
                "test run reported failures",
            ));
        }
        let retained_paths = retain_run_artifacts(config, &artifacts).ok();
        let mut failed_outcome = ExecutionOutcome::new(status)
            .with_diagnostics(diagnostics)
            .with_errors(errors)
            .with_metrics(ExecutionMetrics::from(&report.summary))
            .with_payload(rendered_report);
        let mut interruptions = interruption.into_iter().collect::<Vec<_>>();
        interruptions.extend(deferred_interruptions);
        if !interruptions.is_empty() {
            failed_outcome = failed_outcome.with_interruptions(interruptions);
        }
        let outcome = with_retained_artifacts(failed_outcome, retained_paths);
        let result = make_test_result(
            target,
            mode,
            outcome,
            warnings,
            steps,
            started.elapsed().as_millis() as u64,
        );
        return Err(TestExecutionFailure::with_payload(error, result));
    }

    if process_failed || has_test_failures {
        debug!(
            process_failed,
            has_test_failures, "retaining failed test artifacts"
        );
        let retained_paths = retain_run_artifacts(config, &artifacts).ok();
        let (kind, failure_message) = match failed_exit_code {
            Some(exit_code) => (
                TestErrorKind::EnterpriseExitedNonZero,
                format!("enterprise test run exited with code {exit_code}"),
            ),
            None => (
                TestErrorKind::TestFailures,
                "test run reported failures".to_owned(),
            ),
        };
        let mut failed_outcome =
            ExecutionOutcome::new(test_execution_status(Some(kind.clone()), false))
                .with_diagnostics(diagnostics)
                .with_errors(vec![test_execution_error(kind, failure_message.clone())])
                .with_metrics(ExecutionMetrics::from(&report.summary))
                .with_payload(rendered_report);
        if !deferred_interruptions.is_empty() {
            failed_outcome = failed_outcome.with_interruptions(deferred_interruptions);
        }
        let outcome = with_retained_artifacts(failed_outcome, retained_paths);
        let result = make_test_result(
            target,
            mode,
            outcome,
            warnings,
            steps,
            started.elapsed().as_millis() as u64,
        );
        return Err(TestExecutionFailure::with_payload(
            AppError::Runtime(failure_message),
            result,
        ));
    }

    debug!(path = %artifacts.run_dir.display(), "cleaning successful test run directory");
    cleanup_run_dir(&artifacts);
    let mut successful_outcome = ExecutionOutcome::new(ExecutionStatus::Succeeded)
        .with_diagnostics(diagnostics)
        .with_metrics(ExecutionMetrics::from(&report.summary))
        .with_payload(rendered_report);
    if !deferred_interruptions.is_empty() {
        successful_outcome = successful_outcome.with_interruptions(deferred_interruptions);
    }
    Ok(make_test_result(
        target,
        mode,
        successful_outcome,
        warnings,
        steps,
        started.elapsed().as_millis() as u64,
    ))
}
