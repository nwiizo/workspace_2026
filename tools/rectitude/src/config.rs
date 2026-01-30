//! Configuration module for Rectitude
//!
//! Provides configuration file loading and environment variable support.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main configuration struct for Rectitude
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RectitudeConfig {
    /// Base URL for all requests
    pub base_url: Option<String>,

    /// Request timeout in seconds
    pub timeout: Option<u64>,

    /// Tags to include when running scenarios
    pub include_tags: Option<Vec<String>>,

    /// Tags to exclude when running scenarios
    pub exclude_tags: Option<Vec<String>>,

    /// Output format: "text" or "json"
    pub output: Option<String>,

    /// Custom variables available in scenarios
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

impl RectitudeConfig {
    /// Load configuration from the current directory
    ///
    /// Searches for:
    /// 1. `rectitude.toml`
    /// 2. `.rectitude.toml`
    ///
    /// Returns default config if no file is found.
    pub fn load() -> Result<Self> {
        Self::load_from_paths(&["rectitude.toml", ".rectitude.toml"])
    }

    /// Load configuration from specified paths
    pub fn load_from_paths(paths: &[&str]) -> Result<Self> {
        for path in paths {
            if Path::new(path).exists() {
                let content = std::fs::read_to_string(path)?;
                return Self::from_toml(&content);
            }
        }
        Ok(Self::default())
    }

    /// Parse configuration from TOML string
    pub fn from_toml(content: &str) -> Result<Self> {
        toml::from_str(content)
            .map_err(|e| Error::InvalidConfig(format!("TOML parse error: {}", e)))
    }

    /// Get the timeout with default fallback
    pub fn timeout_or_default(&self) -> u64 {
        self.timeout.unwrap_or(30)
    }

    /// Get output format with default fallback
    pub fn output_or_default(&self) -> &str {
        self.output.as_deref().unwrap_or("text")
    }

    /// Check if a scenario should be run based on tags
    pub fn should_run_scenario(&self, scenario_tags: &[String]) -> bool {
        // Check exclude tags first
        if let Some(exclude) = &self.exclude_tags {
            for tag in scenario_tags {
                if exclude.contains(tag) {
                    return false;
                }
            }
        }

        // If include tags specified, at least one must match
        if let Some(include) = &self.include_tags {
            if include.is_empty() {
                return true;
            }
            for tag in scenario_tags {
                if include.contains(tag) {
                    return true;
                }
            }
            return false;
        }

        true
    }

    /// Get a variable value
    pub fn get_var(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// Generate default configuration template
    pub fn template() -> &'static str {
        r#"# Rectitude Configuration

# Base URL for all requests
# base_url = "http://localhost:3000"

# Request timeout in seconds
timeout = 30

# Output format: "text" or "json"
output = "text"

# Tags to include (only run scenarios with these tags)
# include_tags = ["security", "auth"]

# Tags to exclude (skip scenarios with these tags)
# exclude_tags = ["slow", "integration"]

# Custom variables available in scenarios
[variables]
# API_KEY = "test-api-key"
# ADMIN_EMAIL = "admin@example.com"
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RectitudeConfig::default();
        assert!(config.base_url.is_none());
        assert_eq!(config.timeout_or_default(), 30);
        assert_eq!(config.output_or_default(), "text");
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
base_url = "http://localhost:3000"
timeout = 60
output = "json"

[variables]
API_KEY = "secret"
"#;
        let config = RectitudeConfig::from_toml(toml).unwrap();
        assert_eq!(config.base_url, Some("http://localhost:3000".to_string()));
        assert_eq!(config.timeout, Some(60));
        assert_eq!(config.output, Some("json".to_string()));
        assert_eq!(config.get_var("API_KEY"), Some(&"secret".to_string()));
    }

    #[test]
    fn test_should_run_scenario() {
        let config = RectitudeConfig {
            include_tags: Some(vec!["security".to_string()]),
            exclude_tags: Some(vec!["slow".to_string()]),
            ..Default::default()
        };

        // Should run: has included tag
        assert!(config.should_run_scenario(&["security".to_string(), "auth".to_string()]));

        // Should not run: has excluded tag
        assert!(!config.should_run_scenario(&["security".to_string(), "slow".to_string()]));

        // Should not run: no matching include tag
        assert!(!config.should_run_scenario(&["auth".to_string()]));
    }
}
