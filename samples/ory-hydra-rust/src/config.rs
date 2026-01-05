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
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-key-change-in-production".to_string()),
            jwt_issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "auth.example.com".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://hydra:secret@localhost:5432/hydra".to_string()),
        }
    }
}
