use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{EngineerWithSpecialties, Proficiency};
use crate::services::EngineerService;
use crate::state::AppState;

/// Query parameters for listing engineers
#[derive(Debug, Deserialize)]
pub struct ListEngineersQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// Request to add specialty to engineer
#[derive(Debug, Deserialize)]
pub struct AddSpecialtyRequest {
    pub specialty_id: Uuid,
    pub proficiency: String,
}

/// Request to fire an engineer
#[derive(Debug, Deserialize)]
pub struct FireEngineerRequest {
    pub reason: String,
}

/// Response for firing an engineer
#[derive(Debug, Serialize)]
pub struct FireEngineerResponse {
    pub success: bool,
    pub message: String,
}

/// Get schema name from tenant_id
async fn get_schema_name(state: &AppState, tenant_id: Uuid) -> Result<String, AppError> {
    let tenant = state.tenant.get_by_id(tenant_id).await?;
    Ok(tenant.schema_name)
}

/// GET /api/v1/tenant/engineers - List all active engineers
#[instrument(skip(state))]
pub async fn list_engineers(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<ListEngineersQuery>,
) -> Result<Json<Vec<EngineerWithSpecialties>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let role = claims.get_role();
    let schema = get_schema_name(&state, tenant_id).await?;
    let service = EngineerService::new(state.pool.clone());

    let engineers = service
        .list_active(&schema, role, query.limit, query.offset)
        .await?;

    Ok(Json(engineers))
}

/// GET /api/v1/tenant/engineers/:id - Get engineer details
#[instrument(skip(state))]
pub async fn get_engineer(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EngineerWithSpecialties>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let role = claims.get_role();
    let schema = get_schema_name(&state, tenant_id).await?;
    let service = EngineerService::new(state.pool.clone());

    let engineer = service.get_with_specialties(&schema, id, role).await?;
    Ok(Json(engineer))
}

/// POST /api/v1/tenant/engineers/:id/specialties - Add specialty to engineer
#[instrument(skip(state))]
pub async fn add_specialty(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AddSpecialtyRequest>,
) -> Result<StatusCode, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can add specialties
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can modify specialties".to_string(),
        ));
    }

    let proficiency: Proficiency = req
        .proficiency
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid proficiency level".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = EngineerService::new(state.pool.clone());

    service
        .add_specialty(&schema, id, req.specialty_id, proficiency)
        .await?;

    Ok(StatusCode::OK)
}

/// POST /api/v1/tenant/engineers/:id/fire - Fire an engineer
#[instrument(skip(state))]
pub async fn fire_engineer(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(_req): Json<FireEngineerRequest>,
) -> Result<Json<FireEngineerResponse>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can fire
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can fire engineers".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = EngineerService::new(state.pool.clone());

    let engineer = service.fire(&schema, id).await?;

    Ok(Json(FireEngineerResponse {
        success: true,
        message: format!("Engineer {} has been fired", engineer.id),
    }))
}

/// GET /api/v1/tenant/engineers/salary-total - Get total salary expense
#[instrument(skip(state))]
pub async fn get_total_salary(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<TotalSalaryResponse>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can view salary info
    let role = claims.get_role();
    if !role.can_view_proficiency() {
        return Err(AppError::Forbidden(
            "Only managers can view salary information".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = EngineerService::new(state.pool.clone());

    let total = service.get_total_salary(&schema).await?;

    Ok(Json(TotalSalaryResponse {
        total_monthly_salary: total,
    }))
}

#[derive(Debug, Serialize)]
pub struct TotalSalaryResponse {
    pub total_monthly_salary: i64,
}
