use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    AssignProjectRequest, ChangeProjectStatusRequest, CreateProjectRequest, Difficulty, Priority,
    Project, ProjectRow, ProjectWithStatus, UpdateProjectHoursRequest, UpdateProjectRequest,
};

/// Project row with status info from JOINed query
#[derive(Debug, sqlx::FromRow)]
struct ProjectWithStatusRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status_id: Uuid,
    pub priority: String,
    pub difficulty: String,
    pub reward: i64,
    pub deadline: Option<DateTime<Utc>>,
    pub estimated_hours: Option<i32>,
    pub actual_hours: i32,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status_name: String,
    pub status_color: String,
    pub is_terminal: bool,
}

impl From<ProjectWithStatusRow> for ProjectWithStatus {
    fn from(row: ProjectWithStatusRow) -> Self {
        Self {
            project: Project {
                id: row.id,
                title: row.title,
                description: row.description,
                status_id: row.status_id,
                priority: row.priority.parse().unwrap_or(Priority::Medium),
                difficulty: row.difficulty.parse().unwrap_or(Difficulty::Normal),
                reward: row.reward,
                deadline: row.deadline,
                estimated_hours: row.estimated_hours,
                actual_hours: row.actual_hours,
                required_specialty_id: row.required_specialty_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
                completed_at: row.completed_at,
            },
            status_name: row.status_name,
            status_color: row.status_color,
            is_completed: row.is_terminal,
            assigned_engineers: vec![],
        }
    }
}

/// Service for managing projects
pub struct ProjectService {
    pool: PgPool,
}

impl ProjectService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new project
    #[instrument(skip(self, request))]
    pub async fn create(
        &self,
        schema: &str,
        request: CreateProjectRequest,
    ) -> Result<Project, AppError> {
        // Get initial status for projects
        let initial_status_id: (Uuid,) = sqlx::query_as(&format!(
            "SELECT id FROM {}.workflow_statuses WHERE entity_type = 'project' AND is_initial = true LIMIT 1",
            schema
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to get initial status: {}", e)))?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let priority = request.priority.unwrap_or_else(|| "medium".to_string());
        let difficulty = request.difficulty.unwrap_or_else(|| "normal".to_string());
        let reward = request
            .reward
            .unwrap_or(Self::calculate_reward(&priority, &difficulty));

        let sql = format!(
            r#"
            INSERT INTO {}.projects
            (id, title, description, status_id, priority, difficulty, reward, deadline,
             estimated_hours, required_specialty_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            RETURNING *
            "#,
            schema
        );

        let row: ProjectRow = sqlx::query_as(&sql)
            .bind(id)
            .bind(&request.title)
            .bind(&request.description)
            .bind(initial_status_id.0)
            .bind(&priority)
            .bind(&difficulty)
            .bind(reward)
            .bind(request.deadline)
            .bind(request.estimated_hours)
            .bind(request.required_specialty_id)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create project: {}", e)))?;

        Ok(row.into())
    }

    /// Get project by ID
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, schema: &str, id: Uuid) -> Result<Project, AppError> {
        let sql = format!("SELECT * FROM {}.projects WHERE id = $1", schema);
        let row: ProjectRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Project not found".to_string()))?;

        Ok(row.into())
    }

    /// Get project with status details
    #[instrument(skip(self))]
    pub async fn get_with_status(
        &self,
        schema: &str,
        id: Uuid,
    ) -> Result<ProjectWithStatus, AppError> {
        let sql = format!(
            r#"
            SELECT p.id, p.title, p.description, p.status_id, p.priority, p.difficulty,
                   p.reward, p.deadline, p.estimated_hours, p.actual_hours,
                   p.required_specialty_id, p.created_at, p.updated_at, p.completed_at,
                   ws.name as status_name, ws.color as status_color, ws.is_terminal
            FROM {}.projects p
            JOIN {}.workflow_statuses ws ON p.status_id = ws.id
            WHERE p.id = $1
            "#,
            schema, schema
        );

        let row: ProjectWithStatusRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Project not found".to_string()))?;

        Ok(row.into())
    }

    /// List projects
    #[instrument(skip(self))]
    pub async fn list(
        &self,
        schema: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ProjectWithStatus>, AppError> {
        let sql = format!(
            r#"
            SELECT p.id, p.title, p.description, p.status_id, p.priority, p.difficulty,
                   p.reward, p.deadline, p.estimated_hours, p.actual_hours,
                   p.required_specialty_id, p.created_at, p.updated_at, p.completed_at,
                   ws.name as status_name, ws.color as status_color, ws.is_terminal
            FROM {}.projects p
            JOIN {}.workflow_statuses ws ON p.status_id = ws.id
            ORDER BY p.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            schema, schema
        );

        let rows: Vec<ProjectWithStatusRow> = sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    /// Update a project
    #[instrument(skip(self, request))]
    pub async fn update(
        &self,
        schema: &str,
        id: Uuid,
        request: UpdateProjectRequest,
    ) -> Result<Project, AppError> {
        let now = Utc::now();

        let sql = format!(
            r#"
            UPDATE {}.projects SET
                title = COALESCE($1, title),
                description = COALESCE($2, description),
                priority = COALESCE($3, priority),
                difficulty = COALESCE($4, difficulty),
                reward = COALESCE($5, reward),
                deadline = COALESCE($6, deadline),
                estimated_hours = COALESCE($7, estimated_hours),
                required_specialty_id = COALESCE($8, required_specialty_id),
                updated_at = $9
            WHERE id = $10
            RETURNING *
            "#,
            schema
        );

        let row: ProjectRow = sqlx::query_as(&sql)
            .bind(&request.title)
            .bind(&request.description)
            .bind(&request.priority)
            .bind(&request.difficulty)
            .bind(request.reward)
            .bind(request.deadline)
            .bind(request.estimated_hours)
            .bind(request.required_specialty_id)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update project: {}", e)))?;

        Ok(row.into())
    }

    /// Assign an engineer to a project
    #[instrument(skip(self))]
    pub async fn assign(
        &self,
        schema: &str,
        project_id: Uuid,
        request: AssignProjectRequest,
        assigned_by: Uuid,
    ) -> Result<(), AppError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let sql = format!(
            r#"
            INSERT INTO {}.assignments
            (id, assignable_type, assignable_id, engineer_id, role_in_assignment, assigned_at, assigned_by)
            VALUES ($1, 'project', $2, $3, $4, $5, $6)
            ON CONFLICT (assignable_type, assignable_id, engineer_id)
            DO UPDATE SET role_in_assignment = $4, assigned_at = $5
            "#,
            schema
        );

        sqlx::query(&sql)
            .bind(id)
            .bind(project_id)
            .bind(request.engineer_id)
            .bind(request.role.unwrap_or_else(|| "assignee".to_string()))
            .bind(now)
            .bind(assigned_by)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to assign engineer: {}", e)))?;

        // Update project status to "In Progress" if it's in backlog
        let _ = sqlx::query(&format!(
            r#"
            UPDATE {}.projects p SET
                status_id = (
                    SELECT id FROM {}.workflow_statuses
                    WHERE entity_type = 'project' AND name = 'In Progress' LIMIT 1
                ),
                updated_at = $1
            WHERE p.id = $2
            AND (SELECT name FROM {}.workflow_statuses WHERE id = p.status_id) IN ('Backlog', 'Planning')
            "#,
            schema, schema, schema
        ))
        .bind(now)
        .bind(project_id)
        .execute(&self.pool)
        .await;

        Ok(())
    }

    /// Change project status
    #[instrument(skip(self))]
    pub async fn change_status(
        &self,
        schema: &str,
        id: Uuid,
        request: ChangeProjectStatusRequest,
    ) -> Result<Project, AppError> {
        let now = Utc::now();

        // Check if new status is terminal
        let status_info: (bool,) = sqlx::query_as(&format!(
            "SELECT is_terminal FROM {}.workflow_statuses WHERE id = $1",
            schema
        ))
        .bind(request.status_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Status not found: {}", e)))?;

        let completed_at = if status_info.0 { Some(now) } else { None };

        let sql = format!(
            r#"
            UPDATE {}.projects SET
                status_id = $1,
                completed_at = COALESCE($2, completed_at),
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            schema
        );

        let row: ProjectRow = sqlx::query_as(&sql)
            .bind(request.status_id)
            .bind(completed_at)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to change status: {}", e)))?;

        Ok(row.into())
    }

    /// Update project hours
    #[instrument(skip(self))]
    pub async fn update_hours(
        &self,
        schema: &str,
        id: Uuid,
        request: UpdateProjectHoursRequest,
    ) -> Result<Project, AppError> {
        let now = Utc::now();

        let sql = format!(
            r#"
            UPDATE {}.projects SET
                actual_hours = actual_hours + $1,
                updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            schema
        );

        let row: ProjectRow = sqlx::query_as(&sql)
            .bind(request.hours_to_add)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update hours: {}", e)))?;

        Ok(row.into())
    }

    /// Delete a project
    #[instrument(skip(self))]
    pub async fn delete(&self, schema: &str, id: Uuid) -> Result<(), AppError> {
        let sql = format!("DELETE FROM {}.projects WHERE id = $1", schema);
        let result = sqlx::query(&sql)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        Ok(())
    }

    /// Calculate reward based on priority and difficulty
    fn calculate_reward(priority: &str, difficulty: &str) -> i64 {
        let priority_multiplier = match priority {
            "high" => 2.0,
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

        (5000.0 * priority_multiplier * difficulty_multiplier) as i64
    }

    /// Get project statistics for dashboard
    #[instrument(skip(self))]
    pub async fn get_statistics(&self, schema: &str) -> Result<ProjectStatistics, AppError> {
        let sql = format!(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE ws.is_terminal = false) as active_count,
                COUNT(*) FILTER (WHERE ws.is_terminal = true) as completed_count,
                COUNT(*) FILTER (WHERE p.deadline IS NOT NULL AND p.deadline < NOW() AND ws.is_terminal = false) as overdue_count,
                COUNT(*) as total_count
            FROM {}.projects p
            JOIN {}.workflow_statuses ws ON p.status_id = ws.id
            "#,
            schema, schema
        );

        let stats: (i64, i64, i64, i64) = sqlx::query_as(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(ProjectStatistics {
            active_count: stats.0 as i32,
            completed_count: stats.1 as i32,
            overdue_count: stats.2 as i32,
            total_count: stats.3 as i32,
        })
    }
}

/// Project statistics for dashboard
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectStatistics {
    pub active_count: i32,
    pub completed_count: i32,
    pub overdue_count: i32,
    pub total_count: i32,
}
