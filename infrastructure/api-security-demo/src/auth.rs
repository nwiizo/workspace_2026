//! Authentication and authorization utilities

use crate::error::AppError;
use crate::models::UserClaims;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};

/// Secret key for HS256 (for demonstration purposes only)
/// In production, use environment variables or secret management
pub const JWT_SECRET: &str = "super-secret-key-for-demonstration-only";

/// JWT issuer
pub const JWT_ISSUER: &str = "secure-apis-rust";

/// JWT audience
pub const JWT_AUDIENCE: &str = "https://api.example.com";

/// Extract bearer token from Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    auth_header.strip_prefix("Bearer ")
}

/// Generate a JWT token with HS256
pub fn generate_token_hs256(claims: &UserClaims) -> Result<String, AppError> {
    let header = Header::new(Algorithm::HS256);
    let token = encode(
        &header,
        claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )?;
    Ok(token)
}

/// Validate a JWT token with HS256 (secure version with full validation)
pub fn validate_token_hs256(token: &str) -> Result<UserClaims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.validate_exp = true;

    let token_data = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Validate a JWT token WITHOUT proper validation (vulnerable version)
/// This demonstrates broken authentication - only checks signature, not claims
pub fn validate_token_vulnerable(token: &str) -> Result<UserClaims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    // VULNERABLE: Not validating issuer, audience, or expiration
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.insecure_disable_signature_validation();

    let token_data = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Extractor for authenticated user claims (secure version)
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub UserClaims);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = extract_auth_from_parts(parts, false);
        async move { result.map(AuthenticatedUser) }
    }
}

/// Extractor for user claims WITHOUT proper validation (vulnerable version)
#[derive(Debug, Clone)]
pub struct VulnerableAuthUser(pub UserClaims);

impl<S> FromRequestParts<S> for VulnerableAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = extract_auth_from_parts(parts, true);
        async move { result.map(VulnerableAuthUser) }
    }
}

/// Extract authentication from request parts
fn extract_auth_from_parts(parts: &Parts, vulnerable: bool) -> Result<UserClaims, AppError> {
    let auth_header = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = extract_bearer_token(auth_header).ok_or(AppError::Unauthorized)?;

    if vulnerable {
        validate_token_vulnerable(token)
    } else {
        validate_token_hs256(token)
    }
}

/// Check if user has admin permission
pub fn is_admin(claims: &UserClaims) -> bool {
    claims.permissions.iter().any(|p| p == "admin")
}

/// Create test token for a regular user
pub fn create_test_user_token(user_id: &str) -> Result<String, AppError> {
    let claims = UserClaims {
        sub: user_id.to_string(),
        permissions: vec!["read".to_string(), "write".to_string()],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        aud: Some(JWT_AUDIENCE.to_string()),
        iss: Some(JWT_ISSUER.to_string()),
    };
    generate_token_hs256(&claims)
}

/// Create test token for an admin user
pub fn create_test_admin_token(user_id: &str) -> Result<String, AppError> {
    let claims = UserClaims {
        sub: user_id.to_string(),
        permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        aud: Some(JWT_AUDIENCE.to_string()),
        iss: Some(JWT_ISSUER.to_string()),
    };
    generate_token_hs256(&claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let claims = UserClaims {
            sub: "user123".to_string(),
            permissions: vec!["read".to_string()],
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
            aud: Some(JWT_AUDIENCE.to_string()),
            iss: Some(JWT_ISSUER.to_string()),
        };

        let token = generate_token_hs256(&claims).unwrap();
        let validated = validate_token_hs256(&token).unwrap();

        assert_eq!(validated.sub, "user123");
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(extract_bearer_token("Bearer abc123"), Some("abc123"));
        assert_eq!(extract_bearer_token("Basic abc123"), None);
    }

    #[test]
    fn test_is_admin() {
        let admin_claims = UserClaims {
            sub: "admin".to_string(),
            permissions: vec!["admin".to_string()],
            ..Default::default()
        };
        let user_claims = UserClaims {
            sub: "user".to_string(),
            permissions: vec!["read".to_string()],
            ..Default::default()
        };

        assert!(is_admin(&admin_claims));
        assert!(!is_admin(&user_claims));
    }
}
