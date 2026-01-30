//! Multi-protocol client abstraction layer
//!
//! Provides a unified interface for different client types:
//! - HTTP (default)
//! - GraphQL (feature: `graphql`)
//! - SQL (features: `sql-postgres`, `sql-mysql`, `sql-sqlite`)
//! - MongoDB (feature: `mongodb`)
//! - Redis (feature: `redis`)
//!
//! # Example
//! ```ignore
//! use rectitude::clients::{Client, ClientConfig};
//!
//! // HTTP client (default)
//! let http = HttpClient::new("http://localhost:3000")?;
//! let resp = http.execute(request).await?;
//!
//! // SQL client (with feature)
//! #[cfg(feature = "sql-postgres")]
//! let sql = SqlClient::postgres("postgres://localhost/db")?;
//! ```

mod http;

pub use http::{HttpClient, HttpRequest, HttpResponse};

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::fmt::Debug;

/// Result type for client operations
pub type ClientResult<T> = Result<T, ClientError>;

/// Error type for client operations
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request failed
    #[error("Request failed: {0}")]
    Request(String),

    /// Response parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Underlying reqwest error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl ClientError {
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    pub fn request(msg: impl Into<String>) -> Self {
        Self::Request(msg.into())
    }

    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
}

/// Common client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Connection URL or host
    pub url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Additional options
    pub options: HashMap<String, String>,
}

impl ClientConfig {
    /// Create a new config with defaults
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            timeout_secs: 30,
            max_retries: 0,
            options: HashMap::new(),
        }
    }

    /// Set timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set max retries
    pub fn retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Add an option
    pub fn option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("")
    }
}

/// Generic request trait
pub trait Request: Debug + Send + Sync {
    /// Request type identifier
    fn request_type(&self) -> &str;
}

/// Generic response trait
pub trait Response: Debug + Send + Sync {
    /// Check if the response indicates success
    fn is_success(&self) -> bool;

    /// Get the response body as bytes
    fn body(&self) -> &[u8];

    /// Get the response body as string
    fn text(&self) -> Option<&str>;
}

/// Base trait for all clients
#[async_trait]
pub trait Client: Send + Sync {
    /// The request type this client accepts
    type Request: Request;

    /// The response type this client returns
    type Response: Response;

    /// Get the client type name
    fn client_type(&self) -> &str;

    /// Check if the client is connected
    async fn is_connected(&self) -> bool;

    /// Execute a request
    async fn execute(&self, request: Self::Request) -> ClientResult<Self::Response>;

    /// Close the connection
    async fn close(&self) -> ClientResult<()>;
}

/// Trait for clients that support transactions
#[async_trait]
pub trait Transactional: Client {
    /// Begin a transaction
    async fn begin(&self) -> ClientResult<()>;

    /// Commit the current transaction
    async fn commit(&self) -> ClientResult<()>;

    /// Rollback the current transaction
    async fn rollback(&self) -> ClientResult<()>;
}

/// Trait for clients that support querying with results
#[async_trait]
pub trait Queryable: Client {
    /// Execute a query and return typed results
    async fn query<T: DeserializeOwned + Send>(&self, query: &str) -> ClientResult<Vec<T>>;

    /// Execute a query and return a single result
    async fn query_one<T: DeserializeOwned + Send>(&self, query: &str) -> ClientResult<Option<T>>;
}

/// Trait for clients that support command execution
#[async_trait]
pub trait Executable: Client {
    /// Execute a command that returns affected row count
    async fn execute_cmd(&self, command: &str) -> ClientResult<u64>;

    /// Execute a command with parameters
    async fn execute_with_params<P: Serialize + Send + Sync>(
        &self,
        command: &str,
        params: &P,
    ) -> ClientResult<u64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("http://localhost:3000")
            .timeout(60)
            .retries(3)
            .option("verify_ssl", "false");

        assert_eq!(config.url, "http://localhost:3000");
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.options.get("verify_ssl"), Some(&"false".to_string()));
    }

    #[test]
    fn test_client_error() {
        let err = ClientError::connection("failed to connect");
        assert!(err.to_string().contains("Connection error"));
    }
}
