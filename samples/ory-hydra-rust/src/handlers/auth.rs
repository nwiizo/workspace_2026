use crate::error::AppError;
use crate::models::TokenResponse;
use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Request for user registration
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Request for login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Request for token refresh
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Response for registration
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub email: String,
    pub message: String,
}

/// POST /api/auth/register - Register a new user
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(email = %req.email, "Registering new user");

    let user = state.auth.register(&req.email, &req.password).await?;

    let response = RegisterResponse {
        id: user.id.to_string(),
        email: user.email,
        message: "User registered successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /api/auth/login - Login and get tokens
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    info!(email = %req.email, "User login attempt");

    let user = state.auth.authenticate(&req.email, &req.password).await?;

    let tokens = state.jwt.generate_tokens(&user)?;

    info!(user_id = %user.id, "User logged in successfully");

    Ok(Json(tokens))
}

/// POST /api/auth/refresh - Refresh access token
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    info!("Token refresh attempt");

    // Verify the refresh token
    let claims = state.jwt.verify_refresh_token(&req.refresh_token)?;

    // Get the user
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::Internal(format!("Invalid user ID: {}", e)))?;

    let user = state.auth.get_user_by_id(&user_id).await?;

    // Generate new tokens
    let tokens = state.jwt.generate_tokens(&user)?;

    info!(user_id = %user.id, "Token refreshed successfully");

    Ok(Json(tokens))
}

/// POST /api/auth/logout - Logout (invalidate tokens)
pub async fn logout() -> impl IntoResponse {
    // In a production system, you would:
    // 1. Add the token to a blacklist in Redis
    // 2. Invalidate the session
    // For this demo, we just return success
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Logged out successfully"
        })),
    )
}
