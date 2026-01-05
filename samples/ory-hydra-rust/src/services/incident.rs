use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    AssignIncidentRequest, ChangeIncidentStatusRequest, CreateIncidentRequest, Incident,
    IncidentRow, IncidentWithStatus, UpdateIncidentRequest,
};

/// Incident row with status info from JOINed query
#[derive(Debug, sqlx::FromRow)]
struct IncidentWithStatusRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: String,
    pub difficulty: String,
    pub reward: i64,
    pub status_id: Uuid,
    pub assigned_engineer_id: Option<Uuid>,
    pub reporter_id: Uuid,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub status_name: String,
    pub status_color: String,
    pub is_terminal: bool,
}

impl From<IncidentWithStatusRow> for IncidentWithStatus {
    fn from(row: IncidentWithStatusRow) -> Self {
        use crate::models::{Difficulty, Severity};
        Self {
            incident: Incident {
                id: row.id,
                title: row.title,
                description: row.description,
                severity: row.severity.parse().unwrap_or(Severity::Medium),
                difficulty: row.difficulty.parse().unwrap_or(Difficulty::Normal),
                reward: row.reward,
                status_id: row.status_id,
                assigned_engineer_id: row.assigned_engineer_id,
                reporter_id: row.reporter_id,
                required_specialty_id: row.required_specialty_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
                resolved_at: row.resolved_at,
                closed_at: row.closed_at,
            },
            status_name: row.status_name,
            status_color: row.status_color,
            is_resolved: row.is_terminal,
        }
    }
}

/// Service for managing incidents
pub struct IncidentService {
    pool: PgPool,
}

impl IncidentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new incident
    #[instrument(skip(self, request))]
    pub async fn create(
        &self,
        schema: &str,
        reporter_id: Uuid,
        request: CreateIncidentRequest,
    ) -> Result<Incident, AppError> {
        // Get initial status for incidents
        let initial_status_id: (Uuid,) = sqlx::query_as(&format!(
            "SELECT id FROM {}.workflow_statuses WHERE entity_type = 'incident' AND is_initial = true LIMIT 1",
            schema
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to get initial status: {}", e)))?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let severity = request.severity.unwrap_or_else(|| "medium".to_string());
        let difficulty = request.difficulty.unwrap_or_else(|| "normal".to_string());
        let reward = request
            .reward
            .unwrap_or(Self::calculate_reward(&severity, &difficulty));

        let sql = format!(
            r#"
            INSERT INTO {}.incidents
            (id, title, description, severity, difficulty, reward, status_id, reporter_id, required_specialty_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            RETURNING *
            "#,
            schema
        );

        let row: IncidentRow = sqlx::query_as(&sql)
            .bind(id)
            .bind(&request.title)
            .bind(&request.description)
            .bind(&severity)
            .bind(&difficulty)
            .bind(reward)
            .bind(initial_status_id.0)
            .bind(reporter_id)
            .bind(request.required_specialty_id)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create incident: {}", e)))?;

        Ok(row.into())
    }

    /// Get incident by ID
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, schema: &str, id: Uuid) -> Result<Incident, AppError> {
        let sql = format!("SELECT * FROM {}.incidents WHERE id = $1", schema);
        let row: IncidentRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Incident not found".to_string()))?;

        Ok(row.into())
    }

    /// Get incident with status details
    #[instrument(skip(self))]
    pub async fn get_with_status(
        &self,
        schema: &str,
        id: Uuid,
    ) -> Result<IncidentWithStatus, AppError> {
        let sql = format!(
            r#"
            SELECT i.id, i.title, i.description, i.severity, i.difficulty, i.reward,
                   i.status_id, i.assigned_engineer_id, i.reporter_id, i.required_specialty_id,
                   i.created_at, i.updated_at, i.resolved_at, i.closed_at,
                   ws.name as status_name, ws.color as status_color, ws.is_terminal
            FROM {}.incidents i
            JOIN {}.workflow_statuses ws ON i.status_id = ws.id
            WHERE i.id = $1
            "#,
            schema, schema
        );

        let row: IncidentWithStatusRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Incident not found".to_string()))?;

        Ok(row.into())
    }

    /// List incidents with optional filters
    #[instrument(skip(self))]
    pub async fn list(
        &self,
        schema: &str,
        limit: i64,
        offset: i64,
        _status_id: Option<Uuid>,
        _severity: Option<String>,
        _assigned_to: Option<Uuid>,
    ) -> Result<Vec<IncidentWithStatus>, AppError> {
        // Simplified version - filters can be added later with dynamic query building
        let sql = format!(
            r#"
            SELECT i.id, i.title, i.description, i.severity, i.difficulty, i.reward,
                   i.status_id, i.assigned_engineer_id, i.reporter_id, i.required_specialty_id,
                   i.created_at, i.updated_at, i.resolved_at, i.closed_at,
                   ws.name as status_name, ws.color as status_color, ws.is_terminal
            FROM {}.incidents i
            JOIN {}.workflow_statuses ws ON i.status_id = ws.id
            ORDER BY i.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            schema, schema
        );

        let rows: Vec<IncidentWithStatusRow> = sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    /// Update an incident
    #[instrument(skip(self, request))]
    pub async fn update(
        &self,
        schema: &str,
        id: Uuid,
        request: UpdateIncidentRequest,
    ) -> Result<Incident, AppError> {
        let current = self.get_by_id(schema, id).await?;
        let now = Utc::now();

        let sql = format!(
            r#"
            UPDATE {}.incidents SET
                title = COALESCE($1, title),
                description = COALESCE($2, description),
                severity = COALESCE($3, severity),
                difficulty = COALESCE($4, difficulty),
                reward = COALESCE($5, reward),
                required_specialty_id = COALESCE($6, required_specialty_id),
                updated_at = $7
            WHERE id = $8
            RETURNING *
            "#,
            schema
        );

        let row: IncidentRow = sqlx::query_as(&sql)
            .bind(request.title.as_ref().or(Some(&current.title)))
            .bind(
                request
                    .description
                    .as_ref()
                    .or(current.description.as_ref()),
            )
            .bind(request.severity)
            .bind(request.difficulty)
            .bind(request.reward)
            .bind(request.required_specialty_id)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update incident: {}", e)))?;

        Ok(row.into())
    }

    /// Assign an engineer to an incident
    #[instrument(skip(self))]
    pub async fn assign(
        &self,
        schema: &str,
        id: Uuid,
        request: AssignIncidentRequest,
    ) -> Result<Incident, AppError> {
        let now = Utc::now();

        // Get "Assigned" status
        let assigned_status: Option<(Uuid,)> = sqlx::query_as(&format!(
            "SELECT id FROM {}.workflow_statuses WHERE entity_type = 'incident' AND name = 'Assigned' LIMIT 1",
            schema
        ))
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let sql = format!(
            r#"
            UPDATE {}.incidents SET
                assigned_engineer_id = $1,
                status_id = COALESCE($2, status_id),
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            schema
        );

        let row: IncidentRow = sqlx::query_as(&sql)
            .bind(request.engineer_id)
            .bind(assigned_status.map(|s| s.0))
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to assign incident: {}", e)))?;

        Ok(row.into())
    }

    /// Change incident status
    #[instrument(skip(self))]
    pub async fn change_status(
        &self,
        schema: &str,
        id: Uuid,
        request: ChangeIncidentStatusRequest,
    ) -> Result<Incident, AppError> {
        let now = Utc::now();

        // Check if new status is terminal (resolved/closed)
        let status_info: (bool,) = sqlx::query_as(&format!(
            "SELECT is_terminal FROM {}.workflow_statuses WHERE id = $1",
            schema
        ))
        .bind(request.status_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Status not found: {}", e)))?;

        let resolved_at = if status_info.0 { Some(now) } else { None };

        let sql = format!(
            r#"
            UPDATE {}.incidents SET
                status_id = $1,
                resolved_at = COALESCE($2, resolved_at),
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            schema
        );

        let row: IncidentRow = sqlx::query_as(&sql)
            .bind(request.status_id)
            .bind(resolved_at)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to change status: {}", e)))?;

        Ok(row.into())
    }

    /// Delete an incident
    #[instrument(skip(self))]
    pub async fn delete(&self, schema: &str, id: Uuid) -> Result<(), AppError> {
        let sql = format!("DELETE FROM {}.incidents WHERE id = $1", schema);
        let result = sqlx::query(&sql)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Incident not found".to_string()));
        }

        Ok(())
    }

    /// Calculate reward based on severity and difficulty
    fn calculate_reward(severity: &str, difficulty: &str) -> i64 {
        let severity_multiplier = match severity {
            "critical" => 4.0,
            "high" => 2.5,
            "medium" => 1.5,
            "low" => 1.0,
            _ => 1.0,
        };

        let difficulty_multiplier = match difficulty {
            "extreme" => 5.0,
            "expert" => 3.0,
            "hard" => 2.0,
            "normal" => 1.0,
            "easy" => 0.5,
            _ => 1.0,
        };

        (1000.0 * severity_multiplier * difficulty_multiplier) as i64
    }

    /// Get incident statistics for dashboard
    #[instrument(skip(self))]
    pub async fn get_statistics(&self, schema: &str) -> Result<IncidentStatistics, AppError> {
        let sql = format!(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE ws.is_terminal = false) as open_count,
                COUNT(*) FILTER (WHERE ws.is_terminal = true) as resolved_count,
                COUNT(*) FILTER (WHERE i.severity = 'critical' AND ws.is_terminal = false) as critical_open,
                COUNT(*) as total_count
            FROM {}.incidents i
            JOIN {}.workflow_statuses ws ON i.status_id = ws.id
            "#,
            schema, schema
        );

        let stats: (i64, i64, i64, i64) = sqlx::query_as(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(IncidentStatistics {
            open_count: stats.0 as i32,
            resolved_count: stats.1 as i32,
            critical_open: stats.2 as i32,
            total_count: stats.3 as i32,
        })
    }
}

/// Incident statistics for dashboard
#[derive(Debug, Clone, serde::Serialize)]
pub struct IncidentStatistics {
    pub open_count: i32,
    pub resolved_count: i32,
    pub critical_open: i32,
    pub total_count: i32,
}
