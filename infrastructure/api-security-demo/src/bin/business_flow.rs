//! API6: Unrestricted Access to Sensitive Business Flows
//!
//! This demonstrates vulnerabilities where business logic can be abused:
//! - Coupon codes reused multiple times by the same user
//! - Ticket scalping (buying more than allowed)
//! - Self-referral bonus exploitation
//!
//! Run: cargo run --bin business-flow-demo
//! Test:
//!   # Get token
//!   TOKEN=$(curl -s http://localhost:8080/token/alice | jq -r .access_token)
//!
//!   # Vulnerable: Use same coupon multiple times
//!   curl -X POST http://localhost:8080/vulnerable/coupons/redeem \
//!        -H "Authorization: Bearer $TOKEN" \
//!        -H "Content-Type: application/json" \
//!        -d '{"coupon_code": "SAVE20", "order_total": 100.0}'
//!
//!   # Secure: Second use is blocked
//!   curl -X POST http://localhost:8080/coupons/redeem \
//!        -H "Authorization: Bearer $TOKEN" \
//!        -H "Content-Type: application/json" \
//!        -d '{"coupon_code": "SAVE20", "order_total": 100.0}'

use api_security_demo::{
    auth::{create_test_user_token, AuthenticatedUser},
    db::Database,
    error::AppError,
    models::{
        LoginResponse, PurchaseTicketRequest, PurchaseTicketResponse, RedeemCouponRequest,
        RedeemCouponResponse, ReferralRequest, ReferralResponse,
    },
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
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
                .unwrap_or_else(|_| "business_flow=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::new_in_memory().expect("Failed to create database");
    db.seed_coupons().expect("Failed to seed coupons");
    db.seed_tickets().expect("Failed to seed tickets");

    let state = AppState { db };

    let app = Router::new()
        // Token generation for testing
        .route("/token/{user_id}", get(generate_test_token))
        // Coupon endpoints
        .route("/vulnerable/coupons/redeem", post(redeem_coupon_vulnerable))
        .route("/coupons/redeem", post(redeem_coupon_secure))
        // Ticket endpoints
        .route(
            "/vulnerable/tickets/purchase",
            post(purchase_ticket_vulnerable),
        )
        .route("/tickets/purchase", post(purchase_ticket_secure))
        // Referral endpoints
        .route("/vulnerable/referral", post(create_referral_vulnerable))
        .route("/referral", post(create_referral_secure))
        // Info endpoints
        .route("/tickets", get(list_tickets))
        .route("/coupons", get(list_coupons))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    tracing::info!("API6: Business Flow Demo listening on http://127.0.0.1:8080");
    tracing::info!("Endpoints:");
    tracing::info!("  GET  /token/{{user_id}}              - Get test token");
    tracing::info!("  GET  /coupons                       - List available coupons");
    tracing::info!("  GET  /tickets                       - List available tickets");
    tracing::info!("  POST /vulnerable/coupons/redeem     - Redeem coupon (vulnerable)");
    tracing::info!("  POST /coupons/redeem                - Redeem coupon (secure)");
    tracing::info!("  POST /vulnerable/tickets/purchase   - Purchase tickets (vulnerable)");
    tracing::info!("  POST /tickets/purchase              - Purchase tickets (secure)");
    tracing::info!("  POST /vulnerable/referral           - Create referral (vulnerable)");
    tracing::info!("  POST /referral                      - Create referral (secure)");

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

/// List available coupons
async fn list_coupons(State(state): State<Arc<AppState>>) -> Result<Json<Vec<String>>, AppError> {
    // In a real app, this would query the database
    // For demo, return static list
    Ok(Json(vec![
        "SAVE20 - 20% off (valid)".to_string(),
        "EXPIRED10 - 10% off (expired)".to_string(),
        "MAXED50 - 50% off (max uses reached)".to_string(),
    ]))
}

/// List available tickets
async fn list_tickets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ticket1 = state.db.get_ticket("ticket-001")?;
    let ticket2 = state.db.get_ticket("ticket-002")?;

    Ok(Json(serde_json::json!({
        "tickets": [ticket1, ticket2]
    })))
}

// ============================================
// Coupon Redemption
// ============================================

/// VULNERABLE: Redeem coupon without checking if user already used it
async fn redeem_coupon_vulnerable(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<RedeemCouponRequest>,
) -> Result<Json<RedeemCouponResponse>, AppError> {
    tracing::warn!(
        "VULNERABLE: User {} attempting to redeem coupon {} - NO per-user usage tracking!",
        user.0.sub,
        req.coupon_code
    );

    let coupon = state
        .db
        .get_coupon(&req.coupon_code)?
        .ok_or_else(|| AppError::NotFound("Coupon not found".to_string()))?;

    // VULNERABILITY: Only checks global usage count, not per-user
    // User can redeem the same coupon multiple times!
    if coupon.current_uses >= coupon.max_uses {
        return Ok(Json(RedeemCouponResponse {
            success: false,
            discount_amount: 0.0,
            final_total: req.order_total,
            message: "Coupon has reached maximum uses".to_string(),
        }));
    }

    // VULNERABILITY: No expiration check!
    // VULNERABILITY: No per-user tracking!

    // Just increment global counter
    state.db.increment_coupon_usage(&req.coupon_code)?;

    let discount_amount = req.order_total * (coupon.discount_percent as f64 / 100.0);
    let final_total = req.order_total - discount_amount;

    tracing::warn!(
        "VULNERABLE: Coupon {} applied for user {} - same user can use again!",
        req.coupon_code,
        user.0.sub
    );

    Ok(Json(RedeemCouponResponse {
        success: true,
        discount_amount,
        final_total,
        message: format!(
            "Applied {}% discount (vulnerable - no per-user tracking)",
            coupon.discount_percent
        ),
    }))
}

/// SECURE: Redeem coupon with proper validation
async fn redeem_coupon_secure(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<RedeemCouponRequest>,
) -> Result<Json<RedeemCouponResponse>, AppError> {
    tracing::info!(
        "SECURE: User {} attempting to redeem coupon {}",
        user.0.sub,
        req.coupon_code
    );

    let coupon = state
        .db
        .get_coupon(&req.coupon_code)?
        .ok_or_else(|| AppError::NotFound("Coupon not found".to_string()))?;

    // Check 1: Global usage limit
    if coupon.current_uses >= coupon.max_uses {
        return Ok(Json(RedeemCouponResponse {
            success: false,
            discount_amount: 0.0,
            final_total: req.order_total,
            message: "Coupon has reached maximum uses".to_string(),
        }));
    }

    // Check 2: Expiration date
    let expires_at = chrono::DateTime::parse_from_rfc3339(&coupon.expires_at)
        .map_err(|_| AppError::Internal("Invalid expiration date".to_string()))?;
    if expires_at < chrono::Utc::now() {
        return Ok(Json(RedeemCouponResponse {
            success: false,
            discount_amount: 0.0,
            final_total: req.order_total,
            message: "Coupon has expired".to_string(),
        }));
    }

    // Check 3: Per-user usage (if single_use_per_user is true)
    if coupon.single_use_per_user {
        if state.db.has_user_used_coupon(&req.coupon_code, &user.0.sub)? {
            return Ok(Json(RedeemCouponResponse {
                success: false,
                discount_amount: 0.0,
                final_total: req.order_total,
                message: "You have already used this coupon".to_string(),
            }));
        }
    }

    // All checks passed - apply coupon
    state.db.use_coupon(&req.coupon_code, &user.0.sub)?;

    let discount_amount = req.order_total * (coupon.discount_percent as f64 / 100.0);
    let final_total = req.order_total - discount_amount;

    tracing::info!(
        "SECURE: Coupon {} successfully applied for user {}",
        req.coupon_code,
        user.0.sub
    );

    Ok(Json(RedeemCouponResponse {
        success: true,
        discount_amount,
        final_total,
        message: format!("Applied {}% discount", coupon.discount_percent),
    }))
}

// ============================================
// Ticket Purchase
// ============================================

/// VULNERABLE: Purchase tickets without inventory or per-user limits
async fn purchase_ticket_vulnerable(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<PurchaseTicketRequest>,
) -> Result<Json<PurchaseTicketResponse>, AppError> {
    tracing::warn!(
        "VULNERABLE: User {} attempting to purchase {} tickets - NO limits!",
        user.0.sub,
        req.quantity
    );

    let ticket = state
        .db
        .get_ticket(&req.ticket_id)?
        .ok_or_else(|| AppError::NotFound("Ticket not found".to_string()))?;

    // VULNERABILITY: No inventory check!
    // VULNERABILITY: No per-user purchase limit!
    // Scalpers can buy unlimited tickets!

    let purchase_id = state
        .db
        .purchase_ticket_vulnerable(&req.ticket_id, &user.0.sub, req.quantity)?;

    let total_price = ticket.price * req.quantity as f64;

    tracing::warn!(
        "VULNERABLE: User {} purchased {} tickets (no limits enforced)!",
        user.0.sub,
        req.quantity
    );

    Ok(Json(PurchaseTicketResponse {
        success: true,
        purchase_id,
        quantity: req.quantity,
        total_price,
        message: "Purchase successful (vulnerable - no limits)".to_string(),
    }))
}

/// SECURE: Purchase tickets with proper limits
async fn purchase_ticket_secure(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<PurchaseTicketRequest>,
) -> Result<Json<PurchaseTicketResponse>, AppError> {
    tracing::info!(
        "SECURE: User {} attempting to purchase {} tickets",
        user.0.sub,
        req.quantity
    );

    let ticket = state
        .db
        .get_ticket(&req.ticket_id)?
        .ok_or_else(|| AppError::NotFound("Ticket not found".to_string()))?;

    // Check 1: Inventory availability
    if req.quantity > ticket.available_quantity {
        return Ok(Json(PurchaseTicketResponse {
            success: false,
            purchase_id: String::new(),
            quantity: 0,
            total_price: 0.0,
            message: format!(
                "Only {} tickets available",
                ticket.available_quantity
            ),
        }));
    }

    // Check 2: Per-user purchase limit
    let existing_purchases = state
        .db
        .get_user_ticket_purchases(&req.ticket_id, &user.0.sub)?;
    let total_after_purchase = existing_purchases + req.quantity;

    if total_after_purchase > ticket.max_per_user {
        let remaining = ticket.max_per_user.saturating_sub(existing_purchases);
        return Ok(Json(PurchaseTicketResponse {
            success: false,
            purchase_id: String::new(),
            quantity: 0,
            total_price: 0.0,
            message: format!(
                "Purchase limit exceeded. You can buy {} more tickets (max {} per user)",
                remaining, ticket.max_per_user
            ),
        }));
    }

    // Check 3: Reasonable quantity (anti-bot)
    if req.quantity > 10 {
        return Ok(Json(PurchaseTicketResponse {
            success: false,
            purchase_id: String::new(),
            quantity: 0,
            total_price: 0.0,
            message: "Maximum 10 tickets per transaction".to_string(),
        }));
    }

    // All checks passed
    let purchase_id = state
        .db
        .purchase_ticket_secure(&req.ticket_id, &user.0.sub, req.quantity)?;

    let total_price = ticket.price * req.quantity as f64;

    tracing::info!(
        "SECURE: User {} purchased {} tickets (purchase_id: {})",
        user.0.sub,
        req.quantity,
        purchase_id
    );

    Ok(Json(PurchaseTicketResponse {
        success: true,
        purchase_id,
        quantity: req.quantity,
        total_price,
        message: "Purchase successful".to_string(),
    }))
}

// ============================================
// Referral System
// ============================================

/// VULNERABLE: Create referral without proper validation
async fn create_referral_vulnerable(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<ReferralRequest>,
) -> Result<Json<ReferralResponse>, AppError> {
    tracing::warn!(
        "VULNERABLE: User {} using referral code {} - NO self-referral check!",
        user.0.sub,
        req.referral_code
    );

    // The referral_code IS the referrer's user_id for simplicity
    let referrer_id = &req.referral_code;
    let referred_id = &user.0.sub;

    // VULNERABILITY: No check if user is referring themselves!
    // VULNERABILITY: No check if already been referred!
    // Users can game the system for unlimited bonuses!

    let bonus_amount = 10.0; // $10 bonus

    let _referral = state
        .db
        .create_referral_vulnerable(referrer_id, referred_id, bonus_amount)?;

    tracing::warn!(
        "VULNERABLE: Referral created - referrer {} gets ${}!",
        referrer_id,
        bonus_amount
    );

    Ok(Json(ReferralResponse {
        success: true,
        bonus_amount,
        message: format!(
            "Referral bonus ${} applied (vulnerable - no validation)",
            bonus_amount
        ),
    }))
}

/// SECURE: Create referral with proper validation
async fn create_referral_secure(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<ReferralRequest>,
) -> Result<Json<ReferralResponse>, AppError> {
    tracing::info!(
        "SECURE: User {} using referral code {}",
        user.0.sub,
        req.referral_code
    );

    let referrer_id = &req.referral_code;
    let referred_id = &user.0.sub;

    // Check 1: Prevent self-referral
    if referrer_id == referred_id {
        return Ok(Json(ReferralResponse {
            success: false,
            bonus_amount: 0.0,
            message: "You cannot refer yourself".to_string(),
        }));
    }

    // Check 2: Check if user has already been referred
    if state.db.has_been_referred(referred_id)? {
        return Ok(Json(ReferralResponse {
            success: false,
            bonus_amount: 0.0,
            message: "You have already been referred by someone".to_string(),
        }));
    }

    // Check 3: In production, would also verify referrer exists
    // and check for fraud patterns (same IP, same device, etc.)

    let bonus_amount = 10.0;

    let _referral = state
        .db
        .create_referral(referrer_id, referred_id, bonus_amount)?;

    tracing::info!(
        "SECURE: Referral created - referrer {} gets ${}",
        referrer_id,
        bonus_amount
    );

    Ok(Json(ReferralResponse {
        success: true,
        bonus_amount,
        message: format!("Referral bonus ${} applied", bonus_amount),
    }))
}
