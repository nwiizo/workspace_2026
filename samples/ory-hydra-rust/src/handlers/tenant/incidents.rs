use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::{
    AssignIncidentRequest, ChangeIncidentStatusRequest, CreateIncidentRequest, IncidentWithStatus,
    UpdateIncidentRequest,
};
use crate::services::IncidentService;
use crate::state::AppState;

/// Query parameters for listing incidents
#[derive(Debug, Deserialize)]
pub struct ListIncidentsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status_id: Option<Uuid>,
    pub severity: Option<String>,
    pub assigned_to: Option<Uuid>,
}

fn default_limit() -> i64 {
    20
}

/// Get schema name from tenant_id
async fn get_schema_name(state: &AppState, tenant_id: Uuid) -> Result<String, AppError> {
    let tenant = state.tenant.get_by_id(tenant_id).await?;
    Ok(tenant.schema_name)
}

/// POST /api/v1/tenant/incidents - Create a new incident
#[instrument(skip(state))]
pub async fn create_incident(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateIncidentRequest>,
) -> Result<(StatusCode, Json<IncidentWithStatus>), AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let incident = service.create(&schema, user_id, req).await?;
    let incident_with_status = service.get_with_status(&schema, incident.id).await?;

    Ok((StatusCode::CREATED, Json(incident_with_status)))
}

/// GET /api/v1/tenant/incidents - List incidents
#[instrument(skip(state))]
pub async fn list_incidents(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<ListIncidentsQuery>,
) -> Result<Json<Vec<IncidentWithStatus>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let incidents = service
        .list(
            &schema,
            query.limit,
            query.offset,
            query.status_id,
            query.severity,
            query.assigned_to,
        )
        .await?;

    Ok(Json(incidents))
}

/// GET /api/v1/tenant/incidents/:id - Get incident details
#[instrument(skip(state))]
pub async fn get_incident(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<IncidentWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let incident = service.get_with_status(&schema, id).await?;
    Ok(Json(incident))
}

/// PUT /api/v1/tenant/incidents/:id - Update incident
#[instrument(skip(state))]
pub async fn update_incident(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateIncidentRequest>,
) -> Result<Json<IncidentWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can update incidents
    let role = claims.get_role();
    if !role.can_manage_team() {
        // Check if user is assigned to this incident
        let schema = get_schema_name(&state, tenant_id).await?;
        let service = IncidentService::new(state.pool.clone());
        let incident = service.get_by_id(&schema, id).await?;
        let user_id: Uuid = claims
            .sub
            .parse()
            .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;
        if incident.assigned_engineer_id != Some(user_id) {
            return Err(AppError::Forbidden(
                "Only assigned engineer or manager can update".to_string(),
            ));
        }
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let _ = service.update(&schema, id, req).await?;
    let incident = service.get_with_status(&schema, id).await?;

    Ok(Json(incident))
}

/// POST /api/v1/tenant/incidents/:id/assign - Assign engineer to incident
#[instrument(skip(state))]
pub async fn assign_incident(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignIncidentRequest>,
) -> Result<Json<IncidentWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can assign
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can assign incidents".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let _ = service.assign(&schema, id, req).await?;
    let incident = service.get_with_status(&schema, id).await?;

    Ok(Json(incident))
}

/// PATCH /api/v1/tenant/incidents/:id/status - Change incident status
#[instrument(skip(state))]
pub async fn change_incident_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeIncidentStatusRequest>,
) -> Result<Json<IncidentWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Engineer+ can change status (if assigned or manager)
    let role = claims.get_role();
    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    if !role.can_manage_team() {
        let incident = service.get_by_id(&schema, id).await?;
        let user_id: Uuid = claims
            .sub
            .parse()
            .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;
        if incident.assigned_engineer_id != Some(user_id) {
            return Err(AppError::Forbidden(
                "Only assigned engineer or manager can change status".to_string(),
            ));
        }
    }

    let _ = service.change_status(&schema, id, req).await?;
    let incident = service.get_with_status(&schema, id).await?;

    Ok(Json(incident))
}

/// DELETE /api/v1/tenant/incidents/:id - Delete incident
#[instrument(skip(state))]
pub async fn delete_incident(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can delete
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can delete incidents".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    service.delete(&schema, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/tenant/incidents/stats - Get incident statistics
#[instrument(skip(state))]
pub async fn get_incident_stats(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<crate::services::IncidentStatistics>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = IncidentService::new(state.pool.clone());

    let stats = service.get_statistics(&schema).await?;
    Ok(Json(stats))
}
