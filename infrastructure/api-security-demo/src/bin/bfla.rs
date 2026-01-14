//! Chapter 4: BFLA (Broken Function Level Authorization) Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: Admin function accessible to any authenticated user
//! - Secure endpoint: Admin function only accessible to users with admin permission
//!
//! Run: cargo run --bin ch04-bfla
//! Test:
//!   # Get tokens
//!   USER_TOKEN=$(curl -s http://localhost:8080/token/user | jq -r .access_token)
//!   ADMIN_TOKEN=$(curl -s http://localhost:8080/token/admin | jq -r .access_token)
//!
//!   # Vulnerable: Regular user can access admin endpoint
//!   curl -H "Authorization: Bearer $USER_TOKEN" http://localhost:8080/vulnerable/admin
//!
//!   # Secure: Regular user cannot access admin endpoint
//!   curl -H "Authorization: Bearer $USER_TOKEN" http://localhost:8080/admin

use api_security_demo::{
    auth::{AuthenticatedUser, create_test_admin_token, create_test_user_token, is_admin},
    error::AppError,
    models::LoginResponse,
};
use axum::{Json, Router, extract::Path, http::HeaderMap, routing::get};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize)]
struct AdminResponse {
    message: String,
    user: String,
    admin_data: AdminData,
}

#[derive(Serialize)]
struct AdminData {
    total_users: i32,
    total_revenue: f64,
    sensitive_config: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch04_bfla=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        // Token generation for testing
        .route("/token/{role}", get(generate_test_token))
        // Vulnerable endpoint - BFLA vulnerability (obvious)
        .route("/vulnerable/admin", get(vulnerable_admin_endpoint))
        .route("/vulnerable/admin/users", get(vulnerable_list_users))
        .route("/vulnerable/admin/config", get(vulnerable_get_config))
        // Secure endpoints - proper authorization
        .route("/admin", get(secure_admin_endpoint))
        .route("/admin/users", get(secure_list_users))
        .route("/admin/config", get(secure_get_config))
        // Subtle vulnerabilities
        .route("/subtle/admin/role-in-header", get(subtle_header_role_check))
        .route("/subtle/admin/client-claims", get(subtle_client_claims_check))
        .route("/subtle/admin/string-role", get(subtle_string_role_check))
        .route("/subtle/admin/cached-check", get(subtle_cached_permission_check));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!("Chapter 4: BFLA demonstration server running on http://127.0.0.1:8080");
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  GET /token/user              - Get regular user token");
    tracing::info!("  GET /token/admin             - Get admin token");
    tracing::info!("");
    tracing::info!("  VULNERABLE (any authenticated user can access):");
    tracing::info!("  GET /vulnerable/admin        - Admin dashboard");
    tracing::info!("  GET /vulnerable/admin/users  - List all users");
    tracing::info!("  GET /vulnerable/admin/config - System configuration");
    tracing::info!("");
    tracing::info!("  SECURE (only admin users can access):");
    tracing::info!("  GET /admin                   - Admin dashboard");
    tracing::info!("  GET /admin/users             - List all users");
    tracing::info!("  GET /admin/config            - System configuration");
    tracing::info!("");
    tracing::info!("  SUBTLE VULNERABILITIES (look secure but aren't):");
    tracing::info!("  GET /subtle/admin/role-in-header   - Trusts X-User-Role header");
    tracing::info!("  GET /subtle/admin/client-claims    - Uses claims from JWT without server verification");
    tracing::info!("  GET /subtle/admin/string-role      - Case-sensitive role check bypass");
    tracing::info!("  GET /subtle/admin/cached-check     - Stale permission cache");

    axum::serve(listener, app).await.unwrap();
}

/// Generate test tokens for demonstration
async fn generate_test_token(Path(role): Path<String>) -> Result<Json<LoginResponse>, AppError> {
    let token = if role == "admin" {
        create_test_admin_token("admin-user")?
    } else {
        create_test_user_token("regular-user")?
    };

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}

/// VULNERABLE: Admin endpoint accessible to any authenticated user
///
/// This demonstrates BFLA - the endpoint only checks if the user is authenticated,
/// not whether they have admin permissions.
async fn vulnerable_admin_endpoint(
    user: AuthenticatedUser,
) -> Result<Json<AdminResponse>, AppError> {
    tracing::warn!(
        user = user.0.sub,
        permissions = ?user.0.permissions,
        "VULNERABLE admin endpoint accessed - no permission check!"
    );

    Ok(Json(AdminResponse {
        message: "Welcome to admin panel".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "DATABASE_URL=postgres://admin:secret@db.example.com".to_string(),
        },
    }))
}

/// VULNERABLE: List all users without admin check
async fn vulnerable_list_users(user: AuthenticatedUser) -> Result<Json<Vec<UserInfo>>, AppError> {
    tracing::warn!(user = user.0.sub, "VULNERABLE user listing accessed!");

    Ok(Json(vec![
        UserInfo {
            id: 1,
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
            ssn: "123-45-6789".to_string(), // PII exposure
        },
        UserInfo {
            id: 2,
            email: "user@example.com".to_string(),
            role: "user".to_string(),
            ssn: "987-65-4321".to_string(),
        },
    ]))
}

#[derive(Serialize)]
struct UserInfo {
    id: i64,
    email: String,
    role: String,
    ssn: String, // Sensitive PII
}

/// VULNERABLE: Get system config without admin check
async fn vulnerable_get_config(user: AuthenticatedUser) -> Result<Json<SystemConfig>, AppError> {
    tracing::warn!(user = user.0.sub, "VULNERABLE config endpoint accessed!");

    Ok(Json(SystemConfig {
        database_url: "postgres://admin:supersecret@db.internal:5432/prod".to_string(),
        api_keys: vec![
            "sk_live_abc123".to_string(),
            "stripe_key_xyz789".to_string(),
        ],
        internal_endpoints: vec![
            "http://internal-api:3000".to_string(),
            "http://payment-service:8080".to_string(),
        ],
    }))
}

#[derive(Serialize)]
struct SystemConfig {
    database_url: String,
    api_keys: Vec<String>,
    internal_endpoints: Vec<String>,
}

/// SECURE: Admin endpoint with proper permission check
///
/// This demonstrates proper function-level authorization by checking
/// that the user has the 'admin' permission before granting access.
async fn secure_admin_endpoint(user: AuthenticatedUser) -> Result<Json<AdminResponse>, AppError> {
    // Check for admin permission
    if !is_admin(&user.0) {
        tracing::info!(
            user = user.0.sub,
            permissions = ?user.0.permissions,
            "Non-admin user denied access to admin endpoint"
        );
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    tracing::info!(
        user = user.0.sub,
        "Admin user accessed secure admin endpoint"
    );

    Ok(Json(AdminResponse {
        message: "Welcome to admin panel".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "DATABASE_URL=postgres://admin:secret@db.example.com".to_string(),
        },
    }))
}

/// SECURE: List users with admin check
async fn secure_list_users(user: AuthenticatedUser) -> Result<Json<Vec<SafeUserInfo>>, AppError> {
    if !is_admin(&user.0) {
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    // Return safe user info without sensitive data
    Ok(Json(vec![
        SafeUserInfo {
            id: 1,
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
        },
        SafeUserInfo {
            id: 2,
            email: "user@example.com".to_string(),
            role: "user".to_string(),
        },
    ]))
}

#[derive(Serialize)]
struct SafeUserInfo {
    id: i64,
    email: String,
    role: String,
    // No SSN or other sensitive data
}

/// SECURE: Get config with admin check
async fn secure_get_config(user: AuthenticatedUser) -> Result<Json<SafeSystemConfig>, AppError> {
    if !is_admin(&user.0) {
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    // Return safe config without secrets
    Ok(Json(SafeSystemConfig {
        app_version: "1.0.0".to_string(),
        environment: "production".to_string(),
        features_enabled: vec!["auth".to_string(), "payments".to_string()],
    }))
}

#[derive(Serialize)]
struct SafeSystemConfig {
    app_version: String,
    environment: String,
    features_enabled: Vec<String>,
    // No secrets or internal endpoints
}

// ============ SUBTLE VULNERABILITIES ============

/// SUBTLE VULNERABILITY #1: Trusting client-provided role in HTTP header
///
/// Developer thought: "Frontend sends the user's role in a header for convenience"
/// Reality: Anyone can set any HTTP header
async fn subtle_header_role_check(
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<AdminResponse>, AppError> {
    // BUG: Trusting the X-User-Role header instead of checking the JWT claims
    let role = headers
        .get("X-User-Role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user");

    tracing::info!(
        user = user.0.sub,
        header_role = role,
        jwt_permissions = ?user.0.permissions,
        "Checking role from header (subtle vulnerability!)"
    );

    // Developer: "If header says admin, they must be admin"
    // Reality: curl -H "X-User-Role: admin" ...
    if role != "admin" {
        return Err(AppError::Forbidden("Admin role required".to_string()));
    }

    Ok(Json(AdminResponse {
        message: "Welcome! (accessed via header role check)".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "Exposed via header bypass!".to_string(),
        },
    }))
}

/// SUBTLE VULNERABILITY #2: Trusting JWT claims without server-side verification
///
/// Developer thought: "JWT is signed, so claims must be trustworthy"
/// Reality: Claims might not match current DB state (user demoted, role revoked)
async fn subtle_client_claims_check(
    user: AuthenticatedUser,
) -> Result<Json<AdminResponse>, AppError> {
    // BUG: Only checking JWT claims, not querying the actual permission in DB
    // If user was demoted after token was issued, they still have access!

    // This LOOKS correct - checking permissions from the authenticated user
    let has_admin = user.0.permissions.iter().any(|p| p == "admin");

    tracing::info!(
        user = user.0.sub,
        permissions = ?user.0.permissions,
        has_admin = has_admin,
        "Checking permissions from JWT only (no DB verification)"
    );

    if !has_admin {
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    // Problem: If the user was an admin when token was issued but was
    // demoted 5 minutes ago, this token still grants admin access
    // until it expires (could be hours or days!)

    Ok(Json(AdminResponse {
        message: "Welcome! (accessed via JWT claims only - no DB check)".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "Stale JWT claims vulnerability!".to_string(),
        },
    }))
}

/// SUBTLE VULNERABILITY #3: Case-sensitive role comparison
///
/// Developer thought: "We check if role == 'admin'"
/// Reality: What about "Admin", "ADMIN", "aDmIn"?
async fn subtle_string_role_check(
    user: AuthenticatedUser,
) -> Result<Json<AdminResponse>, AppError> {
    // BUG: Case-sensitive comparison
    let has_admin = user.0.permissions.iter().any(|p| p == "admin");

    // This looks secure, but what if the token generator has a bug
    // and sometimes sets "Admin" or "ADMIN"?
    // Or what if an attacker finds a way to inject "Admin" which passes
    // a different check elsewhere but fails here?

    tracing::info!(
        user = user.0.sub,
        permissions = ?user.0.permissions,
        "Case-sensitive role check"
    );

    if !has_admin {
        // Attacker notes: the check is case-sensitive
        // Maybe I can find another endpoint that sets "Admin" in the token?
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    Ok(Json(AdminResponse {
        message: "Welcome! (case-sensitive check)".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "Case-sensitive comparison!".to_string(),
        },
    }))
}

/// SUBTLE VULNERABILITY #4: Checking permission once and caching
///
/// Simulates a pattern where permission is checked once and then cached
/// in the session/request context
#[derive(Deserialize)]
struct CachedCheckQuery {
    /// Simulates a "permission check result" that was cached earlier in the request pipeline
    /// In real code this might be stored in request extensions or a context object
    permission_verified: Option<bool>,
}

async fn subtle_cached_permission_check(
    user: AuthenticatedUser,
    axum::extract::Query(query): axum::extract::Query<CachedCheckQuery>,
) -> Result<Json<AdminResponse>, AppError> {
    // BUG: Trusting a "cached" permission check result from query/context
    // In real code, this might be request.extensions().get::<PermissionResult>()
    // which could be manipulated if set incorrectly

    let is_verified_admin = query.permission_verified.unwrap_or(false);

    tracing::info!(
        user = user.0.sub,
        cached_result = is_verified_admin,
        "Using cached permission check result"
    );

    if is_verified_admin {
        // Developer: "Permission was already checked earlier in middleware"
        // Reality: The "cached result" came from user input!
        tracing::warn!("Granting access based on cached/query result!");
        return Ok(Json(AdminResponse {
            message: "Welcome! (accessed via cached permission)".to_string(),
            user: user.0.sub,
            admin_data: AdminData {
                total_users: 1234,
                total_revenue: 567890.12,
                sensitive_config: "Cached permission bypass!".to_string(),
            },
        }));
    }

    // Fallback to actual check
    if !is_admin(&user.0) {
        return Err(AppError::Forbidden("Admin permission required".to_string()));
    }

    Ok(Json(AdminResponse {
        message: "Welcome! (accessed via actual check)".to_string(),
        user: user.0.sub,
        admin_data: AdminData {
            total_users: 1234,
            total_revenue: 567890.12,
            sensitive_config: "Properly verified!".to_string(),
        },
    }))
}
