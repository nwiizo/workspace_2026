//! API9: Improper Inventory Management
//!
//! This demonstrates vulnerabilities from poor API inventory management:
//! - Deprecated API versions still accessible without authentication
//! - Undocumented internal endpoints exposed
//! - Beta/test endpoints left in production
//! - Inconsistent security across API versions
//!
//! Run: cargo run --bin inventory-demo
//! Test:
//!   # Vulnerable: Old API version without auth
//!   curl http://localhost:8080/v1/users/1
//!
//!   # Secure: Current version requires auth
//!   TOKEN=$(curl -s http://localhost:8080/token/alice | jq -r .access_token)
//!   curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/v3/users/1

use api_security_demo::{
    auth::{create_test_admin_token, create_test_user_token, AuthenticatedUser},
    db::Database,
    error::AppError,
    models::{ApiVersionInfo, DeprecationNotice, LoginResponse},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "inventory=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::new_in_memory().expect("Failed to create database");
    db.seed_users().expect("Failed to seed users");

    let state = AppState { db };

    let app = Router::new()
        // Token generation
        .route("/token/{user_id}", get(generate_test_token))
        .route("/admin-token/{user_id}", get(generate_admin_token))

        // API Version info
        .route("/api/versions", get(list_api_versions))

        // ===== VULNERABLE: Old API versions =====
        // V1: Deprecated, no authentication (VULNERABLE)
        .route("/v1/users", get(v1_list_users))
        .route("/v1/users/{id}", get(v1_get_user))

        // V2: Deprecated, weak authentication (VULNERABLE)
        .route("/v2/users", get(v2_list_users))
        .route("/v2/users/{id}", get(v2_get_user))

        // ===== SECURE: Current API version =====
        // V3: Current, proper authentication
        .route("/api/v3/users", get(v3_list_users))
        .route("/api/v3/users/{id}", get(v3_get_user))

        // ===== VULNERABLE: Undocumented endpoints =====
        .route("/internal/admin/stats", get(internal_stats))
        .route("/internal/debug/db", get(internal_db_debug))
        .route("/_hidden/backdoor", get(hidden_backdoor))

        // ===== VULNERABLE: Beta/test endpoints =====
        .route("/beta/experimental", get(beta_experimental))
        .route("/test/mock-data", get(test_mock_data))
        .route("/staging/preview", get(staging_preview))

        // ===== VULNERABLE: Old documentation =====
        .route("/swagger.json", get(swagger_json_old))
        .route("/openapi.yaml", get(openapi_yaml_old))

        // ===== SECURE: Proper documentation =====
        .route("/api/v3/openapi.json", get(openapi_json_current))

        // Deprecation info
        .route("/api/deprecations", get(list_deprecations))

        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    tracing::info!("API9: Improper Inventory Management Demo listening on http://127.0.0.1:8080");
    tracing::info!("");
    tracing::info!("=== Vulnerable Endpoints (Poor Inventory Management) ===");
    tracing::info!("Old API versions (deprecated but accessible):");
    tracing::info!("  GET /v1/users              - No auth required (VULNERABLE)");
    tracing::info!("  GET /v1/users/{{id}}         - No auth required (VULNERABLE)");
    tracing::info!("  GET /v2/users              - Weak auth (VULNERABLE)");
    tracing::info!("  GET /v2/users/{{id}}         - Weak auth (VULNERABLE)");
    tracing::info!("");
    tracing::info!("Undocumented internal endpoints:");
    tracing::info!("  GET /internal/admin/stats  - Exposes system stats");
    tracing::info!("  GET /internal/debug/db     - Exposes DB info");
    tracing::info!("  GET /_hidden/backdoor      - Test backdoor left in prod");
    tracing::info!("");
    tracing::info!("Beta/test endpoints:");
    tracing::info!("  GET /beta/experimental     - Unstable features");
    tracing::info!("  GET /test/mock-data        - Test data endpoint");
    tracing::info!("  GET /staging/preview       - Staging preview");
    tracing::info!("");
    tracing::info!("Old documentation:");
    tracing::info!("  GET /swagger.json          - Outdated API docs");
    tracing::info!("  GET /openapi.yaml          - Outdated API docs");
    tracing::info!("");
    tracing::info!("=== Secure Endpoints (Proper Inventory Management) ===");
    tracing::info!("  GET /token/{{user_id}}       - Get user token");
    tracing::info!("  GET /admin-token/{{user_id}} - Get admin token");
    tracing::info!("  GET /api/versions          - List API versions");
    tracing::info!("  GET /api/v3/users          - Current version (auth required)");
    tracing::info!("  GET /api/v3/users/{{id}}     - Current version (auth required)");
    tracing::info!("  GET /api/v3/openapi.json   - Current API documentation");
    tracing::info!("  GET /api/deprecations      - List deprecated endpoints");

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

/// Generate admin token
async fn generate_admin_token(Path(user_id): Path<String>) -> Result<Json<LoginResponse>, AppError> {
    let token = create_test_admin_token(&user_id)?;
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}

// ============================================
// API Version Management
// ============================================

/// List all API versions
async fn list_api_versions() -> Json<Vec<ApiVersionInfo>> {
    Json(vec![
        ApiVersionInfo {
            version: "v1".to_string(),
            deprecated: true,
            sunset_date: Some("2024-01-01".to_string()),
            migration_guide: Some("/docs/migrate-v1-to-v3".to_string()),
        },
        ApiVersionInfo {
            version: "v2".to_string(),
            deprecated: true,
            sunset_date: Some("2024-06-01".to_string()),
            migration_guide: Some("/docs/migrate-v2-to-v3".to_string()),
        },
        ApiVersionInfo {
            version: "v3".to_string(),
            deprecated: false,
            sunset_date: None,
            migration_guide: None,
        },
    ])
}

/// List deprecation notices
async fn list_deprecations() -> Json<Vec<DeprecationNotice>> {
    Json(vec![
        DeprecationNotice {
            endpoint: "/v1/*".to_string(),
            deprecated_since: "2023-01-01".to_string(),
            sunset_date: "2024-01-01".to_string(),
            replacement: "/api/v3/*".to_string(),
        },
        DeprecationNotice {
            endpoint: "/v2/*".to_string(),
            deprecated_since: "2023-06-01".to_string(),
            sunset_date: "2024-06-01".to_string(),
            replacement: "/api/v3/*".to_string(),
        },
        DeprecationNotice {
            endpoint: "/swagger.json".to_string(),
            deprecated_since: "2023-01-01".to_string(),
            sunset_date: "2024-01-01".to_string(),
            replacement: "/api/v3/openapi.json".to_string(),
        },
    ])
}

// ============================================
// V1 API (Deprecated - No Auth) - VULNERABLE
// ============================================

/// V1: List users without authentication
async fn v1_list_users(State(state): State<Arc<AppState>>) -> Response {
    tracing::warn!("VULNERABLE: V1 API accessed - no authentication required!");
    tracing::warn!("This deprecated endpoint should have been disabled!");

    // Return data without any auth check
    let users = vec![
        serde_json::json!({"id": 1, "email": "admin@example.com", "role": "admin"}),
        serde_json::json!({"id": 2, "email": "user@example.com", "role": "user"}),
    ];

    (
        StatusCode::OK,
        [("X-Deprecation-Warning", "This API version is deprecated. Use /api/v3/users")],
        Json(serde_json::json!({
            "users": users,
            "warning": "V1 API - No authentication required (VULNERABLE)"
        })),
    )
        .into_response()
}

/// V1: Get user by ID without authentication
async fn v1_get_user(Path(id): Path<i64>) -> Response {
    tracing::warn!("VULNERABLE: V1 API user/{} accessed without auth!", id);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "email": format!("user{}@example.com", id),
            "role": "user",
            "password_hash": "$argon2id$...", // V1 even exposed password hashes!
            "warning": "V1 API - Exposes sensitive data (VULNERABLE)"
        })),
    )
        .into_response()
}

// ============================================
// V2 API (Deprecated - Weak Auth) - VULNERABLE
// ============================================

/// V2: List users with weak authentication
async fn v2_list_users(
    headers: axum::http::HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    tracing::warn!("VULNERABLE: V2 API accessed - weak authentication!");

    // V2 used simple API key auth that was easily guessable
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    // VULNERABILITY: Hardcoded API keys, no rotation
    let valid_keys = ["api_key_123", "test_key", "dev_key"];

    if let Some(key) = api_key {
        if valid_keys.contains(&key) {
            let users = vec![
                serde_json::json!({"id": 1, "email": "admin@example.com"}),
                serde_json::json!({"id": 2, "email": "user@example.com"}),
            ];

            return (
                StatusCode::OK,
                [("X-Deprecation-Warning", "This API version is deprecated")],
                Json(serde_json::json!({
                    "users": users,
                    "warning": "V2 API - Weak API key auth (VULNERABLE)"
                })),
            )
                .into_response();
        }
    }

    // VULNERABILITY: Error reveals valid auth method
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "Invalid API key",
            "hint": "Use X-API-Key header with valid key",
            "valid_keys_for_demo": valid_keys
        })),
    )
        .into_response()
}

/// V2: Get user by ID with weak authentication
async fn v2_get_user(headers: axum::http::HeaderMap, Path(id): Path<i64>) -> Response {
    let api_key = headers.get("X-API-Key").and_then(|v| v.to_str().ok());
    let valid_keys = ["api_key_123", "test_key", "dev_key"];

    if api_key.map_or(false, |k| valid_keys.contains(&k)) {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": id,
                "email": format!("user{}@example.com", id),
                "warning": "V2 API - Weak auth (VULNERABLE)"
            })),
        )
            .into_response();
    }

    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response()
}

// ============================================
// V3 API (Current - Proper Auth) - SECURE
// ============================================

/// V3: List users with proper authentication
async fn v3_list_users(
    user: AuthenticatedUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: V3 API accessed by user: {}", user.0.sub);

    // Only return data appropriate to user's role
    let users = if user.0.permissions.contains(&"admin".to_string()) {
        vec![
            serde_json::json!({"id": 1, "email": "admin@example.com", "role": "admin"}),
            serde_json::json!({"id": 2, "email": "user@example.com", "role": "user"}),
        ]
    } else {
        // Regular users only see their own data
        vec![serde_json::json!({"id": 1, "email": format!("{}@example.com", user.0.sub)})]
    };

    Ok(Json(serde_json::json!({
        "users": users,
        "version": "v3",
        "authenticated_as": user.0.sub
    })))
}

/// V3: Get user by ID with proper authentication
async fn v3_get_user(
    user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: V3 API user/{} accessed by: {}", id, user.0.sub);

    // Check authorization - users can only access their own data unless admin
    let is_admin = user.0.permissions.contains(&"admin".to_string());
    let is_own_data = user.0.sub == format!("user_{}", id);

    if !is_admin && !is_own_data {
        return Err(AppError::Forbidden("Cannot access other users' data".to_string()));
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "email": format!("user{}@example.com", id),
        "version": "v3"
    })))
}

// ============================================
// Undocumented Internal Endpoints - VULNERABLE
// ============================================

/// Internal stats endpoint (should be firewalled)
async fn internal_stats() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Internal stats endpoint accessed without auth!");

    Json(serde_json::json!({
        "warning": "This endpoint should not be publicly accessible!",
        "stats": {
            "total_users": 15234,
            "active_sessions": 892,
            "revenue_today": 45678.90,
            "failed_logins_24h": 156,
            "admin_users": ["admin@company.com", "superuser@company.com"]
        },
        "infrastructure": {
            "database_host": "db-master.internal.company.com",
            "redis_host": "redis-cluster.internal.company.com",
            "queue_depth": 1523
        }
    }))
}

/// Internal database debug endpoint
async fn internal_db_debug() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Internal DB debug endpoint accessed!");

    Json(serde_json::json!({
        "warning": "Debug endpoint exposed in production!",
        "database": {
            "connection_string": "postgresql://app:password123@db:5432/production",
            "pool_size": 20,
            "active_connections": 15,
            "slow_queries": [
                "SELECT * FROM users WHERE email LIKE '%admin%'",
                "SELECT * FROM orders WHERE total > 10000"
            ]
        }
    }))
}

/// Hidden backdoor left by developer
async fn hidden_backdoor() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Hidden backdoor accessed!");

    Json(serde_json::json!({
        "warning": "Developer backdoor left in production!",
        "access_granted": true,
        "role": "super_admin",
        "note": "This was added for debugging and never removed"
    }))
}

// ============================================
// Beta/Test Endpoints - VULNERABLE
// ============================================

/// Beta experimental features
async fn beta_experimental() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Beta endpoint accessible in production!");

    Json(serde_json::json!({
        "warning": "Beta endpoints should not be in production!",
        "features": {
            "experimental_ai": true,
            "unsafe_mode": true,
            "bypass_validation": true
        },
        "note": "These features are not security tested"
    }))
}

/// Test mock data endpoint
async fn test_mock_data() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Test endpoint with mock data in production!");

    Json(serde_json::json!({
        "warning": "Test endpoint in production!",
        "mock_users": [
            {"email": "test@test.com", "password": "test123"},
            {"email": "admin@test.com", "password": "admin123"}
        ],
        "test_api_keys": ["test_key_1", "test_key_2"]
    }))
}

/// Staging preview endpoint
async fn staging_preview() -> Json<serde_json::Value> {
    tracing::warn!("VULNERABLE: Staging endpoint accessible in production!");

    Json(serde_json::json!({
        "warning": "Staging preview endpoint exposed!",
        "environment": "production", // Should be staging!
        "upcoming_features": ["feature_x", "feature_y"],
        "unreleased_data": true
    }))
}

// ============================================
// Documentation Endpoints
// ============================================

/// Old Swagger JSON (outdated)
async fn swagger_json_old() -> Response {
    tracing::warn!("VULNERABLE: Outdated Swagger documentation accessed!");

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::json!({
            "swagger": "2.0",
            "info": {
                "title": "Legacy API (OUTDATED)",
                "version": "1.0.0",
                "description": "WARNING: This documentation is outdated!"
            },
            "paths": {
                "/v1/users": {
                    "get": {
                        "description": "List users (NO AUTH REQUIRED - OUTDATED DOC)"
                    }
                }
            },
            "warning": "This documentation does not reflect current security requirements!"
        }).to_string(),
    ).into_response()
}

/// Old OpenAPI YAML (outdated)
async fn openapi_yaml_old() -> Response {
    tracing::warn!("VULNERABLE: Outdated OpenAPI documentation accessed!");

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/yaml")],
        r#"
openapi: "3.0.0"
info:
  title: "Legacy API (OUTDATED)"
  version: "2.0.0"
  description: "WARNING: This documentation is outdated and does not reflect current security!"
paths:
  /v2/users:
    get:
      description: "Uses deprecated X-API-Key authentication"
      security:
        - api_key: []  # OUTDATED: Now uses JWT
"#,
    ).into_response()
}

/// Current OpenAPI JSON (secure)
async fn openapi_json_current() -> Json<serde_json::Value> {
    tracing::info!("SECURE: Current API documentation accessed");

    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Secure API v3",
            "version": "3.0.0",
            "description": "Current API version with proper security"
        },
        "security": [
            {"bearerAuth": []}
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        },
        "paths": {
            "/api/v3/users": {
                "get": {
                    "security": [{"bearerAuth": []}],
                    "description": "List users (requires JWT authentication)"
                }
            }
        }
    }))
}
