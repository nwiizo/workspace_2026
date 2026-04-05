use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustMorphError {
    #[error("failed to parse file: {path}: {reason}")]
    Parse { path: String, reason: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, RustMorphError>;
