use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse llvm-cov JSON {path}: {source}")]
    CoverageJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("llvm-cov file does not exist: {0}")]
    MissingCoverage(PathBuf),
    #[error("no Rust files found under {0}")]
    NoRustFiles(PathBuf),
    #[error("baseline analysis failed: {0}")]
    Baseline(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Core(#[from] design_gate_core::CoreError),
}
