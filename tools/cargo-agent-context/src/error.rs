use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Usage(String),
    #[error("empty directory or missing Cargo.toml: {0}")]
    EmptyDirectory(PathBuf),
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse Cargo metadata for {path}: {source}")]
    CargoMetadata {
        path: PathBuf,
        source: cargo_metadata::Error,
    },
    #[error("failed to parse TOML in {path}: {source}")]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to parse JSON in {path}: {source}")]
    JsonFile {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid sibling JSON schema in {tool}: {reason}")]
    SiblingSchema { tool: String, reason: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
    #[error(transparent)]
    Core(#[from] design_gate_core::CoreError),
}
