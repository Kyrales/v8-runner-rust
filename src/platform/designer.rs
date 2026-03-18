use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::platform::process::{ProcessExecutor, ProcessResult, RunOptions};

#[derive(Debug, Error)]
pub enum DesignerError {
    #[error("designer binary not found: {0}")]
    BinaryNotFound(String),

    #[error("designer process failed (exit {code}): {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("failed to spawn designer: {0}")]
    Spawn(#[from] crate::platform::process::ProcessError),
}

/// Connection parameters passed to every Designer invocation.
#[derive(Debug, Clone)]
pub struct DesignerConnection {
    /// Tokenized connection arguments such as `["/F", "/path/to/ib"]`.
    pub connection_args: Vec<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl DesignerConnection {
    /// Build from a raw connection string (e.g. `File=/path/to/ib`).
    pub fn from_connection_string(s: &str) -> Self {
        // Normalise: if it already starts with /F or /S pass through,
        // otherwise wrap as /IBConnectionString.
        let connection_args = if s.starts_with('/') {
            split_arg_string(s)
        } else {
            vec!["/IBConnectionString".to_owned(), s.to_owned()]
        };
        Self {
            connection_args,
            user: None,
            password: None,
        }
    }

    fn args(&self) -> Vec<String> {
        let mut args = self.connection_args.clone();
        if let Some(u) = &self.user {
            args.push("/N".to_owned());
            args.push(u.clone());
        }
        if let Some(p) = &self.password {
            args.push("/P".to_owned());
            args.push(p.clone());
        }
        args
    }
}

fn split_arg_string(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in raw.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Low-level DSL for invoking 1cv8 in DESIGNER (batch) mode.
///
/// Each method maps to one Designer batch command from `spec/designer-spec.md`.
pub struct DesignerDsl {
    binary: PathBuf,
    connection: DesignerConnection,
    /// Optional file to capture Designer's `/Out` log.
    log_file: Option<PathBuf>,
}

impl DesignerDsl {
    pub fn new(binary: PathBuf, connection: DesignerConnection, log_file: Option<PathBuf>) -> Self {
        Self {
            binary,
            connection,
            log_file,
        }
    }

    /// `/LoadConfigFromFiles <dir> -updateConfigDumpInfo`
    ///
    /// Full load of all sources from `source_dir`.
    pub fn load_config_from_files_full(
        &self,
        source_dir: &Path,
        extension: Option<&str>,
    ) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/LoadConfigFromFiles".to_owned());
        args.push(source_dir.display().to_string());
        args.push("-updateConfigDumpInfo".to_owned());
        if let Some(ext) = extension {
            args.push("-Extension".to_owned());
            args.push(ext.to_owned());
        }
        self.run(&args)
    }

    /// `/LoadConfigFromFiles <dir> -partial -listFile <list_file> -updateConfigDumpInfo`
    ///
    /// Partial load using a pre-written list file.
    pub fn load_config_from_files_partial(
        &self,
        source_dir: &Path,
        list_file: &Path,
        extension: Option<&str>,
    ) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/LoadConfigFromFiles".to_owned());
        args.push(source_dir.display().to_string());
        args.push("-partial".to_owned());
        args.push("-listFile".to_owned());
        args.push(list_file.display().to_string());
        args.push("-updateConfigDumpInfo".to_owned());
        if let Some(ext) = extension {
            args.push("-Extension".to_owned());
            args.push(ext.to_owned());
        }
        self.run(&args)
    }

    /// `/UpdateDBCfg`
    ///
    /// Apply the loaded configuration to the database.
    pub fn update_db_cfg(&self, extension: Option<&str>) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/UpdateDBCfg".to_owned());
        if let Some(ext) = extension {
            args.push("-Extension".to_owned());
            args.push(ext.to_owned());
        }
        self.run(&args)
    }

    /// `/DumpConfigToFiles <dir> [-Extension <name>]`
    ///
    /// Full dump of configuration to XML files.
    pub fn dump_config_to_files(
        &self,
        target_dir: &Path,
        extension: Option<&str>,
    ) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/DumpConfigToFiles".to_owned());
        args.push(target_dir.display().to_string());
        if let Some(ext) = extension {
            args.push("-Extension".to_owned());
            args.push(ext.to_owned());
        }
        self.run(&args)
    }

    /// `/CheckConfig [-ThinClient] [-Server] ...`
    pub fn check_config(&self, flags: &[&str]) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/CheckConfig".to_owned());
        for flag in flags {
            args.push(flag.to_string());
        }
        self.run(&args)
    }

    /// `/CheckModules [-ThinClient] [-Server] ...`
    pub fn check_modules(&self, flags: &[&str]) -> Result<ProcessResult, DesignerError> {
        let mut args = self.base_args();
        args.push("/CheckModules".to_owned());
        for flag in flags {
            args.push(flag.to_string());
        }
        self.run(&args)
    }

    // ── internals ────────────────────────────────────────────────────────────

    fn base_args(&self) -> Vec<String> {
        let mut args = vec!["DESIGNER".to_owned()];
        args.push("/DisableStartupDialogs".to_owned());
        args.push("/DisableStartupMessages".to_owned());
        args.extend(self.connection.args());
        if let Some(log) = &self.log_file {
            args.push("/Out".to_owned());
            args.push(log.display().to_string());
            args.push("-NoTruncate".to_owned());
        }
        args
    }

    fn run(&self, args: &[String]) -> Result<ProcessResult, DesignerError> {
        let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
        let opts = RunOptions {
            workdir: None,
            stdout_log: None,
            stderr_log: None,
        };
        let result = ProcessExecutor::run(&self.binary, &str_args, opts)?;
        if !result.success() {
            return Err(DesignerError::ProcessFailed {
                code: result.exit_code,
                stderr: result.stderr.clone(),
            });
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::DesignerConnection;

    #[test]
    fn wraps_plain_connection_string_as_flag_and_value() {
        let connection = DesignerConnection::from_connection_string("File=/tmp/ib");

        assert_eq!(
            connection.args(),
            vec!["/IBConnectionString", "File=/tmp/ib"]
        );
    }

    #[test]
    fn splits_raw_connection_and_auth_into_separate_tokens() {
        let mut connection = DesignerConnection::from_connection_string("/F \"/tmp/my ib\"");
        connection.user = Some("alice".to_owned());
        connection.password = Some("secret".to_owned());

        assert_eq!(
            connection.args(),
            vec!["/F", "/tmp/my ib", "/N", "alice", "/P", "secret"]
        );
    }
}
