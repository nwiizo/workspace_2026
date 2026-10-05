//! Chapter 10: validate configuration at startup rather than hardcoding it.
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("APP_PORT must be a decimal integer between 1 and 65535")]
    InvalidPort,
    #[error("APP_PORT must contain valid Unicode")]
    NonUnicodePort,
}

impl AppConfig {
    /// An absent port uses 8080. A present but invalid value is an error.
    pub fn from_port(value: Option<&str>) -> Result<Self, ConfigError> {
        let port = match value {
            None => 8080,
            Some(value) => value
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or(ConfigError::InvalidPort)?,
        };
        Ok(Self { port })
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        match std::env::var("APP_PORT") {
            Ok(value) => Self::from_port(Some(&value)),
            Err(std::env::VarError::NotPresent) => Self::from_port(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(ConfigError::NonUnicodePort),
        }
    }
}
