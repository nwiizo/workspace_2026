//! エラーハンドリングのデモ
//!
//! このデモでは以下を検証:
//! 1. カスタムエラー型の定義
//! 2. PostgreSQLエラーコードの分類
//! 3. リトライ可能なエラーの処理
//! 4. 適切なエラー伝播

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

// ================================
// カスタムエラー型
// ================================

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found: {0}")]
    NotFound(Uuid),

    #[error("Email already exists: {0}")]
    EmailExists(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    #[error("Foreign key violation")]
    ForeignKeyViolation,

    #[error("Not null violation: {0}")]
    NotNullViolation(String),

    #[error("Check constraint violation: {0}")]
    CheckViolation(String),

    #[error("Serialization failure, please retry")]
    SerializationFailure,

    #[error("Deadlock detected, please retry")]
    Deadlock,

    #[error("Connection lost")]
    ConnectionLost,

    #[error("Other database error: {0}")]
    Other(String),
}

// PostgreSQLエラーコード
mod pg_error_codes {
    pub const UNIQUE_VIOLATION: &str = "23505";
    pub const FOREIGN_KEY_VIOLATION: &str = "23503";
    pub const NOT_NULL_VIOLATION: &str = "23502";
    pub const CHECK_VIOLATION: &str = "23514";
    pub const SERIALIZATION_FAILURE: &str = "40001";
    pub const DEADLOCK_DETECTED: &str = "40P01";
}

fn classify_db_error(err: &sqlx::Error) -> DbError {
    if let sqlx::Error::Database(db_err) = err
        && let Some(code) = db_err.code()
    {
        return match code.as_ref() {
            pg_error_codes::UNIQUE_VIOLATION => {
                DbError::Duplicate(db_err.message().to_string())
            }
            pg_error_codes::FOREIGN_KEY_VIOLATION => DbError::ForeignKeyViolation,
            pg_error_codes::NOT_NULL_VIOLATION => {
                DbError::NotNullViolation(db_err.message().to_string())
            }
            pg_error_codes::CHECK_VIOLATION => {
                DbError::CheckViolation(db_err.message().to_string())
            }
            pg_error_codes::SERIALIZATION_FAILURE => DbError::SerializationFailure,
            pg_error_codes::DEADLOCK_DETECTED => DbError::Deadlock,
            _ => DbError::Other(db_err.message().to_string()),
        };
    }
    DbError::Other(err.to_string())
}

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct User {
    id: Uuid,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Account {
    id: Uuid,
    user_id: Uuid,
    balance: Decimal,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS error_accounts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS error_users CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE error_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE error_accounts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES error_users(id),
            balance DECIMAL(10,2) NOT NULL DEFAULT 0 CHECK (balance >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

// ================================
// Demo: 適切なエラー処理
// ================================

async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<User, UserError> {
    let result = sqlx::query_as(
        r#"
        INSERT INTO error_users (email, name)
        VALUES ($1, $2)
        RETURNING id, email, name, created_at
        "#,
    )
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await;

    match result {
        Ok(user) => Ok(user),
        Err(sqlx::Error::Database(db_err)) => {
            if let Some(code) = db_err.code()
                && code.as_ref() == pg_error_codes::UNIQUE_VIOLATION
            {
                return Err(UserError::EmailExists(email.to_string()));
            }
            Err(UserError::Database(sqlx::Error::Database(db_err)))
        }
        Err(e) => Err(UserError::Database(e)),
    }
}

async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, UserError> {
    sqlx::query_as("SELECT id, email, name, created_at FROM error_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(UserError::NotFound(user_id))
}

async fn demo_proper_error_handling(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Proper error handling ===");

    // 正常なユーザー作成
    let user = create_user(pool, "alice@example.com", "Alice").await?;
    println!("Created user: {} ({})", user.name, user.email);

    // 重複メールでの作成を試みる
    match create_user(pool, "alice@example.com", "Alice Clone").await {
        Ok(_) => println!("Unexpected success"),
        Err(UserError::EmailExists(email)) => {
            println!("Correctly caught: Email '{}' already exists", email);
        }
        Err(e) => println!("Unexpected error: {}", e),
    }

    // 存在しないユーザーの取得
    let fake_id = Uuid::new_v4();
    match get_user(pool, fake_id).await {
        Ok(_) => println!("Unexpected success"),
        Err(UserError::NotFound(id)) => {
            println!("Correctly caught: User {} not found", id);
        }
        Err(e) => println!("Unexpected error: {}", e),
    }

    Ok(())
}

// ================================
// Demo: 制約違反の検出
// ================================

async fn demo_constraint_violations(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Constraint violation detection ===");

    // 外部キー違反
    let fake_user_id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO error_accounts (user_id, balance) VALUES ($1, $2)",
    )
    .bind(fake_user_id)
    .bind(Decimal::new(10000, 2))
    .execute(pool)
    .await;

    match result {
        Err(e) => {
            let db_error = classify_db_error(&e);
            println!("Foreign key violation: {:?}", db_error);
        }
        Ok(_) => println!("Unexpected success"),
    }

    // ユーザーを作成してアカウントを作成
    let user = create_user(pool, "bob@example.com", "Bob").await?;

    sqlx::query("INSERT INTO error_accounts (user_id, balance) VALUES ($1, $2)")
        .bind(user.id)
        .bind(Decimal::new(10000, 2))
        .execute(pool)
        .await?;

    // CHECK制約違反（マイナス残高）
    let result = sqlx::query("UPDATE error_accounts SET balance = -100 WHERE user_id = $1")
        .bind(user.id)
        .execute(pool)
        .await;

    match result {
        Err(e) => {
            let db_error = classify_db_error(&e);
            println!("Check constraint violation: {:?}", db_error);
        }
        Ok(_) => println!("Unexpected success"),
    }

    Ok(())
}

// ================================
// Demo: リトライ可能なエラー
// ================================

async fn execute_with_retry<T, F, Fut>(
    max_retries: u32,
    mut operation: F,
) -> Result<T, DbError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let mut attempts = 0;

    loop {
        attempts += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let db_err = classify_db_error(&e);

                let should_retry = matches!(
                    db_err,
                    DbError::SerializationFailure | DbError::Deadlock | DbError::ConnectionLost
                );

                if should_retry && attempts < max_retries {
                    println!(
                        "  Retryable error (attempt {}/{}): {:?}",
                        attempts, max_retries, db_err
                    );
                    let delay = 10 * 2_u64.pow(attempts - 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    continue;
                }

                return Err(db_err);
            }
        }
    }
}

async fn demo_retry_logic(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Retry logic for transient errors ===");

    // 成功するクエリでリトライロジックをテスト
    let result = execute_with_retry(3, || async {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(pool)
            .await
    })
    .await;

    match result {
        Ok(v) => println!("Query succeeded: {}", v),
        Err(e) => println!("Query failed after retries: {:?}", e),
    }

    println!("\nRetry logic handles:");
    println!("  - Serialization failures (40001)");
    println!("  - Deadlocks (40P01)");
    println!("  - Connection losses");
    println!("  - Uses exponential backoff");

    Ok(())
}

// ================================
// Demo: アンチパターン
// ================================

async fn demo_anti_patterns(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Anti-patterns to avoid ===");

    println!("1. Using .unwrap() on database operations:");
    println!("   BAD:  let user = get_user(pool, id).await.unwrap();");
    println!("   GOOD: let user = get_user(pool, id).await?;");

    println!("\n2. Ignoring errors with let _ =:");
    println!("   BAD:  let _ = sqlx::query(\"DELETE...\").execute(pool).await;");
    println!("   GOOD: sqlx::query(\"DELETE...\").execute(pool).await?;");

    println!("\n3. Generic error messages:");
    println!("   BAD:  .map_err(|_| \"Error occurred\")");
    println!("   GOOD: Use typed errors with context");

    // 正しいエラー処理の例を実演
    let user_id = Uuid::new_v4();
    let result = get_user(pool, user_id).await;

    match result {
        Ok(user) => println!("Found user: {}", user.name),
        Err(UserError::NotFound(id)) => println!("Handled: User {} not found", id),
        Err(UserError::EmailExists(email)) => println!("Handled: Email {} exists", email),
        Err(UserError::Database(e)) => println!("Handled: Database error: {}", e),
    }

    Ok(())
}

// ================================
// メイン
// ================================

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/antipattern".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to PostgreSQL");

    setup_database(&pool).await?;

    demo_proper_error_handling(&pool).await?;
    demo_constraint_violations(&pool).await?;
    demo_retry_logic(&pool).await?;
    demo_anti_patterns(&pool).await?;

    println!("\n=== All error handling demos completed successfully! ===");
    Ok(())
}
