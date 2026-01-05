use chrono::{DateTime, Duration, Utc};
use rand::prelude::*;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    CANDIDATE_TRAITS, Candidate, CandidateRarity, CandidateRow, CandidateStatus,
    CandidateWithDetails, FIRST_NAMES, HireCandidateRequest, HireResult, LAST_NAMES, Proficiency,
};

/// Candidate row with specialty details from JOINed query
#[derive(Debug, sqlx::FromRow)]
struct CandidateWithSpecialtiesRow {
    pub id: Uuid,
    pub name: String,
    pub avatar: String,
    pub rarity: String,
    pub level: i32,
    pub primary_specialty_id: Uuid,
    pub primary_proficiency: String,
    pub secondary_specialty_id: Option<Uuid>,
    pub secondary_proficiency: Option<String>,
    pub expected_salary: i64,
    pub hiring_cost: i64,
    pub base_satisfaction: i32,
    pub trait_name: Option<String>,
    pub trait_description: Option<String>,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub primary_specialty_name: String,
    pub primary_specialty_color: String,
    pub secondary_specialty_name: Option<String>,
    pub secondary_specialty_color: Option<String>,
}

/// Service for managing the recruitment/hiring system
pub struct RecruitmentService {
    pool: PgPool,
}

impl RecruitmentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get available candidates in the pool
    #[instrument(skip(self))]
    pub async fn list_available(
        &self,
        schema: &str,
        tenant_balance: i64,
    ) -> Result<Vec<CandidateWithDetails>, AppError> {
        let sql = format!(
            r#"
            SELECT c.id, c.name, c.avatar, c.rarity, c.level,
                   c.primary_specialty_id, c.primary_proficiency,
                   c.secondary_specialty_id, c.secondary_proficiency,
                   c.expected_salary, c.hiring_cost, c.base_satisfaction,
                   c.trait_name, c.trait_description, c.status, c.expires_at, c.created_at,
                   s1.name as primary_specialty_name, s1.color as primary_specialty_color,
                   s2.name as secondary_specialty_name, s2.color as secondary_specialty_color
            FROM {}.candidates c
            JOIN {}.specialties s1 ON c.primary_specialty_id = s1.id
            LEFT JOIN {}.specialties s2 ON c.secondary_specialty_id = s2.id
            WHERE c.status = 'available'
              AND (c.expires_at IS NULL OR c.expires_at > NOW())
            ORDER BY
                CASE c.rarity
                    WHEN 'legendary' THEN 1
                    WHEN 'epic' THEN 2
                    WHEN 'rare' THEN 3
                    WHEN 'uncommon' THEN 4
                    ELSE 5
                END,
                c.level DESC
            "#,
            schema, schema, schema
        );

        let rows: Vec<CandidateWithSpecialtiesRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let candidate = Candidate {
                    id: row.id,
                    name: row.name,
                    avatar: row.avatar,
                    rarity: row.rarity.parse().unwrap_or(CandidateRarity::Common),
                    level: row.level,
                    primary_specialty_id: row.primary_specialty_id,
                    primary_proficiency: row.primary_proficiency,
                    secondary_specialty_id: row.secondary_specialty_id,
                    secondary_proficiency: row.secondary_proficiency,
                    expected_salary: row.expected_salary,
                    hiring_cost: row.hiring_cost,
                    base_satisfaction: row.base_satisfaction,
                    trait_name: row.trait_name,
                    trait_description: row.trait_description,
                    status: row.status.parse().unwrap_or(CandidateStatus::Available),
                    expires_at: row.expires_at,
                    created_at: row.created_at,
                };
                CandidateWithDetails {
                    can_afford: tenant_balance >= candidate.hiring_cost,
                    primary_specialty_name: row.primary_specialty_name,
                    primary_specialty_color: row.primary_specialty_color,
                    secondary_specialty_name: row.secondary_specialty_name,
                    secondary_specialty_color: row.secondary_specialty_color,
                    candidate,
                }
            })
            .collect())
    }

    /// Get a specific candidate
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, schema: &str, id: Uuid) -> Result<Candidate, AppError> {
        let sql = format!("SELECT * FROM {}.candidates WHERE id = $1", schema);
        let row: CandidateRow = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or(AppError::NotFound("Candidate not found".to_string()))?;

        Ok(row.into())
    }

    /// Refresh the candidate pool with new candidates
    #[instrument(skip(self))]
    pub async fn refresh_pool(&self, schema: &str, count: i32) -> Result<Vec<Candidate>, AppError> {
        // Mark old available candidates as unavailable
        let _ = sqlx::query(&format!(
            "UPDATE {}.candidates SET status = 'unavailable' WHERE status = 'available'",
            schema
        ))
        .execute(&self.pool)
        .await;

        // Get available specialties
        let specialties: Vec<(Uuid, String)> =
            sqlx::query_as(&format!("SELECT id, name FROM {}.specialties", schema))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        if specialties.is_empty() {
            return Err(AppError::BadRequest("No specialties available".to_string()));
        }

        let mut candidates = Vec::new();

        for _ in 0..count {
            let candidate = self.generate_candidate(schema, &specialties).await?;
            candidates.push(candidate);
        }

        // Update last refresh time
        let _ = sqlx::query(&format!(
            "UPDATE {}.recruitment_settings SET last_free_refresh_at = NOW(), updated_at = NOW()",
            schema
        ))
        .execute(&self.pool)
        .await;

        Ok(candidates)
    }

    /// Generate a random candidate
    async fn generate_candidate(
        &self,
        schema: &str,
        specialties: &[(Uuid, String)],
    ) -> Result<Candidate, AppError> {
        // Generate all random values upfront before any await
        // to ensure the future is Send
        let candidate_data = {
            let mut rng = rand::thread_rng();

            // Determine rarity (weighted random)
            let rarity = self.roll_rarity(&mut rng);
            let (min_level, max_level) = rarity.level_range();
            let level = rng.gen_range(min_level..=max_level);

            // Generate name
            let first_name = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())];
            let last_name = LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())];
            let name = format!("{} {}", first_name, last_name);

            // Random specialties
            let primary_idx = rng.gen_range(0..specialties.len());
            let primary_specialty_id = specialties[primary_idx].0;
            let primary_proficiency = self.roll_proficiency(&mut rng, &rarity);

            let (secondary_specialty_id, secondary_proficiency) = if rng.gen_bool(0.4) {
                let mut secondary_idx = rng.gen_range(0..specialties.len());
                while secondary_idx == primary_idx && specialties.len() > 1 {
                    secondary_idx = rng.gen_range(0..specialties.len());
                }
                (
                    Some(specialties[secondary_idx].0),
                    Some(self.roll_proficiency(&mut rng, &rarity)),
                )
            } else {
                (None, None)
            };

            // Calculate costs based on rarity and level
            let base_salary = 40000 + (level as i64 * 2000);
            let expected_salary = (base_salary as f64 * rarity.salary_multiplier()) as i64;
            let hiring_cost = (10000.0 * rarity.cost_multiplier() + (level as f64 * 500.0)) as i64;
            let base_satisfaction = match rarity {
                CandidateRarity::Common => rng.gen_range(60..80),
                CandidateRarity::Uncommon => rng.gen_range(65..85),
                CandidateRarity::Rare => rng.gen_range(70..90),
                CandidateRarity::Epic => rng.gen_range(75..95),
                CandidateRarity::Legendary => rng.gen_range(85..100),
            };

            // Roll for trait (higher chance for higher rarity)
            let trait_chance = match rarity {
                CandidateRarity::Common => 0.1,
                CandidateRarity::Uncommon => 0.2,
                CandidateRarity::Rare => 0.4,
                CandidateRarity::Epic => 0.7,
                CandidateRarity::Legendary => 1.0,
            };

            let (trait_name, trait_description) = if rng.gen_bool(trait_chance) {
                let trait_idx = rng.gen_range(0..CANDIDATE_TRAITS.len());
                let (n, desc) = CANDIDATE_TRAITS[trait_idx];
                (Some(n.to_string()), Some(desc.to_string()))
            } else {
                (None, None)
            };

            // Expires in 24-72 hours
            let expires_hours = rng.gen_range(24..72);
            let expires_at = Utc::now() + Duration::hours(expires_hours);

            let id = Uuid::new_v4();
            let avatar = format!("avatar_{}", rng.gen_range(1..=20));

            // Return all generated data
            (
                id,
                name,
                avatar,
                rarity,
                level,
                primary_specialty_id,
                primary_proficiency,
                secondary_specialty_id,
                secondary_proficiency,
                expected_salary,
                hiring_cost,
                base_satisfaction,
                trait_name,
                trait_description,
                expires_at,
            )
        };

        // Destructure the data
        let (
            id,
            name,
            avatar,
            rarity,
            level,
            primary_specialty_id,
            primary_proficiency,
            secondary_specialty_id,
            secondary_proficiency,
            expected_salary,
            hiring_cost,
            base_satisfaction,
            trait_name,
            trait_description,
            expires_at,
        ) = candidate_data;

        let sql = format!(
            r#"
            INSERT INTO {}.candidates
            (id, name, avatar, rarity, level, primary_specialty_id, primary_proficiency,
             secondary_specialty_id, secondary_proficiency, expected_salary, hiring_cost,
             base_satisfaction, trait_name, trait_description, status, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'available', $15)
            RETURNING *
            "#,
            schema
        );

        let row: CandidateRow = sqlx::query_as(&sql)
            .bind(id)
            .bind(&name)
            .bind(&avatar)
            .bind(rarity.to_string())
            .bind(level)
            .bind(primary_specialty_id)
            .bind(primary_proficiency.to_string())
            .bind(secondary_specialty_id)
            .bind(secondary_proficiency.map(|p| p.to_string()))
            .bind(expected_salary)
            .bind(hiring_cost)
            .bind(base_satisfaction)
            .bind(&trait_name)
            .bind(&trait_description)
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create candidate: {}", e)))?;

        Ok(row.into())
    }

    /// Roll for rarity with weighted probabilities
    fn roll_rarity(&self, rng: &mut impl Rng) -> CandidateRarity {
        let roll: f64 = rng.gen_range(0.0..1.0);
        if roll < 0.01 {
            CandidateRarity::Legendary // 1%
        } else if roll < 0.05 {
            CandidateRarity::Epic // 4%
        } else if roll < 0.15 {
            CandidateRarity::Rare // 10%
        } else if roll < 0.40 {
            CandidateRarity::Uncommon // 25%
        } else {
            CandidateRarity::Common // 60%
        }
    }

    /// Roll for proficiency based on rarity
    fn roll_proficiency(&self, rng: &mut impl Rng, rarity: &CandidateRarity) -> Proficiency {
        let roll: f64 = rng.gen_range(0.0..1.0);
        match rarity {
            CandidateRarity::Common => {
                if roll < 0.7 {
                    Proficiency::Beginner
                } else {
                    Proficiency::Intermediate
                }
            }
            CandidateRarity::Uncommon => {
                if roll < 0.4 {
                    Proficiency::Beginner
                } else if roll < 0.9 {
                    Proficiency::Intermediate
                } else {
                    Proficiency::Expert
                }
            }
            CandidateRarity::Rare => {
                if roll < 0.2 {
                    Proficiency::Beginner
                } else if roll < 0.7 {
                    Proficiency::Intermediate
                } else {
                    Proficiency::Expert
                }
            }
            CandidateRarity::Epic => {
                if roll < 0.3 {
                    Proficiency::Intermediate
                } else {
                    Proficiency::Expert
                }
            }
            CandidateRarity::Legendary => Proficiency::Expert,
        }
    }

    /// Hire a candidate (creates engineer record and user)
    #[instrument(skip(self))]
    pub async fn hire_candidate(
        &self,
        schema: &str,
        request: HireCandidateRequest,
        recruiter_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<HireResult, AppError> {
        let candidate = self.get_by_id(schema, request.candidate_id).await?;

        if candidate.status != CandidateStatus::Available {
            return Err(AppError::BadRequest(
                "Candidate is not available for hiring".to_string(),
            ));
        }

        let salary = request
            .negotiated_salary
            .unwrap_or(candidate.expected_salary);

        // Create user account for the engineer
        let email = request.email.unwrap_or_else(|| {
            format!(
                "{}@tenant-{}.donadona.local",
                candidate.name.to_lowercase().replace(' ', "."),
                &tenant_id.to_string()[..8]
            )
        });

        let user_id = Uuid::new_v4();

        // Create user in public.users
        let _ = sqlx::query(
            r#"
            INSERT INTO public.users (id, email, email_verified, role, tenant_id, status)
            VALUES ($1, $2, true, 'engineer', $3, 'active')
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create user: {}", e)))?;

        // Create engineer record
        let sql = format!(
            r#"
            INSERT INTO {}.engineers
            (id, level, xp, xp_to_next_level, satisfaction, salary, total_revenue,
             completed_projects, resolved_incidents, is_active, hired_at)
            VALUES ($1, $2, 0, $3, $4, $5, 0, 0, 0, true, NOW())
            "#,
            schema
        );

        let xp_to_next = crate::models::Engineer::xp_for_level(candidate.level + 1);

        sqlx::query(&sql)
            .bind(user_id)
            .bind(candidate.level)
            .bind(xp_to_next)
            .bind(candidate.base_satisfaction)
            .bind(salary)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create engineer: {}", e)))?;

        // Add primary specialty
        let _ = sqlx::query(&format!(
            r#"
            INSERT INTO {}.engineer_specialties (engineer_id, specialty_id, proficiency)
            VALUES ($1, $2, $3)
            "#,
            schema
        ))
        .bind(user_id)
        .bind(candidate.primary_specialty_id)
        .bind(&candidate.primary_proficiency)
        .execute(&self.pool)
        .await;

        // Add secondary specialty if exists
        if let Some(secondary_id) = candidate.secondary_specialty_id {
            let _ = sqlx::query(&format!(
                r#"
                INSERT INTO {}.engineer_specialties (engineer_id, specialty_id, proficiency)
                VALUES ($1, $2, $3)
                "#,
                schema
            ))
            .bind(user_id)
            .bind(secondary_id)
            .bind(
                candidate
                    .secondary_proficiency
                    .as_deref()
                    .unwrap_or("intermediate"),
            )
            .execute(&self.pool)
            .await;
        }

        // Update candidate status to hired
        let _ = sqlx::query(&format!(
            "UPDATE {}.candidates SET status = 'hired' WHERE id = $1",
            schema
        ))
        .bind(request.candidate_id)
        .execute(&self.pool)
        .await;

        // Log recruitment event
        let _ = sqlx::query(&format!(
            r#"
            INSERT INTO {}.recruitment_events (id, candidate_id, recruiter_id, event_type, notes)
            VALUES ($1, $2, $3, 'offer_accepted', 'Hired successfully')
            "#,
            schema
        ))
        .bind(Uuid::new_v4())
        .bind(request.candidate_id)
        .bind(recruiter_id)
        .execute(&self.pool)
        .await;

        // Deduct hiring cost from tenant balance
        let new_balance: (i64,) = sqlx::query_as(&format!(
            r#"
            UPDATE {}.tenant_finance
            SET balance = balance - $1, monthly_expenses = monthly_expenses + $1, updated_at = NOW()
            RETURNING balance
            "#,
            schema
        ))
        .bind(candidate.hiring_cost)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update finance: {}", e)))?;

        // Record transaction
        let _ = sqlx::query(&format!(
            r#"
            INSERT INTO {}.transactions (id, tenant_id, transaction_type, amount, description, engineer_id)
            VALUES ($1, $2, 'hiring_bonus', $3, $4, $5)
            "#,
            schema
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(-candidate.hiring_cost)
        .bind(format!("Hired {}", candidate.name))
        .bind(user_id)
        .execute(&self.pool)
        .await;

        Ok(HireResult {
            engineer_id: user_id,
            candidate_id: request.candidate_id,
            hiring_cost: candidate.hiring_cost,
            monthly_salary: salary,
            new_balance: new_balance.0,
        })
    }

    /// Check if free refresh is available
    #[instrument(skip(self))]
    pub async fn can_free_refresh(&self, schema: &str) -> Result<bool, AppError> {
        let sql = format!(
            r#"
            SELECT last_free_refresh_at, free_refresh_interval_hours
            FROM {}.recruitment_settings
            LIMIT 1
            "#,
            schema
        );

        let result: Option<(Option<chrono::DateTime<Utc>>, i32)> = sqlx::query_as(&sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match result {
            Some((Some(last_refresh), interval_hours)) => {
                let next_free = last_refresh + Duration::hours(interval_hours as i64);
                Ok(Utc::now() >= next_free)
            }
            Some((None, _)) => Ok(true), // Never refreshed before
            None => Ok(true),            // No settings, allow
        }
    }
}
