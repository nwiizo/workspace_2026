use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FaultForgeError {
    #[error("topology error: {message}")]
    Topology { message: String },

    #[error("failed to read {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse YAML {path}: {source}")]
    YamlParse {
        path: PathBuf,
        source: serde_yml::Error,
    },

    #[error("validation error: {0}")]
    Validation(String),

    #[error("component not found: {0}")]
    ComponentNotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FaultForgeError>;
