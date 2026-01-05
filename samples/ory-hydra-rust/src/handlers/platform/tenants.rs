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
use crate::models::{CreateTenantRequest, Tenant, TenantPlan, TenantStatus, UpdateTenantRequest};
use crate::services::TenantService;
use crate::state::AppState;

/// Response for tenant operations
#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub schema_name: String,
    pub plan: TenantPlan,
    pub status: TenantStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Tenant> for TenantResponse {
    fn from(tenant: Tenant) -> Self {
        let plan = tenant.get_plan();
        let status = tenant.get_status();
        Self {
            id: tenant.id,
            slug: tenant.slug,
            name: tenant.name,
            schema_name: tenant.schema_name,
            plan,
            status,
            created_at: tenant.created_at,
            updated_at: tenant.updated_at,
        }
    }
}

/// Query parameters for listing tenants
#[derive(Debug, Deserialize)]
pub struct ListTenantsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// Create a new tenant (PlatformAdmin only)
#[instrument(skip(state))]
pub async fn create_tenant(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), AppError> {
    // Check permission
    let role = claims.get_role();
    if !role.can_create_tenants() {
        return Err(AppError::Forbidden(
            "Only platform admins can create tenants".to_string(),
        ));
    }

    // Validate slug format
    if !is_valid_slug(&req.slug) {
        return Err(AppError::ValidationError(
            "Slug must be lowercase alphanumeric with hyphens, 3-50 characters".to_string(),
        ));
    }

    let tenant_service = TenantService::new(state.pool.clone());
    let tenant = tenant_service.create(req).await?;

    Ok((StatusCode::CREATED, Json(TenantResponse::from(tenant))))
}

/// List all tenants (PlatformAdmin only)
#[instrument(skip(state))]
pub async fn list_tenants(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(query): Query<ListTenantsQuery>,
) -> Result<Json<Vec<TenantResponse>>, AppError> {
    // Check permission
    let role = claims.get_role();
    if !role.can_create_tenants() {
        return Err(AppError::Forbidden(
            "Only platform admins can list tenants".to_string(),
        ));
    }

    let tenant_service = TenantService::new(state.pool.clone());
    let tenants = tenant_service.list(query.limit, query.offset).await?;

    Ok(Json(
        tenants.into_iter().map(TenantResponse::from).collect(),
    ))
}

/// Get a tenant by ID (PlatformAdmin only)
#[instrument(skip(state))]
pub async fn get_tenant(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, AppError> {
    // Check permission
    let role = claims.get_role();
    if !role.can_create_tenants() {
        return Err(AppError::Forbidden(
            "Only platform admins can view tenant details".to_string(),
        ));
    }

    let tenant_service = TenantService::new(state.pool.clone());
    let tenant = tenant_service.get_by_id(tenant_id).await?;

    Ok(Json(TenantResponse::from(tenant)))
}

/// Update a tenant (PlatformAdmin only)
#[instrument(skip(state))]
pub async fn update_tenant(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, AppError> {
    // Check permission
    let role = claims.get_role();
    if !role.can_create_tenants() {
        return Err(AppError::Forbidden(
            "Only platform admins can update tenants".to_string(),
        ));
    }

    let tenant_service = TenantService::new(state.pool.clone());
    let updated = tenant_service.update(tenant_id, req).await?;

    Ok(Json(TenantResponse::from(updated)))
}

/// Delete a tenant (PlatformAdmin only, soft delete)
#[instrument(skip(state))]
pub async fn delete_tenant(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Check permission
    let role = claims.get_role();
    if !role.can_create_tenants() {
        return Err(AppError::Forbidden(
            "Only platform admins can delete tenants".to_string(),
        ));
    }

    let tenant_service = TenantService::new(state.pool.clone());
    tenant_service.delete(tenant_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Validate slug format
fn is_valid_slug(slug: &str) -> bool {
    if slug.len() < 3 || slug.len() > 50 {
        return false;
    }

    // Must start with a letter
    if !slug.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return false;
    }

    // Only lowercase alphanumeric and hyphens, no consecutive hyphens
    let mut prev_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false;
        }
    }

    // Must not end with hyphen
    !slug.ends_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_slugs() {
        assert!(is_valid_slug("shop-a"));
        assert!(is_valid_slug("my-store"));
        assert!(is_valid_slug("store123"));
        assert!(is_valid_slug("abc"));
    }

    #[test]
    fn test_invalid_slugs() {
        assert!(!is_valid_slug("ab")); // too short
        assert!(!is_valid_slug("SHOP")); // uppercase
        assert!(!is_valid_slug("shop--a")); // consecutive hyphens
        assert!(!is_valid_slug("-shop")); // starts with hyphen
        assert!(!is_valid_slug("shop-")); // ends with hyphen
        assert!(!is_valid_slug("123shop")); // starts with number
        assert!(!is_valid_slug("shop_a")); // underscore
    }
}
