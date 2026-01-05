use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AchievementRow, Difficulty, EngineerRow, Incident, Project};

/// Game Engine service for XP, level-ups, achievements, and satisfaction
pub struct GameEngineService {
    pool: PgPool,
}

impl GameEngineService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Calculate XP reward for completing an incident
    pub fn calculate_incident_xp(severity: &str, difficulty: &str, time_bonus: bool) -> i64 {
        let base_xp = 50i64;

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

        let time_multiplier = if time_bonus { 1.25 } else { 1.0 };

        (base_xp as f64 * severity_multiplier * difficulty_multiplier * time_multiplier) as i64
    }

    /// Calculate XP reward for completing a project
    pub fn calculate_project_xp(difficulty: &str, under_budget: bool, on_time: bool) -> i64 {
        let base_xp = 100i64;

        let difficulty_multiplier = match difficulty {
            "extreme" => 5.0,
            "expert" => 3.0,
            "hard" => 2.0,
            "normal" => 1.0,
            "easy" => 0.5,
            _ => 1.0,
        };

        let budget_bonus = if under_budget { 1.2 } else { 1.0 };
        let time_bonus = if on_time { 1.15 } else { 1.0 };

        (base_xp as f64 * difficulty_multiplier * budget_bonus * time_bonus) as i64
    }

    /// Calculate satisfaction impact based on task difficulty vs engineer skill
    pub fn calculate_satisfaction_impact(task_difficulty: Difficulty, engineer_level: i32) -> i32 {
        // Convert level to approximate skill tier (1-5)
        let skill_tier = match engineer_level {
            1..=10 => 1,
            11..=25 => 2,
            26..=40 => 3,
            41..=60 => 4,
            _ => 5,
        };

        task_difficulty.satisfaction_impact(skill_tier)
    }

    /// Process task completion - awards XP, updates stats, checks achievements
    #[instrument(skip(self))]
    pub async fn process_incident_completion(
        &self,
        schema: &str,
        engineer_id: Uuid,
        incident: &Incident,
        resolution_time_minutes: i64,
    ) -> Result<CompletionReward, AppError> {
        // Calculate XP
        let time_bonus = resolution_time_minutes < 60; // Bonus for resolving under an hour
        let xp_earned = Self::calculate_incident_xp(
            &incident.severity.to_string(),
            &incident.difficulty.to_string(),
            time_bonus,
        );

        // Get engineer current state
        let engineer: EngineerRow =
            sqlx::query_as(&format!("SELECT * FROM {}.engineers WHERE id = $1", schema))
                .bind(engineer_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        // Calculate new XP and potential level up
        let new_xp = engineer.xp + xp_earned;
        let mut new_level = engineer.level;
        let mut remaining_xp = new_xp;
        let mut xp_to_next = engineer.xp_to_next_level;

        while remaining_xp >= xp_to_next && new_level < 100 {
            remaining_xp -= xp_to_next;
            new_level += 1;
            xp_to_next = crate::models::Engineer::xp_for_level(new_level + 1);
        }

        let leveled_up = new_level > engineer.level;

        // Calculate satisfaction change
        let satisfaction_delta =
            Self::calculate_satisfaction_impact(incident.difficulty, engineer.level);
        let new_satisfaction = (engineer.satisfaction + satisfaction_delta).clamp(0, 100);

        // Update engineer
        let _ = sqlx::query(&format!(
            r#"
            UPDATE {}.engineers SET
                xp = $1,
                level = $2,
                xp_to_next_level = $3,
                satisfaction = $4,
                resolved_incidents = resolved_incidents + 1,
                total_revenue = total_revenue + $5
            WHERE id = $6
            "#,
            schema
        ))
        .bind(remaining_xp)
        .bind(new_level)
        .bind(xp_to_next)
        .bind(new_satisfaction)
        .bind(incident.reward)
        .bind(engineer_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update engineer: {}", e)))?;

        // Check for achievements
        let achievements_unlocked = self
            .check_and_award_achievements(
                schema,
                engineer_id,
                engineer.resolved_incidents + 1,
                new_level,
            )
            .await?;

        Ok(CompletionReward {
            xp_earned,
            new_level: if leveled_up { Some(new_level) } else { None },
            satisfaction_change: satisfaction_delta,
            revenue_earned: incident.reward,
            achievements_unlocked,
        })
    }

    /// Process project completion
    #[instrument(skip(self))]
    pub async fn process_project_completion(
        &self,
        schema: &str,
        engineer_id: Uuid,
        project: &Project,
        under_budget: bool,
        on_time: bool,
    ) -> Result<CompletionReward, AppError> {
        let xp_earned =
            Self::calculate_project_xp(&project.difficulty.to_string(), under_budget, on_time);

        // Get engineer current state
        let engineer: EngineerRow =
            sqlx::query_as(&format!("SELECT * FROM {}.engineers WHERE id = $1", schema))
                .bind(engineer_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        // Calculate new XP and potential level up
        let new_xp = engineer.xp + xp_earned;
        let mut new_level = engineer.level;
        let mut remaining_xp = new_xp;
        let mut xp_to_next = engineer.xp_to_next_level;

        while remaining_xp >= xp_to_next && new_level < 100 {
            remaining_xp -= xp_to_next;
            new_level += 1;
            xp_to_next = crate::models::Engineer::xp_for_level(new_level + 1);
        }

        let leveled_up = new_level > engineer.level;

        // Calculate satisfaction change
        let satisfaction_delta =
            Self::calculate_satisfaction_impact(project.difficulty, engineer.level);
        let new_satisfaction = (engineer.satisfaction + satisfaction_delta).clamp(0, 100);

        // Update engineer
        let _ = sqlx::query(&format!(
            r#"
            UPDATE {}.engineers SET
                xp = $1,
                level = $2,
                xp_to_next_level = $3,
                satisfaction = $4,
                completed_projects = completed_projects + 1,
                total_revenue = total_revenue + $5
            WHERE id = $6
            "#,
            schema
        ))
        .bind(remaining_xp)
        .bind(new_level)
        .bind(xp_to_next)
        .bind(new_satisfaction)
        .bind(project.reward)
        .bind(engineer_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update engineer: {}", e)))?;

        // Check for achievements
        let achievements_unlocked = self
            .check_and_award_achievements(
                schema,
                engineer_id,
                engineer.resolved_incidents,
                new_level,
            )
            .await?;

        Ok(CompletionReward {
            xp_earned,
            new_level: if leveled_up { Some(new_level) } else { None },
            satisfaction_change: satisfaction_delta,
            revenue_earned: project.reward,
            achievements_unlocked,
        })
    }

    /// Check and award any earned achievements
    #[instrument(skip(self))]
    async fn check_and_award_achievements(
        &self,
        schema: &str,
        engineer_id: Uuid,
        incident_count: i32,
        level: i32,
    ) -> Result<Vec<AchievementUnlock>, AppError> {
        // Get engineer's current achievements
        let current_achievements: Vec<(Uuid,)> = sqlx::query_as(&format!(
            "SELECT achievement_id FROM {}.engineer_achievements WHERE engineer_id = $1",
            schema
        ))
        .bind(engineer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let _current_ids: Vec<Uuid> = current_achievements.into_iter().map(|a| a.0).collect();

        // Get all achievements not yet earned
        let achievements: Vec<AchievementRow> = sqlx::query_as(&format!(
            r#"
            SELECT * FROM {}.achievements
            WHERE id NOT IN (SELECT achievement_id FROM {}.engineer_achievements WHERE engineer_id = $1)
            "#,
            schema, schema
        ))
        .bind(engineer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut unlocked = Vec::new();

        for achievement in achievements {
            let earned = match achievement.condition_type.as_str() {
                "incident_count" => incident_count >= achievement.condition_value,
                "reach_level" => level >= achievement.condition_value,
                // Add more condition types as needed
                _ => false,
            };

            if earned {
                // Award achievement
                let _ = sqlx::query(&format!(
                    r#"
                    INSERT INTO {}.engineer_achievements (engineer_id, achievement_id, unlocked_at)
                    VALUES ($1, $2, NOW())
                    ON CONFLICT DO NOTHING
                    "#,
                    schema
                ))
                .bind(engineer_id)
                .bind(achievement.id)
                .execute(&self.pool)
                .await;

                // Award XP from achievement
                if achievement.xp_reward > 0 {
                    let _ = sqlx::query(&format!(
                        "UPDATE {}.engineers SET xp = xp + $1 WHERE id = $2",
                        schema
                    ))
                    .bind(achievement.xp_reward)
                    .bind(engineer_id)
                    .execute(&self.pool)
                    .await;
                }

                unlocked.push(AchievementUnlock {
                    achievement_id: achievement.id,
                    name: achievement.name,
                    description: achievement.description,
                    icon: achievement.icon,
                    xp_reward: achievement.xp_reward,
                });
            }
        }

        Ok(unlocked)
    }

    /// Process monthly satisfaction decay for all engineers
    #[instrument(skip(self))]
    pub async fn process_monthly_satisfaction_decay(&self, schema: &str) -> Result<i32, AppError> {
        // Engineers with low workload lose satisfaction (bored)
        // Engineers with very high workload also lose satisfaction (stressed)
        let result = sqlx::query(&format!(
            r#"
            UPDATE {}.engineers e SET
                satisfaction = GREATEST(0, satisfaction - 5)
            WHERE is_active = true
            AND (
                -- Low workload (less than 2 active tasks)
                (SELECT COUNT(*) FROM {}.assignments a
                 WHERE a.engineer_id = e.id) < 2
                OR
                -- Very high workload (more than 5 active tasks)
                (SELECT COUNT(*) FROM {}.assignments a
                 WHERE a.engineer_id = e.id) > 5
            )
            "#,
            schema, schema, schema
        ))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() as i32)
    }

    /// Get leaderboard data
    #[instrument(skip(self))]
    pub async fn get_leaderboard(
        &self,
        schema: &str,
        leaderboard_type: LeaderboardType,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, AppError> {
        let order_column = match leaderboard_type {
            LeaderboardType::Level => "e.level DESC, e.xp DESC",
            LeaderboardType::Revenue => "e.total_revenue DESC",
            LeaderboardType::Incidents => "e.resolved_incidents DESC",
            LeaderboardType::Projects => "e.completed_projects DESC",
        };

        let sql = format!(
            r#"
            SELECT e.id, u.email, e.level, e.xp, e.total_revenue,
                   e.resolved_incidents, e.completed_projects
            FROM {}.engineers e
            JOIN public.users u ON e.id = u.id
            WHERE e.is_active = true
            ORDER BY {}
            LIMIT $1
            "#,
            schema, order_column
        );

        let rows: Vec<(Uuid, String, i32, i64, i64, i32, i32)> = sqlx::query_as(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .enumerate()
            .map(
                |(idx, (id, email, level, xp, revenue, incidents, projects))| LeaderboardEntry {
                    rank: (idx + 1) as i32,
                    engineer_id: id,
                    email,
                    level,
                    xp,
                    total_revenue: revenue,
                    resolved_incidents: incidents,
                    completed_projects: projects,
                },
            )
            .collect())
    }
}

/// Result of completing a task
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletionReward {
    pub xp_earned: i64,
    pub new_level: Option<i32>,
    pub satisfaction_change: i32,
    pub revenue_earned: i64,
    pub achievements_unlocked: Vec<AchievementUnlock>,
}

/// Achievement unlock notification
#[derive(Debug, Clone, serde::Serialize)]
pub struct AchievementUnlock {
    pub achievement_id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub xp_reward: i64,
}

/// Leaderboard type
#[derive(Debug, Clone, Copy)]
pub enum LeaderboardType {
    Level,
    Revenue,
    Incidents,
    Projects,
}

/// Leaderboard entry
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub engineer_id: Uuid,
    pub email: String,
    pub level: i32,
    pub xp: i64,
    pub total_revenue: i64,
    pub resolved_incidents: i32,
    pub completed_projects: i32,
}
