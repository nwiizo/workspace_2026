use serde::{Deserialize, Serialize};
use std::fmt;

/// The kind of scan job to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanJob {
    /// Try all transforms on all parameters.
    Full,
    /// Find unnecessary clones that could be replaced with borrow/move.
    CloneAudit,
    /// Find owned types (String, Vec, Box) that could be borrowed (&str, &[T], T).
    ApiSlim,
}

impl ScanJob {
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "clone-audit" => Some(Self::CloneAudit),
            "api-slim" => Some(Self::ApiSlim),
            _ => None,
        }
    }
}

impl fmt::Display for ScanJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full Scan"),
            Self::CloneAudit => write!(f, "Clone Audit"),
            Self::ApiSlim => write!(f, "API Slim"),
        }
    }
}

/// Configuration for the scan engine.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// The kind of scan to perform.
    pub job: ScanJob,
    /// Minimum safety score to include (0-100).
    pub min_score: u32,
    /// Maximum candidates to report.
    pub max_candidates: Option<usize>,
    /// Skip self/&self/&mut self parameters.
    pub skip_self_params: bool,
    /// Skip functions that look like tests.
    pub skip_test_functions: bool,
    /// Filter functions by name substring.
    pub function_filter: Option<String>,
    /// Enable cargo check validation (Phase B).
    pub validate: bool,
    /// Score threshold for validation.
    pub validate_threshold: u32,
    /// Timeout for cargo check in seconds.
    pub cargo_check_timeout_secs: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            job: ScanJob::Full,
            min_score: 0,
            max_candidates: None,
            skip_self_params: true,
            skip_test_functions: true,
            function_filter: None,
            validate: false,
            validate_threshold: 50,
            cargo_check_timeout_secs: 60,
        }
    }
}
