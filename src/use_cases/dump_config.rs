use crate::cli::args::DumpArgs;
use crate::config::model::AppConfig;
use crate::output::presenter::Presenter;
use crate::support::error::AppError;

pub fn execute(
    _config: &AppConfig,
    _args: &DumpArgs,
    _presenter: &Presenter,
) -> Result<(), AppError> {
    Err(AppError::Runtime("dump not yet implemented".to_string()))
}
