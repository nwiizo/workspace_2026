use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
