pub mod analyzer;
pub mod baseline;
pub mod blind_spot;
pub mod config;
pub mod error;
pub mod issue;
mod orphan;
pub mod output;
pub(crate) mod parser;
pub mod scoring;

pub use analyzer::{Analysis, analyze_path};
pub use issue::Issue;
