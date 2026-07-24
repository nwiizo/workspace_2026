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
    #[error("no Rust source files found under {0}")]
    NoRustFiles(PathBuf),
    #[error("{0}")]
    Core(#[from] design_gate_core::CoreError),
    #[error("git command failed in {cwd}: {message}")]
    Git { cwd: PathBuf, message: String },
    #[error("baseline analysis failed: {0}")]
    Baseline(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
