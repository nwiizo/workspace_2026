//! HTTP client implementation
//!
//! Wraps the existing SecurityClient with the Client trait.

use super::{Client, ClientConfig, ClientError, ClientResult, Request, Response};
use async_trait::async_trait;
use reqwest::{
    Method, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method
    pub method: Method,
    /// Request path (relative to base URL)
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query: Vec<(String, String)>,
    /// Request body
    pub body: Option<Vec<u8>>,
    /// Content type
    pub content_type: Option<String>,
}

impl HttpRequest {
    /// Create a new GET request
    pub fn get(path: impl Into<String>) -> Self {
        Self::new(Method::GET, path)
    }

    /// Create a new POST request
    pub fn post(path: impl Into<String>) -> Self {
        Self::new(Method::POST, path)
    }

    /// Create a new PUT request
    pub fn put(path: impl Into<String>) -> Self {
        Self::new(Method::PUT, path)
    }

    /// Create a new DELETE request
    pub fn delete(path: impl Into<String>) -> Self {
        Self::new(Method::DELETE, path)
    }

    /// Create a new PATCH request
    pub fn patch(path: impl Into<String>) -> Self {
        Self::new(Method::PATCH, path)
    }

    /// Create a new request with method and path
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: HashMap::new(),
            query: Vec::new(),
            body: None,
            content_type: None,
        }
    }

    /// Add a header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Add a query parameter
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        if let Ok(json) = serde_json::to_vec(body) {
            self.body = Some(json);
            self.content_type = Some("application/json".to_string());
        }
        self
    }

    /// Set raw body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set content type
    pub fn content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    /// Set bearer auth
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }
}

impl Request for HttpRequest {
    fn request_type(&self) -> &str {
        "http"
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: StatusCode,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body_bytes: Vec<u8>,
    /// Response body as text
    pub body_text: Option<String>,
}

impl HttpResponse {
    /// Get status code as u16
    pub fn status_code(&self) -> u16 {
        self.status.as_u16()
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    /// Check if body contains text
    pub fn contains(&self, text: &str) -> bool {
        self.body_text.as_ref().is_some_and(|t| t.contains(text))
    }

    /// Parse body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body_bytes)
    }
}

impl Response for HttpResponse {
    fn is_success(&self) -> bool {
        self.status.is_success()
    }

    fn body(&self) -> &[u8] {
        &self.body_bytes
    }

    fn text(&self) -> Option<&str> {
        self.body_text.as_deref()
    }
}

/// HTTP client with security testing features
pub struct HttpClient {
    inner: reqwest::Client,
    config: ClientConfig,
    base_url: Option<Url>,
    cookies: Arc<RwLock<HashMap<String, String>>>,
    jwt_token: Arc<RwLock<Option<String>>>,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(url: impl Into<String>) -> ClientResult<Self> {
        let config = ClientConfig::new(url);
        Self::with_config(config)
    }

    /// Create with configuration
    pub fn with_config(config: ClientConfig) -> ClientResult<Self> {
        let inner = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(ClientError::Http)?;

        let base_url = if config.url.is_empty() {
            None
        } else {
            Some(Url::parse(&config.url).map_err(|e| ClientError::config(e.to_string()))?)
        };

        Ok(Self {
            inner,
            config,
            base_url,
            cookies: Arc::new(RwLock::new(HashMap::new())),
            jwt_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Set JWT token
    pub async fn set_jwt(&self, token: &str) {
        let mut jwt = self.jwt_token.write().await;
        *jwt = Some(token.to_string());
    }

    /// Clear JWT token
    pub async fn clear_jwt(&self) {
        let mut jwt = self.jwt_token.write().await;
        *jwt = None;
    }

    /// Set a cookie
    pub async fn set_cookie(&self, name: &str, value: &str) {
        let mut cookies = self.cookies.write().await;
        cookies.insert(name.to_string(), value.to_string());
    }

    /// Clear all cookies
    pub async fn clear_cookies(&self) {
        let mut cookies = self.cookies.write().await;
        cookies.clear();
    }

    /// Build full URL
    fn build_url(&self, path: &str) -> ClientResult<Url> {
        if let Some(base) = &self.base_url {
            base.join(path)
                .map_err(|e| ClientError::config(e.to_string()))
        } else if path.starts_with("http://") || path.starts_with("https://") {
            Url::parse(path).map_err(|e| ClientError::config(e.to_string()))
        } else {
            Err(ClientError::config("No base URL and path is not absolute"))
        }
    }

    /// Build headers
    async fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

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
}

#[async_trait]
impl Client for HttpClient {
    type Request = HttpRequest;
    type Response = HttpResponse;

    fn client_type(&self) -> &str {
        "http"
    }

    async fn is_connected(&self) -> bool {
        // HTTP is connectionless, always "connected"
        true
    }

    async fn execute(&self, request: HttpRequest) -> ClientResult<HttpResponse> {
        let url = self.build_url(&request.path)?;

        // Build URL with query params
        let url = if request.query.is_empty() {
            url
        } else {
            let mut url = url;
            url.query_pairs_mut().extend_pairs(request.query.iter());
            url
        };

        // Build headers
        let mut headers = self.build_headers().await;
        for (key, value) in request.headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                headers.insert(name, val);
            }
        }

        // Set content type
        if let Some(ct) = request.content_type
            && let Ok(val) = HeaderValue::from_str(&ct)
        {
            headers.insert(CONTENT_TYPE, val);
        }

        // Build request
        let mut req = self.inner.request(request.method, url);
        req = req.headers(headers);
        if let Some(body) = request.body {
            req = req.body(body);
        }

        // Send
        let response = req.send().await.map_err(ClientError::Http)?;

        // Extract cookies
        for cookie in response.cookies() {
            self.set_cookie(cookie.name(), cookie.value()).await;
        }

        // Build response
        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_bytes = response.bytes().await.map_err(ClientError::Http)?.to_vec();
        let body_text = String::from_utf8(body_bytes.clone()).ok();

        Ok(HttpResponse {
            status,
            headers,
            body_bytes,
            body_text,
        })
    }

    async fn close(&self) -> ClientResult<()> {
        // HTTP is connectionless
        Ok(())
    }
}

impl std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("config", &self.config)
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::get("/api/users")
            .header("X-Custom", "value")
            .query("page", "1")
            .bearer_auth("token123");

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(req.query.len(), 1);
    }

    #[test]
    fn test_http_request_json() {
        let body = serde_json::json!({"name": "test"});
        let req = HttpRequest::post("/api/users").json(&body);

        assert!(req.body.is_some());
        assert_eq!(req.content_type, Some("application/json".to_string()));
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let client = HttpClient::new("http://localhost:3000").unwrap();
        assert!(client.is_connected().await);
    }
}
