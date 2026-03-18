use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LocatorError {
    #[error("platform binary not found: {0}")]
    NotFound(String),
}

pub enum PlatformBinary {
    Designer,
    Enterprise,
    Ibcmd,
    EdtCli,
}

pub fn locate(
    binary: PlatformBinary,
    explicit_path: Option<&PathBuf>,
) -> Result<PathBuf, LocatorError> {
    if let Some(p) = explicit_path {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    let name = match binary {
        PlatformBinary::Designer => "1cv8",
        PlatformBinary::Enterprise => "1cv8c",
        PlatformBinary::Ibcmd => "ibcmd",
        PlatformBinary::EdtCli => "1cedtcli",
    };

    // Try PATH
    if let Ok(path) = which(name) {
        return Ok(path);
    }

    Err(LocatorError::NotFound(name.to_string()))
}

fn which(name: &str) -> Result<PathBuf, ()> {
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths).find_map(|dir| {
                let candidate = dir.join(name);
                if candidate.exists() {
                    Some(candidate)
                } else {
                    None
                }
            })
        })
        .ok_or(())
}
