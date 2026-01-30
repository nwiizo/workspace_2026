//! Scenario-based E2E testing framework
//!
//! Provides a fluent API for building and executing test scenarios with:
//! - Step-by-step execution with shared state
//! - Variable storage across steps
//! - Assertion helpers
//! - Automatic session management

use crate::client::{RequestBuilder, SecurityClient, SecurityResponse};
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Result of a scenario step execution
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Optional message
    pub message: Option<String>,
    /// Data extracted during the step
    pub data: HashMap<String, serde_json::Value>,
}

impl StepResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            data: HashMap::new(),
        }
    }

    /// Create a successful result with a message
    pub fn success_with_message(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(msg.into()),
            data: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failed(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(msg.into()),
            data: HashMap::new(),
        }
    }

    /// Create a skipped result
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(format!("Skipped: {}", reason.into())),
            data: HashMap::new(),
        }
    }

    /// Add data to the result
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Result of running a complete scenario
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Whether all steps passed
    pub success: bool,
    /// Individual step results
    pub steps: Vec<(String, StepResult)>,
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Tags associated with the scenario
    pub tags: Vec<String>,
}

impl ScenarioResult {
    /// Print a summary of the scenario execution
    pub fn print_summary(&self) {
        println!("\n=== Scenario: {} ===", self.name);
        println!(
            "Status: {}",
            if self.success {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        );
        println!("Duration: {}ms\n", self.duration_ms);

        for (name, result) in &self.steps {
            let status = if result.success { "✓" } else { "✗" };
            let msg = result.message.as_deref().unwrap_or("");
            println!("  {} {} {}", status, name, msg);
        }
        println!();
    }
}

/// Execution context for a scenario step (shared via Arc)
#[derive(Clone)]
pub struct ScenarioContext {
    client: SecurityClient,
    variables: Arc<RwLock<HashMap<String, String>>>,
}

impl ScenarioContext {
    fn new(base_url: Option<String>) -> Result<Self> {
        let client = if let Some(ref url) = base_url {
            SecurityClient::with_base_url(url)?
        } else {
            SecurityClient::new()?
        };

        Ok(Self {
            client,
            variables: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get the HTTP client
    pub fn client(&self) -> &SecurityClient {
        &self.client
    }

    /// Create a GET request
    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.request(reqwest::Method::GET, path)
    }

    /// Create a POST request
    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.request(reqwest::Method::POST, path)
    }

    /// Create a PUT request
    pub fn put(&self, path: &str) -> RequestBuilder {
        self.client.request(reqwest::Method::PUT, path)
    }

    /// Create a DELETE request
    pub fn delete(&self, path: &str) -> RequestBuilder {
        self.client.request(reqwest::Method::DELETE, path)
    }

    /// Create a PATCH request
    pub fn patch(&self, path: &str) -> RequestBuilder {
        self.client.request(reqwest::Method::PATCH, path)
    }

    /// Set a variable synchronously (blocking)
    pub async fn set_var_async(&self, name: impl Into<String>, value: impl Into<String>) {
        self.variables
            .write()
            .await
            .insert(name.into(), value.into());
    }

    /// Get a variable asynchronously
    pub async fn get_var_async(&self, name: &str) -> Result<String> {
        self.variables
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| Error::var_not_found(name))
    }

    /// Check if a variable exists
    pub async fn has_var(&self, name: &str) -> bool {
        self.variables.read().await.contains_key(name)
    }

    /// Assert response status code
    pub fn assert_status(&self, response: &SecurityResponse, expected: u16) -> Result<()> {
        if response.status.as_u16() != expected {
            return Err(Error::assertion_failed(format!(
                "Expected status {}, got {}",
                expected,
                response.status.as_u16()
            )));
        }
        Ok(())
    }

    /// Assert response contains text
    pub fn assert_contains(&self, response: &SecurityResponse, needle: &str) -> Result<()> {
        if !response.contains(needle) {
            return Err(Error::assertion_failed(format!(
                "Response does not contain '{}'",
                needle
            )));
        }
        Ok(())
    }

    /// Assert response does not contain text
    pub fn assert_not_contains(&self, response: &SecurityResponse, needle: &str) -> Result<()> {
        if response.contains(needle) {
            return Err(Error::assertion_failed(format!(
                "Response should not contain '{}'",
                needle
            )));
        }
        Ok(())
    }

    /// Assert JSON path value equals expected
    pub fn assert_json_eq(
        &self,
        response: &SecurityResponse,
        path: &str,
        expected: &serde_json::Value,
    ) -> Result<()> {
        let actual = response.json_path(path)?;
        if &actual != expected {
            return Err(Error::assertion_failed(format!(
                "JSON path '{}' expected {:?}, got {:?}",
                path, expected, actual
            )));
        }
        Ok(())
    }

    /// Set JWT token for subsequent requests
    pub async fn set_jwt(&self, token: &str) {
        self.client.set_jwt(token).await;
    }

    /// Set a cookie for subsequent requests
    pub async fn set_cookie(&self, name: &str, value: &str) {
        self.client.set_cookie(name, value).await;
    }

    /// Execute multiple requests in parallel (useful for race condition testing)
    ///
    /// Returns a vector of responses in the same order as the requests
    pub async fn parallel<I, F, Fut>(
        &self,
        count: usize,
        request_fn: F,
    ) -> Vec<Result<SecurityResponse>>
    where
        F: Fn(usize) -> Fut,
        Fut: Future<Output = Result<SecurityResponse>> + Send,
    {
        use futures::future::join_all;

        let futures: Vec<_> = (0..count).map(&request_fn).collect();
        join_all(futures).await
    }

    // ========== Convenience Methods ==========

    /// Login and store the token (common pattern)
    ///
    /// Sends a POST request to the login endpoint, extracts the token from
    /// the specified JSON path, and stores it for subsequent requests.
    pub async fn login(
        &self,
        endpoint: &str,
        credentials: &serde_json::Value,
        token_path: &str,
    ) -> Result<String> {
        let resp = self.post(endpoint).json(credentials).send().await?;

        resp.expect_success()?;

        let token = resp.extract(token_path)?;
        self.set_jwt(&token).await;
        self.set_var_async("auth_token", &token).await;

        Ok(token)
    }

    /// Try multiple payloads and return the first successful one
    ///
    /// Useful for testing multiple attack vectors
    pub async fn try_payloads<F, Fut>(&self, payloads: &[&str], test_fn: F) -> Option<String>
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = bool>,
    {
        for payload in payloads {
            if test_fn(payload.to_string()).await {
                return Some(payload.to_string());
            }
        }
        None
    }

    /// Store a value extracted from a response
    pub async fn store(
        &self,
        name: &str,
        response: &SecurityResponse,
        path: &str,
    ) -> Result<String> {
        let value = response.extract(path)?;
        self.set_var_async(name, &value).await;
        Ok(value)
    }

    /// Get stored value or return default
    pub async fn get_or(&self, name: &str, default: &str) -> String {
        self.get_var_async(name)
            .await
            .unwrap_or_else(|_| default.to_string())
    }

    // ========== Security Testing Helpers ==========

    /// SQLi authentication bypass login
    ///
    /// Attempts to log in using SQL injection, extracts the token from the response,
    /// and stores it for subsequent requests.
    ///
    /// # Example
    /// ```ignore
    /// let token = ctx.sqli_login("/api/login", "admin@example.com").await?;
    /// ```
    pub async fn sqli_login(&self, endpoint: &str, email: &str) -> Result<String> {
        let resp = self
            .post(endpoint)
            .json(&serde_json::json!({
                "email": format!("{}'--", email),
                "password": "x"
            }))
            .send()
            .await?;

        resp.expect_success()?;

        let token = resp.extract("$.authentication.token")?;
        self.set_jwt(&token).await;
        self.set_var_async("auth_token", &token).await;

        Ok(token)
    }

    /// SQLi authentication bypass with custom payload
    ///
    /// More flexible version that allows custom SQL injection payloads.
    ///
    /// # Example
    /// ```ignore
    /// let token = ctx.sqli_login_custom(
    ///     "/api/login",
    ///     "' OR 1=1--",
    ///     "anything",
    ///     "$.authentication.token"
    /// ).await?;
    /// ```
    pub async fn sqli_login_custom(
        &self,
        endpoint: &str,
        email_payload: &str,
        password: &str,
        token_path: &str,
    ) -> Result<String> {
        let resp = self
            .post(endpoint)
            .json(&serde_json::json!({
                "email": email_payload,
                "password": password
            }))
            .send()
            .await?;

        resp.expect_success()?;

        let token = resp.extract(token_path)?;
        self.set_jwt(&token).await;
        self.set_var_async("auth_token", &token).await;

        Ok(token)
    }

    /// Execute identical requests concurrently for race condition testing
    ///
    /// Unlike `parallel`, this sends the exact same request multiple times
    /// simultaneously to test for race conditions.
    ///
    /// # Example
    /// ```ignore
    /// let results = ctx.race(10, || {
    ///     ctx.post("/api/redeem-coupon")
    ///         .json(&serde_json::json!({"code": "DISCOUNT10"}))
    /// }).await;
    ///
    /// let successes = results.iter().filter(|r| r.is_ok()).count();
    /// ```
    pub async fn race<F>(&self, count: usize, builder_fn: F) -> Vec<Result<SecurityResponse>>
    where
        F: Fn() -> RequestBuilder,
    {
        use futures::future::join_all;

        let futures: Vec<_> = (0..count).map(|_| builder_fn().send()).collect();
        join_all(futures).await
    }
}

/// Type alias for step function - takes Arc<ScenarioContext> to avoid lifetime issues
pub type StepFn = Box<
    dyn Fn(Arc<ScenarioContext>) -> Pin<Box<dyn Future<Output = Result<StepResult>> + Send>>
        + Send
        + Sync,
>;

/// A single step in a scenario
pub struct Step {
    /// Step name
    pub name: String,
    /// Step function
    pub func: StepFn,
}

/// Scenario builder and executor
pub struct Scenario {
    name: String,
    base_url: Option<String>,
    steps: Vec<Step>,
    tags: Vec<String>,
}

impl Scenario {
    /// Create a new scenario with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: None,
            steps: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Set the base URL for all requests
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Add a single tag to the scenario
    ///
    /// # Example
    /// ```ignore
    /// Scenario::new("Auth Test")
    ///     .tag("security")
    ///     .tag("auth")
    ///     // ...
    /// ```
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags to the scenario
    ///
    /// # Example
    /// ```ignore
    /// Scenario::new("SQLi Test")
    ///     .tags(&["security", "sqli", "critical"])
    ///     // ...
    /// ```
    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags.extend(tags.iter().map(|s| s.to_string()));
        self
    }

    /// Check if the scenario has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get all tags
    pub fn get_tags(&self) -> &[String] {
        &self.tags
    }

    /// Add a step to the scenario
    ///
    /// The step function receives an Arc<ScenarioContext> that can be cloned
    /// and moved into async blocks without lifetime issues.
    pub fn step<F, Fut>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Arc<ScenarioContext>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<StepResult>> + Send + 'static,
    {
        let name = name.into();
        self.steps.push(Step {
            name,
            func: Box::new(move |ctx| Box::pin(func(ctx))),
        });
        self
    }

    /// Run the scenario
    pub async fn run(self) -> Result<ScenarioResult> {
        let start = std::time::Instant::now();
        info!("Starting scenario: {}", self.name);

        let ctx = Arc::new(ScenarioContext::new(self.base_url)?);
        let mut step_results = Vec::new();
        let mut all_success = true;

        for step in &self.steps {
            info!("Running step: {}", step.name);

            match (step.func)(Arc::clone(&ctx)).await {
                Ok(result) => {
                    if result.success {
                        debug!("Step '{}' passed", step.name);
                    } else {
                        error!("Step '{}' failed: {:?}", step.name, result.message);
                        all_success = false;
                    }
                    step_results.push((step.name.clone(), result));
                }
                Err(e) => {
                    error!("Step '{}' error: {}", step.name, e);
                    all_success = false;
                    step_results.push((step.name.clone(), StepResult::failed(e.to_string())));
                    break; // Stop on first error
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            "Scenario '{}' completed in {}ms: {}",
            self.name,
            duration_ms,
            if all_success { "PASSED" } else { "FAILED" }
        );

        Ok(ScenarioResult {
            name: self.name,
            success: all_success,
            steps: step_results,
            duration_ms,
            tags: self.tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_result() {
        let result = StepResult::success();
        assert!(result.success);
        assert!(result.message.is_none());

        let result = StepResult::failed("error");
        assert!(!result.success);
        assert_eq!(result.message, Some("error".to_string()));

        let result = StepResult::success().with_data("key", serde_json::json!("value"));
        assert!(result.data.contains_key("key"));
    }

    #[tokio::test]
    async fn test_scenario_context_variables() {
        let ctx = ScenarioContext::new(None).unwrap();

        ctx.set_var_async("test", "value").await;
        assert!(ctx.has_var("test").await);

        let value = ctx.get_var_async("test").await.unwrap();
        assert_eq!(value, "value");
    }

    #[tokio::test]
    async fn test_empty_scenario() {
        let result = Scenario::new("Empty Test").run().await.unwrap();

        assert!(result.success);
        assert!(result.steps.is_empty());
    }

    #[tokio::test]
    async fn test_scenario_with_steps() {
        let result = Scenario::new("Test Scenario")
            .step("Step 1", |_ctx| async { Ok(StepResult::success()) })
            .step("Step 2", |_ctx| async {
                Ok(StepResult::success_with_message("Done"))
            })
            .run()
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.steps.len(), 2);
    }

    #[tokio::test]
    async fn test_scenario_failure() {
        let result = Scenario::new("Failing Scenario")
            .step("Will Fail", |_ctx| async {
                Ok(StepResult::failed("Expected failure"))
            })
            .run()
            .await
            .unwrap();

        assert!(!result.success);
    }
}
