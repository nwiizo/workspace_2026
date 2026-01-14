//! DB設計の落とし穴 - アンチパターン検証コード
//!
//! 5つのアンチパターンと PostgreSQL + Rust での回避策を実演します:
//!
//! 1. **ジェイウォーク**: カンマ区切りでデータを格納
//!    → 配列型、交差テーブル、UUIDスキーマ
//!
//! 2. **IDリクワイアド**: 不要な代理キー
//!    → 複合主キー、USING句、UUID実践
//!
//! 3. **キーレスエントリ**: 外部キー制約なし
//!    → ON DELETE オプション、レースコンディション対策
//!
//! 4. **ラウンディングエラー**: FLOATの落とし穴
//!    → DECIMAL型、rust_decimal、セント単位格納、用途に応じた型選択
//!
//! 5. **サーティワンフレーバー**: ENUM乱用
//!    → 参照テーブル、FromStr/Display、sqlx::Type

mod flavors;
mod id_required;
mod jaywalking;
mod keyless_entry;
mod rounding_error;
mod types;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/antipattern";

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    println!("=== DB設計の落とし穴 デモ ===\n");

    let version: (String,) = sqlx::query_as("SELECT version()").fetch_one(&pool).await?;
    println!(
        "PostgreSQL: {}\n",
        version.0.split(',').next().unwrap_or(&version.0)
    );

    // 毎回クリーンな状態から開始
    cleanup_tables(&pool).await?;
    setup_tables(&pool).await?;

    // 1. ジェイウォーク
    jaywalking::demo_problem(&pool).await?;
    jaywalking::demo_array_solution(&pool).await?;
    jaywalking::demo_intersection_table(&pool).await?;
    jaywalking::demo_uuid_schema(&pool).await?;

    // 2. IDリクワイアド
    id_required::demo_problem(&pool).await?;
    id_required::demo_uuid_and_using(&pool).await?;

    // 3. キーレスエントリ
    keyless_entry::demo_problem(&pool).await?;
    keyless_entry::demo_on_delete_options(&pool).await?;
    keyless_entry::demo_race_condition(&pool).await?;

    // 4. ラウンディングエラー
    rounding_error::demo_problem(&pool).await?;
    rounding_error::demo_cumulative_error(&pool).await?;
    rounding_error::demo_cents_storage(&pool).await?;
    rounding_error::demo_float_acceptable_cases(&pool).await?;

    // 5. サーティワンフレーバー
    flavors::demo_problem(&pool).await?;
    flavors::demo_enum_comparison(&pool).await?;
    flavors::demo_rust_integration(&pool).await?;

    cleanup_tables(&pool).await?;

    println!("=== デモ完了 ===");
    Ok(())
}

async fn setup_tables(pool: &PgPool) -> Result<()> {
    // ジェイウォーク: アンチパターン版
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_bad (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            tags TEXT
        )",
    )
    .execute(pool)
    .await?;

    // ジェイウォーク: 配列型
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_array (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            tags TEXT[] DEFAULT '{}'
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_posts_array_tags ON posts_array USING GIN(tags)")
        .execute(pool)
        .await?;

    // ジェイウォーク: 交差テーブル
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_good (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tags (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            slug TEXT GENERATED ALWAYS AS (lower(regexp_replace(name, '[^a-zA-Z0-9]', '-', 'g'))) STORED
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS post_tags (
            post_id INTEGER REFERENCES posts_good(id) ON DELETE CASCADE,
            tag_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        )",
    )
    .execute(pool)
    .await?;

    // IDリクワイアド: アンチパターン版
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_roles_bad (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // ラウンディングエラー
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS products_bad (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            price FLOAT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS products_good (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            price DECIMAL(10, 2) NOT NULL,
            price_with_tax DECIMAL(10, 2) GENERATED ALWAYS AS (price * 1.10) STORED
        )",
    )
    .execute(pool)
    .await?;

    // サーティワンフレーバー: 参照テーブル
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS order_statuses (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            display_order INTEGER NOT NULL,
            is_terminal BOOLEAN DEFAULT FALSE
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS orders (
            id SERIAL PRIMARY KEY,
            status_id INTEGER REFERENCES order_statuses(id)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    let tables = [
        "orders",
        "order_statuses",
        "products_good",
        "products_bad",
        "user_roles_bad",
        "post_tags",
        "tags",
        "posts_good",
        "posts_array",
        "posts_bad",
    ];
    for table in tables {
        sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table))
            .execute(pool)
            .await?;
    }
    Ok(())
}
