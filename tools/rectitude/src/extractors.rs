//! Data extraction utilities for response analysis
//!
//! Provides tools to extract data from HTTP responses:
//! - JSON path extraction
//! - Header extraction
//! - Regex-based extraction
//! - Cookie extraction

use crate::client::SecurityResponse;
use crate::error::{Error, Result};
use regex::Regex;
use serde_json::Value;

/// Trait for extracting data from responses
pub trait Extractor {
    /// Extract a string value
    fn extract_string(&self, response: &SecurityResponse) -> Result<String>;

    /// Extract multiple string values
    fn extract_all(&self, response: &SecurityResponse) -> Result<Vec<String>>;
}

/// JSON path extractor
pub struct JsonExtractor {
    path: String,
}

impl JsonExtractor {
    /// Create a new JSON extractor with the given path
    ///
    /// # Example
    /// ```
    /// use rectitude::extractors::JsonExtractor;
    ///
    /// let extractor = JsonExtractor::new("$.authentication.token");
    /// ```
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Extract value from JSON using path
    pub fn extract(&self, value: &Value) -> Result<Value> {
        extract_json_path(value, &self.path)
    }
}

impl Extractor for JsonExtractor {
    fn extract_string(&self, response: &SecurityResponse) -> Result<String> {
        let json = response.json_value()?;
        let value = self.extract(&json)?;

        match value {
            Value::String(s) => Ok(s),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            _ => Ok(value.to_string()),
        }
    }

    fn extract_all(&self, response: &SecurityResponse) -> Result<Vec<String>> {
        let json = response.json_value()?;
        let value = self.extract(&json)?;

        match value {
            Value::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s),
                    _ => Ok(v.to_string()),
                })
                .collect(),
            _ => Ok(vec![self.extract_string(response)?]),
        }
    }
}

/// Regex-based extractor
pub struct RegexExtractor {
    pattern: Regex,
    group: usize,
}

impl RegexExtractor {
    /// Create a new regex extractor
    ///
    /// # Arguments
    /// * `pattern` - Regular expression pattern
    /// * `group` - Capture group to extract (0 for full match)
    pub fn new(pattern: &str, group: usize) -> Result<Self> {
        Ok(Self {
            pattern: Regex::new(pattern)?,
            group,
        })
    }

    /// Create an extractor for the first capture group
    pub fn first_group(pattern: &str) -> Result<Self> {
        Self::new(pattern, 1)
    }
}

impl Extractor for RegexExtractor {
    fn extract_string(&self, response: &SecurityResponse) -> Result<String> {
        let text = response.text();
        let caps = self.pattern.captures(text).ok_or_else(|| {
            Error::extraction_failed(format!("Pattern '{}' not found", self.pattern))
        })?;

        caps.get(self.group)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| {
                Error::extraction_failed(format!("Capture group {} not found", self.group))
            })
    }

    fn extract_all(&self, response: &SecurityResponse) -> Result<Vec<String>> {
        let text = response.text();
        Ok(self
            .pattern
            .captures_iter(text)
            .filter_map(|caps| caps.get(self.group).map(|m| m.as_str().to_string()))
            .collect())
    }
}

/// Header extractor
pub struct HeaderExtractor {
    name: String,
}

impl HeaderExtractor {
    /// Create a new header extractor
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_lowercase(),
        }
    }
}

impl Extractor for HeaderExtractor {
    fn extract_string(&self, response: &SecurityResponse) -> Result<String> {
        response
            .header(&self.name)
            .map(|s| s.to_string())
            .ok_or_else(|| Error::extraction_failed(format!("Header '{}' not found", self.name)))
    }

    fn extract_all(&self, response: &SecurityResponse) -> Result<Vec<String>> {
        Ok(vec![self.extract_string(response)?])
    }
}

/// Cookie extractor
pub struct CookieExtractor {
    name: String,
}

impl CookieExtractor {
    /// Create a new cookie extractor
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Extractor for CookieExtractor {
    fn extract_string(&self, response: &SecurityResponse) -> Result<String> {
        // Parse Set-Cookie header
        if let Some(set_cookie) = response.header("set-cookie") {
            for cookie in set_cookie.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == self.name {
                    return Ok(parts[1].to_string());
                }
            }
        }

        Err(Error::extraction_failed(format!(
            "Cookie '{}' not found",
            self.name
        )))
    }

    fn extract_all(&self, response: &SecurityResponse) -> Result<Vec<String>> {
        Ok(vec![self.extract_string(response)?])
    }
}

/// Simple JSON path extractor
fn extract_json_path(value: &Value, path: &str) -> Result<Value> {
    let parts: Vec<&str> = path
        .trim_start_matches('$')
        .split('.')
        .filter(|s| !s.is_empty())
        .collect();

    let mut current = value.clone();
    for part in parts {
        // Handle array index: [0], [1], etc.
        if part.contains('[') {
            let idx_start = part.find('[').unwrap();
            let key = &part[..idx_start];

            // First get the key if present
            if !key.is_empty() {
                current = current
                    .get(key)
                    .cloned()
                    .ok_or_else(|| Error::extraction_failed(format!("Key '{}' not found", key)))?;
            }

            // Then handle indices
            let idx_part = &part[idx_start..];
            for idx_match in idx_part.split('[').skip(1) {
                if let Some(idx_str) = idx_match.strip_suffix(']')
                    && let Ok(idx) = idx_str.parse::<usize>()
                {
                    current = current.get(idx).cloned().ok_or_else(|| {
                        Error::extraction_failed(format!("Index {} out of bounds", idx))
                    })?;
                }
            }
            continue;
        }

        current = current
            .get(part)
            .cloned()
            .ok_or_else(|| Error::extraction_failed(format!("Key '{}' not found", part)))?;
    }

    Ok(current)
}

/// Extract builder for fluent API
pub struct ExtractBuilder<'a> {
    response: &'a SecurityResponse,
}

impl<'a> ExtractBuilder<'a> {
    /// Create a new extract builder
    pub fn new(response: &'a SecurityResponse) -> Self {
        Self { response }
    }

    /// Extract using JSON path
    pub fn json_path(&self, path: &str) -> Result<String> {
        JsonExtractor::new(path).extract_string(self.response)
    }

    /// Extract using regex
    pub fn regex(&self, pattern: &str, group: usize) -> Result<String> {
        RegexExtractor::new(pattern, group)?.extract_string(self.response)
    }

    /// Extract header value
    pub fn header(&self, name: &str) -> Result<String> {
        HeaderExtractor::new(name).extract_string(self.response)
    }

    /// Extract cookie value
    pub fn cookie(&self, name: &str) -> Result<String> {
        CookieExtractor::new(name).extract_string(self.response)
    }

    /// Extract all matches using regex
    pub fn regex_all(&self, pattern: &str, group: usize) -> Result<Vec<String>> {
        RegexExtractor::new(pattern, group)?.extract_all(self.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_response(body: &str) -> SecurityResponse {
        SecurityResponse {
            status: reqwest::StatusCode::OK,
            headers: [
                ("content-type".to_string(), "application/json".to_string()),
                ("x-custom".to_string(), "test-value".to_string()),
            ]
            .into_iter()
            .collect(),
            body: body.as_bytes().to_vec(),
            text: Some(body.to_string()),
        }
    }

    #[test]
    fn test_json_extractor() {
        let response = mock_response(r#"{"auth": {"token": "abc123"}}"#);
        let extractor = JsonExtractor::new("$.auth.token");
        assert_eq!(extractor.extract_string(&response).unwrap(), "abc123");
    }

    #[test]
    fn test_regex_extractor() {
        let response = mock_response("Token: abc123, User: admin");
        let extractor = RegexExtractor::first_group(r"Token: (\w+)").unwrap();
        assert_eq!(extractor.extract_string(&response).unwrap(), "abc123");
    }

    #[test]
    fn test_header_extractor() {
        let response = mock_response("{}");
        let extractor = HeaderExtractor::new("x-custom");
        assert_eq!(extractor.extract_string(&response).unwrap(), "test-value");
    }

    #[test]
    fn test_extract_builder() {
        let response = mock_response(r#"{"user": "admin"}"#);
        let builder = ExtractBuilder::new(&response);

        assert_eq!(builder.json_path("$.user").unwrap(), "admin");
        assert_eq!(builder.header("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_json_path_with_array() {
        let json = serde_json::json!({
            "users": [
                {"name": "alice"},
                {"name": "bob"}
            ]
        });

        let result = extract_json_path(&json, "$.users[0].name").unwrap();
        assert_eq!(result, "alice");

        let result = extract_json_path(&json, "$.users[1].name").unwrap();
        assert_eq!(result, "bob");
    }
}
