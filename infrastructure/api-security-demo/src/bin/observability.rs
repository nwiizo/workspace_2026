//! Chapter 11: Observability for API Security
//!
//! This example demonstrates:
//! - Structured logging with tracing
//! - OpenTelemetry integration for distributed tracing
//! - Security-relevant event logging
//! - Metrics collection for security monitoring
//!
//! Run: cargo run --bin ch11-observability
//! Test:
//!   # Normal request
//!   curl http://localhost:8080/api/data
//!
//!   # Suspicious request (will be logged as security event)
//!   curl http://localhost:8080/api/data?id=1%27%20OR%201=1--
//!
//!   # Failed authentication
//!   curl -H "Authorization: Bearer invalid" http://localhost:8080/api/protected

use axum::{
    Json, Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, Method, StatusCode, Uri, header::AUTHORIZATION},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tracing::{info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Security metrics
#[derive(Debug, Default)]
struct SecurityMetrics {
    total_requests: AtomicU64,
    failed_auth_attempts: AtomicU64,
    suspicious_requests: AtomicU64,
    blocked_requests: AtomicU64,
    sql_injection_attempts: AtomicU64,
    xss_attempts: AtomicU64,
}

impl SecurityMetrics {
    fn increment_total(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_failed_auth(&self) {
        self.failed_auth_attempts.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_suspicious(&self) {
        self.suspicious_requests.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn increment_blocked(&self) {
        self.blocked_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_sqli(&self) {
        self.sql_injection_attempts.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_xss(&self) {
        self.xss_attempts.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            failed_auth_attempts: self.failed_auth_attempts.load(Ordering::Relaxed),
            suspicious_requests: self.suspicious_requests.load(Ordering::Relaxed),
            blocked_requests: self.blocked_requests.load(Ordering::Relaxed),
            sql_injection_attempts: self.sql_injection_attempts.load(Ordering::Relaxed),
            xss_attempts: self.xss_attempts.load(Ordering::Relaxed),
        }
    }
}

#[derive(Serialize)]
struct MetricsSnapshot {
    total_requests: u64,
    failed_auth_attempts: u64,
    suspicious_requests: u64,
    blocked_requests: u64,
    sql_injection_attempts: u64,
    xss_attempts: u64,
}

#[derive(Clone)]
struct AppState {
    metrics: Arc<SecurityMetrics>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing with JSON output for structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch11_observability=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let state = AppState {
        metrics: Arc::new(SecurityMetrics::default()),
    };

    let app = Router::new()
        .route("/api/data", get(get_data))
        .route("/api/protected", get(protected_endpoint))
        .route("/api/user/{id}", get(get_user))
        .route("/metrics", get(get_metrics))
        .route("/health", get(health_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security_logging_middleware,
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!(
        target: "security",
        event = "server_start",
        address = "127.0.0.1:8080",
        "Chapter 11: Observability demonstration server starting"
    );

    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  GET /api/data          - Public data endpoint");
    tracing::info!("  GET /api/protected     - Protected endpoint (requires auth)");
    tracing::info!("  GET /api/user/{{id}}     - Get user by ID");
    tracing::info!("  GET /metrics           - Security metrics");
    tracing::info!("  GET /health            - Health check");
    tracing::info!("");
    tracing::info!("Security events logged:");
    tracing::info!("  - All requests with timing and client info");
    tracing::info!("  - Authentication failures");
    tracing::info!("  - Suspicious query patterns (SQLi, XSS)");
    tracing::info!("  - Blocked requests");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// Security logging middleware
async fn security_logging_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = uri.path().to_string();
    let query = uri.query().map(|q| q.to_string());
    let client_ip = addr.ip().to_string();
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    state.metrics.increment_total();

    // Check for suspicious patterns
    let mut security_flags = Vec::new();

    if let Some(ref q) = query {
        // SQL injection patterns
        let sqli_patterns = [
            "'", "\"", "--", ";", "union", "select", "drop", "insert", "delete",
        ];
        let q_lower = q.to_lowercase();
        for pattern in sqli_patterns {
            if q_lower.contains(pattern) {
                security_flags.push("potential_sqli");
                state.metrics.increment_sqli();
                break;
            }
        }

        // XSS patterns
        let xss_patterns = ["<script", "javascript:", "onerror", "onload", "onclick"];
        for pattern in xss_patterns {
            if q_lower.contains(pattern) {
                security_flags.push("potential_xss");
                state.metrics.increment_xss();
                break;
            }
        }
    }

    if !security_flags.is_empty() {
        state.metrics.increment_suspicious();
        warn!(
            target: "security",
            event = "suspicious_request",
            client_ip = %client_ip,
            method = %method,
            path = %path,
            query = ?query,
            user_agent = %user_agent,
            flags = ?security_flags,
            "Suspicious request detected"
        );
    }

    // Process request
    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    // Log completed request
    info!(
        target: "access",
        event = "request_completed",
        client_ip = %client_ip,
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration.as_millis(),
        user_agent = %user_agent,
        security_flags = ?security_flags,
        "Request completed"
    );

    // Track auth failures
    if status == 401 || status == 403 {
        state.metrics.increment_failed_auth();
        warn!(
            target: "security",
            event = "auth_failure",
            client_ip = %client_ip,
            path = %path,
            status = status,
            "Authentication/authorization failure"
        );
    }

    response
}

/// Get data endpoint with logging
#[instrument(skip(_state), fields(otel.kind = "server"))]
async fn get_data(
    State(_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<DataResponse> {
    info!(
        target: "api",
        event = "data_access",
        params = ?params,
        "Data endpoint accessed"
    );

    Json(DataResponse {
        data: vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ],
        count: 3,
    })
}

#[derive(Serialize)]
struct DataResponse {
    data: Vec<String>,
    count: usize,
}

/// Protected endpoint requiring authentication
#[instrument(skip(headers), fields(otel.kind = "server"))]
async fn protected_endpoint(headers: HeaderMap) -> Result<Json<ProtectedData>, StatusCode> {
    let auth_header = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            // Simplified token validation for demo
            if token == "valid-token" {
                info!(
                    target: "security",
                    event = "auth_success",
                    "Authentication successful"
                );
                Ok(Json(ProtectedData {
                    secret: "This is protected data".to_string(),
                    user: "authenticated_user".to_string(),
                }))
            } else {
                warn!(
                    target: "security",
                    event = "auth_failure",
                    reason = "invalid_token",
                    "Invalid token provided"
                );
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            warn!(
                target: "security",
                event = "auth_failure",
                reason = "missing_token",
                "No authorization header provided"
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[derive(Serialize)]
struct ProtectedData {
    secret: String,
    user: String,
}

/// Get user by ID with parameter validation logging
#[instrument(skip(_state), fields(otel.kind = "server"))]
async fn get_user(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<UserData>, StatusCode> {
    // Log the ID access
    info!(
        target: "api",
        event = "user_access",
        user_id = %id,
        "User data requested"
    );

    // Validate ID format
    if id.chars().any(|c| !c.is_alphanumeric() && c != '-') {
        warn!(
            target: "security",
            event = "invalid_input",
            user_id = %id,
            "Invalid user ID format detected"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Simulated user lookup
    if id == "1" || id == "user-123" {
        Ok(Json(UserData {
            id: id.clone(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        }))
    } else {
        info!(
            target: "api",
            event = "user_not_found",
            user_id = %id,
            "User not found"
        );
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Serialize)]
struct UserData {
    id: String,
    name: String,
    email: String,
}

/// Get security metrics
async fn get_metrics(State(state): State<AppState>) -> Json<MetricsSnapshot> {
    Json(state.metrics.snapshot())
}

/// Health check
async fn health_check() -> &'static str {
    "OK"
}
