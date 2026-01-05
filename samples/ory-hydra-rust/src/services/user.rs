use chrono::Utc;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{User, UserRole, UserRow, UserStatus};

/// Service for user management with PostgreSQL backend
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new user
    #[instrument(skip(self, password_hash))]
    pub async fn create(
        &self,
        email: &str,
        password_hash: Option<String>,
        role: UserRole,
        tenant_id: Option<Uuid>,
    ) -> Result<User, AppError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO public.users (id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(false) // email_verified
        .bind(&password_hash)
        .bind(role.to_string())
        .bind(tenant_id)
        .bind(UserStatus::Active.to_string())
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") || e.to_string().contains("unique constraint") {
                AppError::BadRequest("Email already registered".to_string())
            } else {
                AppError::Database(e.to_string())
            }
        })?;

        Ok(row.into())
    }

    /// Get user by ID
    #[instrument(skip(self))]
    pub async fn get_by_id(&self, id: Uuid) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
            FROM public.users
            WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)?;

        Ok(row.into())
    }

    /// Get user by email
    #[instrument(skip(self))]
    pub async fn get_by_email(&self, email: &str) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
            FROM public.users
            WHERE email = $1 AND status != 'deleted'
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)?;

        Ok(row.into())
    }

    /// Update last login timestamp
    #[instrument(skip(self))]
    pub async fn update_last_login(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE public.users
            SET last_login_at = $2, updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update user status
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn update_status(&self, id: Uuid, status: UserStatus) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE public.users
            SET status = $2, updated_at = $3
            WHERE id = $1
            RETURNING id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(id)
        .bind(status.to_string())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.into())
    }

    /// List users for a tenant (or all if tenant_id is None for platform admins)
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn list(
        &self,
        tenant_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, AppError> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, UserRow>(
                r#"
                SELECT id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
                FROM public.users
                WHERE tenant_id = $1 AND status != 'deleted'
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(tid)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UserRow>(
                r#"
                SELECT id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at, last_login_at
                FROM public.users
                WHERE status != 'deleted'
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Check if a user exists by email
    #[instrument(skip(self))]
    pub async fn exists(&self, email: &str) -> Result<bool, AppError> {
        let (exists,): (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM public.users WHERE email = $1 AND status != 'deleted')
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// Seed the demo user if it doesn't exist
    #[instrument(skip(self, password_hash))]
    pub async fn seed_demo_user(&self, password_hash: &str) -> Result<(), AppError> {
        let now = Utc::now();

        // Seed platform admin
        let exists = self.exists("demo@example.com").await?;
        if !exists {
            sqlx::query(
                r#"
                INSERT INTO public.users (id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $8)
                ON CONFLICT (email) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind("demo@example.com")
            .bind(true)
            .bind(password_hash)
            .bind(UserRole::PlatformAdmin.to_string())
            .bind(UserStatus::Active.to_string())
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
            tracing::info!("Demo user seeded: demo@example.com");
        }

        // Get the test tenant
        let tenant_result: Result<(Uuid, String), _> = sqlx::query_as(
            "SELECT id, schema_name FROM public.tenants WHERE slug = 'test-shop' LIMIT 1",
        )
        .fetch_one(&self.pool)
        .await;

        if let Ok((tenant_id, schema_name)) = tenant_result {
            // Seed tenant users with fixed UUIDs for consistent engineer creation
            let users_to_seed = [
                (
                    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                    "manager@example.com",
                    UserRole::Manager,
                ),
                (
                    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
                    "sato@example.com",
                    UserRole::Engineer,
                ),
                (
                    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
                    "tanaka@example.com",
                    UserRole::Engineer,
                ),
                (
                    Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
                    "suzuki@example.com",
                    UserRole::Engineer,
                ),
                (
                    Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap(),
                    "reporter@example.com",
                    UserRole::Reporter,
                ),
            ];

            for (_user_id, email, role) in users_to_seed {
                // Use UPSERT to ensure password hash is updated for existing users
                sqlx::query(
                    r#"
                    INSERT INTO public.users (id, email, email_verified, password_hash, role, tenant_id, status, created_at, updated_at)
                    VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (email) DO UPDATE SET
                        password_hash = EXCLUDED.password_hash,
                        role = EXCLUDED.role,
                        updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(email)
                .bind(true)
                .bind(password_hash)
                .bind(role.to_string())
                .bind(tenant_id)
                .bind(UserStatus::Active.to_string())
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await?;
                tracing::info!("Tenant user seeded/updated: {}", email);
            }

            // Seed engineers in tenant schema (lookup user IDs by email)
            let engineers_to_seed: Vec<(&str, i32, i64, i32, i64, i64, i32, i32)> = vec![
                ("sato@example.com", 5, 2500, 85, 50000, 150000, 8, 3),
                ("tanaka@example.com", 3, 800, 70, 40000, 80000, 4, 1),
                ("suzuki@example.com", 2, 300, 90, 35000, 40000, 2, 1),
            ];

            for (email, level, xp, satisfaction, salary, total_revenue, incidents, projects) in
                engineers_to_seed
            {
                // Lookup user ID by email
                let user_result: Result<(Uuid,), _> = sqlx::query_as(
                    "SELECT id FROM public.users WHERE email = $1 AND tenant_id = $2",
                )
                .bind(email)
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await;

                if let Ok((user_id,)) = user_result {
                    let sql = format!(
                        r#"
                        INSERT INTO {}.engineers (id, level, xp, xp_to_next_level, satisfaction, salary, total_revenue, completed_projects, resolved_incidents, is_active, hired_at)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true, NOW())
                        ON CONFLICT (id) DO NOTHING
                        "#,
                        schema_name
                    );
                    let xp_to_next = level * 1000;
                    sqlx::query(&sql)
                        .bind(user_id)
                        .bind(level)
                        .bind(xp)
                        .bind(xp_to_next)
                        .bind(satisfaction)
                        .bind(salary)
                        .bind(total_revenue)
                        .bind(projects)
                        .bind(incidents)
                        .execute(&self.pool)
                        .await?;
                    tracing::info!("Engineer seeded: {} ({})", email, user_id);
                }
            }
            tracing::info!("Engineers seeded in tenant schema: {}", schema_name);

            // Get specialty IDs and assign to engineers
            let specialties: Vec<(Uuid, String)> =
                sqlx::query_as(&format!("SELECT id, name FROM {}.specialties", schema_name))
                    .fetch_all(&self.pool)
                    .await?;

            if !specialties.is_empty() {
                // Assign specialties to engineers (using email lookup)
                let engineer_specialties: Vec<(&str, &str, &str)> = vec![
                    // sato: SRE (expert), Backend (intermediate)
                    ("sato@example.com", "SRE", "expert"),
                    ("sato@example.com", "Backend", "intermediate"),
                    // tanaka: Frontend (intermediate)
                    ("tanaka@example.com", "Frontend", "intermediate"),
                    // suzuki: Backend (beginner), Infrastructure (beginner)
                    ("suzuki@example.com", "Backend", "beginner"),
                    ("suzuki@example.com", "Infrastructure", "beginner"),
                ];

                for (email, specialty_name, proficiency) in engineer_specialties {
                    // Lookup user ID by email
                    let user_result: Result<(Uuid,), _> = sqlx::query_as(
                        "SELECT id FROM public.users WHERE email = $1 AND tenant_id = $2",
                    )
                    .bind(email)
                    .bind(tenant_id)
                    .fetch_one(&self.pool)
                    .await;

                    if let Ok((engineer_id,)) = user_result {
                        if let Some((specialty_id, _)) =
                            specialties.iter().find(|(_, name)| name == specialty_name)
                        {
                            let sql = format!(
                                r#"
                                INSERT INTO {}.engineer_specialties (engineer_id, specialty_id, proficiency)
                                VALUES ($1, $2, $3)
                                ON CONFLICT (engineer_id, specialty_id) DO NOTHING
                                "#,
                                schema_name
                            );
                            let _ = sqlx::query(&sql)
                                .bind(engineer_id)
                                .bind(specialty_id)
                                .bind(proficiency)
                                .execute(&self.pool)
                                .await;
                        }
                    }
                }
                tracing::info!("Engineer specialties seeded");
            }

            // Seed tenant finance
            let sql = format!(
                r#"
                INSERT INTO {}.tenant_finance (tenant_id, balance, monthly_revenue, monthly_expenses, revenue_target)
                VALUES ($1, 500000, 200000, 125000, 300000)
                ON CONFLICT (tenant_id) DO NOTHING
                "#,
                schema_name
            );
            let _ = sqlx::query(&sql).bind(tenant_id).execute(&self.pool).await;
            tracing::info!("Tenant finance seeded");
        }

        Ok(())
    }
}
