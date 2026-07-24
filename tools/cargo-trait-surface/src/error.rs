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
    #[error("failed to parse config {path}: {source}")]
    ConfigToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("no Rust files found under {0}")]
    NoRustFiles(PathBuf),
    #[error("baseline analysis failed: {0}")]
    Baseline(String),
    #[error(transparent)]
    Core(#[from] design_gate_core::CoreError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
