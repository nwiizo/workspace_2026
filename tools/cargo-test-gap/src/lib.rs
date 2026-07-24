mod analyzer;
mod baseline;
mod blind_spot;
mod churn;
mod coverage;
mod error;
mod issue;
mod output;
mod parser;
mod scoring;

pub use analyzer::{Analysis, AnalyzeOptions, analyze_path};
pub use baseline::{BaselineDiff, diff_against_ref};
pub use error::{Error, Result};
pub use issue::{Issue, IssueKey, IssueType, Severity};
pub use output::{GateReport, OutputOptions, write_ai, write_blind_spots, write_json, write_text};
pub use scoring::Grade;
