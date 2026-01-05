//! Error types for authentication system
//!
//! ユーザー列挙攻撃対策のため、認証エラーは同一メッセージを返す

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AppError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
