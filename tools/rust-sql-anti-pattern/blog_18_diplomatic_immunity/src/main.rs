//! マイグレーション管理とスキーマテストのデモ
//!
//! このデモでは以下を検証:
//! 1. マイグレーションの追跡
//! 2. 制約のテスト
//! 3. スキーマ検証
//! 4. 破壊的変更の段階的実施

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Migration {
    id: i32,
    name: String,
    applied_at: DateTime<Utc>,
}

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
struct TableInfo {
    column_name: String,
    data_type: String,
    is_nullable: String,
}

// ================================
// マイグレーション追跡システム
// ================================

async fn setup_migration_tracking(pool: &PgPool) -> Result<()> {
    println!("=== Setting up migration tracking ===");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL UNIQUE,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Migration tracking table ready");
    Ok(())
}

async fn apply_migration(pool: &PgPool, name: &str, sql: &str) -> Result<bool> {
    // 既に適用済みかチェック
    let applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    if applied {
        return Ok(false);
    }

    // マイグレーションを実行
    let mut tx = pool.begin().await?;

    sqlx::query(sql).execute(&mut *tx).await?;

    sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
        .bind(name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(true)
}

// ================================
// Demo: マイグレーションの追跡
// ================================

async fn demo_migration_tracking(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Migration Tracking ===");

    // テストテーブルを削除してからマイグレーション
    sqlx::query("DROP TABLE IF EXISTS diplo_posts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS diplo_users CASCADE")
        .execute(pool)
        .await?;

    // マイグレーション1: ユーザーテーブル作成
    let migration1 = r#"
        CREATE TABLE diplo_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#;

    if apply_migration(pool, "20241201_create_users", migration1).await? {
        println!("Applied: 20241201_create_users");
    } else {
        println!("Skipped: 20241201_create_users (already applied)");
    }

    // マイグレーション2: 投稿テーブル作成
    let migration2 = r#"
        CREATE TABLE diplo_posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES diplo_users(id) ON DELETE CASCADE,
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#;

    if apply_migration(pool, "20241202_create_posts", migration2).await? {
        println!("Applied: 20241202_create_posts");
    } else {
        println!("Skipped: 20241202_create_posts (already applied)");
    }

    // 適用済みマイグレーション一覧
    let migrations: Vec<Migration> =
        sqlx::query_as("SELECT id, name, applied_at FROM _migrations ORDER BY id")
            .fetch_all(pool)
            .await?;

    println!("\nApplied migrations:");
    for m in &migrations {
        println!("  {} - {} ({})", m.id, m.name, m.applied_at);
    }

    Ok(())
}

// ================================
// Demo: 制約のテスト
// ================================

async fn demo_constraint_testing(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Constraint Testing ===");

    // サンプルユーザーを作成
    sqlx::query("INSERT INTO diplo_users (email, name) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("Test User")
        .execute(pool)
        .await?;

    // UNIQUE制約のテスト
    println!("\nTesting UNIQUE constraint on email:");
    let result = sqlx::query("INSERT INTO diplo_users (email, name) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("Another User")
        .execute(pool)
        .await;

    match result {
        Ok(_) => println!("  FAIL: Duplicate email was allowed"),
        Err(e) => {
            if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                println!("  PASS: Duplicate email correctly rejected");
            } else {
                println!("  Unexpected error: {}", e);
            }
        }
    }

    // 外部キー制約のテスト
    println!("\nTesting FOREIGN KEY constraint:");
    let fake_user_id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO diplo_posts (user_id, title, content) VALUES ($1, $2, $3)",
    )
    .bind(fake_user_id)
    .bind("Test Post")
    .bind("Content")
    .execute(pool)
    .await;

    match result {
        Ok(_) => println!("  FAIL: Non-existent user_id was allowed"),
        Err(e) => {
            if e.to_string().contains("foreign key")
                || e.to_string().contains("violates")
            {
                println!("  PASS: Non-existent user_id correctly rejected");
            } else {
                println!("  Unexpected error: {}", e);
            }
        }
    }

    // CHECK制約のテスト
    println!("\nTesting CHECK constraint on status:");
    let user_id: Uuid =
        sqlx::query_scalar("SELECT id FROM diplo_users WHERE email = $1")
            .bind("test@example.com")
            .fetch_one(pool)
            .await?;

    let result = sqlx::query(
        "INSERT INTO diplo_posts (user_id, title, content, status) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind("Test Post")
    .bind("Content")
    .bind("invalid_status")
    .execute(pool)
    .await;

    match result {
        Ok(_) => println!("  FAIL: Invalid status was allowed"),
        Err(e) => {
            if e.to_string().contains("check") || e.to_string().contains("violates") {
                println!("  PASS: Invalid status correctly rejected");
            } else {
                println!("  Unexpected error: {}", e);
            }
        }
    }

    Ok(())
}

// ================================
// Demo: スキーマ検証
// ================================

async fn demo_schema_verification(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Schema Verification ===");

    // テーブル構造を確認
    let columns: Vec<TableInfo> = sqlx::query_as(
        r#"
        SELECT
            column_name::TEXT,
            data_type::TEXT,
            is_nullable::TEXT
        FROM information_schema.columns
        WHERE table_name = 'diplo_users'
        ORDER BY ordinal_position
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("diplo_users table structure:");
    for col in &columns {
        println!(
            "  {} - {} (nullable: {})",
            col.column_name, col.data_type, col.is_nullable
        );
    }

    // 制約を確認
    let constraints: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT
            conname::TEXT,
            contype::TEXT
        FROM pg_constraint c
        JOIN pg_class t ON c.conrelid = t.oid
        WHERE t.relname = 'diplo_users'
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\nConstraints:");
    for (name, contype) in &constraints {
        let type_name = match contype.as_str() {
            "p" => "PRIMARY KEY",
            "u" => "UNIQUE",
            "f" => "FOREIGN KEY",
            "c" => "CHECK",
            _ => contype,
        };
        println!("  {} ({})", name, type_name);
    }

    // インデックスを確認
    let indexes: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT indexname::TEXT
        FROM pg_indexes
        WHERE tablename = 'diplo_users'
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\nIndexes:");
    for (name,) in &indexes {
        println!("  {}", name);
    }

    Ok(())
}

// ================================
// Demo: 段階的なスキーマ変更
// ================================

async fn demo_gradual_schema_change(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Gradual Schema Change ===");

    println!("Safe column rename process:");
    println!("  Phase 1: Add new column");
    println!("  Phase 2: Copy data & keep both columns in sync");
    println!("  Phase 3: Switch application to new column");
    println!("  Phase 4: Remove old column");

    // Phase 1: 新しいカラムを追加
    let phase1 = "ALTER TABLE diplo_users ADD COLUMN IF NOT EXISTS display_name VARCHAR(100)";
    sqlx::query(phase1).execute(pool).await?;
    println!("\nPhase 1 complete: Added display_name column");

    // Phase 2: データをコピー
    sqlx::query("UPDATE diplo_users SET display_name = name WHERE display_name IS NULL")
        .execute(pool)
        .await?;
    println!("Phase 2 complete: Copied data from name to display_name");

    // 両方のカラムを確認
    let sample: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT name, display_name FROM diplo_users LIMIT 1")
            .fetch_optional(pool)
            .await?;

    if let Some((name, display_name)) = sample {
        println!(
            "\nData verification: name='{}', display_name='{:?}'",
            name, display_name
        );
    }

    println!("\nPhase 3: Application should now read/write display_name");
    println!("Phase 4: After confirming, run: ALTER TABLE diplo_users DROP COLUMN name");

    Ok(())
}

// ================================
// Demo: アンチパターン
// ================================

async fn demo_anti_patterns(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Anti-patterns to avoid ===");

    println!("1. No version control for SQL:");
    println!("   - Store all migrations in version control");
    println!("   - Use descriptive naming: YYYYMMDD_description.sql");

    println!("\n2. No code review for schema changes:");
    println!("   - Apply same review process as application code");
    println!("   - Check for index usage, foreign keys, constraints");

    println!("\n3. No testing of constraints:");
    println!("   - Write tests that verify constraints work");
    println!("   - Test both valid and invalid data");

    println!("\n4. No rollback plan:");
    println!("   - Always know how to undo a migration");
    println!("   - For destructive changes, backup data first");

    println!("\n5. Direct production changes:");
    println!("   - Always test on staging first");
    println!("   - Use same migration process for all environments");

    // 現在のマイグレーション数を表示
    let migration_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _migrations")
            .fetch_one(pool)
            .await?;

    println!("\nCurrent state: {} migrations applied", migration_count);

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

    setup_migration_tracking(&pool).await?;

    demo_migration_tracking(&pool).await?;
    demo_constraint_testing(&pool).await?;
    demo_schema_verification(&pool).await?;
    demo_gradual_schema_change(&pool).await?;
    demo_anti_patterns(&pool).await?;

    println!("\n=== All diplomatic immunity demos completed successfully! ===");
    Ok(())
}
