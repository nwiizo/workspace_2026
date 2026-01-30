//! HTTP client utilities for security testing
//!
//! Provides authenticated requests, header manipulation, and response analysis.

use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
    #[error("Timeout")]
    Timeout,
}

/// HTTP client with security testing features
pub struct SecurityClient {
    client: Client,
    base_url: Option<String>,
    default_headers: HeaderMap,
    cookies: HashMap<String, String>,
    jwt_token: Option<String>,
}

impl Default for SecurityClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityClient {
    /// Create a new security client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true) // For testing environments
            .redirect(reqwest::redirect::Policy::none()) // Don't follow redirects automatically
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: None,
            default_headers: HeaderMap::new(),
            cookies: HashMap::new(),
            jwt_token: None,
        }
    }

    /// Set base URL for relative requests
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = Some(url.trim_end_matches('/').to_string());
        self
    }

    /// Set JWT token for Authorization header
    pub fn with_jwt(mut self, token: &str) -> Self {
        self.jwt_token = Some(token.to_string());
        self
    }

    /// Add a cookie
    pub fn with_cookie(mut self, name: &str, value: &str) -> Self {
        self.cookies.insert(name.to_string(), value.to_string());
        self
    }

    /// Add a default header (builder pattern)
    pub fn with_header(mut self, name: &str, value: &str) -> Result<Self, HttpError> {
        self.add_header(name, value)?;
        Ok(self)
    }

    /// Add a default header (mutable reference)
    pub fn add_header(&mut self, name: &str, value: &str) -> Result<(), HttpError> {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| HttpError::InvalidHeader(name.to_string()))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| HttpError::InvalidHeader(value.to_string()))?;
        self.default_headers.insert(header_name, header_value);
        Ok(())
    }

    /// Build the full URL
    fn build_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            match &self.base_url {
                Some(base) => format!("{}{}", base, path),
                None => path.to_string(),
            }
        }
    }

    /// Build headers for a request
    fn build_headers(&self) -> HeaderMap {
        let mut headers = self.default_headers.clone();

        // Add JWT token
        if let Some(token) = &self.jwt_token {
            if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(AUTHORIZATION, value);
            }
        }

        // Add cookies
        if !self.cookies.is_empty() {
            let cookie_str: String = self
                .cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            if let Ok(value) = HeaderValue::from_str(&cookie_str) {
                headers.insert(COOKIE, value);
            }
        }

        headers
    }

    /// Send a GET request
    pub fn get(&self, path: &str) -> Result<SecurityResponse, HttpError> {
        let url = self.build_url(path);
        let headers = self.build_headers();

        let response = self.client.get(&url).headers(headers).send()?;

        Ok(SecurityResponse::from_response(response)?)
    }

    /// Send a POST request with JSON body
    pub fn post_json<T: Serialize>(&self, path: &str, body: &T) -> Result<SecurityResponse, HttpError> {
        let url = self.build_url(path);
        let mut headers = self.build_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self.client.post(&url).headers(headers).json(body).send()?;

        Ok(SecurityResponse::from_response(response)?)
    }

    /// Send a POST request with form data
    pub fn post_form(&self, path: &str, form: &HashMap<String, String>) -> Result<SecurityResponse, HttpError> {
        let url = self.build_url(path);
        let headers = self.build_headers();

        let response = self.client.post(&url).headers(headers).form(form).send()?;

        Ok(SecurityResponse::from_response(response)?)
    }

    /// Send a raw request with custom method and body
    pub fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        content_type: Option<&str>,
    ) -> Result<SecurityResponse, HttpError> {
        let url = self.build_url(path);
        let mut headers = self.build_headers();

        if let Some(ct) = content_type {
            if let Ok(value) = HeaderValue::from_str(ct) {
                headers.insert(CONTENT_TYPE, value);
            }
        }

        let method = reqwest::Method::from_bytes(method.as_bytes())
            .unwrap_or(reqwest::Method::GET);

        let mut request = self.client.request(method, &url).headers(headers);

        if let Some(b) = body {
            request = request.body(b.to_string());
        }

        let response = request.send()?;
        Ok(SecurityResponse::from_response(response)?)
    }
}

/// Security-focused response wrapper
#[derive(Debug, Clone)]
pub struct SecurityResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub cookies: Vec<CookieInfo>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
}

impl SecurityResponse {
    fn from_response(response: Response) -> Result<Self, HttpError> {
        let status = response.status().as_u16();
        let mut headers = HashMap::new();
        let mut cookies = Vec::new();

        for (name, value) in response.headers() {
            let name_str = name.as_str().to_string();
            let value_str = value.to_str().unwrap_or("").to_string();

            if name_str.to_lowercase() == "set-cookie" {
                if let Some(cookie) = CookieInfo::parse(&value_str) {
                    cookies.push(cookie);
                }
            }

            headers.insert(name_str, value_str);
        }

        let content_type = headers.get("content-type").cloned();
        let content_length = headers
            .get("content-length")
            .and_then(|v| v.parse().ok());

        let body = response.text()?;

        Ok(Self {
            status,
            headers,
            body,
            cookies,
            content_type,
            content_length,
        })
    }

    /// Check if response indicates success
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Check if response is a redirect
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    /// Get redirect location
    pub fn redirect_location(&self) -> Option<&str> {
        self.headers.get("location").map(|s| s.as_str())
    }

    /// Check if body contains a string (case-insensitive)
    pub fn body_contains(&self, needle: &str) -> bool {
        self.body.to_lowercase().contains(&needle.to_lowercase())
    }

    /// Parse body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.body)
    }

    /// Extract value from JSON response
    pub fn json_value(&self, path: &str) -> Option<serde_json::Value> {
        let json: serde_json::Value = serde_json::from_str(&self.body).ok()?;
        let mut current = &json;

        for key in path.split('.') {
            current = current.get(key)?;
        }

        Some(current.clone())
    }
}

/// Cookie information with security analysis
#[derive(Debug, Clone)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub expires: Option<String>,
    pub max_age: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
}

impl CookieInfo {
    /// Parse a Set-Cookie header value
    pub fn parse(cookie_str: &str) -> Option<Self> {
        let parts: Vec<&str> = cookie_str.split(';').collect();
        if parts.is_empty() {
            return None;
        }

        let name_value: Vec<&str> = parts[0].splitn(2, '=').collect();
        if name_value.len() != 2 {
            return None;
        }

        let mut cookie = CookieInfo {
            name: name_value[0].trim().to_string(),
            value: name_value[1].trim().to_string(),
            path: None,
            domain: None,
            expires: None,
            max_age: None,
            secure: false,
            http_only: false,
            same_site: None,
        };

        for part in parts.iter().skip(1) {
            let part = part.trim().to_lowercase();
            if part == "secure" {
                cookie.secure = true;
            } else if part == "httponly" {
                cookie.http_only = true;
            } else if let Some(value) = part.strip_prefix("path=") {
                cookie.path = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("domain=") {
                cookie.domain = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("expires=") {
                cookie.expires = Some(value.to_string());
            } else if let Some(value) = part.strip_prefix("max-age=") {
                cookie.max_age = value.parse().ok();
            } else if let Some(value) = part.strip_prefix("samesite=") {
                cookie.same_site = Some(value.to_string());
            }
        }

        Some(cookie)
    }

    /// Analyze cookie for security issues
    pub fn security_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if !self.secure {
            issues.push("Missing Secure flag - cookie sent over HTTP".to_string());
        }

        if !self.http_only {
            issues.push("Missing HttpOnly flag - accessible to JavaScript".to_string());
        }

        match &self.same_site {
            None => issues.push("Missing SameSite attribute - vulnerable to CSRF".to_string()),
            Some(ss) if ss == "none" && !self.secure => {
                issues.push("SameSite=None requires Secure flag".to_string());
            }
            _ => {}
        }

        // Check for sensitive cookie names without proper protection
        let sensitive_names = ["session", "token", "auth", "jwt", "sid"];
        let name_lower = self.name.to_lowercase();
        if sensitive_names.iter().any(|&s| name_lower.contains(s)) {
            if !self.http_only || !self.secure {
                issues.push(format!(
                    "Sensitive cookie '{}' lacks proper security flags",
                    self.name
                ));
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_parse() {
        let cookie = CookieInfo::parse("session=abc123; Path=/; HttpOnly; Secure; SameSite=Strict")
            .unwrap();
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert!(cookie.http_only);
        assert!(cookie.secure);
        assert_eq!(cookie.same_site, Some("strict".to_string()));
    }

    #[test]
    fn test_cookie_security_issues() {
        let insecure = CookieInfo::parse("session=abc123").unwrap();
        let issues = insecure.security_issues();
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.contains("Secure")));
        assert!(issues.iter().any(|i| i.contains("HttpOnly")));
    }

    #[test]
    fn test_secure_cookie() {
        let secure = CookieInfo::parse("token=xyz; HttpOnly; Secure; SameSite=Strict").unwrap();
        let issues = secure.security_issues();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_client_url_building() {
        let client = SecurityClient::new().with_base_url("http://localhost:3000");
        assert_eq!(client.build_url("/api/users"), "http://localhost:3000/api/users");
        assert_eq!(
            client.build_url("http://other.com/path"),
            "http://other.com/path"
        );
    }
}
