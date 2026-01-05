use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{CandidateWithDetails, HireCandidateRequest, HireResult};
use crate::services::RecruitmentService;
use crate::state::AppState;

fn default_count() -> i32 {
    5
}

/// Response for pool refresh status
#[derive(Debug, Serialize)]
pub struct RefreshStatusResponse {
    pub can_free_refresh: bool,
    pub refresh_cost: i64,
}

/// Get schema name from tenant_id
async fn get_schema_name(state: &AppState, tenant_id: Uuid) -> Result<String, AppError> {
    let tenant = state.tenant.get_by_id(tenant_id).await?;
    Ok(tenant.schema_name)
}

/// Get tenant balance (placeholder - should come from finance service)
async fn get_tenant_balance(state: &AppState, schema: &str) -> Result<i64, AppError> {
    let sql = format!("SELECT balance FROM {}.tenant_finance LIMIT 1", schema);
    let result: Option<(i64,)> = sqlx::query_as(&sql)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(result.map(|r| r.0).unwrap_or(0))
}

/// GET /api/v1/tenant/recruitment/candidates - List available candidates
#[instrument(skip(state))]
pub async fn list_candidates(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<CandidateWithDetails>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can view candidates
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can view candidates".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let balance = get_tenant_balance(&state, &schema).await?;
    let service = RecruitmentService::new(state.pool.clone());

    let candidates = service.list_available(&schema, balance).await?;
    Ok(Json(candidates))
}

/// GET /api/v1/tenant/recruitment/candidates/:id - Get candidate details
#[instrument(skip(state))]
pub async fn get_candidate(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::models::Candidate>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can view candidates
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can view candidates".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = RecruitmentService::new(state.pool.clone());

    let candidate = service.get_by_id(&schema, id).await?;
    Ok(Json(candidate))
}

/// POST /api/v1/tenant/recruitment/refresh - Refresh the candidate pool
#[instrument(skip(state))]
pub async fn refresh_pool(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<crate::models::Candidate>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can refresh pool
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can refresh candidate pool".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = RecruitmentService::new(state.pool.clone());

    // Check if free refresh is available
    let can_free = service.can_free_refresh(&schema).await?;
    if !can_free {
        // TODO: Check balance and deduct cost
        // For now, just allow refresh
    }

    let count = default_count();
    let candidates = service.refresh_pool(&schema, count).await?;
    Ok(Json(candidates))
}

/// POST /api/v1/tenant/recruitment/hire - Hire a candidate
#[instrument(skip(state))]
pub async fn hire_candidate(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<HireCandidateRequest>,
) -> Result<(StatusCode, Json<HireResult>), AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can hire
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can hire candidates".to_string(),
        ));
    }

    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = RecruitmentService::new(state.pool.clone());

    // Check if tenant can afford
    let balance = get_tenant_balance(&state, &schema).await?;
    let candidate = service.get_by_id(&schema, req.candidate_id).await?;

    if balance < candidate.hiring_cost {
        return Err(AppError::BadRequest(format!(
            "Insufficient funds. Need {} but only have {}",
            candidate.hiring_cost, balance
        )));
    }

    let result = service
        .hire_candidate(&schema, req, user_id, tenant_id)
        .await?;

    Ok((StatusCode::CREATED, Json(result)))
}

/// GET /api/v1/tenant/recruitment/status - Get refresh status
#[instrument(skip(state))]
pub async fn get_refresh_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<RefreshStatusResponse>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can view status
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can view recruitment status".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = RecruitmentService::new(state.pool.clone());

    let can_free = service.can_free_refresh(&schema).await?;

    Ok(Json(RefreshStatusResponse {
        can_free_refresh: can_free,
        refresh_cost: if can_free { 0 } else { 5000 },
    }))
}
