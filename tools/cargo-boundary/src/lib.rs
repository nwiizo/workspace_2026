pub mod analyzer;
pub mod baseline;
pub mod config;
pub mod error;
pub mod git;
pub mod lints;
pub mod model;
pub mod output;
pub mod parser;
pub mod scoring;

pub use analyzer::{AnalysisOptions, analyze_path};
pub use error::{BoundaryError, Result};
pub use model::{BoundaryReport, Issue, IssueType, Severity};
