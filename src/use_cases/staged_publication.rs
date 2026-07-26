use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::support::error::AppError;
use crate::support::fs::{
    ensure_dir, metadata_sidecar_path, remove_path_if_exists, replace_dir_atomically,
    replace_file_atomically, write_temp_dir_metadata, TempDirKind,
};
use crate::use_cases::context::{ExecutionContext, ExecutionInterruption};
use crate::use_cases::interruption;

#[derive(Debug, Clone)]
pub(super) struct StagedPublication {
    staging_path: PathBuf,
    target_path: PathBuf,
    run_id: String,
    target_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StagedPublicationOutcome {
    pub cleanup_warning: Option<String>,
    pub deferred_interruption: Option<ExecutionInterruption>,
}

impl StagedPublication {
    pub fn prepare_dir(
        target_path: &Path,
        target_identity: &str,
        stage_prefix: &str,
    ) -> Result<Self, AppError> {
        let target_parent = target_path.parent().ok_or_else(|| {
            AppError::Runtime(format!(
                "target path has no parent: {}",
                target_path.display()
            ))
        })?;
        ensure_dir(target_parent).map_err(|error| {
            AppError::Runtime(format!("failed to create target parent dir: {error}"))
        })?;

        let run_id = make_run_id();
        let publication = Self::new(
            target_path,
            target_identity,
            target_parent.join(format!("{stage_prefix}-{run_id}")),
            run_id,
        );
        if publication.staging_path.exists() {
            return Err(AppError::Runtime(format!(
                "staging dir already exists unexpectedly: {}",
                publication.staging_path.display()
            )));
        }
        std::fs::create_dir(&publication.staging_path)
            .map_err(|error| AppError::Runtime(format!("failed to create staging dir: {error}")))?;
        publication.write_stage_metadata("failed to write stage metadata")?;
        Ok(publication)
    }

    pub fn prepare_file(
        target_path: &Path,
        target_identity: &str,
        stage_prefix: &str,
        extension: &str,
    ) -> Result<Self, AppError> {
        Self::prepare_file_with_metadata_writer(
            target_path,
            target_identity,
            stage_prefix,
            extension,
            |publication| publication.write_stage_metadata("failed to write staging metadata"),
        )
    }

    fn prepare_file_with_metadata_writer(
        target_path: &Path,
        target_identity: &str,
        stage_prefix: &str,
        extension: &str,
        write_metadata: impl FnOnce(&Self) -> Result<(), AppError>,
    ) -> Result<Self, AppError> {
        let target_parent = target_path.parent().ok_or_else(|| {
            AppError::Runtime(format!(
                "target path has no parent: {}",
                target_path.display()
            ))
        })?;
        ensure_dir(target_parent).map_err(|error| {
            AppError::Runtime(format!("failed to create target parent dir: {error}"))
        })?;

        let run_id = make_run_id();
        let publication = Self::new(
            target_path,
            target_identity,
            target_parent.join(format!("{stage_prefix}-{run_id}.{extension}")),
            run_id,
        );
        if publication.staging_path.exists() {
            return Err(AppError::Runtime(format!(
                "staging file already exists unexpectedly: {}",
                publication.staging_path.display()
            )));
        }
        if let Err(error) = write_metadata(&publication) {
            return Err(publication.cleanup_failure(error));
        }
        Ok(publication)
    }

    pub fn staging_path(&self) -> &Path {
        &self.staging_path
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn cleanup_failure(&self, error: AppError) -> AppError {
        cleanup_staging_path(&self.staging_path, error)
    }

    pub fn publish_dir(
        &self,
        context: &ExecutionContext,
        backup_prefix: &str,
        error_prefix: &str,
    ) -> Result<StagedPublicationOutcome, AppError> {
        let publish_phase = context.run_no_process_critical_phase(|| {
            replace_dir_atomically(
                &self.staging_path,
                &self.target_path,
                &self.run_id,
                &self.target_identity,
                backup_prefix,
            )
            .map_err(|error| AppError::Runtime(format!("{error_prefix}: {error}")))
        })?;
        Ok(StagedPublicationOutcome {
            cleanup_warning: publish_phase.value.cleanup_warning,
            deferred_interruption: publish_phase.deferred_interruption,
        })
    }

    pub fn publish_file(
        &self,
        context: &ExecutionContext,
        error_prefix: &str,
    ) -> Result<StagedPublicationOutcome, AppError> {
        let publish_phase = context.run_no_process_critical_phase(|| {
            replace_file_atomically(
                &self.staging_path,
                &self.target_path,
                &self.run_id,
                &self.target_identity,
            )
            .map_err(|error| AppError::Runtime(format!("{error_prefix}: {error}")))
        })?;
        Ok(StagedPublicationOutcome {
            cleanup_warning: publish_phase.value.cleanup_warning,
            deferred_interruption: publish_phase.deferred_interruption,
        })
    }

    fn new(
        target_path: &Path,
        target_identity: &str,
        staging_path: PathBuf,
        run_id: String,
    ) -> Self {
        Self {
            staging_path,
            target_path: target_path.to_path_buf(),
            run_id,
            target_identity: target_identity.to_owned(),
        }
    }

    fn write_stage_metadata(&self, message: &str) -> Result<(), AppError> {
        write_temp_dir_metadata(
            &self.staging_path,
            TempDirKind::Stage,
            &self.run_id,
            &self.target_path,
            &self.target_identity,
        )
        .map_err(|error| AppError::Runtime(format!("{message}: {error}")))
    }
}

pub(super) fn cleanup_staging_path(staging_path: &Path, error: AppError) -> AppError {
    cleanup_staging_path_with(staging_path, error, remove_path_if_exists)
}

fn cleanup_staging_path_with(
    staging_path: &Path,
    error: AppError,
    mut remove: impl FnMut(&Path) -> std::io::Result<()>,
) -> AppError {
    let sidecar = metadata_sidecar_path(staging_path);
    let mut cleanup_errors = Vec::new();
    if let Err(remove_error) = remove(staging_path) {
        cleanup_errors.push(format!(
            "failed to remove staging path '{}': {remove_error}",
            staging_path.display()
        ));
    }
    if let Err(remove_error) = remove(&sidecar) {
        cleanup_errors.push(format!(
            "failed to remove staging metadata '{}': {remove_error}",
            sidecar.display()
        ));
    }

    if cleanup_errors.is_empty() {
        error
    } else {
        error.with_context(format!("cleanup failed: {}", cleanup_errors.join("; ")))
    }
}

pub(super) fn interruption_before_publish(
    context: &ExecutionContext,
    safe_point: impl Into<String>,
) -> Option<AppError> {
    interruption::interruption_before_safe_point(context, safe_point.into())
}

fn make_run_id() -> String {
    make_run_id_with_uuid(std::process::id(), Uuid::new_v4())
}

fn make_run_id_with_uuid(pid: u32, uuid: Uuid) -> String {
    format!("{pid}-{uuid}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use crate::support::error::AppError;
    use crate::support::fs::{metadata_sidecar_path, read_temp_dir_metadata};
    use crate::use_cases::context::{CommandName, ExecutionContext, ExecutionInterruption};

    use super::{
        cleanup_staging_path, cleanup_staging_path_with, interruption_before_publish,
        make_run_id_with_uuid, StagedPublication,
    };

    #[test]
    fn prepare_dir_creates_stage_dir_and_metadata_then_publishes() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target");
        let publication =
            StagedPublication::prepare_dir(&target, "identity", ".stage").expect("prepare");
        fs::write(publication.staging_path().join("payload.txt"), "payload").expect("payload");
        let stage_metadata = metadata_sidecar_path(publication.staging_path());

        let outcome = publication
            .publish_dir(
                &ExecutionContext::cli(CommandName::Dump),
                ".backup",
                "failed to publish staged test dir",
            )
            .expect("publish");

        assert_eq!(outcome.deferred_interruption, None);
        assert_eq!(
            fs::read_to_string(target.join("payload.txt")).expect("target"),
            "payload"
        );
        assert!(!stage_metadata.exists());
    }

    #[test]
    fn run_id_uses_uuid_entropy_and_is_a_valid_file_name() {
        let first = make_run_id_with_uuid(
            42,
            Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("uuid"),
        );
        let second = make_run_id_with_uuid(
            42,
            Uuid::parse_str("123e4567-e89b-12d3-a456-426614174001").expect("uuid"),
        );

        assert_ne!(first, second);
        assert!(first.starts_with("42-"));
        assert!(!first.chars().any(|value| "<>:\"/\\|?*".contains(value)));
    }

    #[test]
    fn prepare_file_writes_metadata_without_materializing_stage_file() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target.cf");

        let publication =
            StagedPublication::prepare_file(&target, "identity", ".stage", "cf").expect("prepare");

        assert!(!publication.staging_path().exists());
        let metadata = read_temp_dir_metadata(publication.staging_path()).expect("metadata");
        assert_eq!(metadata.target_identity, "identity");
        assert_eq!(metadata.target_path, target);
    }

    #[test]
    fn prepare_file_cleans_partial_metadata_failure() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target.cf");
        let mut created_paths = None;

        let error = StagedPublication::prepare_file_with_metadata_writer(
            &target,
            "identity",
            ".stage",
            "cf",
            |publication| {
                fs::write(publication.staging_path(), "partial stage").expect("partial stage");
                let sidecar = metadata_sidecar_path(publication.staging_path());
                fs::write(&sidecar, "partial metadata").expect("partial metadata");
                created_paths = Some((publication.staging_path().to_path_buf(), sidecar));
                Err(AppError::Runtime("metadata write failed".to_owned()))
            },
        )
        .expect_err("metadata failure");

        let (stage, sidecar) = created_paths.expect("created paths");
        assert!(error.to_string().contains("metadata write failed"));
        assert!(!stage.exists());
        assert!(!sidecar.exists());
    }

    #[test]
    fn publish_file_uses_caller_created_stage_file() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target.cf");
        let publication =
            StagedPublication::prepare_file(&target, "identity", ".stage", "cf").expect("prepare");
        fs::write(publication.staging_path(), "package").expect("stage");

        publication
            .publish_file(
                &ExecutionContext::cli(CommandName::Artifacts),
                "failed to publish staged test file",
            )
            .expect("publish");

        assert_eq!(fs::read_to_string(target).expect("target"), "package");
    }

    #[test]
    fn explicit_cleanup_policy_removes_stage_path_and_sidecar() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target");
        let publication =
            StagedPublication::prepare_dir(&target, "identity", ".stage").expect("prepare");
        let metadata = metadata_sidecar_path(publication.staging_path());

        let error = cleanup_staging_path(
            publication.staging_path(),
            AppError::Runtime("failed before publish".to_owned()),
        );

        assert_eq!(error.to_string(), "runtime error: failed before publish");
        assert!(!publication.staging_path().exists());
        assert!(!metadata.exists());
    }

    #[test]
    fn cleanup_failure_reports_errors_for_stage_and_sidecar() {
        let staging_path = std::path::Path::new("stage.cf");
        let sidecar = metadata_sidecar_path(staging_path);
        let mut removed = Vec::new();

        let error = cleanup_staging_path_with(
            staging_path,
            AppError::Runtime("publication failed".to_owned()),
            |path| {
                removed.push(path.to_path_buf());
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("denied {}", path.display()),
                ))
            },
        );

        let message = error.to_string();
        assert_eq!(removed, vec![staging_path.to_path_buf(), sidecar]);
        assert!(message.contains("publication failed"));
        assert!(message.contains("failed to remove staging path"));
        assert!(message.contains("failed to remove staging metadata"));
    }

    #[test]
    fn interruption_check_reports_command_safe_point() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let context = ExecutionContext::cli(CommandName::Dump).with_cancellation(cancellation);

        let error = interruption_before_publish(&context, "dump publication").expect("error");

        assert!(error
            .to_string()
            .contains("before entering dump publication safe point"));
    }

    #[test]
    fn publish_reports_deferred_interruption_from_critical_phase() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target");
        let publication =
            StagedPublication::prepare_dir(&target, "identity", ".stage").expect("prepare");
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let context = ExecutionContext::cli(CommandName::Dump).with_cancellation(cancellation);

        let outcome = publication
            .publish_dir(&context, ".backup", "failed to publish staged test dir")
            .expect("publish");

        assert_eq!(
            outcome.deferred_interruption,
            Some(ExecutionInterruption::Cancelled)
        );
    }
}
