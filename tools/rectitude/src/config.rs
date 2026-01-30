//! Configuration module for Rectitude
//!
//! Provides configuration file loading and environment variable support.
//!
//! # Tag Filtering
//!
//! Tags support advanced filtering with AND logic and NOT prefix:
//! - `"sqli,auth"` - AND logic: scenario must have ALL tags
//! - `"!slow"` - NOT: scenario must NOT have this tag
//! - `"sqli,!flaky"` - Combined: must have sqli AND must not have flaky
//!
//! ## Examples
//! ```ignore
//! // Parse from CLI flag
//! let filter = TagFilter::parse("sqli,auth,!slow");
//!
//! // Check if scenario matches
//! filter.matches(&["sqli", "auth", "fast"]); // true
//! filter.matches(&["sqli", "slow"]);          // false (has excluded tag)
//! filter.matches(&["sqli"]);                  // false (missing "auth")
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

    /// Create a TagFilter from the config's include/exclude tags
    pub fn tag_filter(&self) -> TagFilter {
        let mut filter = TagFilter::new();

        if let Some(include) = &self.include_tags {
            for tag in include {
                filter = filter.require(tag);
            }
        }

        if let Some(exclude) = &self.exclude_tags {
            for tag in exclude {
                filter = filter.exclude(tag);
            }
        }

        filter
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

/// Advanced tag filter with AND logic and NOT support
///
/// # Example
/// ```
/// use rectitude::config::TagFilter;
///
/// let filter = TagFilter::parse("sqli,auth,!slow,!flaky");
///
/// // Must have sqli AND auth, must NOT have slow or flaky
/// assert!(filter.matches(&["sqli", "auth", "fast"]));
/// assert!(!filter.matches(&["sqli", "slow"]));  // has excluded tag
/// assert!(!filter.matches(&["sqli"]));          // missing required tag
/// ```
#[derive(Debug, Clone, Default)]
pub struct TagFilter {
    /// Tags that MUST be present (AND logic)
    pub required: HashSet<String>,
    /// Tags that MUST NOT be present
    pub excluded: HashSet<String>,
    /// Tags where at least one must be present (OR logic within the group)
    pub any_of: Vec<HashSet<String>>,
}

impl TagFilter {
    /// Create a new empty filter (matches everything)
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a filter string
    ///
    /// Format: `"tag1,tag2,!excluded1,!excluded2"`
    /// - Tags without prefix are required (AND logic)
    /// - Tags with `!` prefix are excluded
    /// - Tags separated by `|` are OR'd together
    ///
    /// Examples:
    /// - `"sqli,auth"` - must have both sqli AND auth
    /// - `"sqli,!slow"` - must have sqli AND must not have slow
    /// - `"sqli|xss"` - must have sqli OR xss
    pub fn parse(filter_str: &str) -> Self {
        let mut filter = Self::new();

        if filter_str.is_empty() {
            return filter;
        }

        for part in filter_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Check for OR groups (pipe-separated)
            if part.contains('|') {
                let or_tags: HashSet<String> = part
                    .split('|')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                if !or_tags.is_empty() {
                    filter.any_of.push(or_tags);
                }
                continue;
            }

            // Check for NOT prefix
            if let Some(excluded_tag) = part.strip_prefix('!') {
                if !excluded_tag.is_empty() {
                    filter.excluded.insert(excluded_tag.to_string());
                }
            } else {
                filter.required.insert(part.to_string());
            }
        }

        filter
    }

    /// Add a required tag (AND logic)
    pub fn require(mut self, tag: impl Into<String>) -> Self {
        self.required.insert(tag.into());
        self
    }

    /// Add an excluded tag
    pub fn exclude(mut self, tag: impl Into<String>) -> Self {
        self.excluded.insert(tag.into());
        self
    }

    /// Add an OR group (at least one tag must match)
    pub fn any_of_tags(mut self, tags: &[&str]) -> Self {
        let set: HashSet<String> = tags.iter().map(|t| t.to_string()).collect();
        if !set.is_empty() {
            self.any_of.push(set);
        }
        self
    }

    /// Check if the filter is empty (matches everything)
    pub fn is_empty(&self) -> bool {
        self.required.is_empty() && self.excluded.is_empty() && self.any_of.is_empty()
    }

    /// Check if scenario tags match this filter
    pub fn matches(&self, scenario_tags: &[&str]) -> bool {
        let tags: HashSet<&str> = scenario_tags.iter().copied().collect();

        // Check excluded tags first (any match = fail)
        for excluded in &self.excluded {
            if tags.contains(excluded.as_str()) {
                return false;
            }
        }

        // Check required tags (all must be present)
        for required in &self.required {
            if !tags.contains(required.as_str()) {
                return false;
            }
        }

        // Check any_of groups (at least one from each group must be present)
        for or_group in &self.any_of {
            let has_any = or_group.iter().any(|t| tags.contains(t.as_str()));
            if !has_any {
                return false;
            }
        }

        true
    }

    /// Check if scenario tags match (String slice version)
    pub fn matches_strings(&self, scenario_tags: &[String]) -> bool {
        let as_strs: Vec<&str> = scenario_tags.iter().map(|s| s.as_str()).collect();
        self.matches(&as_strs)
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

    #[test]
    fn test_tag_filter_parse_required() {
        let filter = TagFilter::parse("sqli,auth");
        assert!(filter.matches(&["sqli", "auth", "fast"]));
        assert!(!filter.matches(&["sqli"])); // missing auth
        assert!(!filter.matches(&["auth"])); // missing sqli
    }

    #[test]
    fn test_tag_filter_parse_excluded() {
        let filter = TagFilter::parse("!slow,!flaky");
        assert!(filter.matches(&["fast", "reliable"]));
        assert!(!filter.matches(&["slow"]));
        assert!(!filter.matches(&["flaky"]));
    }

    #[test]
    fn test_tag_filter_parse_combined() {
        let filter = TagFilter::parse("sqli,auth,!slow,!flaky");
        assert!(filter.matches(&["sqli", "auth", "fast"]));
        assert!(!filter.matches(&["sqli", "auth", "slow"])); // has excluded
        assert!(!filter.matches(&["sqli", "fast"])); // missing auth
    }

    #[test]
    fn test_tag_filter_parse_or_groups() {
        let filter = TagFilter::parse("sqli|xss,auth");
        assert!(filter.matches(&["sqli", "auth"]));
        assert!(filter.matches(&["xss", "auth"]));
        assert!(!filter.matches(&["auth"])); // missing sqli or xss
        assert!(!filter.matches(&["sqli"])); // missing auth
    }

    #[test]
    fn test_tag_filter_empty() {
        let filter = TagFilter::new();
        assert!(filter.is_empty());
        assert!(filter.matches(&["any", "tags"]));
        assert!(filter.matches(&[]));
    }

    #[test]
    fn test_tag_filter_builder() {
        let filter = TagFilter::new()
            .require("sqli")
            .require("auth")
            .exclude("slow")
            .any_of_tags(&["web", "api"]);

        assert!(filter.matches(&["sqli", "auth", "web"]));
        assert!(filter.matches(&["sqli", "auth", "api"]));
        assert!(!filter.matches(&["sqli", "auth"])); // missing web or api
        assert!(!filter.matches(&["sqli", "auth", "slow", "web"])); // has excluded
    }

    #[test]
    fn test_config_tag_filter() {
        let config = RectitudeConfig {
            include_tags: Some(vec!["security".to_string(), "auth".to_string()]),
            exclude_tags: Some(vec!["slow".to_string()]),
            ..Default::default()
        };

        let filter = config.tag_filter();
        assert!(filter.matches(&["security", "auth"]));
        assert!(!filter.matches(&["security"])); // missing auth
        assert!(!filter.matches(&["security", "auth", "slow"])); // has excluded
    }
}
