use crate::cli::args::BuildArgs;
use crate::config::model::AppConfig;
use crate::output::presenter::Presenter;
use crate::support::error::AppError;

pub fn execute(
    _config: &AppConfig,
    _args: &BuildArgs,
    _presenter: &Presenter,
) -> Result<(), AppError> {
    Err(AppError::Runtime("build not yet implemented".to_string()))
}
