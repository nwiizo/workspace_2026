use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tracing::info;

use crate::error::AppError;

/// Create a PostgreSQL connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, AppError> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    info!("Database connection established");

    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    info!("Running database migrations...");

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e: sqlx::migrate::MigrateError| {
            AppError::Database(format!("Migration failed: {}", e))
        })?;

    info!("Migrations completed successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running PostgreSQL
    async fn test_create_pool() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://hydra:secret@localhost:5432/hydra".to_string());

        let result = create_pool(&database_url).await;
        assert!(result.is_ok(), "Should connect to database");
    }
}
