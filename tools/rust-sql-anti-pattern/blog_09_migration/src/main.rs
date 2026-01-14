//! マイグレーション戦略のデモ
//!
//! このデモでは以下を検証:
//! 1. ゼロダウンタイムマイグレーション
//! 2. NULLable列の追加
//! 3. バックフィル（データ埋め）パターン
//! 4. カラム名変更の安全な方法
//! 5. デフォルト値とNOT NULL制約の追加

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
struct User {
    id: Uuid,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserWithStatus {
    id: Uuid,
    email: String,
    name: String,
    status: Option<String>,
    created_at: DateTime<Utc>,
}

// ================================
// 初期セットアップ
// ================================

async fn setup_initial_schema(pool: &PgPool) -> Result<()> {
    println!("=== Setting up initial schema ===");

    sqlx::query("DROP TABLE IF EXISTS migration_users CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE migration_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // サンプルデータを挿入
    for i in 1..=10 {
        sqlx::query("INSERT INTO migration_users (email, name) VALUES ($1, $2)")
            .bind(format!("user{}@example.com", i))
            .bind(format!("User {}", i))
            .execute(pool)
            .await?;
    }

    println!("Created table with 10 users");
    Ok(())
}

// ================================
// Demo 1: NULLable列の追加（安全・即座に完了）
// ================================

async fn demo_add_nullable_column(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Adding nullable column (instant) ===");

    // Step 1: NULLを許可する新しいカラムを追加（即座に完了）
    let start = std::time::Instant::now();
    sqlx::query("ALTER TABLE migration_users ADD COLUMN IF NOT EXISTS status VARCHAR(20)")
        .execute(pool)
        .await?;
    println!("Column added in {:?} (metadata change only)", start.elapsed());

    // 既存データはNULLのまま
    let users: Vec<UserWithStatus> = sqlx::query_as(
        "SELECT id, email, name, status, created_at FROM migration_users LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    println!("Users after adding column:");
    for user in &users {
        println!("  {} - status: {:?}", user.name, user.status);
    }

    Ok(())
}

// ================================
// Demo 2: バックフィル（データ埋め）
// ================================

async fn demo_backfill(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Backfill in batches ===");

    let batch_size: i64 = 3;
    let mut total_updated = 0u64;

    loop {
        let result = sqlx::query(
            r#"
            UPDATE migration_users
            SET status = 'active'
            WHERE id IN (
                SELECT id FROM migration_users
                WHERE status IS NULL
                LIMIT $1
            )
            "#,
        )
        .bind(batch_size)
        .execute(pool)
        .await?;

        let rows_affected = result.rows_affected();
        total_updated += rows_affected;
        println!("Batch updated: {} rows", rows_affected);

        if rows_affected == 0 {
            break;
        }

        // 他のクエリに影響を与えないよう、少し待機
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    println!("Total rows updated: {}", total_updated);

    // 確認
    let users: Vec<UserWithStatus> = sqlx::query_as(
        "SELECT id, email, name, status, created_at FROM migration_users LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    println!("Users after backfill:");
    for user in &users {
        println!("  {} - status: {:?}", user.name, user.status);
    }

    Ok(())
}

// ================================
// Demo 3: デフォルト値とNOT NULL制約の追加
// ================================

async fn demo_add_default_and_not_null(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Adding DEFAULT and NOT NULL ===");

    // Step 2: 全データの更新が完了してから、デフォルト値を設定
    sqlx::query("ALTER TABLE migration_users ALTER COLUMN status SET DEFAULT 'pending'")
        .execute(pool)
        .await?;
    println!("Set default value to 'pending'");

    // Step 3: NOT NULL制約を追加
    sqlx::query("ALTER TABLE migration_users ALTER COLUMN status SET NOT NULL")
        .execute(pool)
        .await?;
    println!("Set NOT NULL constraint");

    // 新しいユーザーを追加してデフォルト値を確認
    let new_user: UserWithStatus = sqlx::query_as(
        r#"
        INSERT INTO migration_users (email, name)
        VALUES ('new@example.com', 'New User')
        RETURNING id, email, name, status, created_at
        "#,
    )
    .fetch_one(pool)
    .await?;

    println!(
        "New user created with default status: {:?}",
        new_user.status
    );

    Ok(())
}

// ================================
// Demo 4: カラム名の変更（安全な方法）
// ================================

async fn demo_rename_column_safely(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Safe column rename ===");

    // Step 1: 新しいカラムを追加
    sqlx::query("ALTER TABLE migration_users ADD COLUMN IF NOT EXISTS display_name VARCHAR(100)")
        .execute(pool)
        .await?;
    println!("Step 1: Added new column 'display_name'");

    // Step 2: データをコピー
    sqlx::query("UPDATE migration_users SET display_name = name WHERE display_name IS NULL")
        .execute(pool)
        .await?;
    println!("Step 2: Copied data from 'name' to 'display_name'");

    // Step 3: トリガーで同期（実際の運用では必要）
    // ここでは簡略化のため、トリガーの設定は省略
    println!("Step 3: (In production, set up a trigger to sync both columns)");

    // 確認
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT name, display_name FROM migration_users LIMIT 1")
            .fetch_one(pool)
            .await?;
    println!("  name: {}, display_name: {:?}", row.0, row.1);

    // Step 4: アプリケーションを新しいカラム名に移行後、古いカラムを削除
    println!("Step 4: (After migration, drop old column)");
    println!("  -- ALTER TABLE migration_users DROP COLUMN name;");

    Ok(())
}

// ================================
// Demo 5: テーブル構造の大規模変更
// ================================

async fn demo_schema_restructure(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Schema restructure ===");

    // 新しいテーブルを作成
    sqlx::query("DROP TABLE IF EXISTS user_addresses CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE user_addresses (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            street VARCHAR(255),
            city VARCHAR(100),
            postal_code VARCHAR(20),
            country VARCHAR(100),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created separate 'user_addresses' table");

    // データを移行（実際の環境では既存の非正規化カラムから）
    let users: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM migration_users LIMIT 3")
        .fetch_all(pool)
        .await?;

    for (user_id,) in &users {
        sqlx::query(
            "INSERT INTO user_addresses (user_id, street, city, country) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind("123 Main St")
        .bind("Tokyo")
        .bind("Japan")
        .execute(pool)
        .await?;
    }
    println!("Migrated address data for {} users", users.len());

    // JOINで取得
    let result: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT u.name, a.city
        FROM migration_users u
        JOIN user_addresses a ON u.id = a.user_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Users with addresses:");
    for (name, city) in &result {
        println!("  {} - {}", name, city);
    }

    Ok(())
}

// ================================
// Demo 6: マイグレーション状態の管理
// ================================

async fn demo_migration_tracking(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Migration tracking ===");

    // マイグレーション管理テーブル
    sqlx::query("DROP TABLE IF EXISTS _migrations CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE _migrations (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL UNIQUE,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // マイグレーションの適用を記録
    let migrations = vec![
        "20231215_create_users",
        "20231216_add_status_column",
        "20231217_add_not_null",
    ];

    for migration in &migrations {
        // 既に適用済みかチェック
        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)",
        )
        .bind(migration)
        .fetch_one(pool)
        .await?;

        if !applied {
            // マイグレーションを適用（実際の処理は省略）
            println!("Applying migration: {}", migration);

            // 適用を記録
            sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
                .bind(migration)
                .execute(pool)
                .await?;
        } else {
            println!("Migration already applied: {}", migration);
        }
    }

    // 適用済みマイグレーションを表示
    let applied: Vec<(String, DateTime<Utc>)> =
        sqlx::query_as("SELECT name, applied_at FROM _migrations ORDER BY id")
            .fetch_all(pool)
            .await?;

    println!("\nApplied migrations:");
    for (name, applied_at) in &applied {
        println!("  {} ({})", name, applied_at);
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

    setup_initial_schema(&pool).await?;

    demo_add_nullable_column(&pool).await?;
    demo_backfill(&pool).await?;
    demo_add_default_and_not_null(&pool).await?;
    demo_rename_column_safely(&pool).await?;
    demo_schema_restructure(&pool).await?;
    demo_migration_tracking(&pool).await?;

    println!("\n=== All migration demos completed successfully! ===");
    Ok(())
}
