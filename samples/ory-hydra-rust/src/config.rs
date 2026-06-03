use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Hydra Admin API URL
    pub hydra_admin_url: String,
    /// Hydra Public API URL
    pub hydra_public_url: String,
    /// Browser-visible Hydra Public API URL for authorization redirects
    pub bff_authorization_url: String,
    /// OAuth client ID used by the BFF
    pub bff_client_id: String,
    /// OAuth client secret used by the BFF
    pub bff_client_secret: String,
    /// OAuth redirect URI handled by the BFF
    pub bff_redirect_uri: String,
    /// Browser SPA origin allowed to call the BFF
    pub bff_frontend_origin: String,
    /// Upstream API origin that the BFF is allowed to proxy to
    pub bff_api_upstream_url: String,
    /// JWT secret key
    pub jwt_secret: String,
    /// JWT issuer
    pub jwt_issuer: String,
    /// Database URL
    pub database_url: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            hydra_admin_url: env::var("HYDRA_ADMIN_URL")
                .unwrap_or_else(|_| "http://localhost:4445".to_string()),
            hydra_public_url: env::var("HYDRA_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:4444".to_string()),
            bff_authorization_url: env::var("BFF_AUTHORIZATION_URL")
                .unwrap_or_else(|_| "http://localhost:4444".to_string()),
            bff_client_id: env::var("BFF_CLIENT_ID")
                .or_else(|_| env::var("OAUTH_CLIENT_ID"))
                .unwrap_or_else(|_| "demo-client".to_string()),
            bff_client_secret: env::var("BFF_CLIENT_SECRET")
                .or_else(|_| env::var("OAUTH_CLIENT_SECRET"))
                .unwrap_or_else(|_| "demo-secret".to_string()),
            bff_redirect_uri: env::var("BFF_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:3000/api/bff/callback".to_string()),
            bff_frontend_origin: env::var("BFF_FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            bff_api_upstream_url: env::var("BFF_API_UPSTREAM_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-key-change-in-production".to_string()),
            jwt_issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "auth.example.com".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://hydra:secret@localhost:5432/hydra".to_string()),
        }
    }
}
