//! Error types for authentication system
//!
//! ユーザー列挙攻撃対策のため、認証エラーは同一メッセージを返す
//! OAuth2仕様 (RFC 6749) に沿ったエラーレスポンス形式を採用

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AppError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Hydra API error: {0}")]
    HydraError(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
    error_code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, error, description) = match &self {
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "AUTH_002",
                "invalid_credentials",
                "The provided credentials are invalid".to_string(),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                "bad_request",
                msg.clone(),
            ),
            AppError::UserAlreadyExists => (
                StatusCode::CONFLICT,
                "AUTH_003",
                "user_exists",
                "A user with this email already exists".to_string(),
            ),
            AppError::HydraError(msg) => (
                StatusCode::BAD_GATEWAY,
                "HYDRA_001",
                "hydra_error",
                msg.clone(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
                "internal_error",
                msg.clone(),
            ),
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            error_description: description,
            error_code: error_code.to_string(),
        });

        (status, body).into_response()
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
