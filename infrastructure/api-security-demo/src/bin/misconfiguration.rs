//! API8: Security Misconfiguration
//!
//! This demonstrates common security misconfigurations:
//! - Verbose error messages exposing internal details
//! - Debug endpoints left enabled in production
//! - Overly permissive CORS settings
//! - Exposed metrics/health endpoints with sensitive data
//!
//! Run: cargo run --bin misconfiguration-demo
//! Test:
//!   # Vulnerable: Get debug info with sensitive data
//!   curl http://localhost:8080/vulnerable/debug
//!
//!   # Secure: Debug endpoint disabled or returns minimal info
//!   curl http://localhost:8080/debug

use api_security_demo::{
    auth::{create_test_user_token, AuthenticatedUser},
    error::AppError,
    models::{DebugInfo, HealthResponse, HealthResponseVulnerable, LoginResponse},
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    start_time: Instant,
    environment: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "misconfiguration=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        start_time: Instant::now(),
        environment: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
    };

    // Vulnerable CORS: allows everything
    let vulnerable_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true); // This is actually invalid with Any origin, but demonstrates the intent

    // Secure CORS: restrictive settings
    let secure_cors = CorsLayer::new()
        .allow_origin("https://trusted-domain.com".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        // Token generation for testing
        .route("/token/{user_id}", get(generate_test_token))
        // Vulnerable endpoints
        .route("/vulnerable/debug", get(debug_vulnerable))
        .route("/vulnerable/health", get(health_vulnerable))
        .route("/vulnerable/error/{code}", get(error_vulnerable))
        .route("/vulnerable/metrics", get(metrics_vulnerable))
        .route("/vulnerable/config", get(config_vulnerable))
        .route("/vulnerable/env", get(env_vulnerable))
        // Secure endpoints
        .route("/debug", get(debug_secure))
        .route("/health", get(health_secure))
        .route("/error/{code}", get(error_secure))
        .route("/metrics", get(metrics_secure))
        // CORS demo routes
        .route("/vulnerable/cors-test", get(cors_test).layer(vulnerable_cors))
        .route("/cors-test", get(cors_test).layer(secure_cors))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    tracing::info!("API8: Security Misconfiguration Demo listening on http://127.0.0.1:8080");
    tracing::info!("Endpoints:");
    tracing::info!("  GET /token/{{user_id}}         - Get test token");
    tracing::info!("  GET /vulnerable/debug        - Debug info (vulnerable - exposes secrets)");
    tracing::info!("  GET /debug                   - Debug info (secure - disabled)");
    tracing::info!("  GET /vulnerable/health       - Health check (vulnerable - verbose)");
    tracing::info!("  GET /health                  - Health check (secure - minimal)");
    tracing::info!("  GET /vulnerable/error/{{code}} - Error demo (vulnerable - stack trace)");
    tracing::info!("  GET /error/{{code}}            - Error demo (secure - generic)");
    tracing::info!("  GET /vulnerable/metrics      - Metrics (vulnerable - no auth)");
    tracing::info!("  GET /metrics                 - Metrics (secure - requires auth)");
    tracing::info!("  GET /vulnerable/config       - Config (vulnerable - exposes all)");
    tracing::info!("  GET /vulnerable/env          - Env vars (vulnerable - exposes all)");
    tracing::info!("  GET /vulnerable/cors-test    - CORS test (vulnerable - allow all)");
    tracing::info!("  GET /cors-test               - CORS test (secure - restrictive)");

    axum::serve(listener, app).await.expect("Server failed");
}

/// Generate test token
async fn generate_test_token(Path(user_id): Path<String>) -> Result<Json<LoginResponse>, AppError> {
    let token = create_test_user_token(&user_id)?;
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}

// ============================================
// Debug Endpoints
// ============================================

/// VULNERABLE: Debug endpoint exposing sensitive information
async fn debug_vulnerable(State(state): State<Arc<AppState>>) -> Json<DebugInfo> {
    tracing::warn!("VULNERABLE: Debug endpoint accessed - exposing sensitive information!");

    Json(DebugInfo {
        environment: state.environment.clone(),
        database_url: "postgresql://admin:SuperSecretPass123@db.internal.company.com:5432/production".to_string(),
        api_keys: vec![
            "sk_live_abc123xyz789".to_string(),
            "AKIAIOSFODNN7EXAMPLE".to_string(),
            "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
        ],
        internal_ips: vec![
            "10.0.1.50".to_string(),
            "10.0.1.51".to_string(),
            "192.168.1.100".to_string(),
        ],
        stack_trace: Some("at main.rs:42\nat handler.rs:156\nat axum::serve".to_string()),
    })
}

/// SECURE: Debug endpoint properly secured
async fn debug_secure(State(state): State<Arc<AppState>>) -> Response {
    tracing::info!("SECURE: Debug endpoint accessed");

    // In production, this should be completely disabled or require strong auth
    if state.environment == "production" {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    // Even in development, only return non-sensitive info
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "environment": state.environment,
            "version": env!("CARGO_PKG_VERSION"),
            "message": "Debug mode active - sensitive info hidden"
        })),
    )
        .into_response()
}

// ============================================
// Health Check Endpoints
// ============================================

/// VULNERABLE: Health check exposing too much information
async fn health_vulnerable(State(state): State<Arc<AppState>>) -> Json<HealthResponseVulnerable> {
    tracing::warn!("VULNERABLE: Health endpoint exposing detailed system information!");

    Json(HealthResponseVulnerable {
        status: "healthy".to_string(),
        database_status: "connected".to_string(),
        database_version: "PostgreSQL 15.2 on x86_64-pc-linux-gnu".to_string(),
        server_version: format!("Rust {} / Axum 0.8", env!("CARGO_PKG_VERSION")),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        memory_usage_mb: 256, // Simulated
        active_connections: 42, // Simulated
    })
}

/// SECURE: Minimal health check
async fn health_secure() -> Json<HealthResponse> {
    tracing::info!("SECURE: Health endpoint returning minimal info");

    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

// ============================================
// Error Handling
// ============================================

/// VULNERABLE: Error messages expose internal details
async fn error_vulnerable(Path(code): Path<String>) -> Response {
    tracing::warn!("VULNERABLE: Error endpoint exposing detailed error information!");

    let error_response = match code.as_str() {
        "db" => serde_json::json!({
            "error": "Database error",
            "details": "FATAL: connection to server at \"10.0.1.50\" (10.0.1.50), port 5432 failed: Connection refused",
            "query": "SELECT * FROM users WHERE id = 1; DROP TABLE users; --",
            "stack_trace": [
                "at db::execute (src/db.rs:142)",
                "at handlers::get_user (src/handlers/user.rs:56)",
                "at axum::routing::Router::call (/.cargo/registry/src/axum-0.8.0/src/routing.rs:201)"
            ],
            "internal_state": {
                "connection_pool_size": 10,
                "active_connections": 3,
                "pending_queries": 5
            }
        }),
        "auth" => serde_json::json!({
            "error": "Authentication failed",
            "details": "JWT signature verification failed for key ID 'prod-key-001'",
            "attempted_user": "admin@company.com",
            "ip_address": "192.168.1.100",
            "jwt_header": {
                "alg": "RS256",
                "typ": "JWT",
                "kid": "prod-key-001"
            }
        }),
        "file" => serde_json::json!({
            "error": "File not found",
            "path": "/var/www/app/config/secrets.yml",
            "permissions": "rw-r-----",
            "owner": "www-data"
        }),
        _ => serde_json::json!({
            "error": "Unknown error",
            "code": code,
            "server_id": "prod-web-01.internal.company.com",
            "request_id": "req_123abc456def"
        }),
    };

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
}

/// SECURE: Generic error messages
async fn error_secure(Path(code): Path<String>) -> Response {
    tracing::info!("SECURE: Error endpoint returning generic error message");

    // Log details internally but don't expose to user
    tracing::error!("Internal error occurred: {}", code);

    let error_response = serde_json::json!({
        "error": "An error occurred",
        "message": "Please try again later or contact support",
        "reference_id": uuid::Uuid::new_v4().to_string()
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
}

// ============================================
// Metrics Endpoint
// ============================================

/// VULNERABLE: Metrics without authentication
async fn metrics_vulnerable(State(state): State<Arc<AppState>>) -> String {
    tracing::warn!("VULNERABLE: Metrics endpoint accessible without authentication!");

    // Prometheus-style metrics exposing sensitive info
    format!(
        r#"# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{{method="GET",endpoint="/api/users",status="200"}} 15234
http_requests_total{{method="POST",endpoint="/api/payments",status="200"}} 8923
http_requests_total{{method="POST",endpoint="/admin/delete-all",status="200"}} 3

# HELP db_connections Active database connections
# TYPE db_connections gauge
db_connections{{pool="primary",host="db-master.internal"}} 45
db_connections{{pool="replica",host="db-replica-1.internal"}} 23

# HELP api_key_usage API key usage
# TYPE api_key_usage counter
api_key_usage{{key_prefix="sk_live_abc"}} 9823
api_key_usage{{key_prefix="sk_live_xyz"}} 4521

# HELP user_sessions Active user sessions
# TYPE user_sessions gauge
user_sessions{{role="admin"}} 5
user_sessions{{role="user"}} 1523

# HELP revenue_total Total revenue processed
# TYPE revenue_total counter
revenue_total{{currency="USD"}} 1523456.78
revenue_total{{currency="EUR"}} 892345.12

# HELP server_info Server information
# TYPE server_info gauge
server_info{{version="1.0.0",hostname="prod-web-01",internal_ip="10.0.1.50"}} 1
"#
    )
}

/// SECURE: Metrics require authentication
async fn metrics_secure(user: AuthenticatedUser) -> Result<String, AppError> {
    tracing::info!("SECURE: Metrics endpoint accessed by authenticated user");

    // Check if user has admin permissions
    if !user.0.permissions.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden(
            "Admin access required for metrics".to_string(),
        ));
    }

    // Return only non-sensitive operational metrics
    Ok(format!(
        r#"# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{{status="2xx"}} 50000
http_requests_total{{status="4xx"}} 1200
http_requests_total{{status="5xx"}} 45

# HELP response_time_seconds Response time histogram
# TYPE response_time_seconds histogram
response_time_seconds_bucket{{le="0.1"}} 45000
response_time_seconds_bucket{{le="0.5"}} 49000
response_time_seconds_bucket{{le="1.0"}} 49800
"#
    ))
}

// ============================================
// Configuration Exposure
// ============================================

/// VULNERABLE: Exposes all configuration
async fn config_vulnerable() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Configuration endpoint exposing all settings!");

    Json(serde_json::json!({
        "database": {
            "host": "db.internal.company.com",
            "port": 5432,
            "username": "app_user",
            "password": "Pr0duct10n_P@ssw0rd!",
            "database": "production_db",
            "ssl_mode": "require"
        },
        "redis": {
            "url": "redis://:RedisSecretPass@redis.internal:6379"
        },
        "aws": {
            "access_key_id": "AKIAIOSFODNN7EXAMPLE",
            "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "region": "us-east-1"
        },
        "jwt": {
            "secret": "super-secret-jwt-signing-key-do-not-share",
            "algorithm": "HS256",
            "expiry_hours": 24
        },
        "stripe": {
            "secret_key": "sk_test_DUMMY_NOT_REAL_KEY_FOR_DEMO",
            "webhook_secret": "whsec_DUMMY_NOT_REAL_KEY_FOR_DEMO"
        },
        "feature_flags": {
            "enable_admin_bypass": true,
            "debug_mode": true,
            "disable_rate_limiting": true
        }
    }))
}

/// VULNERABLE: Exposes environment variables
async fn env_vulnerable() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Environment variables endpoint exposing all env vars!");

    // In a real attack, this would use std::env::vars()
    // We simulate common sensitive env vars
    Json(serde_json::json!({
        "DATABASE_URL": "postgresql://admin:password@db:5432/app",
        "REDIS_URL": "redis://:secretpass@redis:6379",
        "AWS_ACCESS_KEY_ID": "AKIAIOSFODNN7EXAMPLE",
        "AWS_SECRET_ACCESS_KEY": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        "STRIPE_SECRET_KEY": "sk_live_xxxxx",
        "GITHUB_TOKEN": "ghp_xxxxxxxxxxxx",
        "JWT_SECRET": "my-super-secret-key",
        "ADMIN_PASSWORD": "admin123",
        "API_KEY": "sk-proj-xxxxxxxx",
        "PRIVATE_KEY": "-----BEGIN RSA PRIVATE KEY-----\nMIIE...",
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "HOME": "/root"
    }))
}

// ============================================
// CORS Test
// ============================================

/// Simple endpoint for CORS testing
async fn cors_test() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "CORS test successful",
        "data": "This response includes CORS headers"
    }))
}
