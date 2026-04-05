use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid source code: {0}")]
    ParseError(String),

    #[error("Compilation failed: {0}")]
    CompileError(String),

    #[error("Analysis failed: {0}")]
    AnalysisError(String),

    #[error("Challenge not found: {0}")]
    NotFound(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ParseError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::CompileError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::AnalysisError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Timeout => (StatusCode::REQUEST_TIMEOUT, "Execution timeout".to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
