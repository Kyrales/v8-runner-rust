use crate::domain::issue::Issue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxCheckResult {
    pub ok: bool,
    pub check_name: String,
    pub issues: Vec<Issue>,
    pub duration_ms: u64,
}
