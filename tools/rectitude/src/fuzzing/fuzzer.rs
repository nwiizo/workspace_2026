//! Core fuzzer traits and implementations
//!
//! Provides the Fuzzer trait and various fuzzer implementations for
//! different types of security testing.

use crate::client::SecurityResponse;
use crate::error::Result;
use crate::payloads::sqli::DbType;
use async_trait::async_trait;
use reqwest::Method;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::mutation::MutationStrategy;
use super::result::{FuzzingError, FuzzingHit, FuzzingMiss, FuzzingResult};

/// Success criteria for fuzzing
#[derive(Clone)]
pub enum SuccessCriteria {
    /// Match specific status code
    StatusCode(u16),
    /// Match status code range (inclusive)
    StatusRange(u16, u16),
    /// Response body contains string
    BodyContains(String),
    /// Response body does not contain string
    BodyNotContains(String),
    /// Response body matches regex
    BodyRegex(String),
    /// Custom function for evaluation
    Custom(Arc<dyn Fn(&SecurityResponse) -> bool + Send + Sync>),
    /// All criteria must match
    All(Vec<SuccessCriteria>),
    /// Any criteria must match
    Any(Vec<SuccessCriteria>),
}

impl std::fmt::Debug for SuccessCriteria {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StatusCode(code) => f.debug_tuple("StatusCode").field(code).finish(),
            Self::StatusRange(min, max) => {
                f.debug_tuple("StatusRange").field(min).field(max).finish()
            }
            Self::BodyContains(s) => f.debug_tuple("BodyContains").field(s).finish(),
            Self::BodyNotContains(s) => f.debug_tuple("BodyNotContains").field(s).finish(),
            Self::BodyRegex(s) => f.debug_tuple("BodyRegex").field(s).finish(),
            Self::Custom(_) => f.debug_tuple("Custom").field(&"<function>").finish(),
            Self::All(v) => f.debug_tuple("All").field(v).finish(),
            Self::Any(v) => f.debug_tuple("Any").field(v).finish(),
        }
    }
}

impl SuccessCriteria {
    /// Check if the response matches the success criteria
    pub fn matches(&self, response: &SecurityResponse) -> bool {
        match self {
            Self::StatusCode(code) => response.status.as_u16() == *code,
            Self::StatusRange(min, max) => {
                let status = response.status.as_u16();
                status >= *min && status <= *max
            }
            Self::BodyContains(needle) => response.contains(needle),
            Self::BodyNotContains(needle) => !response.contains(needle),
            Self::BodyRegex(pattern) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(response.text())
                } else {
                    false
                }
            }
            Self::Custom(func) => func(response),
            Self::All(criteria) => criteria.iter().all(|c| c.matches(response)),
            Self::Any(criteria) => criteria.iter().any(|c| c.matches(response)),
        }
    }

    /// Create a success criteria for 2xx status codes
    pub fn is_success() -> Self {
        Self::StatusRange(200, 299)
    }

    /// Create a combined criteria
    pub fn and(self, other: Self) -> Self {
        match self {
            Self::All(mut criteria) => {
                criteria.push(other);
                Self::All(criteria)
            }
            _ => Self::All(vec![self, other]),
        }
    }

    /// Create an alternative criteria
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::Any(mut criteria) => {
                criteria.push(other);
                Self::Any(criteria)
            }
            _ => Self::Any(vec![self, other]),
        }
    }
}

/// Trait for fuzzer implementations
#[async_trait]
pub trait Fuzzer: Send + Sync {
    /// The type of payload this fuzzer uses
    type Payload: Clone + Send + Sync;

    /// Run fuzzing with the given payloads
    async fn fuzz(&self, payloads: Vec<Self::Payload>) -> Result<FuzzingResult<Self::Payload>>;

    /// Check if a response indicates success
    fn is_successful(&self, response: &SecurityResponse) -> bool;
}

/// HTTP Parameter Fuzzer
///
/// Fuzzes a single parameter in an HTTP request.
pub struct ParamFuzzer {
    client: crate::client::SecurityClient,
    endpoint: String,
    method: Method,
    param_name: String,
    param_location: ParamLocation,
    mutation: MutationStrategy,
    success_criteria: SuccessCriteria,
    concurrency: usize,
    delay: Option<Duration>,
    additional_params: Vec<(String, String)>,
    headers: Vec<(String, String)>,
}

/// Where to place the parameter
#[derive(Debug, Clone, Default)]
pub enum ParamLocation {
    /// Query string parameter
    #[default]
    Query,
    /// JSON body parameter
    JsonBody,
    /// Form body parameter
    FormBody,
    /// URL path segment (use {param} in endpoint)
    Path,
    /// Header
    Header,
}

impl ParamFuzzer {
    /// Create a new parameter fuzzer
    pub fn new(
        client: crate::client::SecurityClient,
        endpoint: impl Into<String>,
        param_name: impl Into<String>,
    ) -> Self {
        Self {
            client,
            endpoint: endpoint.into(),
            method: Method::GET,
            param_name: param_name.into(),
            param_location: ParamLocation::default(),
            mutation: MutationStrategy::None,
            success_criteria: SuccessCriteria::is_success(),
            concurrency: 1,
            delay: None,
            additional_params: Vec::new(),
            headers: Vec::new(),
        }
    }

    /// Set the HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Set parameter location
    pub fn location(mut self, location: ParamLocation) -> Self {
        self.param_location = location;
        self
    }

    /// Set mutation strategy
    pub fn mutation(mut self, strategy: MutationStrategy) -> Self {
        self.mutation = strategy;
        self
    }

    /// Set success criteria
    pub fn success_when(mut self, criteria: SuccessCriteria) -> Self {
        self.success_criteria = criteria;
        self
    }

    /// Set concurrency level
    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n.max(1);
        self
    }

    /// Set delay between requests
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Add an additional static parameter
    pub fn with_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_params.push((name.into(), value.into()));
        self
    }

    /// Add a header to all requests
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Execute a single fuzz attempt
    async fn execute_single(&self, payload: &str) -> Result<(SecurityResponse, Duration)> {
        let start = Instant::now();

        let mut request = self.client.request(self.method.clone(), &self.endpoint);

        // Add headers
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }

        // Add parameter based on location
        match &self.param_location {
            ParamLocation::Query => {
                request = request.query(&self.param_name, payload);
                for (name, value) in &self.additional_params {
                    request = request.query(name, value);
                }
            }
            ParamLocation::JsonBody => {
                let mut body = serde_json::Map::new();
                body.insert(
                    self.param_name.clone(),
                    serde_json::Value::String(payload.to_string()),
                );
                for (name, value) in &self.additional_params {
                    body.insert(name.clone(), serde_json::Value::String(value.clone()));
                }
                request = request.json(&serde_json::Value::Object(body));
            }
            ParamLocation::FormBody => {
                let mut params = vec![(self.param_name.clone(), payload.to_string())];
                for (name, value) in &self.additional_params {
                    params.push((name.clone(), value.clone()));
                }
                request = request.form(&params);
            }
            ParamLocation::Path => {
                // Replace {param_name} in endpoint with payload
                let endpoint = self
                    .endpoint
                    .replace(&format!("{{{}}}", self.param_name), payload);
                request = self.client.request(self.method.clone(), &endpoint);
                for (name, value) in &self.headers {
                    request = request.header(name, value);
                }
            }
            ParamLocation::Header => {
                request = request.header(&self.param_name, payload);
            }
        }

        let response = request.send().await?;
        let duration = start.elapsed();

        Ok((response, duration))
    }

    /// Run fuzzing sequentially
    async fn fuzz_sequential(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        let start = Instant::now();
        let mut result = FuzzingResult::new();

        for payload in payloads {
            // Apply mutations
            let mutated = self.mutation.apply(&payload);

            for mutated_payload in mutated {
                match self.execute_single(&mutated_payload).await {
                    Ok((response, duration)) => {
                        let status = response.status.as_u16();

                        if self.success_criteria.matches(&response) {
                            let snippet = response
                                .text
                                .as_ref()
                                .map(|t| t.chars().take(200).collect::<String>())
                                .unwrap_or_default();

                            result.add_hit(
                                FuzzingHit::new(mutated_payload, status, duration)
                                    .with_snippet(snippet),
                            );
                        } else {
                            result.add_miss(FuzzingMiss::new(mutated_payload, status, duration));
                        }
                    }
                    Err(e) => {
                        result.add_error(
                            FuzzingError::new(e.to_string()).with_payload(&mutated_payload),
                        );
                    }
                }

                // Apply delay if configured
                if let Some(delay) = self.delay {
                    tokio::time::sleep(delay).await;
                }
            }
        }

        result.finalize(start.elapsed());
        result
    }

    /// Run fuzzing with concurrency
    async fn fuzz_concurrent(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        use futures::stream::{self, StreamExt};

        let start = Instant::now();

        // Expand all payloads with mutations
        let expanded: Vec<String> = payloads
            .iter()
            .flat_map(|p| self.mutation.apply(p))
            .collect();

        // Process with concurrency limit
        let results: Vec<_> = stream::iter(expanded)
            .map(|payload| async move {
                let result = self.execute_single(&payload).await;
                (payload, result)
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        // Collect results
        let mut fuzzing_result = FuzzingResult::new();
        for (payload, result) in results {
            match result {
                Ok((response, duration)) => {
                    let status = response.status.as_u16();
                    if self.success_criteria.matches(&response) {
                        let snippet = response
                            .text
                            .as_ref()
                            .map(|t| t.chars().take(200).collect::<String>())
                            .unwrap_or_default();

                        fuzzing_result.add_hit(
                            FuzzingHit::new(payload, status, duration).with_snippet(snippet),
                        );
                    } else {
                        fuzzing_result.add_miss(FuzzingMiss::new(payload, status, duration));
                    }
                }
                Err(e) => {
                    fuzzing_result
                        .add_error(FuzzingError::new(e.to_string()).with_payload(&payload));
                }
            }
        }

        fuzzing_result.finalize(start.elapsed());
        fuzzing_result
    }

    /// Run the fuzzer
    pub async fn run(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        if self.concurrency <= 1 {
            self.fuzz_sequential(payloads).await
        } else {
            self.fuzz_concurrent(payloads).await
        }
    }
}

#[async_trait]
impl Fuzzer for ParamFuzzer {
    type Payload = String;

    async fn fuzz(&self, payloads: Vec<Self::Payload>) -> Result<FuzzingResult<Self::Payload>> {
        Ok(self.run(payloads).await)
    }

    fn is_successful(&self, response: &SecurityResponse) -> bool {
        self.success_criteria.matches(response)
    }
}

/// SQL Injection Fuzzer
///
/// Specialized fuzzer for SQL injection testing.
pub struct SqliFuzzer {
    param_fuzzer: ParamFuzzer,
    db_type: Option<DbType>,
    include_blind: bool,
    include_time_based: bool,
}

impl SqliFuzzer {
    /// Create a new SQLi fuzzer
    pub fn new(
        client: crate::client::SecurityClient,
        endpoint: impl Into<String>,
        param_name: impl Into<String>,
    ) -> Self {
        let param_fuzzer = ParamFuzzer::new(client, endpoint, param_name)
            .method(Method::POST)
            .location(ParamLocation::JsonBody);

        Self {
            param_fuzzer,
            db_type: None,
            include_blind: false,
            include_time_based: false,
        }
    }

    /// Set the target database type for targeted payloads
    pub fn db_type(mut self, db_type: DbType) -> Self {
        self.db_type = Some(db_type);
        self
    }

    /// Include blind SQLi payloads
    pub fn include_blind(mut self, include: bool) -> Self {
        self.include_blind = include;
        self
    }

    /// Include time-based blind SQLi payloads
    pub fn include_time_based(mut self, include: bool) -> Self {
        self.include_time_based = include;
        self
    }

    /// Set HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.param_fuzzer = self.param_fuzzer.method(method);
        self
    }

    /// Set parameter location
    pub fn location(mut self, location: ParamLocation) -> Self {
        self.param_fuzzer = self.param_fuzzer.location(location);
        self
    }

    /// Set success criteria
    pub fn success_when(mut self, criteria: SuccessCriteria) -> Self {
        self.param_fuzzer = self.param_fuzzer.success_when(criteria);
        self
    }

    /// Set concurrency
    pub fn concurrency(mut self, n: usize) -> Self {
        self.param_fuzzer = self.param_fuzzer.concurrency(n);
        self
    }

    /// Add an additional parameter
    pub fn with_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.param_fuzzer = self.param_fuzzer.with_param(name, value);
        self
    }

    /// Generate SQLi payloads based on configuration
    pub fn generate_payloads(&self) -> Vec<String> {
        use crate::payloads::sqli;

        let mut payloads: Vec<String> = sqli::auth_bypass_payloads()
            .into_iter()
            .map(|p| p.payload)
            .collect();

        // Add DB-specific payloads
        if let Some(ref db_type) = self.db_type {
            let db_payloads = match db_type {
                DbType::Sqlite => sqli::sqlite_payloads(),
                DbType::MySql => sqli::mysql_payloads(),
                DbType::PostgreSql => sqli::postgresql_payloads(),
                _ => Vec::new(),
            };
            payloads.extend(db_payloads.into_iter().map(|p| p.payload));
        }

        // Add blind payloads
        if self.include_blind {
            payloads.extend(
                sqli::blind_boolean_payloads()
                    .into_iter()
                    .map(|p| p.payload),
            );
        }

        // Add time-based payloads
        if self.include_time_based {
            payloads.extend(sqli::blind_time_payloads().into_iter().map(|p| p.payload));
        }

        payloads
    }

    /// Run the fuzzer with auto-generated payloads
    pub async fn run(&self) -> FuzzingResult<String> {
        let payloads = self.generate_payloads();
        self.param_fuzzer.run(payloads).await
    }

    /// Run the fuzzer with custom payloads
    pub async fn run_with(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        self.param_fuzzer.run(payloads).await
    }
}

/// XSS Fuzzer
///
/// Specialized fuzzer for XSS testing.
pub struct XssFuzzer {
    param_fuzzer: ParamFuzzer,
    context: XssContext,
    include_polyglots: bool,
}

/// Context where XSS payload will be inserted
#[derive(Debug, Clone, Default)]
pub enum XssContext {
    /// In HTML body
    #[default]
    Html,
    /// In HTML attribute value
    Attribute,
    /// In JavaScript code
    JavaScript,
    /// In URL
    Url,
    /// In CSS
    Css,
}

impl XssFuzzer {
    /// Create a new XSS fuzzer
    pub fn new(
        client: crate::client::SecurityClient,
        endpoint: impl Into<String>,
        param_name: impl Into<String>,
    ) -> Self {
        let param_fuzzer = ParamFuzzer::new(client, endpoint, param_name)
            .mutation(MutationStrategy::xss_bypass_encodings());

        Self {
            param_fuzzer,
            context: XssContext::default(),
            include_polyglots: true,
        }
    }

    /// Set the XSS context
    pub fn context(mut self, context: XssContext) -> Self {
        self.context = context;
        self
    }

    /// Include polyglot payloads
    pub fn include_polyglots(mut self, include: bool) -> Self {
        self.include_polyglots = include;
        self
    }

    /// Set HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.param_fuzzer = self.param_fuzzer.method(method);
        self
    }

    /// Set parameter location
    pub fn location(mut self, location: ParamLocation) -> Self {
        self.param_fuzzer = self.param_fuzzer.location(location);
        self
    }

    /// Set success criteria
    pub fn success_when(mut self, criteria: SuccessCriteria) -> Self {
        self.param_fuzzer = self.param_fuzzer.success_when(criteria);
        self
    }

    /// Set concurrency
    pub fn concurrency(mut self, n: usize) -> Self {
        self.param_fuzzer = self.param_fuzzer.concurrency(n);
        self
    }

    /// Generate XSS payloads based on configuration
    pub fn generate_payloads(&self) -> Vec<String> {
        use crate::payloads::xss;

        let mut payloads = xss::basic_payloads_str();

        // Add context-specific payloads
        payloads.extend(match self.context {
            XssContext::Html => xss::html_context_payloads(),
            XssContext::Attribute => xss::attribute_context_payloads(),
            XssContext::JavaScript => xss::javascript_context_payloads(),
            XssContext::Url => xss::url_context_payloads(),
            XssContext::Css => Vec::new(), // TODO: Add CSS context payloads
        });

        // Add polyglots
        if self.include_polyglots {
            payloads.extend(xss::polyglot_payloads());
        }

        payloads
    }

    /// Run the fuzzer with auto-generated payloads
    pub async fn run(&self) -> FuzzingResult<String> {
        let payloads = self.generate_payloads();
        self.param_fuzzer.run(payloads).await
    }

    /// Run the fuzzer with custom payloads
    pub async fn run_with(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        self.param_fuzzer.run(payloads).await
    }
}

/// Path Traversal Fuzzer
///
/// Specialized fuzzer for path traversal testing.
pub struct TraversalFuzzer {
    param_fuzzer: ParamFuzzer,
    target_file: String,
    max_depth: usize,
    encoding_variations: bool,
}

impl TraversalFuzzer {
    /// Create a new path traversal fuzzer
    pub fn new(
        client: crate::client::SecurityClient,
        endpoint: impl Into<String>,
        param_name: impl Into<String>,
    ) -> Self {
        Self {
            param_fuzzer: ParamFuzzer::new(client, endpoint, param_name),
            target_file: "/etc/passwd".to_string(),
            max_depth: 10,
            encoding_variations: true,
        }
    }

    /// Set the target file to try to access
    pub fn target_file(mut self, file: impl Into<String>) -> Self {
        self.target_file = file.into();
        self
    }

    /// Set maximum traversal depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Include encoding variations
    pub fn encoding_variations(mut self, include: bool) -> Self {
        self.encoding_variations = include;
        self
    }

    /// Set HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.param_fuzzer = self.param_fuzzer.method(method);
        self
    }

    /// Set parameter location
    pub fn location(mut self, location: ParamLocation) -> Self {
        self.param_fuzzer = self.param_fuzzer.location(location);
        self
    }

    /// Set success criteria
    pub fn success_when(mut self, criteria: SuccessCriteria) -> Self {
        self.param_fuzzer = self.param_fuzzer.success_when(criteria);
        self
    }

    /// Generate path traversal payloads
    pub fn generate_payloads(&self) -> Vec<String> {
        use crate::payloads::traversal;

        let mut payloads = traversal::unix_traversal_payloads(&self.target_file, self.max_depth);
        payloads.extend(traversal::windows_traversal_payloads(
            &self.target_file,
            self.max_depth,
        ));

        if self.encoding_variations {
            payloads.extend(traversal::encoded_traversal_payloads(
                &self.target_file,
                self.max_depth,
            ));
        }

        payloads
    }

    /// Run the fuzzer with auto-generated payloads
    pub async fn run(&self) -> FuzzingResult<String> {
        let payloads = self.generate_payloads();
        self.param_fuzzer.run(payloads).await
    }

    /// Run the fuzzer with custom payloads
    pub async fn run_with(&self, payloads: Vec<String>) -> FuzzingResult<String> {
        self.param_fuzzer.run(payloads).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_criteria_status_code() {
        let criteria = SuccessCriteria::StatusCode(200);

        // Mock response would be needed for full testing
        // This tests the API
        assert!(matches!(criteria, SuccessCriteria::StatusCode(200)));
    }

    #[test]
    fn test_success_criteria_and() {
        let criteria =
            SuccessCriteria::StatusCode(200).and(SuccessCriteria::BodyContains("ok".to_string()));

        match criteria {
            SuccessCriteria::All(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected All variant"),
        }
    }

    #[test]
    fn test_success_criteria_or() {
        let criteria = SuccessCriteria::StatusCode(200).or(SuccessCriteria::StatusCode(201));

        match criteria {
            SuccessCriteria::Any(v) => assert_eq!(v.len(), 2),
            _ => panic!("Expected Any variant"),
        }
    }

    #[test]
    fn test_sqli_fuzzer_payloads() {
        let client = crate::client::SecurityClient::new().unwrap();
        let fuzzer = SqliFuzzer::new(client, "/login", "email")
            .db_type(DbType::Sqlite)
            .include_blind(true);

        let payloads = fuzzer.generate_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("OR 1=1")));
    }

    #[test]
    fn test_xss_fuzzer_payloads() {
        let client = crate::client::SecurityClient::new().unwrap();
        let fuzzer = XssFuzzer::new(client, "/search", "q")
            .context(XssContext::Html)
            .include_polyglots(true);

        let payloads = fuzzer.generate_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_traversal_fuzzer_payloads() {
        let client = crate::client::SecurityClient::new().unwrap();
        let fuzzer = TraversalFuzzer::new(client, "/download", "file")
            .target_file("/etc/passwd")
            .max_depth(5);

        let payloads = fuzzer.generate_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("..")));
    }
}
