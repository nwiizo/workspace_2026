//! Error types for probitas-security

use thiserror::Error;

/// Result type alias for probitas-security operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for probitas-security
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing/serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Variable not found in context
    #[error("Variable '{0}' not found in context")]
    VariableNotFound(String),

    /// Extraction failed
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    /// Assertion failed
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    /// Step execution failed
    #[error("Step '{0}' failed: {1}")]
    StepFailed(String, String),

    /// Scenario execution failed
    #[error("Scenario '{0}' failed: {1}")]
    ScenarioFailed(String, String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Regex error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Base64 decode error
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Step skipped (not an error, but used for control flow)
    #[error("Skip: {0}")]
    Skip(String),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new variable not found error
    pub fn var_not_found(name: impl Into<String>) -> Self {
        Self::VariableNotFound(name.into())
    }

    /// Create a new extraction failed error
    pub fn extraction_failed(msg: impl Into<String>) -> Self {
        Self::ExtractionFailed(msg.into())
    }

    /// Create a new assertion failed error
    pub fn assertion_failed(msg: impl Into<String>) -> Self {
        Self::AssertionFailed(msg.into())
    }

    /// Create a new step failed error
    pub fn step_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::StepFailed(name.into(), reason.into())
    }

    /// Create a new scenario failed error
    pub fn scenario_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ScenarioFailed(name.into(), reason.into())
    }

    /// Create a new generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Create a skip "error" (for control flow)
    pub fn skip(reason: impl Into<String>) -> Self {
        Self::Skip(reason.into())
    }

    /// Check if this is a skip
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip(_))
    }

    /// Get the skip reason if this is a skip
    pub fn skip_reason(&self) -> Option<&str> {
        match self {
            Self::Skip(reason) => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::var_not_found("token");
        assert!(err.to_string().contains("token"));

        let err = Error::assertion_failed("status mismatch");
        assert!(err.to_string().contains("status mismatch"));
    }
}
