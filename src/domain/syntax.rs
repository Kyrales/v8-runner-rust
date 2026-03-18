use serde::{Deserialize, Serialize};
use crate::domain::issue::Issue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxCheckResult {
    pub ok: bool,
    pub check_name: String,
    pub issues: Vec<Issue>,
    pub duration_ms: u64,
}
