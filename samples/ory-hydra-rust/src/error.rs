use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Hydra API error: {0}")]
    HydraError(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Tenant not found")]
    TenantNotFound,

    #[error("Validation error: {0}")]
    ValidationError(String),
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
            AppError::AuthenticationFailed(msg) => (
                StatusCode::UNAUTHORIZED,
                "AUTH_001",
                "authentication_failed",
                msg.clone(),
            ),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "AUTH_002",
                "invalid_credentials",
                "The provided credentials are invalid".to_string(),
            ),
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "AUTH_003",
                "user_not_found",
                "User not found".to_string(),
            ),
            AppError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, "AUTH_004", "forbidden", msg.clone())
            }
            AppError::HydraError(msg) => (
                StatusCode::BAD_GATEWAY,
                "HYDRA_001",
                "hydra_error",
                msg.clone(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_001",
                "internal_error",
                msg.clone(),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                "bad_request",
                msg.clone(),
            ),
            AppError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_001",
                "database_error",
                msg.clone(),
            ),
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", "not_found", msg.clone())
            }
            AppError::TenantNotFound => (
                StatusCode::NOT_FOUND,
                "TENANT_001",
                "tenant_not_found",
                "Tenant not found".to_string(),
            ),
            AppError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_001",
                "validation_error",
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

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::HydraError(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Internal(format!("JWT error: {}", err))
    }
}

impl From<password_hash::Error> for AppError {
    fn from(err: password_hash::Error) -> Self {
        AppError::Internal(format!("Password hash error: {}", err))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}
