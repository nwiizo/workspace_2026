//! アンチパターン1: ジェイウォーク（カンマ区切りデータ）
//!
//! ## 問題
//! - カンマ区切りで複数値を1カラムに格納
//! - LIKE検索での誤検出、集約の困難さ、参照整合性なし
//!
//! ## 回避策
//! - PostgreSQL配列型（シンプルなケース）
//! - 交差テーブル（正規化、メタ情報管理）
//! - UUIDベースのスキーマ

use anyhow::Result;
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

/// 問題のデモ: カンマ区切りでの格納
pub async fn demo_problem(pool: &PgPool) -> Result<()> {
    println!("--- 1. ジェイウォーク（問題：カンマ区切り） ---");

    sqlx::query("INSERT INTO posts_bad (title, tags) VALUES ($1, $2)")
        .bind("Rustの魅力")
        .bind("rust,programming,systems")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO posts_bad (title, tags) VALUES ($1, $2)")
        .bind("信頼性について")
        .bind("trustworthy,reliable")
        .execute(pool)
        .await?;

    println!("  問題1: LIKE検索の誤検出リスク");
    let result: Vec<(String, String)> =
        sqlx::query_as("SELECT title, tags FROM posts_bad WHERE tags LIKE '%rust%'")
            .fetch_all(pool)
            .await?;

    println!("    '%rust%' で検索: {} 件", result.len());
    for (title, tags) in &result {
        println!("      - {} [{}]", title, tags);
    }

    println!("\n  問題2: アプリケーション側でのパース処理");
    let row: (String,) = sqlx::query_as("SELECT tags FROM posts_bad WHERE id = 1")
        .fetch_one(pool)
        .await?;
    let tags: Vec<&str> = row.0.split(',').collect();
    println!("    Rust側でパース: {:?}", tags);
    println!();

    Ok(())
}

/// 回避策1: PostgreSQL配列型
pub async fn demo_array_solution(pool: &PgPool) -> Result<()> {
    println!("--- 1b. ジェイウォーク（回避策：PostgreSQL配列型） ---");

    sqlx::query("INSERT INTO posts_array (title, tags) VALUES ($1, $2)")
        .bind("Rustの魅力")
        .bind(&["rust", "programming", "systems"] as &[&str])
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO posts_array (title, tags) VALUES ($1, $2)")
        .bind("信頼性について")
        .bind(&["trustworthy", "reliable"] as &[&str])
        .execute(pool)
        .await?;

    println!("  ANY演算子で正確に検索:");
    let result: Vec<(String, Vec<String>)> =
        sqlx::query_as("SELECT title, tags FROM posts_array WHERE 'rust' = ANY(tags)")
            .fetch_all(pool)
            .await?;
    println!("    'rust' = ANY(tags): {} 件（正確）", result.len());

    println!("\n  配列演算子:");
    let result: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM posts_array WHERE tags @> ARRAY['rust', 'programming']")
            .fetch_all(pool)
            .await?;
    println!("    @> (両方含む): {} 件", result.len());

    println!("\n  Rustでの扱い: Vec<String> として直接取得可能");
    println!();

    Ok(())
}

/// 回避策2: 交差テーブル
pub async fn demo_intersection_table(pool: &PgPool) -> Result<()> {
    println!("--- 1c. ジェイウォーク（回避策：交差テーブル） ---");

    // テストデータ追加
    for i in 1..=100 {
        sqlx::query("INSERT INTO posts_good (title) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(format!("記事{}", i))
            .execute(pool)
            .await?;
    }

    let tags = [
        "rust",
        "postgresql",
        "programming",
        "web",
        "database",
        "performance",
        "security",
        "testing",
        "async",
        "api",
    ];
    for (i, tag) in tags.iter().enumerate() {
        sqlx::query("INSERT INTO tags (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind((i + 2) as i32)
            .bind(*tag)
            .execute(pool)
            .await?;
    }

    for i in 1..=100 {
        let tag_id = (i % 10) + 1;
        let _ = sqlx::query(
            "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(i)
        .bind(tag_id)
        .execute(pool)
        .await;
    }

    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM posts_good p
         JOIN post_tags pt ON p.id = pt.post_id
         JOIN tags t ON pt.tag_id = t.id
         WHERE t.name = 'rust'",
    )
    .fetch_one(pool)
    .await?;
    println!("  交差テーブルでの検索: 'rust' タグ {} 件", result.0);

    // パフォーマンス比較
    for i in 1..=100 {
        let tags_vec: Vec<&str> = vec![tags[i as usize % 10], tags[(i as usize + 1) % 10]];
        sqlx::query("INSERT INTO posts_array (title, tags) VALUES ($1, $2)")
            .bind(format!("配列記事{}", i))
            .bind(&tags_vec)
            .execute(pool)
            .await?;
    }

    println!("\n  パフォーマンス比較（100件）:");
    let start = Instant::now();
    let _: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM posts_array WHERE 'rust' = ANY(tags)")
            .fetch_all(pool)
            .await?;
    println!("    配列型: {:?}", start.elapsed());

    let start = Instant::now();
    let _: Vec<(String,)> = sqlx::query_as(
        "SELECT p.title FROM posts_good p
         JOIN post_tags pt ON p.id = pt.post_id
         JOIN tags t ON pt.tag_id = t.id WHERE t.name = 'rust'",
    )
    .fetch_all(pool)
    .await?;
    println!("    交差テーブル: {:?}", start.elapsed());
    println!();

    Ok(())
}

/// 回避策3: UUIDベースのスキーマ（ブログ記事推奨）
pub async fn demo_uuid_schema(pool: &PgPool) -> Result<()> {
    println!("--- 1d. ジェイウォーク（回避策：UUIDスキーマ） ---");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_uuid (
            post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(200) NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tags_uuid (
            tag_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(50) NOT NULL UNIQUE,
            slug VARCHAR(50) NOT NULL UNIQUE
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS post_tags_uuid (
            post_id UUID NOT NULL REFERENCES posts_uuid(post_id) ON DELETE CASCADE,
            tag_id UUID NOT NULL REFERENCES tags_uuid(tag_id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        )",
    )
    .execute(pool)
    .await?;

    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts_uuid (post_id, title) VALUES ($1, $2)")
        .bind(post_id)
        .bind("Rustでの安全なDB設計")
        .execute(pool)
        .await?;

    let rust_tag_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tags_uuid (tag_id, name, slug) VALUES ($1, $2, $3)")
        .bind(rust_tag_id)
        .bind("Rust")
        .bind("rust")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO post_tags_uuid (post_id, tag_id) VALUES ($1, $2)")
        .bind(post_id)
        .bind(rust_tag_id)
        .execute(pool)
        .await?;

    println!("  find_posts_by_tag 関数:");
    let posts: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT p.post_id, p.title FROM posts_uuid p
         INNER JOIN post_tags_uuid pt ON p.post_id = pt.post_id
         INNER JOIN tags_uuid t ON pt.tag_id = t.tag_id
         WHERE t.slug = $1",
    )
    .bind("rust")
    .fetch_all(pool)
    .await?;
    println!("    'rust': {} 件", posts.len());

    println!("\n  get_tag_counts 関数:");
    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT t.name, COUNT(pt.post_id) FROM tags_uuid t
         LEFT JOIN post_tags_uuid pt ON t.tag_id = pt.tag_id
         GROUP BY t.tag_id, t.name",
    )
    .fetch_all(pool)
    .await?;
    for (name, count) in &counts {
        println!("    {}: {} 件", name, count);
    }

    // クリーンアップ
    sqlx::query("DROP TABLE IF EXISTS post_tags_uuid, tags_uuid, posts_uuid CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
