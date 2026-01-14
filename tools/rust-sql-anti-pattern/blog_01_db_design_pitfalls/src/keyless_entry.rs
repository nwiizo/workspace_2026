//! アンチパターン3: キーレスエントリ（外部キー制約なし）
//!
//! ## 問題
//! - 外部キー制約を省略
//! - 孤立データの発生、アプリ依存の整合性チェック
//! - レースコンディションの発生
//!
//! ## 回避策
//! - 外部キー制約の宣言
//! - ON DELETE オプションの適切な選択
//! - FK違反をキャッチしてAppErrorに変換
//! - 外部キー列へのインデックス作成

use crate::types::AppError;
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// 問題のデモ: FK制約なしでの孤立データ
pub async fn demo_problem(pool: &PgPool) -> Result<()> {
    println!("--- 3. キーレスエントリ（問題：FK制約なし） ---");

    // FK制約なし → 存在しないIDでも挿入可能
    let result = sqlx::query("INSERT INTO user_roles_bad (user_id, role_id) VALUES (9999, 9999)")
        .execute(pool)
        .await;
    match &result {
        Ok(_) => println!("  FK制約なし: 存在しないID → 成功（問題！）"),
        Err(e) => println!("  FK制約なし: → 失敗: {}", e),
    }

    // FK制約あり → 拒否される
    sqlx::query("INSERT INTO posts_good (id, title) VALUES (1, 'Test') ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO tags (id, name) VALUES (1, 'test') ON CONFLICT DO NOTHING")
        .execute(pool)
        .await?;

    let result = sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES (9999, 1)")
        .execute(pool)
        .await;
    match &result {
        Ok(_) => println!("  FK制約あり: 存在しないID → 成功（予期しない）"),
        Err(e) => {
            if let sqlx::Error::Database(db_err) = e {
                if db_err.is_foreign_key_violation() {
                    println!("  FK制約あり: 存在しないID → 拒否（正しい）");
                }
            }
        }
    }

    println!("\n  PostgreSQL: 外部キーにはインデックスが自動作成されない！");
    println!("    CREATE INDEX idx_child_parent_id ON child(parent_id);");
    println!();

    Ok(())
}

/// 回避策1: ON DELETE オプションの比較
pub async fn demo_on_delete_options(pool: &PgPool) -> Result<()> {
    println!("--- 3b. キーレスエントリ（回避策：ON DELETEオプション） ---");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS parent_test (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS child_cascade (id SERIAL PRIMARY KEY, parent_id INTEGER REFERENCES parent_test(id) ON DELETE CASCADE)")
        .execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS child_set_null (id SERIAL PRIMARY KEY, parent_id INTEGER REFERENCES parent_test(id) ON DELETE SET NULL)")
        .execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS child_restrict (id SERIAL PRIMARY KEY, parent_id INTEGER REFERENCES parent_test(id) ON DELETE RESTRICT)")
        .execute(pool).await?;

    sqlx::query("INSERT INTO parent_test (id, name) VALUES (1, 'P1'), (2, 'P2'), (3, 'P3')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO child_cascade (parent_id) VALUES (1)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO child_set_null (parent_id) VALUES (2)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO child_restrict (parent_id) VALUES (3)")
        .execute(pool)
        .await?;

    // CASCADE
    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM child_cascade")
        .fetch_one(pool)
        .await?;
    sqlx::query("DELETE FROM parent_test WHERE id = 1")
        .execute(pool)
        .await?;
    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM child_cascade")
        .fetch_one(pool)
        .await?;
    println!("  CASCADE: {} → {} 件（子も削除）", before.0, after.0);

    // SET NULL
    let before: (Option<i32>,) = sqlx::query_as("SELECT parent_id FROM child_set_null LIMIT 1")
        .fetch_one(pool)
        .await?;
    sqlx::query("DELETE FROM parent_test WHERE id = 2")
        .execute(pool)
        .await?;
    let after: (Option<i32>,) = sqlx::query_as("SELECT parent_id FROM child_set_null LIMIT 1")
        .fetch_one(pool)
        .await?;
    println!("  SET NULL: {:?} → {:?}（NULLに）", before.0, after.0);

    // RESTRICT
    let result = sqlx::query("DELETE FROM parent_test WHERE id = 3")
        .execute(pool)
        .await;
    println!(
        "  RESTRICT: 削除{}",
        if result.is_err() { "拒否" } else { "成功" }
    );

    sqlx::query(
        "DROP TABLE IF EXISTS child_restrict, child_set_null, child_cascade, parent_test CASCADE",
    )
    .execute(pool)
    .await?;
    println!();

    Ok(())
}

/// 回避策2: レースコンディション対策
pub async fn demo_race_condition(pool: &PgPool) -> Result<()> {
    println!("--- 3c. キーレスエントリ（回避策：レースコンディション対策） ---");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts_race (post_id UUID PRIMARY KEY, title TEXT NOT NULL)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS comments_race (comment_id UUID PRIMARY KEY, post_id UUID NOT NULL REFERENCES posts_race(post_id) ON DELETE CASCADE, body TEXT NOT NULL)")
        .execute(pool).await?;

    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts_race (post_id, title) VALUES ($1, $2)")
        .bind(post_id)
        .bind("テスト")
        .execute(pool)
        .await?;

    println!("  問題: 存在チェック → 挿入 の間にレースコンディション");
    println!();
    println!("    // 悪い例:");
    println!("    let exists = query!(\"SELECT EXISTS...\").await?;");
    println!("    if !exists {{ return Err(NotFound); }}");
    println!("    // ← ここで別プロセスが投稿を削除する可能性");
    println!("    query!(\"INSERT INTO comments...\").await?;");

    println!();
    println!("  回避策: FK制約に任せてエラーをハンドリング");

    async fn add_comment(pool: &PgPool, post_id: Uuid, body: &str) -> Result<Uuid, AppError> {
        let comment_id = Uuid::new_v4();
        let result = sqlx::query(
            "INSERT INTO comments_race (comment_id, post_id, body) VALUES ($1, $2, $3)",
        )
        .bind(comment_id)
        .bind(post_id)
        .bind(body)
        .execute(pool)
        .await;

        match result {
            Ok(_) => Ok(comment_id),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(AppError::PostNotFound)
            }
            Err(e) => Err(AppError::Database(e)),
        }
    }

    match add_comment(pool, post_id, "正常").await {
        Ok(id) => println!("    存在する投稿 → 成功 ({}...)", &id.to_string()[..8]),
        Err(e) => println!("    存在する投稿 → 失敗: {}", e),
    }

    match add_comment(pool, Uuid::new_v4(), "孤立").await {
        Ok(_) => println!("    存在しない投稿 → 成功（問題！）"),
        Err(AppError::PostNotFound) => println!("    存在しない投稿 → PostNotFound（正しい）"),
        Err(e) => println!("    存在しない投稿 → {}", e),
    }

    sqlx::query("DROP TABLE IF EXISTS comments_race, posts_race CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
