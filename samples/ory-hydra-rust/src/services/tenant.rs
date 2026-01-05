use chrono::Utc;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::db::TenantSchemaManager;
use crate::error::AppError;
use crate::models::{CreateTenantRequest, Tenant, TenantPlan, TenantStatus, UpdateTenantRequest};

/// Service for tenant management
pub struct TenantService {
    pool: PgPool,
    schema_manager: TenantSchemaManager,
}

impl TenantService {
    pub fn new(pool: PgPool) -> Self {
        let schema_manager = TenantSchemaManager::new(pool.clone());
        Self {
            pool,
            schema_manager,
        }
    }

    /// Create a new tenant with its own database schema
    #[instrument(skip(self))]
    pub async fn create(&self, request: CreateTenantRequest) -> Result<Tenant, AppError> {
        // Generate tenant ID and schema name
        let tenant_id = Uuid::new_v4();
        let schema_name = TenantSchemaManager::schema_name_from_id(tenant_id);
        let plan = request.plan.unwrap_or_else(|| TenantPlan::Free.to_string());
        let now = Utc::now();

        // Insert tenant record
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO public.tenants (id, slug, name, schema_name, plan, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, slug, name, schema_name, plan, status, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(&request.slug)
        .bind(&request.name)
        .bind(&schema_name)
        .bind(&plan)
        .bind(TenantStatus::Active.to_string())
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::BadRequest(format!("Tenant slug '{}' already exists", request.slug))
            } else {
                AppError::Database(e.to_string())
            }
        })?;

        // Create tenant schema with tables
        self.schema_manager.create_schema(&schema_name).await?;

        Ok(tenant)
    }

    /// Get tenant by ID
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Tenant, AppError> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, slug, name, schema_name, plan, status, created_at, updated_at
            FROM public.tenants
            WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::TenantNotFound)
    }

    /// Get tenant by slug (for subdomain routing)
    #[instrument(skip(self))]
    pub async fn get_by_slug(&self, slug: &str) -> Result<Tenant, AppError> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, slug, name, schema_name, plan, status, created_at, updated_at
            FROM public.tenants
            WHERE slug = $1 AND status != 'deleted'
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::TenantNotFound)
    }

    /// List all tenants (for platform admin)
    #[instrument(skip(self))]
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, slug, name, schema_name, plan, status, created_at, updated_at
            FROM public.tenants
            WHERE status != 'deleted'
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update tenant
    #[instrument(skip(self))]
    pub async fn update(&self, id: Uuid, request: UpdateTenantRequest) -> Result<Tenant, AppError> {
        let tenant = self.get_by_id(id).await?;

        let name = request.name.unwrap_or(tenant.name);
        let plan = request.plan.unwrap_or(tenant.plan);
        let status = request.status.unwrap_or(tenant.status);
        let now = Utc::now();

        sqlx::query_as::<_, Tenant>(
            r#"
            UPDATE public.tenants
            SET name = $2, plan = $3, status = $4, updated_at = $5
            WHERE id = $1
            RETURNING id, slug, name, schema_name, plan, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(plan)
        .bind(status)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Soft delete tenant
    #[instrument(skip(self))]
    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE public.tenants
            SET status = 'deleted', updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::TenantNotFound);
        }

        Ok(())
    }

    /// Count total tenants
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn count(&self) -> Result<i64, AppError> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM public.tenants WHERE status != 'deleted'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_name_generation() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let schema_name = TenantSchemaManager::schema_name_from_id(id);
        assert!(schema_name.starts_with("tenant_"));
        assert!(!schema_name.contains('-'));
    }
}
