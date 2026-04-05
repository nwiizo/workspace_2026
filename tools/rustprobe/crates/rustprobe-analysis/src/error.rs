use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("no probe data found in {0}")]
    NoData(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
