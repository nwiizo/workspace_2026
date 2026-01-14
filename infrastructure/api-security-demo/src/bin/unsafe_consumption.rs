//! API10: Unsafe Consumption of APIs
//!
//! This demonstrates vulnerabilities when consuming external APIs:
//! - Trusting webhook payloads without signature verification
//! - Accepting external data without validation
//! - Following redirects blindly from external APIs
//! - No schema validation on external responses
//!
//! Run: cargo run --bin unsafe-consumption-demo
//! Test:
//!   # Vulnerable: Webhook without signature verification
//!   curl -X POST http://localhost:8080/vulnerable/webhook \
//!        -H "Content-Type: application/json" \
//!        -d '{"event_type": "payment.success", "data": {"amount": 1000000}}'
//!
//!   # Secure: Webhook with HMAC signature
//!   # (signature must be computed with secret)

use api_security_demo::{
    auth::{create_test_user_token, AuthenticatedUser},
    error::AppError,
    models::{ExternalUserData, LoginResponse, PaymentCallback, WebhookPayload},
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type HmacSha256 = Hmac<Sha256>;

const WEBHOOK_SECRET: &str = "whsec_demo_secret_key_12345";
const PAYMENT_SECRET: &str = "payment_callback_secret_67890";

#[derive(Clone)]
struct AppState {
    // In real app, would have database connection
    webhook_secret: String,
    payment_secret: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "unsafe_consumption=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        webhook_secret: WEBHOOK_SECRET.to_string(),
        payment_secret: PAYMENT_SECRET.to_string(),
    };

    let app = Router::new()
        // Token generation for testing
        .route("/token/{user_id}", get(generate_test_token))
        // Webhook endpoints
        .route("/vulnerable/webhook", post(webhook_vulnerable))
        .route("/webhook", post(webhook_secure))
        // Payment callback endpoints
        .route("/vulnerable/payment/callback", post(payment_callback_vulnerable))
        .route("/payment/callback", post(payment_callback_secure))
        // External data enrichment
        .route("/vulnerable/enrich/{user_id}", get(enrich_user_vulnerable))
        .route("/enrich/{user_id}", get(enrich_user_secure))
        // Proxy endpoints
        .route("/vulnerable/proxy", post(proxy_vulnerable))
        .route("/proxy", post(proxy_secure))
        // Helper: Generate valid signature for testing
        .route("/test/generate-signature", post(generate_test_signature))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    tracing::info!("API10: Unsafe Consumption Demo listening on http://127.0.0.1:8080");
    tracing::info!("Endpoints:");
    tracing::info!("  GET  /token/{{user_id}}                - Get test token");
    tracing::info!("  POST /vulnerable/webhook             - Webhook (no signature check)");
    tracing::info!("  POST /webhook                        - Webhook (with HMAC verification)");
    tracing::info!("  POST /vulnerable/payment/callback    - Payment callback (no verification)");
    tracing::info!("  POST /payment/callback               - Payment callback (with verification)");
    tracing::info!("  GET  /vulnerable/enrich/{{user_id}}    - User enrichment (no validation)");
    tracing::info!("  GET  /enrich/{{user_id}}               - User enrichment (with validation)");
    tracing::info!("  POST /vulnerable/proxy               - Proxy request (follows redirects)");
    tracing::info!("  POST /proxy                          - Proxy request (strict validation)");
    tracing::info!("  POST /test/generate-signature        - Generate test HMAC signature");
    tracing::info!("");
    tracing::info!("Webhook secret: {}", WEBHOOK_SECRET);
    tracing::info!("Payment secret: {}", PAYMENT_SECRET);

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
// Webhook Processing
// ============================================

/// VULNERABLE: Process webhook without signature verification
async fn webhook_vulnerable(
    Json(payload): Json<WebhookPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!(
        "VULNERABLE: Processing webhook without signature verification! Event: {}",
        payload.event_type
    );

    // VULNERABILITY: No signature verification!
    // Attacker can send fake webhook events

    // Process the webhook blindly
    match payload.event_type.as_str() {
        "payment.success" => {
            tracing::warn!(
                "VULNERABLE: Processing payment success - could be forged! Data: {:?}",
                payload.data
            );
            // In real app, this would update payment status, fulfill order, etc.
        }
        "subscription.cancelled" => {
            tracing::warn!("VULNERABLE: Processing subscription cancellation - could be forged!");
        }
        "user.deleted" => {
            tracing::warn!("VULNERABLE: Processing user deletion - could be forged!");
        }
        _ => {
            tracing::warn!("VULNERABLE: Processing unknown event type: {}", payload.event_type);
        }
    }

    Ok(Json(serde_json::json!({
        "status": "processed",
        "event_type": payload.event_type,
        "warning": "No signature verification performed!"
    })))
}

/// SECURE: Process webhook with HMAC signature verification
async fn webhook_secure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: Processing webhook with signature verification");

    // Get signature from header
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized)?;

    // Verify HMAC signature
    let mut mac = HmacSha256::new_from_slice(state.webhook_secret.as_bytes())
        .map_err(|_| AppError::Internal("HMAC initialization failed".to_string()))?;
    mac.update(&body);

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature != expected_signature {
        tracing::warn!("SECURE: Invalid webhook signature rejected");
        return Err(AppError::Unauthorized);
    }

    // Parse payload after verification
    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    tracing::info!("SECURE: Valid signature, processing event: {}", payload.event_type);

    // Additional validation: check timestamp to prevent replay attacks
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&payload.timestamp) {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(ts);
        if age.num_minutes() > 5 {
            tracing::warn!("SECURE: Webhook timestamp too old, rejecting");
            return Err(AppError::BadRequest("Webhook expired".to_string()));
        }
    }

    Ok(Json(serde_json::json!({
        "status": "processed",
        "event_type": payload.event_type,
        "verified": true
    })))
}

// ============================================
// Payment Callback
// ============================================

/// VULNERABLE: Process payment callback without verification
async fn payment_callback_vulnerable(
    Json(callback): Json<PaymentCallback>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!(
        "VULNERABLE: Processing payment callback without verification! Transaction: {}",
        callback.transaction_id
    );

    // VULNERABILITY: Trusting the callback completely
    // Attacker can claim any payment was successful

    if callback.status == "success" {
        tracing::warn!(
            "VULNERABLE: Marking transaction {} as successful - could be forged! Amount: ${}",
            callback.transaction_id,
            callback.amount
        );
        // In real app, this would:
        // - Update order status
        // - Send confirmation email
        // - Trigger fulfillment
    }

    Ok(Json(serde_json::json!({
        "status": "acknowledged",
        "transaction_id": callback.transaction_id,
        "warning": "No verification performed!"
    })))
}

/// SECURE: Process payment callback with signature verification
async fn payment_callback_secure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(callback): Json<PaymentCallback>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: Processing payment callback with verification");

    // Build the string to sign (common pattern for payment providers)
    let sign_string = format!(
        "{}|{}|{}|{}",
        callback.transaction_id, callback.status, callback.amount, callback.timestamp
    );

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(state.payment_secret.as_bytes())
        .map_err(|_| AppError::Internal("HMAC initialization failed".to_string()))?;
    mac.update(sign_string.as_bytes());

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if callback.signature != expected_signature {
        tracing::warn!("SECURE: Invalid payment callback signature rejected");
        return Err(AppError::Unauthorized);
    }

    // Verify timestamp
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&callback.timestamp) {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(ts);
        if age.num_minutes() > 5 {
            return Err(AppError::BadRequest("Callback expired".to_string()));
        }
    }

    // Additional: Check against our records
    // In real app, verify transaction_id exists in our system

    tracing::info!(
        "SECURE: Verified payment callback for transaction {}",
        callback.transaction_id
    );

    Ok(Json(serde_json::json!({
        "status": "acknowledged",
        "transaction_id": callback.transaction_id,
        "verified": true
    })))
}

// ============================================
// External Data Enrichment
// ============================================

/// VULNERABLE: Use external data without validation
async fn enrich_user_vulnerable(
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!(
        "VULNERABLE: Enriching user {} with external data - no validation!",
        user_id
    );

    // Simulate fetching from external API
    let external_data = simulate_external_api(&user_id);

    // VULNERABILITY: Trusting external data completely
    // Could contain:
    // - XSS payloads in name/email
    // - SQL injection in fields that get stored
    // - Invalid data types
    // - Excessive data that could cause DoS

    tracing::warn!(
        "VULNERABLE: Storing external data without sanitization: {:?}",
        external_data
    );

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "enriched_data": external_data,
        "warning": "Data stored without validation!"
    })))
}

/// SECURE: Validate external data before use
async fn enrich_user_secure(
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: Enriching user {} with validated external data", user_id);

    let external_data = simulate_external_api(&user_id);

    // Validation 1: Check required fields
    if external_data.name.is_empty() || external_data.email.is_empty() {
        return Err(AppError::BadRequest("Missing required fields from external API".to_string()));
    }

    // Validation 2: Sanitize strings (remove potential XSS/injection)
    let sanitized_name = sanitize_string(&external_data.name);
    let sanitized_email = sanitize_string(&external_data.email);

    // Validation 3: Validate email format
    if !is_valid_email(&sanitized_email) {
        return Err(AppError::BadRequest("Invalid email from external API".to_string()));
    }

    // Validation 4: Bound numeric values
    let credit_score = external_data.credit_score.map(|s| s.min(850).max(300));

    // Validation 5: Validate enum values
    let risk_level = external_data.risk_level.and_then(|r| {
        match r.to_lowercase().as_str() {
            "low" | "medium" | "high" => Some(r),
            _ => None, // Reject unknown values
        }
    });

    tracing::info!("SECURE: External data validated and sanitized");

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "enriched_data": {
            "name": sanitized_name,
            "email": sanitized_email,
            "credit_score": credit_score,
            "risk_level": risk_level
        },
        "validated": true
    })))
}

// ============================================
// Proxy Endpoints
// ============================================

#[derive(serde::Deserialize)]
struct ProxyRequest {
    url: String,
    #[serde(default)]
    follow_redirects: bool,
}

/// VULNERABLE: Proxy that follows redirects blindly
async fn proxy_vulnerable(
    Json(req): Json<ProxyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!("VULNERABLE: Proxying request to {} with no restrictions!", req.url);

    // VULNERABILITY: No URL validation, follows all redirects
    // Could be used for:
    // - SSRF to internal services
    // - Credential theft via malicious redirects
    // - Data exfiltration

    // Simulate the vulnerability - in real code would use reqwest
    let response_data = if req.url.contains("internal") || req.url.contains("localhost") {
        serde_json::json!({
            "warning": "SSRF: Accessed internal resource!",
            "internal_data": {
                "database_password": "secret123",
                "api_keys": ["key1", "key2"]
            }
        })
    } else {
        serde_json::json!({
            "status": "fetched",
            "url": req.url
        })
    };

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": response_data,
        "warning": "No URL validation performed!"
    })))
}

/// SECURE: Proxy with proper validation
async fn proxy_secure(Json(req): Json<ProxyRequest>) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("SECURE: Validating proxy request to {}", req.url);

    // Validation 1: Parse URL
    let url = url::Url::parse(&req.url)
        .map_err(|_| AppError::BadRequest("Invalid URL".to_string()))?;

    // Validation 2: Only allow HTTPS
    if url.scheme() != "https" {
        return Err(AppError::BadRequest("Only HTTPS URLs allowed".to_string()));
    }

    // Validation 3: Check against allowlist
    let allowed_hosts = ["api.github.com", "api.stripe.com", "api.example.com"];
    let host = url.host_str().unwrap_or("");
    if !allowed_hosts.contains(&host) {
        return Err(AppError::BadRequest(format!(
            "Host '{}' not in allowlist",
            host
        )));
    }

    // Validation 4: Block private IP ranges
    // (In real implementation, would resolve and check IP)

    // Validation 5: Limit redirect following
    if req.follow_redirects {
        tracing::warn!("SECURE: Redirect following disabled for security");
    }

    tracing::info!("SECURE: URL validated, proceeding with request");

    Ok(Json(serde_json::json!({
        "status": "success",
        "url": req.url,
        "validated": true,
        "note": "Request would proceed with validated URL"
    })))
}

// ============================================
// Helper Functions
// ============================================

/// Generate test HMAC signature
async fn generate_test_signature(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Json<serde_json::Value> {
    let mut mac = HmacSha256::new_from_slice(state.webhook_secret.as_bytes()).unwrap();
    mac.update(&body);
    let signature = hex::encode(mac.finalize().into_bytes());

    Json(serde_json::json!({
        "signature": signature,
        "header_name": "X-Webhook-Signature",
        "secret_used": "webhook_secret"
    }))
}

/// Simulate external API response
fn simulate_external_api(user_id: &str) -> ExternalUserData {
    // Simulate various external API responses, some with malicious content
    match user_id {
        "malicious" => ExternalUserData {
            user_id: user_id.to_string(),
            name: "<script>alert('XSS')</script>".to_string(),
            email: "'; DROP TABLE users; --".to_string(),
            credit_score: Some(9999), // Invalid score
            risk_level: Some("SUPER_HIGH".to_string()), // Invalid enum value
        },
        "overflow" => ExternalUserData {
            user_id: user_id.to_string(),
            name: "A".repeat(1_000_000), // DoS via large data
            email: "test@example.com".to_string(),
            credit_score: Some(750),
            risk_level: Some("low".to_string()),
        },
        _ => ExternalUserData {
            user_id: user_id.to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            credit_score: Some(750),
            risk_level: Some("low".to_string()),
        },
    }
}

/// Sanitize string to prevent XSS/injection
fn sanitize_string(input: &str) -> String {
    // Remove HTML tags and limit length
    let cleaned: String = input
        .chars()
        .filter(|c| !['<', '>', '"', '\'', ';', '-'].contains(c))
        .take(255) // Limit length
        .collect();
    cleaned.trim().to_string()
}

/// Simple email validation
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() < 255
}
