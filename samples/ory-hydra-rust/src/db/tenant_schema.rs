use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::error::AppError;

/// Manages tenant-specific database schemas
pub struct TenantSchemaManager {
    pool: PgPool,
}

impl TenantSchemaManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate a schema name from tenant ID
    pub fn schema_name_from_id(tenant_id: Uuid) -> String {
        format!("tenant_{}", tenant_id.to_string().replace('-', "_"))
    }

    /// Create a new schema for a tenant
    #[instrument(skip(self))]
    pub async fn create_schema(&self, schema_name: &str) -> Result<(), AppError> {
        info!("Creating tenant schema: {}", schema_name);

        // Validate schema name to prevent SQL injection
        if !Self::is_valid_schema_name(schema_name) {
            return Err(AppError::BadRequest(format!(
                "Invalid schema name: {}",
                schema_name
            )));
        }

        // Read the template and replace placeholders
        let template = include_str!("../../sql/tenant_schema_template.sql");
        let sql = template.replace("{{schema_name}}", schema_name);

        // Execute the schema creation SQL
        sqlx::raw_sql(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create schema: {}", e)))?;

        info!("Tenant schema created successfully: {}", schema_name);

        Ok(())
    }

    /// Drop a tenant schema (use with caution!)
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn drop_schema(&self, schema_name: &str) -> Result<(), AppError> {
        info!("Dropping tenant schema: {}", schema_name);

        if !Self::is_valid_schema_name(schema_name) {
            return Err(AppError::BadRequest(format!(
                "Invalid schema name: {}",
                schema_name
            )));
        }

        let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name);

        sqlx::raw_sql(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to drop schema: {}", e)))?;

        info!("Tenant schema dropped: {}", schema_name);

        Ok(())
    }

    /// Check if a schema exists
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn schema_exists(&self, schema_name: &str) -> Result<bool, AppError> {
        let result: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = $1)",
        )
        .bind(schema_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.0)
    }

    /// Validate schema name to prevent SQL injection
    fn is_valid_schema_name(name: &str) -> bool {
        // Schema name must start with "tenant_" and contain only alphanumeric and underscore
        if !name.starts_with("tenant_") {
            return false;
        }

        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_name_from_id() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let name = TenantSchemaManager::schema_name_from_id(id);
        assert_eq!(name, "tenant_550e8400_e29b_41d4_a716_446655440000");
    }

    #[test]
    fn test_is_valid_schema_name() {
        assert!(TenantSchemaManager::is_valid_schema_name(
            "tenant_abc123_def456"
        ));
        assert!(TenantSchemaManager::is_valid_schema_name("tenant_12345"));

        assert!(!TenantSchemaManager::is_valid_schema_name("public"));
        assert!(!TenantSchemaManager::is_valid_schema_name(
            "tenant-with-dash"
        ));
        assert!(!TenantSchemaManager::is_valid_schema_name(
            "tenant_with space"
        ));
        assert!(!TenantSchemaManager::is_valid_schema_name(
            "tenant_with;injection"
        ));
        assert!(!TenantSchemaManager::is_valid_schema_name("other_prefix"));
    }
}
