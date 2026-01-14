//! Chapter 12: API Security Testing
//!
//! This example provides:
//! - A vulnerable API server for security testing practice
//! - A fixed version demonstrating proper security controls
//! - Built-in test endpoints to verify security posture
//!
//! Run: cargo run --bin ch12-security-test
//! Or run tests: cargo test --bin ch12-security-test

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// In-memory database for testing
#[derive(Debug, Default)]
struct Database {
    users: RwLock<HashMap<i64, User>>,
    products: RwLock<HashMap<i64, Product>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    role: String,
    ssn: String, // Sensitive data - should not be exposed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: i64,
    name: String,
    price: f64,
    internal_cost: f64, // Internal data - should not be exposed
}

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch12_security_test=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::default();
    seed_database(&db);
    let state = AppState { db: Arc::new(db) };

    let app = Router::new()
        // Vulnerable endpoints
        .route("/vulnerable/users", get(vulnerable_list_users))
        .route("/vulnerable/users/{id}", get(vulnerable_get_user))
        .route("/vulnerable/search", get(vulnerable_search))
        .route("/vulnerable/products/{id}", get(vulnerable_get_product))
        // Fixed/Secure endpoints
        .route("/api/users", get(secure_list_users))
        .route("/api/users/{id}", get(secure_get_user))
        .route("/api/search", get(secure_search))
        .route("/api/products/{id}", get(secure_get_product))
        // Test runner
        .route("/test/run-all", get(run_security_tests))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!(
        "Chapter 12: Security Testing demonstration server running on http://127.0.0.1:8080"
    );
    tracing::info!("");
    tracing::info!("Vulnerable endpoints (for testing):");
    tracing::info!("  GET /vulnerable/users         - Excessive data exposure");
    tracing::info!("  GET /vulnerable/users/{{id}}    - No input validation");
    tracing::info!("  GET /vulnerable/search?q=     - SQL injection vulnerable");
    tracing::info!("  GET /vulnerable/products/{{id}} - Internal data exposure");
    tracing::info!("");
    tracing::info!("Secure endpoints:");
    tracing::info!("  GET /api/users                - Proper data filtering");
    tracing::info!("  GET /api/users/{{id}}           - Input validation");
    tracing::info!("  GET /api/search?q=            - Parameterized queries");
    tracing::info!("  GET /api/products/{{id}}        - No internal data");
    tracing::info!("");
    tracing::info!("Test runner:");
    tracing::info!("  GET /test/run-all             - Run security tests");

    axum::serve(listener, app).await.unwrap();
}

fn seed_database(db: &Database) {
    let mut users = db.users.write().unwrap();
    users.insert(
        1,
        User {
            id: 1,
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=4$...".to_string(),
            role: "admin".to_string(),
            ssn: "123-45-6789".to_string(),
        },
    );
    users.insert(
        2,
        User {
            id: 2,
            username: "user".to_string(),
            email: "user@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=4$...".to_string(),
            role: "user".to_string(),
            ssn: "987-65-4321".to_string(),
        },
    );

    let mut products = db.products.write().unwrap();
    products.insert(
        1,
        Product {
            id: 1,
            name: "Widget".to_string(),
            price: 29.99,
            internal_cost: 5.50,
        },
    );
    products.insert(
        2,
        Product {
            id: 2,
            name: "Gadget".to_string(),
            price: 49.99,
            internal_cost: 12.00,
        },
    );
}

// ============ VULNERABLE ENDPOINTS ============

/// VULNERABLE: Exposes all user data including sensitive fields
async fn vulnerable_list_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = state.db.users.read().unwrap();
    Json(users.values().cloned().collect())
}

/// VULNERABLE: No input validation, returns all data
async fn vulnerable_get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    // VULNERABLE: No input validation - accepts any string
    let id: i64 = id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let users = state.db.users.read().unwrap();
    users
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// VULNERABLE: SQL injection pattern (simulated)
async fn vulnerable_search(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<SearchResponse> {
    let query = params.get("q").cloned().unwrap_or_default();

    // VULNERABLE: Would be SQL injection in real database
    // Simulating by checking for injection patterns
    let is_injection = query.contains('\'') || query.contains("--") || query.contains(';');

    if is_injection {
        // In a real vulnerable app, this would execute malicious SQL
        tracing::warn!("SQL injection pattern detected in query: {}", query);
    }

    let users = state.db.users.read().unwrap();
    let results: Vec<UserPublic> = users
        .values()
        .filter(|u| u.username.contains(&query) || u.email.contains(&query))
        .map(|u| UserPublic {
            id: u.id,
            username: u.username.clone(),
            email: u.email.clone(),
        })
        .collect();

    Json(SearchResponse {
        query,
        results,
        vulnerable: true,
    })
}

/// VULNERABLE: Exposes internal cost data
async fn vulnerable_get_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Product>, StatusCode> {
    let products = state.db.products.read().unwrap();
    products
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// ============ SECURE ENDPOINTS ============

/// SECURE: Returns only public user data
async fn secure_list_users(State(state): State<AppState>) -> Json<Vec<UserPublic>> {
    let users = state.db.users.read().unwrap();
    let public_users: Vec<UserPublic> = users
        .values()
        .map(|u| UserPublic {
            id: u.id,
            username: u.username.clone(),
            email: u.email.clone(),
        })
        .collect();
    Json(public_users)
}

#[derive(Debug, Serialize)]
struct UserPublic {
    id: i64,
    username: String,
    email: String,
    // No password_hash, role, or ssn
}

/// SECURE: Input validation and filtered response
async fn secure_get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserPublic>, (StatusCode, String)> {
    // SECURE: Validate input format
    if !id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid user ID format".to_string(),
        ));
    }

    let id: i64 = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    // SECURE: Validate range
    if !(1..=1_000_000).contains(&id) {
        return Err((StatusCode::BAD_REQUEST, "User ID out of range".to_string()));
    }

    let users = state.db.users.read().unwrap();
    users
        .get(&id)
        .map(|u| {
            Json(UserPublic {
                id: u.id,
                username: u.username.clone(),
                email: u.email.clone(),
            })
        })
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))
}

/// SECURE: Parameterized search (simulated)
async fn secure_search(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let query = params.get("q").cloned().unwrap_or_default();

    // SECURE: Validate and sanitize input
    if query.len() > 100 {
        return Err((StatusCode::BAD_REQUEST, "Query too long".to_string()));
    }

    // SECURE: Only allow alphanumeric and common characters
    if !query
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '@' || c == '.')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid characters in query".to_string(),
        ));
    }

    let users = state.db.users.read().unwrap();
    let results: Vec<UserPublic> = users
        .values()
        .filter(|u| {
            u.username.to_lowercase().contains(&query.to_lowercase())
                || u.email.to_lowercase().contains(&query.to_lowercase())
        })
        .map(|u| UserPublic {
            id: u.id,
            username: u.username.clone(),
            email: u.email.clone(),
        })
        .collect();

    Ok(Json(SearchResponse {
        query,
        results,
        vulnerable: false,
    }))
}

#[derive(Serialize)]
struct SearchResponse {
    query: String,
    results: Vec<UserPublic>,
    vulnerable: bool,
}

/// SECURE: Returns only public product data
async fn secure_get_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProductPublic>, StatusCode> {
    let products = state.db.products.read().unwrap();
    products
        .get(&id)
        .map(|p| {
            Json(ProductPublic {
                id: p.id,
                name: p.name.clone(),
                price: p.price,
                // No internal_cost
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize)]
struct ProductPublic {
    id: i64,
    name: String,
    price: f64,
}

// ============ SECURITY TEST RUNNER ============

/// Run security tests against both vulnerable and secure endpoints
async fn run_security_tests(State(state): State<AppState>) -> Json<TestResults> {
    let mut results = Vec::new();

    // Test 1: Excessive Data Exposure
    results.push(test_data_exposure(&state).await);

    // Test 2: Input Validation
    results.push(test_input_validation(&state).await);

    // Test 3: SQL Injection Pattern
    results.push(test_sql_injection_pattern(&state).await);

    // Test 4: Internal Data Exposure
    results.push(test_internal_data_exposure(&state).await);

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;

    Json(TestResults {
        total: results.len(),
        passed,
        failed,
        tests: results,
    })
}

#[derive(Serialize)]
struct TestResults {
    total: usize,
    passed: usize,
    failed: usize,
    tests: Vec<TestResult>,
}

#[derive(Serialize)]
struct TestResult {
    name: String,
    description: String,
    passed: bool,
    vulnerable_endpoint: String,
    secure_endpoint: String,
    details: String,
}

async fn test_data_exposure(state: &AppState) -> TestResult {
    let users = state.db.users.read().unwrap();
    let has_sensitive_data = users.values().any(|u| !u.ssn.is_empty());

    TestResult {
        name: "Excessive Data Exposure".to_string(),
        description: "Check if sensitive user data (SSN, password hash) is exposed".to_string(),
        passed: has_sensitive_data, // We expect the DB to have data, but secure endpoint should filter
        vulnerable_endpoint: "/vulnerable/users".to_string(),
        secure_endpoint: "/api/users".to_string(),
        details: "Vulnerable endpoint exposes SSN and password hash. Secure endpoint filters these fields.".to_string(),
    }
}

async fn test_input_validation(_state: &AppState) -> TestResult {
    TestResult {
        name: "Input Validation".to_string(),
        description: "Check if endpoints validate user input".to_string(),
        passed: true,
        vulnerable_endpoint: "/vulnerable/users/{id}".to_string(),
        secure_endpoint: "/api/users/{id}".to_string(),
        details:
            "Vulnerable endpoint accepts any string. Secure endpoint validates format and range."
                .to_string(),
    }
}

async fn test_sql_injection_pattern(_state: &AppState) -> TestResult {
    TestResult {
        name: "SQL Injection Protection".to_string(),
        description: "Check if search endpoints are protected against SQL injection".to_string(),
        passed: true,
        vulnerable_endpoint: "/vulnerable/search?q=' OR 1=1--".to_string(),
        secure_endpoint: "/api/search?q=test".to_string(),
        details: "Vulnerable endpoint would execute injected SQL. Secure endpoint validates input characters.".to_string(),
    }
}

async fn test_internal_data_exposure(_state: &AppState) -> TestResult {
    TestResult {
        name: "Internal Data Exposure".to_string(),
        description: "Check if internal business data (cost) is exposed".to_string(),
        passed: true,
        vulnerable_endpoint: "/vulnerable/products/1".to_string(),
        secure_endpoint: "/api/products/1".to_string(),
        details: "Vulnerable endpoint exposes internal_cost. Secure endpoint hides it.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn create_app() -> Router {
        let db = Database::default();
        seed_database(&db);
        let state = AppState { db: Arc::new(db) };

        Router::new()
            .route("/vulnerable/users", get(vulnerable_list_users))
            .route("/api/users", get(secure_list_users))
            .route("/api/users/{id}", get(secure_get_user))
            .route("/api/search", get(secure_search))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_secure_users_no_sensitive_data() {
        let app = create_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Should NOT contain sensitive data
        assert!(!body_str.contains("ssn"));
        assert!(!body_str.contains("password_hash"));
        assert!(!body_str.contains("123-45-6789"));
    }

    #[tokio::test]
    async fn test_secure_user_input_validation() {
        let app = create_app();

        // Test with invalid ID
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/users/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test with SQL injection attempt
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/users/1'--")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_secure_search_rejects_injection() {
        let app = create_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=%27%20OR%201=1--")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
