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
    #[error("no Rust source files found under {0}")]
    NoRustFiles(PathBuf),
    #[error("{0}")]
    Core(#[from] design_gate_core::CoreError),
    #[error("baseline analysis failed: {0}")]
    Baseline(String),
}
