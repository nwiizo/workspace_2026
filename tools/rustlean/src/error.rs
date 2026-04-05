use thiserror::Error;

pub type Result<T> = std::result::Result<T, RustLeanError>;

#[derive(Error, Debug)]
pub enum RustLeanError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Compiler error: {0}")]
    Compiler(String),

    #[error("Analysis error: {0}")]
    Analysis(String),
}
