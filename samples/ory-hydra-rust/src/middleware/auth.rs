use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::models::Claims;
use crate::state::AppState;

/// Middleware to require authentication via JWT
///
/// Extracts and validates the JWT token from the Authorization header.
/// On success, adds the Claims to request extensions.
#[instrument(skip(state, request, next))]
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::AuthenticationFailed(
            "Missing Authorization header".to_string(),
        ))?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::AuthenticationFailed(
            "Invalid Authorization header format. Expected: Bearer <token>".to_string(),
        ))?;

    // Verify token
    let claims = state
        .jwt
        .verify_access_token(token)
        .map_err(|_| AppError::AuthenticationFailed("Invalid or expired token".to_string()))?;

    // Add claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extension trait to get claims from request
#[allow(unused)]
pub trait ClaimsExt {
    fn claims(&self) -> Option<&Claims>;
    fn require_claims(&self) -> Result<&Claims, AppError>;
}

impl<B> ClaimsExt for axum::http::Request<B> {
    fn claims(&self) -> Option<&Claims> {
        self.extensions().get::<Claims>()
    }

    fn require_claims(&self) -> Result<&Claims, AppError> {
        self.claims().ok_or(AppError::AuthenticationFailed(
            "Authentication required".to_string(),
        ))
    }
}

/// Extractor for claims in handlers
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let claims =
            parts
                .extensions
                .get::<Claims>()
                .cloned()
                .ok_or(AppError::AuthenticationFailed(
                    "Authentication required".to_string(),
                ))?;

        Ok(AuthUser(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_token_parsing() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = header.strip_prefix("Bearer ");
        assert!(token.is_some());
        assert_eq!(token.unwrap(), "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_invalid_header_format() {
        let header = "Basic dXNlcjpwYXNz";
        let token = header.strip_prefix("Bearer ");
        assert!(token.is_none());
    }
}
