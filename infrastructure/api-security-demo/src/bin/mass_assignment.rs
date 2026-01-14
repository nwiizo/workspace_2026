//! Chapter 4: Mass Assignment Vulnerability Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: Accepts any fields from client, including 'status' and 'id'
//! - Secure endpoint: Only accepts whitelisted fields (amount, currency)
//!
//! Run: cargo run --bin ch04-mass-assignment
//! Test:
//!   # Get token
//!   TOKEN=$(curl -s http://localhost:8080/token/user | jq -r .access_token)
//!
//!   # Vulnerable: Attacker can set status to 'approved'
//!   curl -X POST http://localhost:8080/vulnerable/payments \
//!     -H "Authorization: Bearer $TOKEN" \
//!     -H "Content-Type: application/json" \
//!     -d '{"amount": 100, "currency": "USD", "status": "approved"}'
//!
//!   # Secure: Status field is ignored, always starts as 'pending'
//!   curl -X POST http://localhost:8080/payments \
//!     -H "Authorization: Bearer $TOKEN" \
//!     -H "Content-Type: application/json" \
//!     -d '{"amount": 100, "currency": "USD"}'

use api_security_demo::{
    auth::{AuthenticatedUser, create_test_user_token},
    db::Database,
    error::AppError,
    models::{CreatePaymentRequest, LoginResponse, Payment, UnsafePaymentRequest},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
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
                .unwrap_or_else(|_| "ch04_mass_assignment=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::new_in_memory().expect("Failed to create database");
    let state = AppState { db };

    let app = Router::new()
        // Token generation for testing
        .route("/token/{user_id}", get(generate_test_token))
        // Vulnerable endpoint - Mass Assignment vulnerability (obvious)
        .route("/vulnerable/payments", post(vulnerable_create_payment))
        // "Secure" endpoint - Only whitelisted fields accepted
        .route("/payments", post(secure_create_payment))
        // Subtle vulnerability: serde flatten allows extra fields
        .route("/subtle/payments", post(subtle_flatten_payment))
        // Subtle vulnerability: partial update with merge
        .route("/subtle/payments/{payment_id}", post(subtle_update_payment))
        // Get payment
        .route("/payments/{payment_id}", get(get_payment))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!(
        "Chapter 4: Mass Assignment demonstration server running on http://127.0.0.1:8080"
    );
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  GET  /token/{{user_id}}           - Get test token");
    tracing::info!("  POST /vulnerable/payments       - VULNERABLE: Accepts any fields");
    tracing::info!("  POST /payments                  - SECURE: Only amount & currency");
    tracing::info!("  GET  /payments/{{payment_id}}     - Get payment by ID");
    tracing::info!("");
    tracing::info!("Subtle vulnerability endpoints:");
    tracing::info!("  POST /subtle/payments           - Flatten allows extra fields");
    tracing::info!("  POST /subtle/payments/{{id}}      - Partial update allows status override");
    tracing::info!("");
    tracing::info!("Try the attack:");
    tracing::info!("  curl -X POST http://localhost:8080/vulnerable/payments \\");
    tracing::info!("    -H 'Authorization: Bearer <token>' \\");
    tracing::info!("    -H 'Content-Type: application/json' \\");
    tracing::info!("    -d '{{\"amount\": 100, \"currency\": \"USD\", \"status\": \"approved\"}}'");

    axum::serve(listener, app).await.unwrap();
}

/// Generate a test token for demonstration purposes
async fn generate_test_token(Path(user_id): Path<String>) -> Result<Json<LoginResponse>, AppError> {
    let token = create_test_user_token(&user_id)?;
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}

/// VULNERABLE: Creates payment accepting any fields from the request body
///
/// This demonstrates Mass Assignment vulnerability. An attacker can:
/// - Set 'status' to 'approved' to bypass payment processing
/// - Set 'id' to overwrite existing payments
/// - Potentially set other internal fields
async fn vulnerable_create_payment(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
    Json(req): Json<UnsafePaymentRequest>,
) -> Result<Json<Payment>, AppError> {
    tracing::warn!(
        amount = req.amount,
        currency = req.currency,
        status = ?req.status,
        id = ?req.id,
        "VULNERABLE payment creation - accepting all fields!"
    );

    // VULNERABLE: Using attacker-controlled values directly
    let payment = Payment {
        id: req.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        amount: req.amount,
        currency: req.currency,
        status: req.status.unwrap_or_else(|| "pending".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // If status is 'approved', the payment would be processed immediately
    // without actual payment verification!
    if payment.status == "approved" {
        tracing::error!(
            payment_id = payment.id,
            "SECURITY ISSUE: Payment created with 'approved' status!"
        );
    }

    state.db.create_payment(&payment)?;
    Ok(Json(payment))
}

/// SECURE: Creates payment with only whitelisted fields
///
/// This demonstrates proper input handling:
/// - Only accepts 'amount' and 'currency' from the request
/// - 'status' is always set to 'pending' server-side
/// - 'id' is always generated server-side
async fn secure_create_payment(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
    Json(req): Json<CreatePaymentRequest>,
) -> Result<Json<Payment>, AppError> {
    tracing::info!(
        amount = req.amount,
        currency = req.currency,
        "Secure payment creation - only accepting amount and currency"
    );

    // SECURE: Server controls sensitive fields
    let payment = Payment::new(req.amount, req.currency);

    state.db.create_payment(&payment)?;

    tracing::info!(
        payment_id = payment.id,
        status = payment.status,
        "Payment created with server-controlled status"
    );

    Ok(Json(payment))
}

/// Get payment by ID
async fn get_payment(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
    Path(payment_id): Path<String>,
) -> Result<Json<Payment>, AppError> {
    let payment = state
        .db
        .get_payment_by_id(&payment_id)?
        .ok_or_else(|| AppError::NotFound(format!("Payment {} not found", payment_id)))?;

    Ok(Json(payment))
}

// ============ SUBTLE VULNERABILITIES ============

/// SUBTLE VULNERABILITY #1: serde flatten with HashMap
///
/// Developer thought: "I'll use a typed struct for known fields, and capture
/// anything else in a HashMap for extensibility/logging"
///
/// Reality: The HashMap captures ALL fields including dangerous ones,
/// and the code later uses these values unsafely.
#[derive(Deserialize, Serialize)]
struct FlattenedPaymentRequest {
    amount: f64,
    currency: String,
    // This looks innocent - "just for logging unknown fields"
    #[serde(flatten)]
    extra_fields: HashMap<String, serde_json::Value>,
}

async fn subtle_flatten_payment(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
    Json(req): Json<FlattenedPaymentRequest>,
) -> Result<Json<Payment>, AppError> {
    // Log extra fields "for debugging"
    if !req.extra_fields.is_empty() {
        tracing::info!(extra = ?req.extra_fields, "Received extra fields");
    }

    let mut payment = Payment::new(req.amount, req.currency.clone());

    // BUG: Developer added "helpful" feature to honor extra fields
    // "If the client sends a status, maybe they know what they're doing?"
    if let Some(status) = req.extra_fields.get("status") {
        if let Some(s) = status.as_str() {
            // "Only allow valid statuses" - but approved is valid!
            if ["pending", "approved", "rejected"].contains(&s) {
                payment.status = s.to_string();
                tracing::warn!(
                    status = s,
                    "Using client-provided status (subtle vulnerability!)"
                );
            }
        }
    }

    // BUG: Same for ID - "for idempotency support"
    if let Some(id) = req.extra_fields.get("id") {
        if let Some(i) = id.as_str() {
            payment.id = i.to_string();
            tracing::warn!(
                id = i,
                "Using client-provided ID (subtle vulnerability!)"
            );
        }
    }

    state.db.create_payment(&payment)?;
    Ok(Json(payment))
}

/// SUBTLE VULNERABILITY #2: Partial update merging
///
/// This is a common pattern for PATCH endpoints:
/// "Merge the incoming fields with existing data"
///
/// The vulnerability: there's no distinction between "user-updatable"
/// and "system-only" fields during the merge.
#[derive(Deserialize)]
struct PartialPaymentUpdate {
    amount: Option<f64>,
    currency: Option<String>,
    // Developer forgot to add: status should NOT be here
    // Or they thought "we'll check it later" but didn't
    #[serde(default)]
    status: Option<String>,
    // "For adding notes to payments"
    #[serde(default)]
    notes: Option<String>,
}

async fn subtle_update_payment(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser,
    Path(payment_id): Path<String>,
    Json(update): Json<PartialPaymentUpdate>,
) -> Result<Json<Payment>, AppError> {
    let mut payment = state
        .db
        .get_payment_by_id(&payment_id)?
        .ok_or_else(|| AppError::NotFound(format!("Payment {} not found", payment_id)))?;

    // "Apply only the fields that were provided"
    if let Some(amount) = update.amount {
        // Some validation... but not enough
        if amount > 0.0 {
            payment.amount = amount;
        }
    }

    if let Some(currency) = update.currency {
        payment.currency = currency;
    }

    // BUG: Status update has a "validation" but it's flawed
    if let Some(status) = update.status {
        // Developer thought: "We check the current status, so it's safe"
        // Reality: This allows pending -> approved transition!
        if payment.status == "pending" && status == "approved" {
            // "This should only happen through our payment processor"
            // But we're allowing it through the API!
            tracing::warn!(
                payment_id = payment_id,
                old_status = payment.status,
                new_status = status,
                "Status changed via API (subtle vulnerability!)"
            );
            payment.status = status;
        } else if payment.status == "pending" && status == "cancelled" {
            // This one is actually OK - users can cancel pending payments
            payment.status = status;
        }
        // Silently ignore other transitions (but the damage is done)
    }

    // Notes seem harmless... until you realize they're logged
    if let Some(notes) = update.notes {
        tracing::info!(
            payment_id = payment_id,
            notes = notes, // XSS vector if logs are viewed in a web UI!
            "Payment notes updated"
        );
    }

    // In a real app, this would update the DB
    // For demo purposes, we just return the modified payment
    Ok(Json(payment))
}
