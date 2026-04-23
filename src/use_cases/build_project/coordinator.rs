use super::*;

pub(super) fn run_build_designer(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &BuildArgs,
) -> Result<BuildResult, BuildExecutionFailure> {
    debug!(full_rebuild = args.full_rebuild, "preparing build plan");

    let started = Instant::now();
    let service = SourceSetsService::new(config);
    let contexts = service.designer_contexts();
    let contexts_by_name: HashMap<String, SourceSetContext> = contexts
        .into_iter()
        .map(|context| (context.name().to_owned(), context))
        .collect();
    let ordered_source_sets = ordered_source_sets(config);

    let analysis_by_name = if args.full_rebuild {
        None
    } else {
        Some(analyze_contexts_by_name(
            &service,
            &contexts_by_name.values().cloned().collect::<Vec<_>>(),
        ))
    };

    let mut utilities = PlatformUtilities::from_config(config);
    let mut designer_binary: Option<PathBuf> = None;
    let mut steps = Vec::new();

    for (index, source_set) in ordered_source_sets.iter().enumerate() {
        let Some(source_context) = contexts_by_name.get(&source_set.name).cloned() else {
            continue;
        };

        if source_set.purpose.is_external() {
            let step_started = Instant::now();
            let result = discover_designer_external_artifacts(
                &source_set.name,
                &resolve_source_set_path(config, source_set),
                source_set_external_kind(source_set).expect("external kind"),
            );
            match result {
                Ok(descriptors) => push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::Skipped,
                    true,
                    format!(
                        "prepared {} external artifact(s) for packaging",
                        descriptors.len()
                    ),
                    step_started.elapsed().as_millis() as u64,
                ),
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::Skipped,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(error, result));
                }
            }
            continue;
        }

        let plan = if args.full_rebuild {
            StepPlan::Execute {
                mode: BuildMode::Full,
                message: "forced full rebuild".to_owned(),
                partial_paths: None,
                commit: StepCommit::RescanFull {
                    recover_storage: true,
                },
            }
        } else {
            match analysis_by_name
                .as_ref()
                .and_then(|analysis| analysis.get(&source_set.name))
                .cloned()
                .expect("every source-set must have an analysis result")
            {
                Ok(AnalysisOutcome::NoChanges) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        found_changes = 0,
                        "change analysis result: found 0 change(s)"
                    );
                    StepPlan::Skip {
                        message: "no changes".to_owned(),
                        ok: true,
                    }
                }
                Ok(AnalysisOutcome::Fallback) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        "change analysis result: fallback to full load after recoverable issue"
                    );
                    log_timeline_stage(
                        &source_set.name,
                        "changes",
                        "fallback to full load after recoverable issue",
                        TimelineStageStatus::Succeeded,
                    );
                    StepPlan::Execute {
                        mode: BuildMode::Full,
                        message: "fallback to full load after recoverable change-detection issue"
                            .to_owned(),
                        partial_paths: None,
                        commit: StepCommit::RescanFull {
                            recover_storage: false,
                        },
                    }
                }
                Ok(AnalysisOutcome::Changes { changes, prepared }) => {
                    log_change_analysis(source_set.name.as_str(), &changes);
                    match partial_load::decide(
                        &changes,
                        source_context.path(),
                        config.build.partial_load_threshold,
                    ) {
                        LoadDecision::Partial(paths) => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                partial_file_count = paths.len(),
                                threshold = config.build.partial_load_threshold,
                                "change analysis decision: partial load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Partial {
                                    file_count: paths.len(),
                                },
                                message: format!("partial load of {} files", paths.len()),
                                partial_paths: Some(paths),
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                        LoadDecision::Full => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                threshold = config.build.partial_load_threshold,
                                "change analysis decision: full load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Full,
                                message: "full load selected by partial-load rules".to_owned(),
                                partial_paths: None,
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                    }
                }
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::Skipped,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(
                        AppError::Runtime(error.to_string()),
                        result,
                    ));
                }
            }
        };

        match plan {
            StepPlan::Skip { message, ok } => {
                debug!(
                    source_set = source_set.name.as_str(),
                    message = message.as_str(),
                    "skipping build step"
                );
                push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::Skipped,
                    ok,
                    message,
                    0,
                )
            }
            StepPlan::Execute {
                mode,
                message,
                partial_paths,
                commit,
            } => {
                debug!(
                    source_set = source_set.name.as_str(),
                    mode = ?mode,
                    message = message.as_str(),
                    "executing build step"
                );
                let binary = match designer_binary.clone() {
                    Some(path) => path,
                    None => {
                        let location = match utilities.locate(UtilityType::V8) {
                            Ok(location) => location,
                            Err(error) => {
                                let result = fail_with_remaining_steps(
                                    started,
                                    steps,
                                    ordered_source_sets
                                        .iter()
                                        .skip(index)
                                        .copied()
                                        .collect::<Vec<_>>(),
                                    source_set,
                                    mode.clone(),
                                    error.to_string(),
                                );
                                return Err(BuildExecutionFailure::with_payload(
                                    AppError::from(error),
                                    result,
                                ));
                            }
                        };
                        designer_binary = Some(location.path.clone());
                        location.path
                    }
                };

                let step_started = Instant::now();
                match execute_source_set_step(
                    context,
                    config,
                    &binary,
                    utilities.runner_for(UtilityType::V8),
                    source_set,
                    &source_context,
                    &source_context,
                    index,
                    partial_paths.as_deref(),
                    &commit,
                ) {
                    Ok(warnings) => push_build_step(
                        &mut steps,
                        &source_set.name,
                        mode,
                        true,
                        merge_step_message(message, &warnings),
                        step_started.elapsed().as_millis() as u64,
                    ),
                    Err(error) => {
                        let result = fail_with_remaining_steps(
                            started,
                            steps,
                            ordered_source_sets
                                .iter()
                                .skip(index)
                                .copied()
                                .collect::<Vec<_>>(),
                            source_set,
                            mode,
                            error.to_string(),
                        );
                        return Err(BuildExecutionFailure::with_payload(error, result));
                    }
                }
            }
        }
    }

    Ok(BuildResult {
        ok: true,
        steps,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub(super) fn run_build_ibcmd(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &BuildArgs,
) -> Result<BuildResult, BuildExecutionFailure> {
    debug!(
        full_rebuild = args.full_rebuild,
        "preparing ibcmd build plan"
    );

    let started = Instant::now();
    let service = SourceSetsService::new(config);
    let contexts = service.designer_contexts();
    let contexts_by_name: HashMap<String, SourceSetContext> = contexts
        .into_iter()
        .map(|context| (context.name().to_owned(), context))
        .collect();
    let ordered_source_sets = ordered_source_sets(config);

    let analysis_by_name = if args.full_rebuild {
        None
    } else {
        Some(analyze_contexts_by_name(
            &service,
            &contexts_by_name.values().cloned().collect::<Vec<_>>(),
        ))
    };

    let mut utilities = PlatformUtilities::from_config(config);
    let mut ibcmd_binary: Option<PathBuf> = None;
    let mut steps = Vec::new();

    for (index, source_set) in ordered_source_sets.iter().enumerate() {
        let Some(source_context) = contexts_by_name.get(&source_set.name).cloned() else {
            continue;
        };

        let plan = if args.full_rebuild {
            StepPlan::Execute {
                mode: BuildMode::Full,
                message: "forced full rebuild".to_owned(),
                partial_paths: None,
                commit: StepCommit::RescanFull {
                    recover_storage: true,
                },
            }
        } else {
            match analysis_by_name
                .as_ref()
                .and_then(|analysis| analysis.get(&source_set.name))
                .cloned()
                .expect("every source-set must have an analysis result")
            {
                Ok(AnalysisOutcome::NoChanges) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        found_changes = 0,
                        "change analysis result: found 0 change(s)"
                    );
                    StepPlan::Skip {
                        message: "no changes".to_owned(),
                        ok: true,
                    }
                }
                Ok(AnalysisOutcome::Fallback) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        "change analysis result: fallback to full load after recoverable issue"
                    );
                    log_timeline_stage(
                        &source_set.name,
                        "changes",
                        "fallback to full load after recoverable issue",
                        TimelineStageStatus::Succeeded,
                    );
                    StepPlan::Execute {
                        mode: BuildMode::Full,
                        message: "fallback to full load after recoverable change-detection issue"
                            .to_owned(),
                        partial_paths: None,
                        commit: StepCommit::RescanFull {
                            recover_storage: false,
                        },
                    }
                }
                Ok(AnalysisOutcome::Changes { changes, prepared }) => {
                    log_change_analysis(source_set.name.as_str(), &changes);
                    match partial_load::decide(
                        &changes,
                        source_context.path(),
                        config.build.partial_load_threshold,
                    ) {
                        LoadDecision::Partial(paths) => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                partial_file_count = paths.len(),
                                threshold = config.build.partial_load_threshold,
                                "change analysis decision: partial load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Partial {
                                    file_count: paths.len(),
                                },
                                message: format!("partial load of {} files", paths.len()),
                                partial_paths: Some(paths),
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                        LoadDecision::Full => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                threshold = config.build.partial_load_threshold,
                                "change analysis decision: full load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Full,
                                message: "full load selected by partial-load rules".to_owned(),
                                partial_paths: None,
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                    }
                }
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::Skipped,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(
                        AppError::Runtime(error.to_string()),
                        result,
                    ));
                }
            }
        };

        match plan {
            StepPlan::Skip { message, ok } => {
                debug!(
                    source_set = source_set.name.as_str(),
                    message = message.as_str(),
                    "skipping build step"
                );
                push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::Skipped,
                    ok,
                    message,
                    0,
                )
            }
            StepPlan::Execute {
                mode,
                message,
                partial_paths,
                commit,
            } => {
                debug!(
                    source_set = source_set.name.as_str(),
                    mode = ?mode,
                    message = message.as_str(),
                    "executing ibcmd build step"
                );
                let binary = match ibcmd_binary.clone() {
                    Some(path) => path,
                    None => {
                        let location = match utilities.locate(UtilityType::Ibcmd) {
                            Ok(location) => location,
                            Err(error) => {
                                let result = fail_with_remaining_steps(
                                    started,
                                    steps,
                                    ordered_source_sets
                                        .iter()
                                        .skip(index)
                                        .copied()
                                        .collect::<Vec<_>>(),
                                    source_set,
                                    mode.clone(),
                                    error.to_string(),
                                );
                                return Err(BuildExecutionFailure::with_payload(
                                    AppError::from(error),
                                    result,
                                ));
                            }
                        };
                        ibcmd_binary = Some(location.path.clone());
                        location.path
                    }
                };

                let step_started = Instant::now();
                match execute_source_set_step_ibcmd(
                    context,
                    config,
                    &binary,
                    utilities.runner_for(UtilityType::Ibcmd),
                    source_set,
                    &source_context,
                    &source_context,
                    partial_paths.as_deref(),
                    &commit,
                ) {
                    Ok(warnings) => push_build_step(
                        &mut steps,
                        &source_set.name,
                        mode,
                        true,
                        merge_step_message(message, &warnings),
                        step_started.elapsed().as_millis() as u64,
                    ),
                    Err(error) => {
                        let result = fail_with_remaining_steps(
                            started,
                            steps,
                            ordered_source_sets
                                .iter()
                                .skip(index)
                                .copied()
                                .collect::<Vec<_>>(),
                            source_set,
                            mode,
                            error.to_string(),
                        );
                        return Err(BuildExecutionFailure::with_payload(error, result));
                    }
                }
            }
        }
    }

    Ok(BuildResult {
        ok: true,
        steps,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub(super) fn run_build_edt(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &BuildArgs,
) -> Result<BuildResult, BuildExecutionFailure> {
    debug!(full_rebuild = args.full_rebuild, "preparing edt build plan");
    if let Some(error) = validate_edt_supported_matrix(config) {
        return Err(BuildExecutionFailure::with_payload(
            error,
            BuildResult {
                ok: false,
                steps: vec![],
                duration_ms: 0,
            },
        ));
    }

    let started = Instant::now();
    let service = SourceSetsService::new(config);
    let edt_contexts = service.edt_contexts();
    let designer_contexts = service.designer_contexts();
    let edt_contexts_by_name: HashMap<String, SourceSetContext> = edt_contexts
        .into_iter()
        .map(|context| (context.name().to_owned(), context))
        .collect();
    let designer_contexts_by_name: HashMap<String, SourceSetContext> = designer_contexts
        .into_iter()
        .map(|context| (context.name().to_owned(), context))
        .collect();
    let ordered_source_sets = ordered_source_sets(config);

    let edt_analysis_by_name = if args.full_rebuild {
        None
    } else {
        Some(analyze_contexts_by_name(
            &service,
            &edt_contexts_by_name.values().cloned().collect::<Vec<_>>(),
        ))
    };

    let mut utilities = PlatformUtilities::from_config(config);
    let mut designer_binary: Option<PathBuf> = None;
    let mut ibcmd_binary: Option<PathBuf> = None;
    let mut edt_binary: Option<PathBuf> = None;
    let mut interactive_edt = None;
    let mut steps = Vec::new();

    for (index, source_set) in ordered_source_sets.iter().enumerate() {
        let Some(edt_context) = edt_contexts_by_name.get(&source_set.name).cloned() else {
            continue;
        };
        let Some(designer_context) = designer_contexts_by_name.get(&source_set.name).cloned()
        else {
            continue;
        };

        let edt_stage = if args.full_rebuild {
            StepPlan::Execute {
                mode: BuildMode::EdtExport,
                message: "forced EDT export (--full-rebuild)".to_owned(),
                partial_paths: None,
                commit: StepCommit::RescanFull {
                    recover_storage: true,
                },
            }
        } else {
            match edt_analysis_by_name
                .as_ref()
                .and_then(|analysis| analysis.get(&source_set.name))
                .cloned()
                .expect("every source-set must have an EDT analysis result")
            {
                Ok(AnalysisOutcome::NoChanges) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        found_changes = 0,
                        "edt change analysis result: found 0 change(s)"
                    );
                    StepPlan::Skip {
                        message: "no changes".to_owned(),
                        ok: true,
                    }
                }
                Ok(AnalysisOutcome::Fallback) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        "edt change analysis result: fallback to full export/load after recoverable issue"
                    );
                    log_timeline_stage(
                        &source_set.name,
                        "changes",
                        "fallback to full export/load after recoverable issue",
                        TimelineStageStatus::Succeeded,
                    );
                    StepPlan::Execute {
                        mode: BuildMode::EdtExport,
                        message: "fallback to EDT export after recoverable change-detection issue"
                            .to_owned(),
                        partial_paths: None,
                        commit: StepCommit::RescanFull {
                            recover_storage: false,
                        },
                    }
                }
                Ok(AnalysisOutcome::Changes { changes, prepared }) => {
                    log_change_analysis(source_set.name.as_str(), &changes);
                    StepPlan::Execute {
                        mode: BuildMode::EdtExport,
                        message: "EDT export after change detection".to_owned(),
                        partial_paths: None,
                        commit: StepCommit::Prepared(prepared),
                    }
                }
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::Skipped,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(
                        AppError::Runtime(error.to_string()),
                        result,
                    ));
                }
            }
        };

        if source_set.purpose.is_external() {
            let edt = match edt_binary.clone() {
                Some(path) => path,
                None => {
                    let location = match utilities.locate(UtilityType::EdtCli) {
                        Ok(location) => location,
                        Err(error) => {
                            let result = fail_with_remaining_steps(
                                started,
                                steps,
                                ordered_source_sets
                                    .iter()
                                    .skip(index)
                                    .copied()
                                    .collect::<Vec<_>>(),
                                source_set,
                                BuildMode::EdtExport,
                                error.to_string(),
                            );
                            return Err(BuildExecutionFailure::with_payload(
                                AppError::from(error),
                                result,
                            ));
                        }
                    };
                    edt_binary = Some(location.path.clone());
                    location.path
                }
            };
            let export_started = Instant::now();
            if let Some(error) = interruption_before_safe_point(
                context,
                format!(
                    "EDT external artifact export for source-set '{}'",
                    source_set.name
                ),
            ) {
                let result = fail_with_remaining_steps(
                    started,
                    steps,
                    ordered_source_sets
                        .iter()
                        .skip(index)
                        .copied()
                        .collect::<Vec<_>>(),
                    source_set,
                    BuildMode::EdtExport,
                    error.to_string(),
                );
                return Err(BuildExecutionFailure::with_payload(error, result));
            }
            let export_result = if config.tools.edt_cli.interactive_mode {
                if interactive_edt.is_none() {
                    interactive_edt = Some(
                        match EdtSessionManager::for_config(
                            config,
                            EdtSessionHostOptions::for_cli_command(config),
                        ) {
                            Ok(manager) => match EdtDsl::new_shared_session(
                                edt.clone(),
                                config.work_path.join("edt-workspace"),
                                Arc::new(manager),
                                Duration::from_millis(config.tools.edt_cli.startup_timeout_ms),
                                Duration::from_millis(config.tools.edt_cli.command_timeout_ms),
                            ) {
                                Ok(dsl) => dsl.with_execution_policy(context.process_policy(
                                    InterruptionSafetyClass::GracefulThenKill,
                                    None,
                                )),
                                Err(error) => {
                                    let app_error = AppError::from(error);
                                    let result = fail_with_remaining_steps(
                                        started,
                                        steps,
                                        ordered_source_sets
                                            .iter()
                                            .skip(index)
                                            .copied()
                                            .collect::<Vec<_>>(),
                                        source_set,
                                        BuildMode::EdtExport,
                                        app_error.to_string(),
                                    );
                                    return Err(BuildExecutionFailure::with_payload(
                                        app_error, result,
                                    ));
                                }
                            },
                            Err(error) => {
                                let app_error = AppError::from(error);
                                let result = fail_with_remaining_steps(
                                    started,
                                    steps,
                                    ordered_source_sets
                                        .iter()
                                        .skip(index)
                                        .copied()
                                        .collect::<Vec<_>>(),
                                    source_set,
                                    BuildMode::EdtExport,
                                    app_error.to_string(),
                                );
                                return Err(BuildExecutionFailure::with_payload(app_error, result));
                            }
                        },
                    );
                }
                prepare_edt_external_artifacts(
                    config,
                    source_set,
                    interactive_edt.as_ref().expect("interactive edt dsl"),
                )
            } else {
                let one_shot_edt = EdtDsl::new(
                    edt.clone(),
                    config.work_path.join("edt-workspace"),
                    utilities.runner_for(UtilityType::EdtCli),
                )
                .with_execution_policy(
                    context.process_policy(InterruptionSafetyClass::GracefulThenKill, None),
                );
                prepare_edt_external_artifacts(config, source_set, &one_shot_edt)
            };
            match export_result {
                Ok(descriptors) => {
                    match &edt_stage {
                        StepPlan::Execute { commit, .. } => match commit {
                            StepCommit::Prepared(prepared) => {
                                if let Err(error) = analyzer::commit_success(
                                    &edt_context,
                                    &config.work_path,
                                    prepared,
                                ) {
                                    let app_error = AppError::Runtime(error.to_string());
                                    let result = fail_with_remaining_steps(
                                        started,
                                        steps,
                                        ordered_source_sets
                                            .iter()
                                            .skip(index)
                                            .copied()
                                            .collect::<Vec<_>>(),
                                        source_set,
                                        BuildMode::EdtExport,
                                        app_error.to_string(),
                                    );
                                    return Err(BuildExecutionFailure::with_payload(
                                        app_error, result,
                                    ));
                                }
                            }
                            StepCommit::RescanFull { recover_storage } => {
                                if let Err(app_error) = commit_full_rescan(
                                    &edt_context,
                                    &config.work_path,
                                    *recover_storage,
                                ) {
                                    let result = fail_with_remaining_steps(
                                        started,
                                        steps,
                                        ordered_source_sets
                                            .iter()
                                            .skip(index)
                                            .copied()
                                            .collect::<Vec<_>>(),
                                        source_set,
                                        BuildMode::EdtExport,
                                        app_error.to_string(),
                                    );
                                    return Err(BuildExecutionFailure::with_payload(
                                        app_error, result,
                                    ));
                                }
                            }
                        },
                        StepPlan::Skip { .. } => {}
                    }
                    push_build_step(
                        &mut steps,
                        &source_set.name,
                        BuildMode::EdtExport,
                        true,
                        merge_step_message(
                            format!(
                                "exported {} external artifact(s) to designer runtime",
                                descriptors.len()
                            ),
                            &[],
                        ),
                        export_started.elapsed().as_millis() as u64,
                    )
                }
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::EdtExport,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(error, result));
                }
            }
            continue;
        }

        let edt_stage_skipped = matches!(&edt_stage, StepPlan::Skip { .. });

        match edt_stage {
            StepPlan::Skip { message, ok } => {
                push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::Skipped,
                    ok,
                    message,
                    0,
                );
            }
            StepPlan::Execute {
                message: _,
                partial_paths: _,
                commit,
                mode: _,
            } => {
                let edt = match edt_binary.clone() {
                    Some(path) => path,
                    None => {
                        let location = match utilities.locate(UtilityType::EdtCli) {
                            Ok(location) => location,
                            Err(error) => {
                                let result = fail_with_remaining_steps(
                                    started,
                                    steps,
                                    ordered_source_sets
                                        .iter()
                                        .skip(index)
                                        .copied()
                                        .collect::<Vec<_>>(),
                                    source_set,
                                    BuildMode::EdtExport,
                                    error.to_string(),
                                );
                                return Err(BuildExecutionFailure::with_payload(
                                    AppError::from(error),
                                    result,
                                ));
                            }
                        };
                        edt_binary = Some(location.path.clone());
                        location.path
                    }
                };

                let export_started = Instant::now();
                log_timeline_stage(
                    &source_set.name,
                    "edt_export",
                    "[EDT] Конвертация в файлы конфигуратора",
                    TimelineStageStatus::Running,
                );
                let export_result = if config.tools.edt_cli.interactive_mode {
                    if interactive_edt.is_none() {
                        interactive_edt = Some(
                            match EdtSessionManager::for_config(
                                config,
                                EdtSessionHostOptions::for_cli_command(config),
                            ) {
                                Ok(manager) => match EdtDsl::new_shared_session(
                                    edt.clone(),
                                    config.work_path.join("edt-workspace"),
                                    Arc::new(manager),
                                    Duration::from_millis(config.tools.edt_cli.startup_timeout_ms),
                                    Duration::from_millis(config.tools.edt_cli.command_timeout_ms),
                                ) {
                                    Ok(dsl) => dsl.with_execution_policy(context.process_policy(
                                        InterruptionSafetyClass::GracefulThenKill,
                                        None,
                                    )),
                                    Err(error) => {
                                        let app_error = AppError::from(error);
                                        let result = fail_with_remaining_steps(
                                            started,
                                            steps,
                                            ordered_source_sets
                                                .iter()
                                                .skip(index)
                                                .copied()
                                                .collect::<Vec<_>>(),
                                            source_set,
                                            BuildMode::EdtExport,
                                            app_error.to_string(),
                                        );
                                        return Err(BuildExecutionFailure::with_payload(
                                            app_error, result,
                                        ));
                                    }
                                },
                                Err(error) => {
                                    let app_error = AppError::from(error);
                                    let result = fail_with_remaining_steps(
                                        started,
                                        steps,
                                        ordered_source_sets
                                            .iter()
                                            .skip(index)
                                            .copied()
                                            .collect::<Vec<_>>(),
                                        source_set,
                                        BuildMode::EdtExport,
                                        app_error.to_string(),
                                    );
                                    return Err(BuildExecutionFailure::with_payload(
                                        app_error, result,
                                    ));
                                }
                            },
                        );
                    }
                    execute_edt_export_step(
                        context,
                        config,
                        interactive_edt.as_ref().expect("interactive edt dsl"),
                        source_set,
                        &edt_context,
                        &designer_context,
                    )
                } else {
                    let one_shot_edt = EdtDsl::new(
                        edt.clone(),
                        config.work_path.join("edt-workspace"),
                        utilities.runner_for(UtilityType::EdtCli),
                    )
                    .with_execution_policy(
                        context.process_policy(InterruptionSafetyClass::GracefulThenKill, None),
                    );
                    execute_edt_export_step(
                        context,
                        config,
                        &one_shot_edt,
                        source_set,
                        &edt_context,
                        &designer_context,
                    )
                };
                let export_warnings = match export_result {
                    Ok(warnings) => warnings,
                    Err(error) => {
                        let result = fail_with_remaining_steps(
                            started,
                            steps,
                            ordered_source_sets
                                .iter()
                                .skip(index)
                                .copied()
                                .collect::<Vec<_>>(),
                            source_set,
                            BuildMode::EdtExport,
                            error.to_string(),
                        );
                        return Err(BuildExecutionFailure::with_payload(error, result));
                    }
                };
                match &commit {
                    StepCommit::Prepared(prepared) => {
                        if let Err(error) =
                            analyzer::commit_success(&edt_context, &config.work_path, prepared)
                        {
                            let app_error = AppError::Runtime(error.to_string());
                            let result = fail_with_remaining_steps(
                                started,
                                steps,
                                ordered_source_sets
                                    .iter()
                                    .skip(index)
                                    .copied()
                                    .collect::<Vec<_>>(),
                                source_set,
                                BuildMode::EdtExport,
                                app_error.to_string(),
                            );
                            return Err(BuildExecutionFailure::with_payload(app_error, result));
                        }
                    }
                    StepCommit::RescanFull { recover_storage } => {
                        if let Err(app_error) =
                            commit_full_rescan(&edt_context, &config.work_path, *recover_storage)
                        {
                            let result = fail_with_remaining_steps(
                                started,
                                steps,
                                ordered_source_sets
                                    .iter()
                                    .skip(index)
                                    .copied()
                                    .collect::<Vec<_>>(),
                                source_set,
                                BuildMode::EdtExport,
                                app_error.to_string(),
                            );
                            return Err(BuildExecutionFailure::with_payload(app_error, result));
                        }
                    }
                }

                push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::EdtExport,
                    true,
                    merge_step_message("EDT export completed".to_owned(), &export_warnings),
                    export_started.elapsed().as_millis() as u64,
                );
            }
        }

        let designer_stage = if edt_stage_skipped && !designer_context.path().exists() {
            StepPlan::Skip {
                message: "no changes".to_owned(),
                ok: true,
            }
        } else if args.full_rebuild {
            StepPlan::Execute {
                mode: BuildMode::Full,
                message: "full load from EDT export (--full-rebuild)".to_owned(),
                partial_paths: None,
                commit: StepCommit::RescanFull {
                    recover_storage: true,
                },
            }
        } else {
            match analyzer::analyze_context(&designer_context, &config.work_path).outcome {
                Ok(AnalysisOutcome::NoChanges) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        found_changes = 0,
                        "generated designer change analysis result: found 0 change(s)"
                    );
                    StepPlan::Skip {
                        message: "no changes".to_owned(),
                        ok: true,
                    }
                }
                Ok(AnalysisOutcome::Fallback) => {
                    debug!(
                        source_set = source_set.name.as_str(),
                        "generated designer change analysis result: fallback to full load after recoverable issue"
                    );
                    log_timeline_stage(
                        &source_set.name,
                        "changes",
                        "fallback to full load after recoverable issue",
                        TimelineStageStatus::Succeeded,
                    );
                    StepPlan::Execute {
                        mode: BuildMode::Full,
                        message: "fallback to full load after recoverable change-detection issue"
                            .to_owned(),
                        partial_paths: None,
                        commit: StepCommit::RescanFull {
                            recover_storage: false,
                        },
                    }
                }
                Ok(AnalysisOutcome::Changes { changes, prepared }) => {
                    log_change_analysis(source_set.name.as_str(), &changes);
                    let generated_load_decision = if source_set.purpose
                        == SourceSetPurpose::Extension
                    {
                        debug!(
                                source_set = source_set.name.as_str(),
                                "generated designer change analysis decision: forcing full load for EDT extension source-set"
                            );
                        LoadDecision::Full
                    } else {
                        partial_load::decide(
                            &changes,
                            designer_context.path(),
                            config.build.partial_load_threshold,
                        )
                    };
                    match generated_load_decision {
                        LoadDecision::Partial(paths) => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                partial_file_count = paths.len(),
                                threshold = config.build.partial_load_threshold,
                                "generated designer change analysis decision: partial load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Partial {
                                    file_count: paths.len(),
                                },
                                message: format!("partial load of {} files", paths.len()),
                                partial_paths: Some(paths),
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                        LoadDecision::Full => {
                            debug!(
                                source_set = source_set.name.as_str(),
                                threshold = config.build.partial_load_threshold,
                                "generated designer change analysis decision: full load"
                            );
                            StepPlan::Execute {
                                mode: BuildMode::Full,
                                message: if source_set.purpose == SourceSetPurpose::Extension {
                                    "full load required for EDT extension source-set".to_owned()
                                } else {
                                    "full load selected by partial-load rules".to_owned()
                                },
                                partial_paths: None,
                                commit: StepCommit::Prepared(prepared),
                            }
                        }
                    }
                }
                Err(error) => {
                    let result = fail_with_remaining_steps(
                        started,
                        steps,
                        ordered_source_sets
                            .iter()
                            .skip(index)
                            .copied()
                            .collect::<Vec<_>>(),
                        source_set,
                        BuildMode::Skipped,
                        error.to_string(),
                    );
                    return Err(BuildExecutionFailure::with_payload(
                        AppError::Runtime(error.to_string()),
                        result,
                    ));
                }
            }
        };

        match designer_stage {
            StepPlan::Skip { message, ok } => {
                push_build_step(
                    &mut steps,
                    &source_set.name,
                    BuildMode::Skipped,
                    ok,
                    message,
                    0,
                );
            }
            StepPlan::Execute {
                mode,
                message,
                partial_paths,
                commit,
            } => {
                let load_started = Instant::now();
                let load_result = match config.builder {
                    BuilderBackend::Designer => {
                        let designer = match designer_binary.clone() {
                            Some(path) => path,
                            None => {
                                let location = match utilities.locate(UtilityType::V8) {
                                    Ok(location) => location,
                                    Err(error) => {
                                        let result = fail_with_remaining_steps(
                                            started,
                                            steps,
                                            ordered_source_sets
                                                .iter()
                                                .skip(index)
                                                .copied()
                                                .collect::<Vec<_>>(),
                                            source_set,
                                            mode.clone(),
                                            error.to_string(),
                                        );
                                        return Err(BuildExecutionFailure::with_payload(
                                            AppError::from(error),
                                            result,
                                        ));
                                    }
                                };
                                designer_binary = Some(location.path.clone());
                                location.path
                            }
                        };
                        execute_source_set_step(
                            context,
                            config,
                            &designer,
                            utilities.runner_for(UtilityType::V8),
                            source_set,
                            &designer_context,
                            &designer_context,
                            index,
                            partial_paths.as_deref(),
                            &commit,
                        )
                    }
                    BuilderBackend::Ibcmd => {
                        let ibcmd = match ibcmd_binary.clone() {
                            Some(path) => path,
                            None => {
                                let location = match utilities.locate(UtilityType::Ibcmd) {
                                    Ok(location) => location,
                                    Err(error) => {
                                        let result = fail_with_remaining_steps(
                                            started,
                                            steps,
                                            ordered_source_sets
                                                .iter()
                                                .skip(index)
                                                .copied()
                                                .collect::<Vec<_>>(),
                                            source_set,
                                            mode.clone(),
                                            error.to_string(),
                                        );
                                        return Err(BuildExecutionFailure::with_payload(
                                            AppError::from(error),
                                            result,
                                        ));
                                    }
                                };
                                ibcmd_binary = Some(location.path.clone());
                                location.path
                            }
                        };
                        execute_source_set_step_ibcmd(
                            context,
                            config,
                            &ibcmd,
                            utilities.runner_for(UtilityType::Ibcmd),
                            source_set,
                            &designer_context,
                            &designer_context,
                            partial_paths.as_deref(),
                            &commit,
                        )
                    }
                };
                match load_result {
                    Ok(warnings) => push_build_step(
                        &mut steps,
                        &source_set.name,
                        mode,
                        true,
                        merge_step_message(message, &warnings),
                        load_started.elapsed().as_millis() as u64,
                    ),
                    Err(error) => {
                        let result = fail_with_remaining_steps(
                            started,
                            steps,
                            ordered_source_sets
                                .iter()
                                .skip(index)
                                .copied()
                                .collect::<Vec<_>>(),
                            source_set,
                            mode,
                            error.to_string(),
                        );
                        return Err(BuildExecutionFailure::with_payload(error, result));
                    }
                }
            }
        }
    }

    Ok(BuildResult {
        ok: true,
        steps,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}
