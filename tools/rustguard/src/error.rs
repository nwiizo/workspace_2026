use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RustGuardError {
    #[error("configuration error: {message}")]
    Config { message: String },

    #[error("failed to read config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("analysis error: {message}")]
    Analysis { message: String },

    #[error("output error: {message}")]
    Output { message: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RustGuardError>;
