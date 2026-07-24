mod analyzer;
pub mod baseline;
mod blind_spot;
pub mod config;
pub mod error;
pub mod issue;
pub mod output;
mod parser;
mod scoring;

pub use analyzer::{Analysis, TraitDetail, analyze_path};
pub use config::Config;
pub use error::{Error, Result};
pub use issue::{Issue, IssueType, Severity};
