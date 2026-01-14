//! SELECT * の問題と明示的カラム指定のデモ
//!
//! このデモでは以下を検証:
//! 1. SELECT * のアンチパターン
//! 2. 明示的なカラム指定の利点
//! 3. 用途別の構造体定義
//! 4. JOINでの明示的カラム指定

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造（用途別）
// ================================

// 最小限の情報
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserSummary {
    id: Uuid,
    name: String,
}

// 公開情報
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserPublic {
    id: Uuid,
    name: String,
    bio: Option<String>,
}

// 詳細情報（内部用）
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserDetail {
    id: Uuid,
    email: String,
    name: String,
    bio: Option<String>,
    created_at: DateTime<Utc>,
}

// POST with Author
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct PostWithAuthor {
    post_id: Uuid,
    post_title: String,
    author_name: String,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS column_posts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS column_users CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル（多くのカラムを持つ）
    sqlx::query(
        r#"
        CREATE TABLE column_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            bio TEXT,
            avatar_url TEXT,
            password_hash VARCHAR(255) NOT NULL,
            api_secret VARCHAR(255),
            last_login_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 投稿テーブル
    sqlx::query(
        r#"
        CREATE TABLE column_posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES column_users(id),
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    println!("\n=== Inserting Sample Data ===");

    let user1: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO column_users (email, name, bio, password_hash, api_secret)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind("alice@example.com")
    .bind("Alice")
    .bind(Some("Software developer"))
    .bind("hashed_password_123")
    .bind("secret_api_key_xyz")
    .fetch_one(pool)
    .await?;

    let user2: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO column_users (email, name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind("bob@example.com")
    .bind("Bob")
    .bind("hashed_password_456")
    .fetch_one(pool)
    .await?;

    // 投稿
    sqlx::query("INSERT INTO column_posts (user_id, title, content) VALUES ($1, $2, $3)")
        .bind(user1)
        .bind("Hello World")
        .bind("This is my first post")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO column_posts (user_id, title, content) VALUES ($1, $2, $3)")
        .bind(user2)
        .bind("Rust Tips")
        .bind("Here are some useful tips")
        .execute(pool)
        .await?;

    println!("Created 2 users and 2 posts");
    Ok(())
}

// ================================
// Demo: アンチパターン SELECT *
// ================================

async fn demo_anti_pattern(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: SELECT * Anti-pattern ===");

    // SELECT * は全カラムを取得（password_hash, api_secretも含む）
    println!("When using SELECT *, you get ALL columns including sensitive data:");
    println!("  - password_hash");
    println!("  - api_secret");
    println!("  - All metadata columns");

    // カラム数を確認
    let column_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM information_schema.columns
        WHERE table_name = 'column_users'
        "#,
    )
    .fetch_one(pool)
    .await?;

    println!("\ncolumn_users has {} columns", column_count);
    println!("Using SELECT * would transfer all of them every time");

    Ok(())
}

// ================================
// Demo: 明示的カラム指定
// ================================

async fn demo_explicit_columns(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Explicit Column Selection ===");

    // 最小限のデータ取得
    let summaries: Vec<UserSummary> = sqlx::query_as(
        "SELECT id, name FROM column_users ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    println!("UserSummary (minimal data):");
    for user in &summaries {
        println!("  {} - {}", user.id, user.name);
    }

    // 公開情報のみ取得
    let public_users: Vec<UserPublic> = sqlx::query_as(
        "SELECT id, name, bio FROM column_users ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    println!("\nUserPublic (safe for API response):");
    for user in &public_users {
        println!("  {} - {:?}", user.name, user.bio);
    }

    // 詳細情報（内部用）
    let details: Vec<UserDetail> = sqlx::query_as(
        "SELECT id, email, name, bio, created_at FROM column_users ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    println!("\nUserDetail (internal use, no password/secrets):");
    for user in &details {
        println!("  {} <{}> - {}", user.name, user.email, user.created_at);
    }

    Ok(())
}

// ================================
// Demo: JOINでの明示的指定
// ================================

async fn demo_join_explicit(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Explicit columns in JOIN ===");

    // JOINで明示的にカラムを指定しエイリアスを付ける
    let posts: Vec<PostWithAuthor> = sqlx::query_as(
        r#"
        SELECT
            p.id as post_id,
            p.title as post_title,
            u.name as author_name
        FROM column_posts p
        JOIN column_users u ON p.user_id = u.id
        ORDER BY p.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Posts with author (explicit columns):");
    for post in &posts {
        println!("  {} by {}", post.post_title, post.author_name);
    }

    // 両テーブルにidがあってもエイリアスで区別可能
    println!("\nNo ambiguous column references with explicit selection!");

    Ok(())
}

// ================================
// Demo: カバリングインデックス
// ================================

async fn demo_covering_index(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Covering Index Benefit ===");

    // カバリングインデックスを作成
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_name_id ON column_users(name) INCLUDE (id)")
        .execute(pool)
        .await?;

    // このクエリはインデックスのみで完結（テーブルアクセス不要）
    let plan: Vec<(String,)> = sqlx::query_as(
        r#"
        EXPLAIN (FORMAT TEXT)
        SELECT id, name FROM column_users WHERE name LIKE 'A%'
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Query plan for SELECT id, name WHERE name LIKE 'A%':");
    for (line,) in plan.iter().take(3) {
        println!("  {}", line);
    }

    println!("\nWith SELECT *, table access would always be required!");
    println!("Explicit column selection enables index-only scans");

    Ok(())
}

// ================================
// Demo: パフォーマンス比較
// ================================

async fn demo_performance_comparison(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Performance Comparison ===");

    // 多数のユーザーを追加
    for i in 0..100 {
        sqlx::query(
            r#"
            INSERT INTO column_users (email, name, bio, password_hash, avatar_url)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(format!("user{}@example.com", i))
        .bind(format!("User {}", i))
        .bind(Some(format!("Bio for user {}", i)))
        .bind(format!("hash_{}", i))
        .bind(Some(format!("https://example.com/avatar/{}.png", i)))
        .execute(pool)
        .await?;
    }

    // SELECT * の場合
    let start = std::time::Instant::now();
    #[allow(clippy::type_complexity)]
    let _all_columns: Vec<(Uuid, String, String, Option<String>, Option<String>, String, Option<String>, Option<DateTime<Utc>>, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, email, name, bio, avatar_url, password_hash, api_secret, last_login_at, created_at, updated_at FROM column_users",
    )
    .fetch_all(pool)
    .await?;
    let all_time = start.elapsed();

    // SELECT id, name のみ
    let start = std::time::Instant::now();
    let _minimal: Vec<UserSummary> = sqlx::query_as("SELECT id, name FROM column_users")
        .fetch_all(pool)
        .await?;
    let minimal_time = start.elapsed();

    println!("All columns: {:?}", all_time);
    println!("Minimal columns: {:?}", minimal_time);
    println!("Selecting only needed columns is faster and uses less memory");

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
    insert_sample_data(&pool).await?;

    demo_anti_pattern(&pool).await?;
    demo_explicit_columns(&pool).await?;
    demo_join_explicit(&pool).await?;
    demo_covering_index(&pool).await?;
    demo_performance_comparison(&pool).await?;

    println!("\n=== All implicit columns demos completed successfully! ===");
    Ok(())
}
