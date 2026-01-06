use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::Claims;
use crate::state::AppState;

/// Middleware to require authentication via JWT or Hydra token
///
/// Extracts and validates the token from the Authorization header.
/// Supports both:
/// - JWT tokens issued by this service
/// - Ory Hydra access tokens (validated via introspection)
///
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

    // Try JWT verification first
    let claims = match state.jwt.verify_access_token(token) {
        Ok(claims) => {
            debug!("JWT token verified successfully");
            claims
        }
        Err(_) => {
            // JWT verification failed, try Hydra introspection
            debug!("JWT verification failed, trying Hydra introspection");

            let introspection = state
                .hydra
                .introspect_token(token)
                .await
                .map_err(|e| {
                    debug!("Hydra introspection failed: {:?}", e);
                    AppError::AuthenticationFailed("Invalid or expired token".to_string())
                })?;

            debug!("Hydra token introspection successful: sub={:?}", introspection.sub);

            // Convert introspection response to Claims
            let sub = introspection.sub.ok_or_else(|| {
                AppError::AuthenticationFailed("Token has no subject".to_string())
            })?;

            // Extract custom claims from ext (set during consent)
            let (email, role, tenant_id) = if let Some(ext) = &introspection.ext {
                (
                    ext.get("email").and_then(|v| v.as_str()).map(String::from),
                    ext.get("role").and_then(|v| v.as_str()).map(String::from),
                    ext.get("tenant_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok()),
                )
            } else {
                (None, None, None)
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize;

            Claims {
                sub,
                exp: introspection.exp.unwrap_or(0) as usize,
                iat: introspection.iat.unwrap_or(now as i64) as usize,
                iss: introspection.iss.unwrap_or_default(),
                aud: introspection.aud.unwrap_or_default(),
                email,
                role,
                tenant_id,
            }
        }
    };

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
