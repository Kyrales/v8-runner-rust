use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::support::error::AppError;
use crate::support::path::stable_path_identity;
use crate::use_cases::context::{ExecutionContext, ExecutionInterruption};
use crate::use_cases::staged_publication::{
    interruption_before_publish, StagedPublication, StagedPublicationOutcome,
};

const STAGE_PREFIX: &str = ".junit-stage";

trait SyncWrite: Write {
    fn sync_all(&mut self) -> std::io::Result<()>;
}

impl SyncWrite for File {
    fn sync_all(&mut self) -> std::io::Result<()> {
        File::sync_all(self)
    }
}

#[derive(Debug, Clone)]
pub(super) struct JunitExport {
    target: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JunitExportOutcome {
    pub path: PathBuf,
    pub cleanup_warning: Option<String>,
    pub deferred_interruption: Option<ExecutionInterruption>,
}

#[derive(Clone, Copy)]
enum PublicationStart {
    Normal,
    ResumeAfterEnterpriseFailure,
}

impl JunitExport {
    pub fn prepare(target: PathBuf) -> Result<Self, AppError> {
        if target.as_os_str().is_empty() || target.file_name().is_none() {
            return Err(AppError::Validation(
                "JUnit export target must name a file".to_owned(),
            ));
        }
        if target.is_dir() {
            return Err(AppError::Validation(format!(
                "JUnit export target is a directory: {}",
                target.display()
            )));
        }

        if let Some(parent) = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::Runtime(format!(
                    "failed to create JUnit export parent '{}': {error}",
                    parent.display()
                ))
            })?;
        }
        match fs::remove_file(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::Runtime(format!(
                    "failed to remove stale JUnit export '{}': {error}",
                    target.display()
                )));
            }
        }

        Ok(Self { target })
    }

    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn publish(
        &self,
        context: &ExecutionContext,
        bytes: &[u8],
    ) -> Result<JunitExportOutcome, AppError> {
        self.publish_with_ops(
            context,
            bytes,
            open_staged_file,
            |publication, context| {
                publication.publish_file(context, "failed to publish JUnit export")
            },
            |publication, error| publication.cleanup_failure(error),
        )
    }

    pub fn publish_after_enterprise_failure(
        &self,
        context: &ExecutionContext,
        bytes: &[u8],
    ) -> Result<JunitExportOutcome, AppError> {
        self.publish_with_ops_mode(
            context,
            bytes,
            open_staged_file,
            |publication, context| {
                publication.publish_file(context, "failed to publish JUnit export")
            },
            |publication, error| publication.cleanup_failure(error),
            PublicationStart::ResumeAfterEnterpriseFailure,
        )
    }

    fn publish_with_ops<W: SyncWrite>(
        &self,
        context: &ExecutionContext,
        bytes: &[u8],
        open: impl FnOnce(&Path) -> std::io::Result<W>,
        publish: impl FnOnce(
            &StagedPublication,
            &ExecutionContext,
        ) -> Result<StagedPublicationOutcome, AppError>,
        cleanup: impl Fn(&StagedPublication, AppError) -> AppError,
    ) -> Result<JunitExportOutcome, AppError> {
        self.publish_with_ops_mode(
            context,
            bytes,
            open,
            publish,
            cleanup,
            PublicationStart::Normal,
        )
    }

    fn publish_with_ops_mode<W: SyncWrite>(
        &self,
        context: &ExecutionContext,
        bytes: &[u8],
        open: impl FnOnce(&Path) -> std::io::Result<W>,
        publish: impl FnOnce(
            &StagedPublication,
            &ExecutionContext,
        ) -> Result<StagedPublicationOutcome, AppError>,
        cleanup: impl Fn(&StagedPublication, AppError) -> AppError,
        start: PublicationStart,
    ) -> Result<JunitExportOutcome, AppError> {
        if bytes.is_empty() {
            return Err(AppError::Runtime(
                "cannot export empty JUnit report".to_owned(),
            ));
        }

        let publication = StagedPublication::prepare_file(
            &self.target,
            &stable_path_identity(&self.target),
            STAGE_PREFIX,
            "xml",
        )?;
        let mut file = open(publication.staging_path()).map_err(|error| {
            cleanup(
                &publication,
                AppError::Runtime(format!("failed to create staged JUnit export: {error}")),
            )
        })?;
        file.write_all(bytes).map_err(|error| {
            cleanup(
                &publication,
                AppError::Runtime(format!("failed to write staged JUnit export: {error}")),
            )
        })?;
        file.sync_all().map_err(|error| {
            cleanup(
                &publication,
                AppError::Runtime(format!("failed to sync staged JUnit export: {error}")),
            )
        })?;

        match start {
            PublicationStart::Normal => {
                if let Some(error) =
                    interruption_before_publish(context, "JUnit export publication")
                {
                    return Err(publication.cleanup_failure(error));
                }
            }
            PublicationStart::ResumeAfterEnterpriseFailure => {}
        }

        let outcome = publish(&publication, context)
            .map_err(|error| self.cleanup_publication_failure(&publication, error))?;
        Ok(JunitExportOutcome {
            path: self.target.clone(),
            cleanup_warning: outcome.cleanup_warning,
            deferred_interruption: outcome.deferred_interruption,
        })
    }

    fn cleanup_publication_failure(
        &self,
        publication: &StagedPublication,
        error: AppError,
    ) -> AppError {
        let error = publication.cleanup_failure(error);
        match fs::remove_file(&self.target) {
            Ok(()) => error,
            Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => error,
            Err(remove_error) => error.with_context(format!(
                "cleanup failed: failed to remove JUnit export target '{}': {remove_error}",
                self.target.display()
            )),
        }
    }
}

fn open_staged_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().create_new(true).write(true).open(path)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, Write};
    use std::path::Path;

    use tempfile::tempdir;

    use crate::use_cases::context::{CommandName, ExecutionContext};

    use super::{JunitExport, SyncWrite};

    struct FaultWriter {
        file: File,
        write_error: Option<&'static str>,
        sync_error: Option<&'static str>,
    }

    impl Write for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Some(message) = self.write_error {
                return Err(io::Error::other(message));
            }
            self.file.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()
        }
    }

    impl SyncWrite for FaultWriter {
        fn sync_all(&mut self) -> io::Result<()> {
            if let Some(message) = self.sync_error {
                return Err(io::Error::other(message));
            }
            self.file.sync_all()
        }
    }

    #[test]
    fn prepare_creates_parents_and_removes_stale_file() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("nested").join("report.xml");

        let export = JunitExport::prepare(target.clone()).expect("prepare missing parent");

        assert_eq!(export.target(), target);
        assert!(target.parent().expect("parent").is_dir());

        fs::write(&target, b"stale").expect("stale");

        let export = JunitExport::prepare(target.clone()).expect("prepare");

        assert_eq!(export.target(), target);
        assert!(target.parent().expect("parent").is_dir());
        assert!(!target.exists());
    }

    #[test]
    fn prepare_rejects_directory_without_removing_it() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        fs::create_dir(&target).expect("target dir");

        let error = JunitExport::prepare(target.clone()).expect_err("directory rejected");

        assert!(error.to_string().contains("validation error"));
        assert!(target.is_dir());
    }

    #[test]
    fn prepare_rejects_path_without_file_name() {
        let error = JunitExport::prepare(Path::new("").to_path_buf()).expect_err("empty path");

        assert!(error.to_string().contains("validation error"));
    }

    #[test]
    fn publish_preserves_rich_xml_bytes_and_removes_staging_metadata() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");
        let bytes = b"\xef\xbb\xbf<?xml version=\"1.0\"?><testsuite name=\"\xd0\xa2\xd0\xb5\xd1\x81\xd1\x82 &amp; check\">\r\n</testsuite>\r\n";

        let outcome = export
            .publish(&ExecutionContext::cli(CommandName::Test), bytes)
            .expect("publish");

        assert_eq!(outcome.path, target);
        assert_eq!(outcome.cleanup_warning, None);
        assert_eq!(outcome.deferred_interruption, None);
        assert_eq!(fs::read(&target).expect("target bytes"), bytes);
        assert_no_staging_leftovers(dir.path());
    }

    #[test]
    fn publish_after_enterprise_failure_defers_existing_cancellation() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");
        let cancellation = tokio_util::sync::CancellationToken::new();
        cancellation.cancel();
        let context = ExecutionContext::cli(CommandName::Test).with_cancellation(cancellation);

        let outcome = export
            .publish_after_enterprise_failure(&context, b"<testsuite/>")
            .expect("publish completed report");

        assert_eq!(fs::read(target).expect("target"), b"<testsuite/>");
        assert!(outcome.deferred_interruption.is_some());
    }

    #[test]
    fn publish_rejects_empty_bytes_without_leaving_staging_files() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");

        let error = export
            .publish(&ExecutionContext::cli(CommandName::Test), b"")
            .expect_err("empty bytes rejected");

        assert!(error.to_string().contains("empty"));
        assert!(!target.exists());
        assert_no_staging_leftovers(dir.path());
    }

    #[test]
    fn open_failure_cleans_metadata_without_target_or_stage() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");

        let error = export
            .publish_with_ops::<FaultWriter>(
                &ExecutionContext::cli(CommandName::Test),
                b"report",
                |_stage| Err(io::Error::other("injected open failure")),
                |publication, context| {
                    publication.publish_file(context, "failed to publish JUnit export")
                },
                |publication, error| publication.cleanup_failure(error),
            )
            .expect_err("open failure");

        assert!(error.to_string().contains("injected open failure"));
        assert!(!target.exists());
        assert_no_staging_leftovers(dir.path());
    }

    #[test]
    fn write_all_failure_cleans_target_stage_and_metadata() {
        assert_writer_failure_cleanup(Some("injected write failure"), None);
    }

    #[test]
    fn sync_all_failure_cleans_target_stage_and_metadata() {
        assert_writer_failure_cleanup(None, Some("injected sync failure"));
    }

    #[test]
    fn publication_failure_cleans_target_stage_and_metadata() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");

        let error = export
            .publish_with_ops(
                &ExecutionContext::cli(CommandName::Test),
                b"report",
                |stage| {
                    let file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(stage)?;
                    Ok(FaultWriter {
                        file,
                        write_error: None,
                        sync_error: None,
                    })
                },
                |_publication, _context| {
                    fs::write(&target, b"published before fsync failure").expect("target");
                    Err(crate::support::error::AppError::Runtime(
                        "injected publication failure".to_owned(),
                    ))
                },
                |publication, error| publication.cleanup_failure(error),
            )
            .expect_err("publication failure");

        assert!(error.to_string().contains("injected publication failure"));
        assert!(!target.exists());
        assert_no_staging_leftovers(dir.path());
    }

    #[test]
    fn cleanup_failure_is_visible_after_paths_are_removed() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");

        let error = export
            .publish_with_ops::<FaultWriter>(
                &ExecutionContext::cli(CommandName::Test),
                b"report",
                |_stage| Err(io::Error::other("injected open failure")),
                |publication, context| {
                    publication.publish_file(context, "failed to publish JUnit export")
                },
                |publication, error| {
                    publication
                        .cleanup_failure(error)
                        .with_context("injected cleanup failure")
                },
            )
            .expect_err("cleanup failure");

        assert!(error.to_string().contains("injected cleanup failure"));
        assert!(!target.exists());
        assert_no_staging_leftovers(dir.path());
    }

    fn assert_writer_failure_cleanup(
        write_error: Option<&'static str>,
        sync_error: Option<&'static str>,
    ) {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("report.xml");
        let export = JunitExport::prepare(target.clone()).expect("prepare");
        let expected = write_error.or(sync_error).expect("injected failure");

        let error = export
            .publish_with_ops(
                &ExecutionContext::cli(CommandName::Test),
                b"report",
                |stage| {
                    let file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(stage)?;
                    Ok(FaultWriter {
                        file,
                        write_error,
                        sync_error,
                    })
                },
                |publication, context| {
                    publication.publish_file(context, "failed to publish JUnit export")
                },
                |publication, error| publication.cleanup_failure(error),
            )
            .expect_err("writer failure");

        assert!(error.to_string().contains(expected));
        assert!(!target.exists());
        assert_no_staging_leftovers(dir.path());
    }

    fn assert_no_staging_leftovers(parent: &Path) {
        let entries = fs::read_dir(parent)
            .expect("read parent")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert!(entries.iter().all(|path| {
            let name = path.file_name().expect("file name").to_string_lossy();
            !name.contains("junit-stage")
        }));
    }
}
