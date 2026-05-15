#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json codec failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("non-success status: {0}")]
    Status(reqwest::StatusCode),
}
