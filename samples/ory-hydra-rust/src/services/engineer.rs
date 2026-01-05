use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    Engineer, EngineerRow, EngineerWithSpecialties, Proficiency, SpecialtyWithProficiency, UserRole,
};

/// Engineer row with email from JOINed query
#[derive(Debug, sqlx::FromRow)]
struct EngineerWithEmailRow {
    pub id: Uuid,
    pub level: i32,
    pub xp: i64,
    pub xp_to_next_level: i64,
    pub satisfaction: i32,
    pub salary: i64,
    pub total_revenue: i64,
    pub completed_projects: i32,
    pub resolved_incidents: i32,
    pub is_active: bool,
    pub hired_at: DateTime<Utc>,
    pub fired_at: Option<DateTime<Utc>>,
    pub email: String,
}

/// Service for managing engineers
pub struct EngineerService {
    pool: PgPool,
}

impl EngineerService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new engineer record (when user is hired)
    #[instrument(skip(self))]
    pub async fn create(
        &self,
        schema: &str,
        user_id: Uuid,
        level: i32,
        salary: i64,
        satisfaction: i32,
    ) -> Result<Engineer, AppError> {
        let xp_to_next = Engineer::xp_for_level(level + 1);

        let sql = format!(
            r#"
            INSERT INTO {}.engineers
            (id, level, xp, xp_to_next_level, satisfaction, salary, total_revenue,
             completed_projects, resolved_incidents, is_active, hired_at)
            VALUES ($1, $2, 0, $3, $4, $5, 0, 0, 0, true, NOW())
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(user_id)
            .bind(level)
            .bind(xp_to_next)
            .bind(satisfaction)
            .bind(salary)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create engineer: {}", e)))?;

        Ok(row.into())
    }

    /// Get engineer by ID
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, schema: &str, id: Uuid) -> Result<Engineer, AppError> {
        let sql = format!("SELECT * FROM {}.engineers WHERE id = $1", schema);
        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Engineer not found".to_string()))?;

        Ok(row.into())
    }

    /// Get engineer with specialties (respects role-based proficiency visibility)
    #[instrument(skip(self))]
    pub async fn get_with_specialties(
        &self,
        schema: &str,
        id: Uuid,
        viewer_role: UserRole,
    ) -> Result<EngineerWithSpecialties, AppError> {
        let engineer = self.get_by_id(schema, id).await?;

        // Get user email
        let user: (String,) = sqlx::query_as("SELECT email FROM public.users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Get specialties
        let specialties = self.get_specialties(schema, id, viewer_role).await?;

        // Get active assignment count
        let active_assignments = self.get_active_assignment_count(schema, id).await?;

        let can_view_proficiency = viewer_role.can_view_proficiency();

        Ok(EngineerWithSpecialties {
            id: engineer.id,
            email: user.0,
            level: engineer.level,
            xp: engineer.xp,
            xp_to_next_level: engineer.xp_to_next_level,
            satisfaction: engineer.satisfaction,
            salary: if can_view_proficiency {
                Some(engineer.salary)
            } else {
                None
            },
            total_revenue: if can_view_proficiency {
                Some(engineer.total_revenue)
            } else {
                None
            },
            completed_projects: engineer.completed_projects,
            resolved_incidents: engineer.resolved_incidents,
            is_active: engineer.is_active,
            specialties,
            active_assignments,
        })
    }

    /// Get engineer's specialties
    #[instrument(skip(self))]
    async fn get_specialties(
        &self,
        schema: &str,
        engineer_id: Uuid,
        viewer_role: UserRole,
    ) -> Result<Vec<SpecialtyWithProficiency>, AppError> {
        let sql = format!(
            r#"
            SELECT s.id, s.name, s.description, s.color, es.proficiency
            FROM {}.engineer_specialties es
            JOIN {}.specialties s ON es.specialty_id = s.id
            WHERE es.engineer_id = $1
            "#,
            schema, schema
        );

        let rows: Vec<(Uuid, String, Option<String>, String, String)> = sqlx::query_as(&sql)
            .bind(engineer_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let can_view_proficiency = viewer_role.can_view_proficiency();

        Ok(rows
            .into_iter()
            .map(
                |(id, name, description, color, proficiency)| SpecialtyWithProficiency {
                    id,
                    name,
                    description,
                    color,
                    proficiency: if can_view_proficiency {
                        Some(proficiency.parse().unwrap_or(Proficiency::Intermediate))
                    } else {
                        None
                    },
                },
            )
            .collect())
    }

    /// Get active assignment count for an engineer
    async fn get_active_assignment_count(
        &self,
        schema: &str,
        engineer_id: Uuid,
    ) -> Result<i32, AppError> {
        let sql = format!(
            r#"
            SELECT COUNT(*) as count FROM (
                SELECT 1 FROM {}.assignments a
                JOIN {}.incidents i ON a.assignable_type = 'incident' AND a.assignable_id = i.id
                JOIN {}.workflow_statuses ws ON i.status_id = ws.id
                WHERE a.engineer_id = $1 AND ws.is_terminal = false
                UNION ALL
                SELECT 1 FROM {}.assignments a
                JOIN {}.projects p ON a.assignable_type = 'project' AND a.assignable_id = p.id
                JOIN {}.workflow_statuses ws ON p.status_id = ws.id
                WHERE a.engineer_id = $1 AND ws.is_terminal = false
            ) sub
            "#,
            schema, schema, schema, schema, schema, schema
        );

        let count: (i64,) = sqlx::query_as(&sql)
            .bind(engineer_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count.0 as i32)
    }

    /// List all active engineers
    #[instrument(skip(self))]
    pub async fn list_active(
        &self,
        schema: &str,
        viewer_role: UserRole,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EngineerWithSpecialties>, AppError> {
        let sql = format!(
            r#"
            SELECT e.id, e.level, e.xp, e.xp_to_next_level, e.satisfaction, e.salary,
                   e.total_revenue, e.completed_projects, e.resolved_incidents, e.is_active,
                   e.hired_at, e.fired_at, u.email
            FROM {}.engineers e
            JOIN public.users u ON e.id = u.id
            WHERE e.is_active = true
            ORDER BY e.level DESC, e.xp DESC
            LIMIT $1 OFFSET $2
            "#,
            schema
        );

        let rows: Vec<EngineerWithEmailRow> = sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let specialties = self.get_specialties(schema, row.id, viewer_role).await?;
            let active_assignments = self.get_active_assignment_count(schema, row.id).await?;
            let can_view = viewer_role.can_view_proficiency();

            result.push(EngineerWithSpecialties {
                id: row.id,
                email: row.email,
                level: row.level,
                xp: row.xp,
                xp_to_next_level: row.xp_to_next_level,
                satisfaction: row.satisfaction,
                salary: if can_view { Some(row.salary) } else { None },
                total_revenue: if can_view {
                    Some(row.total_revenue)
                } else {
                    None
                },
                completed_projects: row.completed_projects,
                resolved_incidents: row.resolved_incidents,
                is_active: row.is_active,
                specialties,
                active_assignments,
            });
        }

        Ok(result)
    }

    /// Add XP to an engineer
    #[instrument(skip(self))]
    pub async fn add_xp(
        &self,
        schema: &str,
        id: Uuid,
        xp_amount: i64,
    ) -> Result<Engineer, AppError> {
        let mut engineer = self.get_by_id(schema, id).await?;
        engineer.xp += xp_amount;

        // Check for level up
        while engineer.can_level_up() {
            engineer.level_up();
        }

        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                xp = $1,
                level = $2,
                xp_to_next_level = $3
            WHERE id = $4
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(engineer.xp)
            .bind(engineer.level)
            .bind(engineer.xp_to_next_level)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update XP: {}", e)))?;

        Ok(row.into())
    }

    /// Update engineer satisfaction
    #[instrument(skip(self))]
    pub async fn update_satisfaction(
        &self,
        schema: &str,
        id: Uuid,
        delta: i32,
    ) -> Result<Engineer, AppError> {
        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                satisfaction = GREATEST(0, LEAST(100, satisfaction + $1))
            WHERE id = $2
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(delta)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update satisfaction: {}", e)))?;

        Ok(row.into())
    }

    /// Increment resolved incidents counter
    #[instrument(skip(self))]
    pub async fn increment_resolved_incidents(
        &self,
        schema: &str,
        id: Uuid,
    ) -> Result<Engineer, AppError> {
        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                resolved_incidents = resolved_incidents + 1
            WHERE id = $1
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update: {}", e)))?;

        Ok(row.into())
    }

    /// Increment completed projects counter
    #[instrument(skip(self))]
    pub async fn increment_completed_projects(
        &self,
        schema: &str,
        id: Uuid,
    ) -> Result<Engineer, AppError> {
        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                completed_projects = completed_projects + 1
            WHERE id = $1
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update: {}", e)))?;

        Ok(row.into())
    }

    /// Add revenue to engineer's total
    #[instrument(skip(self))]
    pub async fn add_revenue(
        &self,
        schema: &str,
        id: Uuid,
        amount: i64,
    ) -> Result<Engineer, AppError> {
        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                total_revenue = total_revenue + $1
            WHERE id = $2
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(amount)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update revenue: {}", e)))?;

        Ok(row.into())
    }

    /// Fire an engineer
    #[instrument(skip(self))]
    pub async fn fire(&self, schema: &str, id: Uuid) -> Result<Engineer, AppError> {
        let now = Utc::now();

        let sql = format!(
            r#"
            UPDATE {}.engineers SET
                is_active = false,
                fired_at = $1
            WHERE id = $2
            RETURNING *
            "#,
            schema
        );

        let row: EngineerRow = sqlx::query_as(&sql)
            .bind(now)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fire engineer: {}", e)))?;

        Ok(row.into())
    }

    /// Add specialty to engineer
    #[instrument(skip(self))]
    pub async fn add_specialty(
        &self,
        schema: &str,
        engineer_id: Uuid,
        specialty_id: Uuid,
        proficiency: Proficiency,
    ) -> Result<(), AppError> {
        let sql = format!(
            r#"
            INSERT INTO {}.engineer_specialties (engineer_id, specialty_id, proficiency)
            VALUES ($1, $2, $3)
            ON CONFLICT (engineer_id, specialty_id) DO UPDATE SET proficiency = $3
            "#,
            schema
        );

        sqlx::query(&sql)
            .bind(engineer_id)
            .bind(specialty_id)
            .bind(proficiency.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to add specialty: {}", e)))?;

        Ok(())
    }

    /// Get total salary expense for active engineers
    #[instrument(skip(self))]
    pub async fn get_total_salary(&self, schema: &str) -> Result<i64, AppError> {
        let sql = format!(
            "SELECT COALESCE(SUM(salary), 0) FROM {}.engineers WHERE is_active = true",
            schema
        );

        let total: (i64,) = sqlx::query_as(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(total.0)
    }
}
