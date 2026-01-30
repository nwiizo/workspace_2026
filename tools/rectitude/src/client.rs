//! HTTP client with security testing features
//!
//! Provides a wrapper around reqwest with:
//! - Automatic cookie jar management
//! - JWT token handling
//! - Request/response logging
//! - Common security headers

use crate::error::{Error, Result};
use regex::Regex;
use reqwest::{
    Client, Method, Response, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, HeaderMap, HeaderName, HeaderValue, USER_AGENT},
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// Security-focused HTTP client
#[derive(Clone)]
pub struct SecurityClient {
    inner: Client,
    base_url: Option<Url>,
    default_headers: HeaderMap,
    cookies: Arc<RwLock<HashMap<String, String>>>,
    jwt_token: Arc<RwLock<Option<String>>>,
}

impl SecurityClient {
    /// Create a new builder for SecurityClient
    pub fn builder() -> SecurityClientBuilder {
        SecurityClientBuilder::new()
    }

    /// Create a new client with default settings
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Create a new client with a base URL
    pub fn with_base_url(base_url: &str) -> Result<Self> {
        Self::builder().base_url(base_url).build()
    }

    /// Set the base URL
    pub fn set_base_url(&mut self, url: &str) -> Result<()> {
        self.base_url = Some(Url::parse(url)?);
        Ok(())
    }

    /// Set a JWT token for authentication
    pub async fn set_jwt(&self, token: &str) {
        let mut jwt = self.jwt_token.write().await;
        *jwt = Some(token.to_string());
    }

    /// Clear the JWT token
    pub async fn clear_jwt(&self) {
        let mut jwt = self.jwt_token.write().await;
        *jwt = None;
    }

    /// Get the current JWT token
    pub async fn get_jwt(&self) -> Option<String> {
        self.jwt_token.read().await.clone()
    }

    /// Set a cookie
    pub async fn set_cookie(&self, name: &str, value: &str) {
        let mut cookies = self.cookies.write().await;
        cookies.insert(name.to_string(), value.to_string());
    }

    /// Get a cookie value
    pub async fn get_cookie(&self, name: &str) -> Option<String> {
        self.cookies.read().await.get(name).cloned()
    }

    /// Clear all cookies
    pub async fn clear_cookies(&self) {
        let mut cookies = self.cookies.write().await;
        cookies.clear();
    }

    /// Build full URL from path
    fn build_url(&self, path: &str) -> Result<Url> {
        if let Some(base) = &self.base_url {
            Ok(base.join(path)?)
        } else if path.starts_with("http://") || path.starts_with("https://") {
            Ok(Url::parse(path)?)
        } else {
            Err(Error::InvalidConfig(
                "No base URL set and path is not absolute".to_string(),
            ))
        }
    }

    /// Build headers with cookies and JWT
    async fn build_headers(&self) -> HeaderMap {
        let mut headers = self.default_headers.clone();

        // Add cookies
        let cookies = self.cookies.read().await;
        if !cookies.is_empty() {
            let cookie_str = cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            if let Ok(value) = HeaderValue::from_str(&cookie_str) {
                headers.insert(COOKIE, value);
            }
        }

        // Add JWT
        if let Some(token) = self.jwt_token.read().await.as_ref()
            && let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token))
        {
            headers.insert(AUTHORIZATION, value);
        }

        headers
    }

    /// Send a GET request
    pub async fn get(&self, path: &str) -> Result<SecurityResponse> {
        self.request(Method::GET, path).send().await
    }

    /// Send a POST request with JSON body
    pub async fn post_json<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<SecurityResponse> {
        self.request(Method::POST, path).json(body).send().await
    }

    /// Send a POST request with form data
    pub async fn post_form<T: Serialize + ?Sized>(
        &self,
        path: &str,
        form: &T,
    ) -> Result<SecurityResponse> {
        self.request(Method::POST, path).form(form).send().await
    }

    /// Create a request builder
    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        RequestBuilder::new(self.clone(), method, path.to_string())
    }
}

impl Default for SecurityClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default client")
    }
}

/// Builder for SecurityClient
pub struct SecurityClientBuilder {
    base_url: Option<String>,
    headers: HeaderMap,
    timeout_secs: u64,
    follow_redirects: bool,
    user_agent: String,
}

impl SecurityClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            base_url: None,
            headers: HeaderMap::new(),
            timeout_secs: 30,
            follow_redirects: true,
            user_agent: "probitas-security/0.1".to_string(),
        }
    }

    /// Set the base URL
    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = Some(url.to_string());
        self
    }

    /// Set a default header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set whether to follow redirects
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    /// Set the user agent
    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = ua.to_string();
        self
    }

    /// Build the client
    pub fn build(self) -> Result<SecurityClient> {
        let mut headers = self.headers;
        if let Ok(ua) = HeaderValue::from_str(&self.user_agent) {
            headers.insert(USER_AGENT, ua);
        }

        let inner = Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .redirect(if self.follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()?;

        let base_url = self.base_url.map(|u| Url::parse(&u)).transpose()?;

        Ok(SecurityClient {
            inner,
            base_url,
            default_headers: headers,
            cookies: Arc::new(RwLock::new(HashMap::new())),
            jwt_token: Arc::new(RwLock::new(None)),
        })
    }
}

impl Default for SecurityClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Request builder for SecurityClient
pub struct RequestBuilder {
    client: SecurityClient,
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    query: Vec<(String, String)>,
    follow_redirects: bool,
}

impl RequestBuilder {
    fn new(client: SecurityClient, method: Method, path: String) -> Self {
        Self {
            client,
            method,
            path,
            headers: HeaderMap::new(),
            body: None,
            query: Vec::new(),
            follow_redirects: true,
        }
    }

    /// Disable following redirects for this request
    pub fn no_redirect(mut self) -> Self {
        self.follow_redirects = false;
        self
    }

    /// Add a header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    /// Set bearer authentication
    pub fn bearer_auth(mut self, token: &str) -> Self {
        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
            self.headers.insert(AUTHORIZATION, value);
        }
        self
    }

    /// Add a query parameter
    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.query.push((key.to_string(), value.to_string()));
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize + ?Sized>(mut self, body: &T) -> Self {
        if let Ok(json) = serde_json::to_vec(body) {
            self.body = Some(json);
            self.headers
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        self
    }

    /// Set form body
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> Self {
        if let Ok(encoded) = serde_json::to_string(form) {
            self.body = Some(encoded.into_bytes());
            self.headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }
        self
    }

    /// Set raw body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Send the request
    pub async fn send(self) -> Result<SecurityResponse> {
        let url = self.client.build_url(&self.path)?;

        // Build URL with query params
        let url = if self.query.is_empty() {
            url
        } else {
            let mut url = url;
            url.query_pairs_mut().extend_pairs(self.query.iter());
            url
        };

        // Build headers
        let mut headers = self.client.build_headers().await;
        for (key, value) in self.headers.iter() {
            headers.insert(key.clone(), value.clone());
        }

        // Use a no-redirect client if needed
        let response = if self.follow_redirects {
            let mut req = self.client.inner.request(self.method.clone(), url);
            req = req.headers(headers);
            if let Some(body) = self.body {
                req = req.body(body);
            }
            req.send().await?
        } else {
            // Create a one-off client that doesn't follow redirects
            let no_redirect_client = Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?;
            let mut req = no_redirect_client.request(self.method.clone(), url);
            req = req.headers(headers);
            if let Some(body) = self.body {
                req = req.body(body);
            }
            req.send().await?
        };

        // Extract cookies from response
        for cookie in response.cookies() {
            self.client.set_cookie(cookie.name(), cookie.value()).await;
        }

        SecurityResponse::from_response(response).await
    }
}

/// Response wrapper with convenience methods
#[derive(Debug, Clone)]
pub struct SecurityResponse {
    /// HTTP status code
    pub status: StatusCode,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body as bytes
    pub body: Vec<u8>,
    /// Response body as string (if valid UTF-8)
    pub text: Option<String>,
}

impl SecurityResponse {
    async fn from_response(response: Response) -> Result<Self> {
        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response.bytes().await?.to_vec();
        let text = String::from_utf8(body.clone()).ok();

        Ok(Self {
            status,
            headers,
            body,
            text,
        })
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Get response as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Get response as serde_json::Value
    pub fn json_value(&self) -> Result<serde_json::Value> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Get text body (panics if not valid UTF-8)
    pub fn text(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    /// Check if body contains a string
    pub fn contains(&self, needle: &str) -> bool {
        self.text().contains(needle)
    }

    /// Extract value using JSON path (simple implementation)
    pub fn json_path(&self, path: &str) -> Result<serde_json::Value> {
        let value = self.json_value()?;
        extract_json_path(&value, path)
    }

    // ========== Fluent Assertions ==========

    /// Assert status code equals expected (fluent API)
    ///
    /// # Example
    /// ```ignore
    /// resp.expect_status(200)?;
    /// ```
    pub fn expect_status(&self, expected: u16) -> Result<&Self> {
        if self.status.as_u16() != expected {
            return Err(Error::assertion_failed(format!(
                "Expected status {}, got {}",
                expected,
                self.status.as_u16()
            )));
        }
        Ok(self)
    }

    /// Assert response is successful (2xx)
    pub fn expect_success(&self) -> Result<&Self> {
        if !self.is_success() {
            return Err(Error::assertion_failed(format!(
                "Expected success status, got {}",
                self.status.as_u16()
            )));
        }
        Ok(self)
    }

    /// Assert body contains text
    pub fn expect_contains(&self, needle: &str) -> Result<&Self> {
        if !self.contains(needle) {
            return Err(Error::assertion_failed(format!(
                "Response does not contain '{}'",
                needle
            )));
        }
        Ok(self)
    }

    /// Assert body does not contain text
    pub fn expect_not_contains(&self, needle: &str) -> Result<&Self> {
        if self.contains(needle) {
            return Err(Error::assertion_failed(format!(
                "Response should not contain '{}'",
                needle
            )));
        }
        Ok(self)
    }

    /// Assert JSON path value equals expected
    pub fn expect_json(&self, path: &str, expected: &serde_json::Value) -> Result<&Self> {
        let actual = self.json_path(path)?;
        if &actual != expected {
            return Err(Error::assertion_failed(format!(
                "JSON path '{}': expected {:?}, got {:?}",
                path, expected, actual
            )));
        }
        Ok(self)
    }

    /// Assert header exists and optionally matches value
    pub fn expect_header(&self, name: &str, expected: Option<&str>) -> Result<&Self> {
        match (self.header(name), expected) {
            (None, _) => Err(Error::assertion_failed(format!(
                "Header '{}' not found",
                name
            ))),
            (Some(actual), Some(exp)) if actual != exp => Err(Error::assertion_failed(format!(
                "Header '{}': expected '{}', got '{}'",
                name, exp, actual
            ))),
            _ => Ok(self),
        }
    }

    /// Extract JSON path value as string (convenience method)
    pub fn extract(&self, path: &str) -> Result<String> {
        let value = self.json_path(path)?;
        Ok(match value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => value.to_string().trim_matches('"').to_string(),
        })
    }

    // ========== Content Analysis ==========

    /// Count occurrences of a pattern in the response body
    ///
    /// Useful for analyzing response content, e.g., counting items returned.
    ///
    /// # Example
    /// ```ignore
    /// let count = resp.count_matches("<tr>");  // Count table rows
    /// ```
    pub fn count_matches(&self, pattern: &str) -> usize {
        self.text().matches(pattern).count()
    }

    /// Count occurrences using regex pattern
    ///
    /// # Example
    /// ```ignore
    /// let email_count = resp.count_regex_matches(r"\b[\w.-]+@[\w.-]+\.\w+\b")?;
    /// ```
    pub fn count_regex_matches(&self, pattern: &str) -> Result<usize> {
        let re = Regex::new(pattern).map_err(|e| Error::InvalidInput(e.to_string()))?;
        Ok(re.find_iter(self.text()).count())
    }

    /// Find all matches of a regex pattern
    ///
    /// # Example
    /// ```ignore
    /// let emails = resp.find_all(r"\b[\w.-]+@[\w.-]+\.\w+\b")?;
    /// ```
    pub fn find_all(&self, pattern: &str) -> Result<Vec<String>> {
        let re = Regex::new(pattern).map_err(|e| Error::InvalidInput(e.to_string()))?;
        Ok(re
            .find_iter(self.text())
            .map(|m| m.as_str().to_string())
            .collect())
    }

    /// Check if response indicates an error (non-2xx or contains error indicators)
    pub fn has_error_indicators(&self) -> bool {
        !self.is_success()
            || self.contains("error")
            || self.contains("Error")
            || self.contains("failed")
            || self.contains("Failed")
    }

    /// Get response body length
    pub fn body_len(&self) -> usize {
        self.body.len()
    }
}

/// Simple JSON path extractor
fn extract_json_path(value: &serde_json::Value, path: &str) -> Result<serde_json::Value> {
    let parts: Vec<&str> = path
        .trim_start_matches('$')
        .split('.')
        .filter(|s| !s.is_empty())
        .collect();

    let mut current = value.clone();
    for part in parts {
        // Handle array index: [0], [1], etc.
        if let Some(idx_str) = part.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
            && let Ok(idx) = idx_str.parse::<usize>()
        {
            current = current.get(idx).cloned().ok_or_else(|| {
                Error::extraction_failed(format!("Array index {} out of bounds", idx))
            })?;
            continue;
        }

        // Handle object key
        current = current
            .get(part)
            .cloned()
            .ok_or_else(|| Error::extraction_failed(format!("Key '{}' not found in JSON", part)))?;
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_path_extraction() {
        let json = serde_json::json!({
            "authentication": {
                "token": "abc123",
                "user": {
                    "id": 1,
                    "name": "admin"
                }
            },
            "items": [1, 2, 3]
        });

        let token = extract_json_path(&json, "$.authentication.token").unwrap();
        assert_eq!(token, "abc123");

        let id = extract_json_path(&json, "$.authentication.user.id").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_client_builder() {
        let client = SecurityClient::builder()
            .base_url("http://localhost:3000")
            .header("X-Custom", "test")
            .timeout(60)
            .build()
            .unwrap();

        assert!(client.base_url.is_some());
    }

    #[tokio::test]
    async fn test_cookie_management() {
        let client = SecurityClient::new().unwrap();

        client.set_cookie("session", "abc123").await;
        assert_eq!(
            client.get_cookie("session").await,
            Some("abc123".to_string())
        );

        client.clear_cookies().await;
        assert_eq!(client.get_cookie("session").await, None);
    }

    #[tokio::test]
    async fn test_jwt_management() {
        let client = SecurityClient::new().unwrap();

        client.set_jwt("eyJhbGciOiJIUzI1NiJ9.test").await;
        assert!(client.get_jwt().await.is_some());

        client.clear_jwt().await;
        assert!(client.get_jwt().await.is_none());
    }
}
