//! Chapter 4: BOLA (Broken Object Level Authorization) Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: Returns any order by ID without checking ownership
//! - Secure endpoint: Only returns orders belonging to the authenticated user
//!
//! Run: cargo run --bin ch04-bola
//! Test:
//!   # Get tokens
//!   ALICE_TOKEN=$(curl -s http://localhost:8080/token/alice | jq -r .access_token)
//!   BOB_TOKEN=$(curl -s http://localhost:8080/token/bob | jq -r .access_token)
//!
//!   # Vulnerable: Bob can access Alice's order (ID 1)
//!   curl -H "Authorization: Bearer $BOB_TOKEN" http://localhost:8080/vulnerable/orders/1
//!
//!   # Secure: Bob cannot access Alice's order
//!   curl -H "Authorization: Bearer $BOB_TOKEN" http://localhost:8080/orders/1

use api_security_demo::{
    auth::{AuthenticatedUser, create_test_user_token},
    db::Database,
    error::AppError,
    models::{CreateOrderRequest, LoginResponse, Order},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
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
                .unwrap_or_else(|_| "ch04_bola=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::new_in_memory().expect("Failed to create database");
    db.seed_orders().expect("Failed to seed orders");

    let state = AppState { db };

    let app = Router::new()
        // Token generation for testing
        .route("/token/{user_id}", get(generate_test_token))
        // Vulnerable endpoint - BOLA vulnerability (obvious)
        .route("/vulnerable/orders/{order_id}", get(vulnerable_get_order))
        // "Secure" endpoint - but has subtle bugs
        .route("/orders/{order_id}", get(secure_get_order))
        // Subtle vulnerability: client-controlled user_id in query parameter
        .route("/subtle/orders/{order_id}", get(subtle_vulnerable_get_order))
        // Subtle vulnerability: TOCTOU race condition
        .route("/race/orders/{order_id}", get(race_condition_get_order))
        // Subtle vulnerability: logging leaks data before authorization
        .route("/logging/orders/{order_id}", get(logging_before_auth_get_order))
        .route("/orders", get(list_my_orders))
        .route("/orders", post(create_order))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!("Chapter 4: BOLA demonstration server running on http://127.0.0.1:8080");
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  GET  /token/{{user_id}}              - Get test token for user");
    tracing::info!("  GET  /vulnerable/orders/{{order_id}} - VULNERABLE: Returns any order");
    tracing::info!(
        "  GET  /orders/{{order_id}}            - SECURE: Only returns user's own orders"
    );
    tracing::info!("");
    tracing::info!("Subtle vulnerability endpoints (look secure but aren't):");
    tracing::info!("  GET  /subtle/orders/{{order_id}}?user_id=alice  - Query param override");
    tracing::info!("  GET  /race/orders/{{order_id}}                  - TOCTOU race condition");
    tracing::info!("  GET  /logging/orders/{{order_id}}               - Logging before authz");
    tracing::info!("");
    tracing::info!("  GET  /orders                        - List authenticated user's orders");
    tracing::info!("  POST /orders                        - Create new order");
    tracing::info!("");
    tracing::info!("Sample data:");
    tracing::info!("  Order 1: user=alice, product=Widget A");
    tracing::info!("  Order 2: user=bob, product=Widget B");
    tracing::info!("  Order 3: user=alice, product=Widget C");

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

/// VULNERABLE: Returns any order by ID without checking ownership
///
/// This demonstrates BOLA (Broken Object Level Authorization).
/// Any authenticated user can access any order just by knowing its ID.
async fn vulnerable_get_order(
    State(state): State<Arc<AppState>>,
    _user: AuthenticatedUser, // Only checks if user is authenticated, not authorized
    Path(order_id): Path<i64>,
) -> Result<Json<Order>, AppError> {
    tracing::warn!(
        order_id = order_id,
        "VULNERABLE endpoint accessed - no ownership check!"
    );

    let order = state
        .db
        .get_order_by_id(order_id)?
        .ok_or_else(|| AppError::NotFound(format!("Order {} not found", order_id)))?;

    Ok(Json(order))
}

/// SECURE: Returns order only if it belongs to the authenticated user
///
/// This implements proper object-level authorization by checking
/// that the order belongs to the requesting user.
async fn secure_get_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(order_id): Path<i64>,
) -> Result<Json<Order>, AppError> {
    let user_id = &user.0.sub;

    tracing::info!(
        order_id = order_id,
        user_id = user_id,
        "Secure endpoint - checking ownership"
    );

    let order = state
        .db
        .get_order_by_id_for_user(order_id, user_id)?
        .ok_or_else(|| {
            AppError::NotFound(format!("Order {} not found or access denied", order_id))
        })?;

    Ok(Json(order))
}

/// List all orders belonging to the authenticated user
async fn list_my_orders(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Order>>, AppError> {
    let user_id = &user.0.sub;
    let orders = state.db.get_orders_for_user(user_id)?;
    Ok(Json(orders))
}

/// Create a new order for the authenticated user
async fn create_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<Order>, AppError> {
    let user_id = &user.0.sub;
    let order = state.db.create_order(user_id, &req.product, req.quantity)?;

    tracing::info!(
        order_id = order.id,
        user_id = user_id,
        product = req.product,
        "New order created"
    );

    Ok(Json(order))
}

// ============ SUBTLE VULNERABILITIES ============
// These look secure at first glance but contain subtle bugs

#[derive(Deserialize)]
struct UserIdQuery {
    user_id: Option<String>,
}

/// SUBTLE VULNERABILITY #1: Query parameter override
///
/// This looks secure because it uses AuthenticatedUser, but the developer
/// added a "helpful" feature to allow specifying user_id in query params
/// for "debugging" or "admin override" purposes.
///
/// Attack: GET /subtle/orders/1?user_id=alice (while authenticated as bob)
async fn subtle_vulnerable_get_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(order_id): Path<i64>,
    Query(query): Query<UserIdQuery>,
) -> Result<Json<Order>, AppError> {
    // BUG: Query parameter overrides authenticated user!
    // Developer thought: "This is useful for admin debugging"
    // Reality: Anyone can override the user_id
    let user_id = query.user_id.unwrap_or_else(|| user.0.sub.clone());

    tracing::info!(
        order_id = order_id,
        user_id = user_id,
        authenticated_as = user.0.sub,
        "Subtle vulnerability: user_id may be overridden"
    );

    let order = state
        .db
        .get_order_by_id_for_user(order_id, &user_id)?
        .ok_or_else(|| {
            AppError::NotFound(format!("Order {} not found or access denied", order_id))
        })?;

    Ok(Json(order))
}

/// SUBTLE VULNERABILITY #2: TOCTOU (Time-of-Check-Time-of-Use) Race Condition
///
/// This looks secure because it checks ownership, but the check and use
/// are separate operations. In a concurrent system, the order could be
/// reassigned between check and use.
///
/// Also demonstrates: returning too much info in the "check" phase
async fn race_condition_get_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(order_id): Path<i64>,
) -> Result<Json<Order>, AppError> {
    let user_id = &user.0.sub;

    // Step 1: Check if order exists (TOCTOU vulnerability starts here)
    let order = state
        .db
        .get_order_by_id(order_id)?  // Gets ANY order, not filtered by user
        .ok_or_else(|| AppError::NotFound(format!("Order {} not found", order_id)))?;

    // BUG: We've already fetched the full order data!
    // Even if we reject access below, we've loaded sensitive data into memory
    // and it could be leaked through timing attacks, error messages, or logs

    // Step 2: Check ownership (too late - we already have the data)
    if order.user != *user_id {
        // BUG: This error message leaks information!
        // Attacker now knows: 1) Order exists, 2) Who owns it
        tracing::warn!(
            order_id = order_id,
            order_owner = order.user,  // Logging the actual owner!
            requester = user_id,
            "Unauthorized access attempt"
        );
        return Err(AppError::Forbidden(format!(
            "Order {} belongs to another user",  // Confirms existence!
            order_id
        )));
    }

    Ok(Json(order))
}

/// SUBTLE VULNERABILITY #3: Logging before authorization
///
/// This demonstrates a common pattern where developers log the request
/// details (including sensitive data) before checking authorization.
/// Even if access is denied, the data has been exposed in logs.
async fn logging_before_auth_get_order(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Path(order_id): Path<i64>,
) -> Result<Json<Order>, AppError> {
    // BUG: Fetching order BEFORE authorization check
    // "We need to log what order was requested for audit purposes"
    let order = state.db.get_order_by_id(order_id)?;

    // BUG: Logging sensitive data before authorization!
    // Even if we deny access, order details are now in the logs
    if let Some(ref o) = order {
        tracing::info!(
            order_id = o.id,
            order_user = o.user,       // Sensitive!
            order_product = o.product, // Sensitive!
            order_quantity = o.quantity, // Sensitive!
            requester = user.0.sub,
            "Order access attempted"  // Log says "attempted" but data is already exposed
        );
    }

    // Now we check authorization (too late!)
    let order = order.ok_or_else(|| {
        AppError::NotFound(format!("Order {} not found", order_id))
    })?;

    if order.user != user.0.sub {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    Ok(Json(order))
}
