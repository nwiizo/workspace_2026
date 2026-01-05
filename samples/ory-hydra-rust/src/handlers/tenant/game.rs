use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::services::{GameEngineService, LeaderboardEntry, LeaderboardType};
use crate::state::AppState;

/// Query parameters for leaderboard
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default = "default_type")]
    pub leaderboard_type: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_type() -> String {
    "level".to_string()
}

fn default_limit() -> i64 {
    10
}

/// Get schema name from tenant_id
async fn get_schema_name(state: &AppState, tenant_id: Uuid) -> Result<String, AppError> {
    let tenant = state.tenant.get_by_id(tenant_id).await?;
    Ok(tenant.schema_name)
}

/// GET /api/v1/tenant/leaderboard - Get leaderboard
#[instrument(skip(state))]
pub async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = GameEngineService::new(state.pool.clone());

    let leaderboard_type = match query.leaderboard_type.to_lowercase().as_str() {
        "level" => LeaderboardType::Level,
        "revenue" => LeaderboardType::Revenue,
        "incidents" => LeaderboardType::Incidents,
        "projects" => LeaderboardType::Projects,
        _ => LeaderboardType::Level,
    };

    let entries = service
        .get_leaderboard(&schema, leaderboard_type, query.limit)
        .await?;

    Ok(Json(entries))
}

/// GET /api/v1/tenant/leaderboard/level - Get level leaderboard
#[instrument(skip(state))]
pub async fn get_level_leaderboard(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = GameEngineService::new(state.pool.clone());

    let entries = service
        .get_leaderboard(&schema, LeaderboardType::Level, query.limit)
        .await?;

    Ok(Json(entries))
}

/// GET /api/v1/tenant/leaderboard/revenue - Get revenue leaderboard
#[instrument(skip(state))]
pub async fn get_revenue_leaderboard(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = GameEngineService::new(state.pool.clone());

    let entries = service
        .get_leaderboard(&schema, LeaderboardType::Revenue, query.limit)
        .await?;

    Ok(Json(entries))
}

/// GET /api/v1/tenant/leaderboard/incidents - Get incidents leaderboard
#[instrument(skip(state))]
pub async fn get_incidents_leaderboard(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = GameEngineService::new(state.pool.clone());

    let entries = service
        .get_leaderboard(&schema, LeaderboardType::Incidents, query.limit)
        .await?;

    Ok(Json(entries))
}

/// GET /api/v1/tenant/leaderboard/projects - Get projects leaderboard
#[instrument(skip(state))]
pub async fn get_projects_leaderboard(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = GameEngineService::new(state.pool.clone());

    let entries = service
        .get_leaderboard(&schema, LeaderboardType::Projects, query.limit)
        .await?;

    Ok(Json(entries))
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}
