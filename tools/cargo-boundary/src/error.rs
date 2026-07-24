use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BoundaryError>;

#[derive(Debug, Error)]
pub enum BoundaryError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write output: {0}")]
    Write(#[from] std::io::Error),
    #[error("failed to parse TOML in {path}: {source}")]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no Rust source files found under {0}")]
    NoRustFiles(PathBuf),
    #[error("{0}")]
    Core(#[from] design_gate_core::CoreError),
    #[error("git command failed: {0}")]
    Git(String),
}
