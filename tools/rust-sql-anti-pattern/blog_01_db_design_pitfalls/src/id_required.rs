//! アンチパターン2: IDリクワイアド（不要な代理キー）
//!
//! ## 問題
//! - すべてのテーブルに id SERIAL PRIMARY KEY を追加
//! - 交差テーブルでの重複挿入を許可してしまう
//!
//! ## 回避策
//! - 複合主キーの使用
//! - 意味のあるカラム名（user_id, post_id）
//! - UUID vs SERIAL の適切な使い分け
//! - USING句の活用

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// 問題のデモ: 不要なidによる重複許可
pub async fn demo_problem(pool: &PgPool) -> Result<()> {
    println!("--- 2. IDリクワイアド（問題：不要なid） ---");

    sqlx::query("INSERT INTO user_roles_bad (user_id, role_id) VALUES (1, 1)")
        .execute(pool)
        .await?;

    let result = sqlx::query("INSERT INTO user_roles_bad (user_id, role_id) VALUES (1, 1)")
        .execute(pool)
        .await;

    println!("  問題: アンチパターンでは重複挿入が可能");
    match &result {
        Ok(_) => println!("    user_id=1, role_id=1 の重複 → 成功（問題！）"),
        Err(e) => println!("    重複 → 失敗: {}", e),
    }

    // 複合主キーによる解決
    sqlx::query("INSERT INTO posts_good (id, title) VALUES (100, 'Test') ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO tags (id, name) VALUES (100, 'demo') ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES (100, 100) ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;

    let result = sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES (100, 100)")
        .execute(pool)
        .await;

    println!("\n  回避策: 複合主キーで重複防止");
    match &result {
        Ok(_) => println!("    重複 → 成功（予期しない）"),
        Err(_) => println!("    重複 → 拒否（正しい動作）"),
    }

    println!("\n  UUID vs SERIAL:");
    println!("    ┌────────────────┬──────────────────┬──────────────────┐");
    println!("    │ 特性           │ SERIAL           │ UUID             │");
    println!("    ├────────────────┼──────────────────┼──────────────────┤");
    println!("    │ サイズ         │ 4/8バイト        │ 16バイト         │");
    println!("    │ 分散生成       │ 不可             │ 可能             │");
    println!("    │ 予測可能性     │ 予測可能         │ 予測不可能       │");
    println!("    │ 推奨場面       │ 内部ID           │ 公開API          │");
    println!("    └────────────────┴──────────────────┴──────────────────┘");
    println!();

    Ok(())
}

/// 回避策: UUID実践とUSING句
pub async fn demo_uuid_and_using(pool: &PgPool) -> Result<()> {
    println!("--- 2b. IDリクワイアド（回避策：UUID + USING句） ---");

    // 意味のあるカラム名を使用
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users_demo (
            user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username TEXT NOT NULL UNIQUE
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_demo (
            post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users_demo(user_id),
            title TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS comments_demo (
            comment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL REFERENCES posts_demo(post_id),
            user_id UUID NOT NULL REFERENCES users_demo(user_id),
            body TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users_demo (user_id, username) VALUES ($1, $2)")
        .bind(user_id)
        .bind("alice")
        .execute(pool)
        .await?;

    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts_demo (post_id, user_id, title) VALUES ($1, $2, $3)")
        .bind(post_id)
        .bind(user_id)
        .bind("はじめての投稿")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO comments_demo (post_id, user_id, body) VALUES ($1, $2, $3)")
        .bind(post_id)
        .bind(user_id)
        .bind("素晴らしい！")
        .execute(pool)
        .await?;

    println!("  意味のあるカラム名:");
    println!("    良い例: user_id UUID REFERENCES users(user_id)");
    println!("    悪い例: id UUID (すべてが id では混乱)");

    // USING句のデモ
    println!("\n  USING句の活用:");
    let result: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT u.username, p.title, c.body FROM users_demo u
         JOIN posts_demo p USING (user_id)
         JOIN comments_demo c USING (post_id)",
    )
    .fetch_all(pool)
    .await?;

    for (username, title, body) in &result {
        println!("    {} の「{}」: {}", username, title, body);
    }

    println!("\n  UUID生成:");
    let rust_uuid = Uuid::new_v4();
    let db_uuid: (Uuid,) = sqlx::query_as("SELECT gen_random_uuid()")
        .fetch_one(pool)
        .await?;
    println!("    Rust: {}", rust_uuid);
    println!("    DB:   {}", db_uuid.0);

    // クリーンアップ
    sqlx::query("DROP TABLE IF EXISTS comments_demo, posts_demo, users_demo CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
