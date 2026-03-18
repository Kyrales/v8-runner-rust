use std::path::Path;
use std::process::{Command, Output};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("failed to spawn process '{cmd}': {source}")]
    SpawnFailed { cmd: String, source: std::io::Error },

    #[error("process '{cmd}' failed with exit code {code}")]
    NonZeroExit { cmd: String, code: i32 },
}

#[derive(Debug)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ProcessResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

pub struct ProcessExecutor;

impl ProcessExecutor {
    pub fn run(program: &Path, args: &[&str], workdir: Option<&Path>) -> Result<ProcessResult, ProcessError> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(wd) = workdir {
            cmd.current_dir(wd);
        }

        let output: Output = cmd.output().map_err(|e| ProcessError::SpawnFailed {
            cmd: program.display().to_string(),
            source: e,
        })?;

        let exit_code = output.status.code().unwrap_or(-1);
        Ok(ProcessResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
