use crate::config::loader::ConfigLoadError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error(transparent)]
    Config(#[from] ConfigLoadError),
}
