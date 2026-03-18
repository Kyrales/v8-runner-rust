use crate::config::model::AppConfig;
use crate::cli::args::SyntaxArgs;
use crate::output::presenter::Presenter;
use crate::support::error::AppError;

pub fn execute(_config: &AppConfig, _args: &SyntaxArgs, _presenter: &Presenter) -> Result<(), AppError> {
    Err(AppError::Runtime("syntax not yet implemented".to_string()))
}
