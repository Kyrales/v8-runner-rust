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

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Validation(_) | AppError::Config(_) => {
                crate::output::exit_codes::VALIDATION_ERROR
            }
            AppError::Runtime(_) => crate::output::exit_codes::RUNTIME_ERROR,
            AppError::Platform(_) => crate::output::exit_codes::PLATFORM_ERROR,
        }
    }
}
