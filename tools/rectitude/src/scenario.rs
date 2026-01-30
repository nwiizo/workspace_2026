//! Scenario-based E2E testing framework
//!
//! Provides a fluent API for building and executing test scenarios with:
//! - Step-by-step execution with shared state
//! - Access to previous step results via `ctx.previous()` and `ctx.results()`
//! - Setup/cleanup lifecycle hooks
//! - Step-level timeout and retry configuration
//! - Skip functionality for conditional execution
//! - Parallel scenario execution

use crate::client::{RequestBuilder, SecurityClient, SecurityResponse};
use crate::error::{Error, Result};
use crate::resource::{Resource, ResourceManager};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// ============ Skip Functionality ============

/// Error type for skipping a step or scenario
#[derive(Debug, Clone)]
pub struct Skip {
    pub reason: String,
}

impl Skip {
    /// Create a new Skip with a reason
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for Skip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skip: {}", self.reason)
    }
}

impl std::error::Error for Skip {}

// ============ Step Result ============

/// Result of a scenario step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Whether the step was skipped
    pub skipped: bool,
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
            skipped: false,
            message: None,
            data: HashMap::new(),
        }
    }

    /// Create a successful result with a message
    pub fn success_with_message(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            skipped: false,
            message: Some(msg.into()),
            data: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failed(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            skipped: false,
            message: Some(msg.into()),
            data: HashMap::new(),
        }
    }

    /// Create a skipped result
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            success: true,
            skipped: true,
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

// ============ Scenario Result ============

/// Result of running a complete scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            let status = if result.skipped {
                "⊘"
            } else if result.success {
                "✓"
            } else {
                "✗"
            };
            let msg = result.message.as_deref().unwrap_or("");
            println!("  {} {} {}", status, name, msg);
        }
        println!();
    }
}

// ============ Step Configuration ============

/// Configuration for a single step
#[derive(Clone)]
pub struct StepConfig {
    /// Step timeout (None = use scenario default)
    pub timeout: Option<Duration>,
    /// Number of retries on failure
    pub retries: u32,
    /// Delay between retries
    pub retry_delay: Duration,
    /// Continue on failure (don't stop scenario)
    pub continue_on_failure: bool,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            retries: 0,
            retry_delay: Duration::from_millis(100),
            continue_on_failure: false,
        }
    }
}

impl StepConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }

    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }
}

// ============ Scenario Context ============

/// Type-erased store value wrapper
struct StoreValue(Box<dyn Any + Send + Sync>);

/// Execution context for a scenario step (shared via Arc)
#[derive(Clone)]
pub struct ScenarioContext {
    client: SecurityClient,
    variables: Arc<RwLock<HashMap<String, String>>>,
    /// Results from all previous steps (step_name -> result)
    results: Arc<RwLock<Vec<(String, StepResult)>>>,
    /// Current step index (0-based, incremented before each step runs)
    current_step: Arc<RwLock<usize>>,
    /// Typed key-value store for arbitrary data
    store: Arc<RwLock<HashMap<String, StoreValue>>>,
    /// Resource manager for automatic cleanup
    resources: Arc<ResourceManager>,
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
            results: Arc::new(RwLock::new(Vec::new())),
            current_step: Arc::new(RwLock::new(0)),
            store: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(ResourceManager::new()),
        })
    }

    // ========== Probitas-compatible API ==========

    /// Get the result of the immediately preceding step
    ///
    /// Returns None if this is the first step.
    pub async fn previous(&self) -> Option<StepResult> {
        let results = self.results.read().await;
        let current = *self.current_step.read().await;
        // current is 1-indexed (1 = first step, 2 = second step, etc.)
        // For step 2, we want results[0] (the first step's result)
        // So we need: current >= 2 and (current - 2) is a valid index
        if current >= 2 && current - 2 < results.len() {
            Some(results[current - 2].1.clone())
        } else {
            None
        }
    }

    /// Get all previous step results
    ///
    /// Returns a vector of (step_name, result) tuples.
    pub async fn results(&self) -> Vec<(String, StepResult)> {
        self.results.read().await.clone()
    }

    /// Get a specific step result by name
    pub async fn result(&self, step_name: &str) -> Option<StepResult> {
        self.results
            .read()
            .await
            .iter()
            .find(|(name, _)| name == step_name)
            .map(|(_, r)| r.clone())
    }

    /// Get data from a previous step's result
    pub async fn get_data(&self, step_name: &str, key: &str) -> Option<serde_json::Value> {
        self.result(step_name)
            .await
            .and_then(|r| r.data.get(key).cloned())
    }

    // ========== HTTP Methods ==========

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

    // ========== Variable Storage (store) ==========

    /// Set a variable (Probitas: ctx.store)
    pub async fn set_var_async(&self, name: impl Into<String>, value: impl Into<String>) {
        self.variables
            .write()
            .await
            .insert(name.into(), value.into());
    }

    /// Get a variable
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

    /// Get stored value or return default
    pub async fn get_or(&self, name: &str, default: &str) -> String {
        self.get_var_async(name)
            .await
            .unwrap_or_else(|_| default.to_string())
    }

    // ========== Step Index (Probitas: ctx.index) ==========

    /// Get the current step index (0-based)
    ///
    /// This returns the index of the currently executing step.
    /// Useful for conditional logic based on step position.
    ///
    /// # Example
    /// ```ignore
    /// if ctx.index().await == 0 {
    ///     // This is the first step
    /// }
    /// ```
    pub async fn index(&self) -> usize {
        let step = *self.current_step.read().await;
        // current_step is 1-indexed after increment, so subtract 1 for 0-indexed
        if step > 0 { step - 1 } else { 0 }
    }

    // ========== Typed Store (Probitas: ctx.store) ==========

    /// Store a typed value in the context
    ///
    /// Values are stored by key and can be retrieved with `store_get<T>()`.
    /// The type must implement `Send + Sync + Clone + 'static`.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Clone)]
    /// struct UserData { id: u32, name: String }
    ///
    /// ctx.store_set("user", UserData { id: 1, name: "admin".into() }).await;
    /// ```
    pub async fn store_set<T: Send + Sync + Clone + 'static>(
        &self,
        key: impl Into<String>,
        value: T,
    ) {
        let mut store = self.store.write().await;
        store.insert(key.into(), StoreValue(Box::new(value)));
    }

    /// Retrieve a typed value from the context
    ///
    /// Returns `None` if the key doesn't exist or the type doesn't match.
    ///
    /// # Example
    /// ```ignore
    /// if let Some(user) = ctx.store_get::<UserData>("user").await {
    ///     println!("User: {}", user.name);
    /// }
    /// ```
    pub async fn store_get<T: Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
        let store = self.store.read().await;
        store
            .get(key)
            .and_then(|v| v.0.downcast_ref::<T>().cloned())
    }

    /// Check if a key exists in the typed store
    pub async fn store_has(&self, key: &str) -> bool {
        self.store.read().await.contains_key(key)
    }

    /// Remove a value from the typed store
    pub async fn store_remove(&self, key: &str) {
        self.store.write().await.remove(key);
    }

    /// Store a JSON-serializable value (convenience method)
    ///
    /// This stores the value as `serde_json::Value` for easy serialization.
    pub async fn store_json<T: Serialize>(&self, key: impl Into<String>, value: &T) {
        if let Ok(json) = serde_json::to_value(value) {
            self.store_set(key, json).await;
        }
    }

    /// Retrieve a JSON value and deserialize it
    pub async fn store_get_json<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let json: serde_json::Value = self.store_get(key).await?;
        serde_json::from_value(json).ok()
    }

    // ========== Resource Management (Probitas: Disposable pattern) ==========

    /// Register a resource for automatic cleanup
    ///
    /// Resources are disposed in reverse order of registration (LIFO)
    /// when the scenario completes.
    ///
    /// # Example
    /// ```ignore
    /// let db = ctx.register_resource(DatabaseConnection::new()).await;
    /// // Use db...
    /// // Connection will be automatically closed when scenario ends
    /// ```
    pub async fn register_resource<R: Resource + 'static>(&self, resource: R) -> Arc<R> {
        self.resources.register_and_use(resource).await
    }

    /// Register a cleanup function as a resource
    ///
    /// The cleanup function will be called when the scenario completes.
    pub async fn on_cleanup<F, Fut>(&self, name: impl Into<String>, cleanup: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::resource::ResourceResult<()>> + Send + 'static,
    {
        use crate::resource::CleanupResource;

        let resource = CleanupResource::new(name, move || Box::pin(cleanup()));
        self.resources
            .register(Arc::new(resource) as Arc<dyn Resource>)
            .await;
    }

    /// Get the resource manager
    pub fn resources(&self) -> &ResourceManager {
        &self.resources
    }

    // ========== Assertions ==========

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

    // ========== Session Management ==========

    /// Set JWT token for subsequent requests
    pub async fn set_jwt(&self, token: &str) {
        self.client.set_jwt(token).await;
    }

    /// Set a cookie for subsequent requests
    pub async fn set_cookie(&self, name: &str, value: &str) {
        self.client.set_cookie(name, value).await;
    }

    // ========== Parallel Execution ==========

    /// Execute multiple requests in parallel
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

    /// Execute identical requests concurrently for race condition testing
    pub async fn race<F>(&self, count: usize, builder_fn: F) -> Vec<Result<SecurityResponse>>
    where
        F: Fn() -> RequestBuilder,
    {
        use futures::future::join_all;
        let futures: Vec<_> = (0..count).map(|_| builder_fn().send()).collect();
        join_all(futures).await
    }

    // ========== Convenience Methods ==========

    /// Login and store the token
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

    /// Try multiple payloads and return the first successful one
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

    // ========== Security Testing Helpers ==========

    /// SQLi authentication bypass login
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

    // ========== Internal Methods ==========

    async fn record_result(&self, name: String, result: StepResult) {
        self.results.write().await.push((name, result));
    }

    async fn increment_step(&self) {
        *self.current_step.write().await += 1;
    }
}

// ============ Step Definition ============

/// Type alias for cleanup function
pub type CleanupFn = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Type alias for step function
pub type StepFn = Box<
    dyn Fn(Arc<ScenarioContext>) -> Pin<Box<dyn Future<Output = Result<StepResult>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for setup function
pub type SetupFn = Box<
    dyn Fn(Arc<ScenarioContext>) -> Pin<Box<dyn Future<Output = Result<Option<CleanupFn>>> + Send>>
        + Send
        + Sync,
>;

/// A single step in a scenario
pub struct Step {
    /// Step name
    pub name: String,
    /// Step function
    pub func: StepFn,
    /// Step configuration
    pub config: StepConfig,
}

// ============ Scenario Builder ============

/// Scenario builder and executor
pub struct Scenario {
    name: String,
    base_url: Option<String>,
    steps: Vec<Step>,
    tags: Vec<String>,
    setup: Option<SetupFn>,
    default_timeout: Option<Duration>,
}

impl Scenario {
    /// Create a new scenario with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: None,
            steps: Vec::new(),
            tags: Vec::new(),
            setup: None,
            default_timeout: None,
        }
    }

    /// Set the base URL for all requests
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set default timeout for all steps
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.default_timeout = Some(duration);
        self
    }

    /// Add a single tag to the scenario
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags to the scenario
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

    /// Add a setup function with optional cleanup
    ///
    /// The setup function runs before any steps and can return a cleanup function
    /// that runs after all steps complete.
    ///
    /// # Example
    /// ```ignore
    /// Scenario::new("Test")
    ///     .setup(|ctx| Box::pin(async move {
    ///         // Setup code
    ///         ctx.set_var_async("test_data", "value").await;
    ///
    ///         // Return cleanup function
    ///         Ok(Some(Box::new(|| Box::pin(async {
    ///             println!("Cleanup!");
    ///         })) as CleanupFn))
    ///     }))
    /// ```
    pub fn setup<F, Fut>(mut self, func: F) -> Self
    where
        F: Fn(Arc<ScenarioContext>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<CleanupFn>>> + Send + 'static,
    {
        self.setup = Some(Box::new(move |ctx| Box::pin(func(ctx))));
        self
    }

    /// Add a step to the scenario
    pub fn step<F, Fut>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Arc<ScenarioContext>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<StepResult>> + Send + 'static,
    {
        let name = name.into();
        self.steps.push(Step {
            name,
            func: Box::new(move |ctx| Box::pin(func(ctx))),
            config: StepConfig::default(),
        });
        self
    }

    /// Add a step with custom configuration
    pub fn step_with_config<F, Fut>(
        mut self,
        name: impl Into<String>,
        config: StepConfig,
        func: F,
    ) -> Self
    where
        F: Fn(Arc<ScenarioContext>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<StepResult>> + Send + 'static,
    {
        let name = name.into();
        self.steps.push(Step {
            name,
            func: Box::new(move |ctx| Box::pin(func(ctx))),
            config,
        });
        self
    }

    /// Run the scenario
    pub async fn run(self) -> Result<ScenarioResult> {
        let start = std::time::Instant::now();
        info!("Starting scenario: {}", self.name);

        let ctx = Arc::new(ScenarioContext::new(self.base_url.clone())?);
        let mut step_results = Vec::new();
        let mut all_success = true;
        let mut cleanup: Option<CleanupFn> = None;

        // Run setup if provided
        if let Some(setup_fn) = &self.setup {
            info!("Running setup");
            match setup_fn(Arc::clone(&ctx)).await {
                Ok(cleanup_fn) => {
                    cleanup = cleanup_fn;
                }
                Err(e) => {
                    error!("Setup failed: {}", e);
                    return Ok(ScenarioResult {
                        name: self.name,
                        success: false,
                        steps: vec![("setup".to_string(), StepResult::failed(e.to_string()))],
                        duration_ms: start.elapsed().as_millis() as u64,
                        tags: self.tags,
                    });
                }
            }
        }

        // Run steps
        for step in &self.steps {
            info!("Running step: {}", step.name);
            ctx.increment_step().await;

            let step_result = self.run_step_with_retry(&ctx, step).await;

            match step_result {
                Ok(result) => {
                    if result.skipped {
                        warn!("Step '{}' skipped: {:?}", step.name, result.message);
                    } else if result.success {
                        debug!("Step '{}' passed", step.name);
                    } else {
                        error!("Step '{}' failed: {:?}", step.name, result.message);
                        all_success = false;
                    }
                    ctx.record_result(step.name.clone(), result.clone()).await;
                    step_results.push((step.name.clone(), result));

                    if !all_success && !step.config.continue_on_failure {
                        break;
                    }
                }
                Err(e) => {
                    // Check if it's a Skip
                    if let Some(reason) = e.skip_reason() {
                        let result = StepResult::skipped(reason);
                        ctx.record_result(step.name.clone(), result.clone()).await;
                        step_results.push((step.name.clone(), result));
                        warn!("Step '{}' skipped: {}", step.name, reason);
                    } else {
                        error!("Step '{}' error: {}", step.name, e);
                        all_success = false;
                        let result = StepResult::failed(e.to_string());
                        ctx.record_result(step.name.clone(), result.clone()).await;
                        step_results.push((step.name.clone(), result));

                        if !step.config.continue_on_failure {
                            break;
                        }
                    }
                }
            }
        }

        // Run cleanup if provided
        if let Some(cleanup_fn) = cleanup {
            info!("Running cleanup");
            cleanup_fn().await;
        }

        // Dispose all registered resources (in reverse order)
        info!("Disposing resources");
        ctx.resources.dispose_all().await;

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

    async fn run_step_with_retry(
        &self,
        ctx: &Arc<ScenarioContext>,
        step: &Step,
    ) -> Result<StepResult> {
        let timeout = step
            .config
            .timeout
            .or(self.default_timeout)
            .unwrap_or(Duration::from_secs(30));

        let mut last_error: Option<Error> = None;

        for attempt in 0..=step.config.retries {
            if attempt > 0 {
                debug!(
                    "Retrying step '{}' (attempt {}/{})",
                    step.name,
                    attempt + 1,
                    step.config.retries + 1
                );
                tokio::time::sleep(step.config.retry_delay).await;
            }

            match tokio::time::timeout(timeout, (step.func)(Arc::clone(ctx))).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(_) => {
                    last_error = Some(Error::Timeout(format!(
                        "Step '{}' timed out after {:?}",
                        step.name, timeout
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Other("Unknown error".to_string())))
    }
}

// ============ Scenario Runner (Parallel Execution) ============

/// Runner for executing multiple scenarios
pub struct ScenarioRunner {
    scenarios: Vec<Scenario>,
    concurrency: usize,
    failed_only: bool,
    failed_scenarios: Arc<RwLock<Vec<String>>>,
}

impl ScenarioRunner {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            concurrency: 1,
            failed_only: false,
            failed_scenarios: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a scenario to run
    pub fn add_scenario(mut self, scenario: Scenario) -> Self {
        self.scenarios.push(scenario);
        self
    }

    /// Set concurrency level for parallel execution
    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n.max(1);
        self
    }

    /// Only run previously failed scenarios
    pub fn failed_only(mut self, failed_names: Vec<String>) -> Self {
        self.failed_only = true;
        *self.failed_scenarios.blocking_write() = failed_names;
        self
    }

    /// Run all scenarios
    pub async fn run(self) -> Vec<ScenarioResult> {
        use futures::stream::{self, StreamExt};

        let failed_names = self.failed_scenarios.read().await.clone();

        let scenarios_to_run: Vec<_> = if self.failed_only {
            self.scenarios
                .into_iter()
                .filter(|s| failed_names.contains(&s.name))
                .collect()
        } else {
            self.scenarios
        };

        if self.concurrency == 1 {
            // Sequential execution
            let mut results = Vec::new();
            for scenario in scenarios_to_run {
                match scenario.run().await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        error!("Scenario error: {}", e);
                    }
                }
            }
            results
        } else {
            // Parallel execution
            stream::iter(scenarios_to_run)
                .map(|s| async move { s.run().await })
                .buffer_unordered(self.concurrency)
                .filter_map(|r| async { r.ok() })
                .collect()
                .await
        }
    }

    /// Run and return failed scenario names for --failed flag
    pub async fn run_with_tracking(self) -> (Vec<ScenarioResult>, Vec<String>) {
        let results = self.run().await;
        let failed: Vec<String> = results
            .iter()
            .filter(|r| !r.success)
            .map(|r| r.name.clone())
            .collect();
        (results, failed)
    }
}

impl Default for ScenarioRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_result() {
        let result = StepResult::success();
        assert!(result.success);
        assert!(!result.skipped);
        assert!(result.message.is_none());

        let result = StepResult::failed("error");
        assert!(!result.success);
        assert_eq!(result.message, Some("error".to_string()));

        let result = StepResult::skipped("not applicable");
        assert!(result.success);
        assert!(result.skipped);

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

    #[tokio::test]
    async fn test_previous_result() {
        let result = Scenario::new("Previous Test")
            .step("Step 1", |_ctx| async {
                Ok(StepResult::success().with_data("value", serde_json::json!(42)))
            })
            .step("Step 2", |ctx| async move {
                let prev = ctx.previous().await;
                assert!(prev.is_some());
                let data = prev.unwrap().data.get("value").cloned();
                assert_eq!(data, Some(serde_json::json!(42)));
                Ok(StepResult::success())
            })
            .run()
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_step_config_retry() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let attempt_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&attempt_count);

        let result = Scenario::new("Retry Test")
            .step_with_config(
                "Retrying Step",
                StepConfig::new()
                    .retries(2)
                    .retry_delay(Duration::from_millis(10)),
                move |_ctx| {
                    let count = Arc::clone(&count_clone);
                    async move {
                        let n = count.fetch_add(1, Ordering::SeqCst);
                        if n < 2 {
                            // Return an Err to trigger retry
                            Err(Error::Other("Not yet".to_string()))
                        } else {
                            Ok(StepResult::success())
                        }
                    }
                },
            )
            .run()
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_step_index() {
        let indices = Arc::new(RwLock::new(Vec::new()));
        let indices_clone = Arc::clone(&indices);

        let result = Scenario::new("Index Test")
            .step("Step 0", {
                let indices = Arc::clone(&indices_clone);
                move |ctx| {
                    let indices = Arc::clone(&indices);
                    async move {
                        indices.write().await.push(ctx.index().await);
                        Ok(StepResult::success())
                    }
                }
            })
            .step("Step 1", {
                let indices = Arc::clone(&indices_clone);
                move |ctx| {
                    let indices = Arc::clone(&indices);
                    async move {
                        indices.write().await.push(ctx.index().await);
                        Ok(StepResult::success())
                    }
                }
            })
            .step("Step 2", {
                move |ctx| {
                    let indices = Arc::clone(&indices_clone);
                    async move {
                        indices.write().await.push(ctx.index().await);
                        Ok(StepResult::success())
                    }
                }
            })
            .run()
            .await
            .unwrap();

        assert!(result.success);
        let recorded = indices.read().await;
        assert_eq!(*recorded, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_typed_store() {
        let ctx = ScenarioContext::new(None).unwrap();

        // Test basic store operations
        ctx.store_set("count", 42i32).await;
        assert!(ctx.store_has("count").await);

        let count: Option<i32> = ctx.store_get("count").await;
        assert_eq!(count, Some(42));

        // Test type mismatch returns None
        let wrong_type: Option<String> = ctx.store_get("count").await;
        assert!(wrong_type.is_none());

        // Test remove
        ctx.store_remove("count").await;
        assert!(!ctx.store_has("count").await);
    }

    #[tokio::test]
    async fn test_typed_store_complex() {
        #[derive(Clone, PartialEq, Debug)]
        struct UserData {
            id: u32,
            name: String,
        }

        let ctx = ScenarioContext::new(None).unwrap();

        let user = UserData {
            id: 1,
            name: "admin".to_string(),
        };
        ctx.store_set("user", user.clone()).await;

        let retrieved: Option<UserData> = ctx.store_get("user").await;
        assert_eq!(retrieved, Some(user));
    }

    #[tokio::test]
    async fn test_store_json() {
        let ctx = ScenarioContext::new(None).unwrap();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Config {
            enabled: bool,
            count: u32,
        }

        let config = Config {
            enabled: true,
            count: 5,
        };
        ctx.store_json("config", &config).await;

        let retrieved: Option<Config> = ctx.store_get_json("config").await;
        assert_eq!(retrieved, Some(config));
    }
}
