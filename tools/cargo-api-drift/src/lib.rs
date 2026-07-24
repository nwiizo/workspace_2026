mod analyzer;
mod blind_spot;
mod error;
mod issue;
mod output;
mod parser;
mod scoring;

pub use analyzer::{Analysis, analyze_path};
pub use error::{Error, Result};
pub use issue::{Issue, Severity};
pub use output::{
    OutputOptions, write_ai, write_blind_spots, write_changelog, write_json, write_text,
};
