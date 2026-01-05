use crate::error::AppError;
use crate::models::{Claims, TokenResponse, User};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use tracing::instrument;
use uuid::Uuid;

/// JWT service for token generation and verification
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: Vec<String>,
    access_token_expiry: Duration,
    refresh_token_expiry: Duration,
}

impl JwtService {
    /// Create a new JWT service with the given secret
    pub fn new(secret: &[u8], issuer: String, audience: Vec<String>) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            issuer,
            audience,
            access_token_expiry: Duration::minutes(15),
            refresh_token_expiry: Duration::days(30),
        }
    }

    /// Generate access and refresh tokens for a user
    #[instrument(skip(self, user))]
    pub fn generate_tokens(&self, user: &User) -> Result<TokenResponse, AppError> {
        let access_token = self.generate_access_token(user)?;
        let refresh_token = self.generate_refresh_token(user)?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expiry.num_seconds(),
        })
    }

    /// Generate an access token
    fn generate_access_token(&self, user: &User) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = now
            .checked_add_signed(self.access_token_expiry)
            .ok_or_else(|| AppError::Internal("Failed to calculate expiration".to_string()))?;

        let claims = Claims {
            sub: user.id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            email: Some(user.email.clone()),
            role: Some(user.role.to_string()),
            tenant_id: user.tenant_id,
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(AppError::from)
    }

    /// Generate a refresh token
    fn generate_refresh_token(&self, user: &User) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = now
            .checked_add_signed(self.refresh_token_expiry)
            .ok_or_else(|| AppError::Internal("Failed to calculate expiration".to_string()))?;

        let claims = Claims {
            sub: user.id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            email: None, // Refresh tokens don't need email
            role: Some(user.role.to_string()),
            tenant_id: user.tenant_id,
        };

        // Use a different token ID for refresh tokens
        let header = Header {
            kid: Some(format!("refresh-{}", Uuid::new_v4())),
            ..Default::default()
        };

        encode(&header, &claims, &self.encoding_key).map_err(AppError::from)
    }

    /// Verify and decode an access token
    #[instrument(skip(self, token))]
    #[allow(dead_code)]
    pub fn verify_access_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&self.audience);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        Ok(token_data.claims)
    }

    /// Verify and decode a refresh token
    #[instrument(skip(self, token))]
    pub fn verify_refresh_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&self.audience);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        Ok(token_data.claims)
    }

    /// Extract user ID from a token without full verification (for refresh)
    #[instrument(skip(self, token))]
    #[allow(dead_code)]
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid, AppError> {
        let claims = self.verify_access_token(token)?;
        Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(format!("Invalid user ID in token: {}", e)))
    }
}
