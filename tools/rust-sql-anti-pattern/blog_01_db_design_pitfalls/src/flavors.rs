//! アンチパターン5: サーティワンフレーバー（ENUM乱用）
//!
//! ## 問題
//! - PostgreSQL ENUMの値変更が困難
//! - 値の削除・名前変更は型の作り直しが必要
//!
//! ## 回避策
//! - 参照テーブルの使用
//! - Rust enum + FromStr/Display
//! - sqlx::Type によるENUM直接マッピング

use crate::types::{AppError, PostStatus, Priority};
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// 問題のデモ: ENUMの制限
pub async fn demo_problem(pool: &PgPool) -> Result<()> {
    println!("--- 5. サーティワンフレーバー（問題：ENUM制限） ---");

    // 参照テーブル方式
    let statuses = [
        ("pending", 1, false),
        ("processing", 2, false),
        ("shipped", 3, false),
        ("delivered", 4, true),
        ("cancelled", 5, true),
    ];

    for (name, order, is_terminal) in &statuses {
        sqlx::query("INSERT INTO order_statuses (name, display_order, is_terminal) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(name).bind(order).bind(is_terminal).execute(pool).await?;
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    enum OrderStatus {
        Pending,
        Processing,
        Shipped,
        Delivered,
        Cancelled,
    }
    impl OrderStatus {
        fn from_str(s: &str) -> Option<Self> {
            match s {
                "pending" => Some(Self::Pending),
                "processing" => Some(Self::Processing),
                "shipped" => Some(Self::Shipped),
                "delivered" => Some(Self::Delivered),
                "cancelled" => Some(Self::Cancelled),
                _ => None,
            }
        }
    }

    let rows: Vec<(String, i32, bool)> = sqlx::query_as(
        "SELECT name, display_order, is_terminal FROM order_statuses ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    println!("  参照テーブル方式:");
    for (name, order, is_terminal) in &rows {
        let status = OrderStatus::from_str(name);
        let terminal = if *is_terminal { " [終了]" } else { "" };
        println!("    {} (order: {}){} → {:?}", name, order, terminal, status);
    }
    println!();

    Ok(())
}

/// 回避策1: PostgreSQL ENUMとの比較
pub async fn demo_enum_comparison(pool: &PgPool) -> Result<()> {
    println!("--- 5b. サーティワンフレーバー（PostgreSQL ENUM） ---");

    sqlx::query("DROP TYPE IF EXISTS priority_level CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("CREATE TYPE priority_level AS ENUM ('low', 'medium', 'high', 'critical')")
        .execute(pool)
        .await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS tasks_enum (id SERIAL PRIMARY KEY, title TEXT NOT NULL, priority priority_level NOT NULL)")
        .execute(pool).await?;

    sqlx::query("INSERT INTO tasks_enum (title, priority) VALUES ('タスク1', 'high')")
        .execute(pool)
        .await?;

    println!("  ENUM特徴:");
    println!("    値の追加: ALTER TYPE ... ADD VALUE（可能）");
    println!("    値の削除: 直接削除は不可能");

    sqlx::query("ALTER TYPE priority_level ADD VALUE IF NOT EXISTS 'urgent' AFTER 'critical'")
        .execute(pool)
        .await?;

    let values: Vec<(String,)> =
        sqlx::query_as("SELECT unnest(enum_range(NULL::priority_level))::text")
            .fetch_all(pool)
            .await?;
    println!(
        "    現在の値: {:?}",
        values.iter().map(|v| &v.0).collect::<Vec<_>>()
    );

    println!("\n  比較表:");
    println!("    ┌────────────────┬─────────────┬─────────────┐");
    println!("    │ 観点           │ ENUM        │ 参照テーブル│");
    println!("    ├────────────────┼─────────────┼─────────────┤");
    println!("    │ 値の追加       │ ◎          │ ◎          │");
    println!("    │ 値の削除       │ ×          │ ◎          │");
    println!("    │ メタデータ     │ ×          │ ◎          │");
    println!("    │ パフォーマンス │ ◎          │ ○          │");
    println!("    └────────────────┴─────────────┴─────────────┘");

    sqlx::query("DROP TABLE IF EXISTS tasks_enum CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TYPE IF EXISTS priority_level CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}

/// 回避策2: Rust統合
pub async fn demo_rust_integration(pool: &PgPool) -> Result<()> {
    println!("--- 5c. サーティワンフレーバー（回避策：Rust統合） ---");

    // 参照テーブル作成
    sqlx::query("CREATE TABLE IF NOT EXISTS post_statuses (status_id SERIAL PRIMARY KEY, code VARCHAR(50) NOT NULL UNIQUE, display_name VARCHAR(100) NOT NULL, sort_order INTEGER NOT NULL)")
        .execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS posts_status (post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(), title TEXT NOT NULL, status_code VARCHAR(50) NOT NULL DEFAULT 'draft' REFERENCES post_statuses(code))")
        .execute(pool).await?;

    for (code, name, order) in [
        ("draft", "下書き", 1),
        ("pending_review", "レビュー待ち", 2),
        ("published", "公開済み", 3),
        ("archived", "アーカイブ", 4),
    ] {
        sqlx::query(
            "INSERT INTO post_statuses (code, display_name, sort_order) VALUES ($1, $2, $3)",
        )
        .bind(code)
        .bind(name)
        .bind(order)
        .execute(pool)
        .await?;
    }

    println!("  FromStr/Display トレイト:");
    println!(
        "    PostStatus::Draft.to_string() = \"{}\"",
        PostStatus::Draft
    );
    println!(
        "    \"published\".parse() = {:?}",
        "published".parse::<PostStatus>()
    );
    println!(
        "    \"invalid\".parse() = {:?}",
        "invalid".parse::<PostStatus>()
    );

    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts_status (post_id, title, status_code) VALUES ($1, $2, $3)")
        .bind(post_id)
        .bind("新しい記事")
        .bind(PostStatus::Draft.as_str())
        .execute(pool)
        .await?;

    async fn update_status(
        pool: &PgPool,
        post_id: Uuid,
        status: PostStatus,
    ) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE posts_status SET status_code = $1 WHERE post_id = $2")
            .bind(status.as_str())
            .bind(post_id)
            .execute(pool)
            .await;
        match result {
            Ok(r) if r.rows_affected() > 0 => Ok(()),
            Ok(_) => Err(AppError::PostNotFound),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    println!("\n  ステータス更新:");
    match update_status(pool, post_id, PostStatus::Published).await {
        Ok(()) => println!("    Draft → Published: 成功"),
        Err(e) => println!("    失敗: {}", e),
    }

    let row: (String,) = sqlx::query_as("SELECT status_code FROM posts_status WHERE post_id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;
    let current: PostStatus = row.0.parse()?;
    println!("    現在: {:?}", current);

    // sqlx::Type デモ
    println!("\n  sqlx::Type デモ:");
    sqlx::query("DROP TYPE IF EXISTS priority_level CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("CREATE TYPE priority_level AS ENUM ('low', 'medium', 'high', 'critical')")
        .execute(pool)
        .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS tasks_priority (task_id UUID PRIMARY KEY DEFAULT gen_random_uuid(), title TEXT NOT NULL, priority priority_level NOT NULL)")
        .execute(pool).await?;

    sqlx::query("INSERT INTO tasks_priority (title, priority) VALUES ($1, $2)")
        .bind("重要タスク")
        .bind(Priority::High)
        .execute(pool)
        .await?;

    let task: (String, Priority) =
        sqlx::query_as("SELECT title, priority FROM tasks_priority LIMIT 1")
            .fetch_one(pool)
            .await?;
    println!("    タスク: {} (優先度: {:?})", task.0, task.1);

    if task.1 == Priority::High {
        println!("    → 高優先度！");
    }

    // クリーンアップ
    sqlx::query("DROP TABLE IF EXISTS tasks_priority CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TYPE IF EXISTS priority_level CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS posts_status CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS post_statuses CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
