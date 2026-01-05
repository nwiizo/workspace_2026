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
    AssignProjectRequest, ChangeProjectStatusRequest, CreateProjectRequest, ProjectWithStatus,
    UpdateProjectHoursRequest, UpdateProjectRequest,
};
use crate::services::ProjectService;
use crate::state::AppState;

/// Query parameters for listing projects
#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// Get schema name from tenant_id
async fn get_schema_name(state: &AppState, tenant_id: Uuid) -> Result<String, AppError> {
    let tenant = state.tenant.get_by_id(tenant_id).await?;
    Ok(tenant.schema_name)
}

/// POST /api/v1/tenant/projects - Create a new project
#[instrument(skip(state))]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectWithStatus>), AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can create projects
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can create projects".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let project = service.create(&schema, req).await?;
    let project_with_status = service.get_with_status(&schema, project.id).await?;

    Ok((StatusCode::CREATED, Json(project_with_status)))
}

/// GET /api/v1/tenant/projects - List projects
#[instrument(skip(state))]
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<Vec<ProjectWithStatus>>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let projects = service.list(&schema, query.limit, query.offset).await?;
    Ok(Json(projects))
}

/// GET /api/v1/tenant/projects/:id - Get project details
#[instrument(skip(state))]
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let project = service.get_with_status(&schema, id).await?;
    Ok(Json(project))
}

/// PUT /api/v1/tenant/projects/:id - Update project
#[instrument(skip(state))]
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can update projects
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can update projects".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let _ = service.update(&schema, id, req).await?;
    let project = service.get_with_status(&schema, id).await?;

    Ok(Json(project))
}

/// POST /api/v1/tenant/projects/:id/assign - Assign engineer to project
#[instrument(skip(state))]
pub async fn assign_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignProjectRequest>,
) -> Result<StatusCode, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can assign
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can assign projects".to_string(),
        ));
    }

    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    service.assign(&schema, id, req, user_id).await?;
    Ok(StatusCode::OK)
}

/// PATCH /api/v1/tenant/projects/:id/status - Change project status
#[instrument(skip(state))]
pub async fn change_project_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeProjectStatusRequest>,
) -> Result<Json<ProjectWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Only Manager+ can change status
    let role = claims.get_role();
    if !role.can_manage_team() {
        return Err(AppError::Forbidden(
            "Only managers can change project status".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let _ = service.change_status(&schema, id, req).await?;
    let project = service.get_with_status(&schema, id).await?;

    Ok(Json(project))
}

/// PATCH /api/v1/tenant/projects/:id/hours - Update project hours
#[instrument(skip(state))]
pub async fn update_project_hours(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectHoursRequest>,
) -> Result<Json<ProjectWithStatus>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    // Engineer+ can update hours
    let role = claims.get_role();
    if !role.can_work_on_tasks() {
        return Err(AppError::Forbidden(
            "Only engineers can update project hours".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let _ = service.update_hours(&schema, id, req).await?;
    let project = service.get_with_status(&schema, id).await?;

    Ok(Json(project))
}

/// DELETE /api/v1/tenant/projects/:id - Delete project
#[instrument(skip(state))]
pub async fn delete_project(
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
            "Only managers can delete projects".to_string(),
        ));
    }

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    service.delete(&schema, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/tenant/projects/stats - Get project statistics
#[instrument(skip(state))]
pub async fn get_project_stats(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<crate::services::ProjectStatistics>, AppError> {
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Forbidden("No tenant context".to_string()))?;

    let schema = get_schema_name(&state, tenant_id).await?;
    let service = ProjectService::new(state.pool.clone());

    let stats = service.get_statistics(&schema).await?;
    Ok(Json(stats))
}
