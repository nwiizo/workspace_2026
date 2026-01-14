//! Rustでのデータベース統合テストデモ
//!
//! このデモでは以下を検証:
//! 1. データベース操作関数の実装
//! 2. 制約違反の検出
//! 3. トランザクションの動作
//! 4. NULL処理

use anyhow::Result;
use sqlx::Acquire;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub phone: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub total: rust_decimal::Decimal,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ================================
// データベース操作関数
// ================================

pub async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO users (email, name)
        VALUES ($1, $2)
        RETURNING id, email, name, phone, created_at
        "#,
    )
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn create_user_with_phone(
    pool: &PgPool,
    email: &str,
    name: &str,
    phone: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO users (email, name, phone)
        VALUES ($1, $2, $3)
        RETURNING id, email, name, phone, created_at
        "#,
    )
    .bind(email)
    .bind(name)
    .bind(phone)
    .fetch_one(pool)
    .await
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Option<User> {
    sqlx::query_as("SELECT id, email, name, phone, created_at FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Option<User> {
    sqlx::query_as("SELECT id, email, name, phone, created_at FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn create_order(
    pool: &PgPool,
    user_id: Uuid,
    total: rust_decimal::Decimal,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO orders (user_id, total) VALUES ($1, $2) RETURNING id")
        .bind(user_id)
        .bind(total)
        .fetch_one(pool)
        .await
}

// トランザクション内でユーザーを作成
pub async fn create_user_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO users (email, name)
        VALUES ($1, $2)
        RETURNING id, email, name, phone, created_at
        "#,
    )
    .bind(email)
    .bind(name)
    .fetch_one(&mut **tx)
    .await
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS orders CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            phone TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 注文テーブル
    sqlx::query(
        r#"
        CREATE TABLE orders (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            total DECIMAL(12,2) NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

// ================================
// テストケースのシミュレーション
// ================================

async fn demo_unique_constraint(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Unique Constraint Test ===");

    // 最初のユーザーを作成
    let user1 = create_user(pool, "alice@example.com", "Alice").await?;
    println!("Created user: {} ({})", user1.name, user1.email);

    // 同じメールで作成しようとする（失敗するはず）
    let result = create_user(pool, "alice@example.com", "Alice Clone").await;

    match result {
        Ok(_) => {
            println!("ERROR: Should have failed but succeeded!");
        }
        Err(e) => {
            println!("Expected error: Duplicate email detected");
            let error_str = e.to_string();
            assert!(
                error_str.contains("duplicate") || error_str.contains("unique"),
                "Expected unique constraint error"
            );
        }
    }

    Ok(())
}

async fn demo_foreign_key_constraint(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Foreign Key Constraint Test ===");

    // 存在しないユーザーIDで注文を作成しようとする
    let fake_user_id = Uuid::new_v4();
    let result = create_order(pool, fake_user_id, rust_decimal::Decimal::new(10000, 2)).await;

    match result {
        Ok(_) => {
            println!("ERROR: Should have failed but succeeded!");
        }
        Err(e) => {
            println!("Expected error: Foreign key violation detected");
            let error_str = e.to_string();
            assert!(
                error_str.contains("violates foreign key")
                    || error_str.contains("not present in table"),
                "Expected foreign key error"
            );
        }
    }

    // 正しいユーザーIDで注文を作成
    let user = create_user(pool, "bob@example.com", "Bob").await?;
    let order_id = create_order(pool, user.id, rust_decimal::Decimal::new(10000, 2)).await?;
    println!("Created order {} for user {}", order_id, user.name);

    Ok(())
}

async fn demo_transaction_commit(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Transaction Commit Test ===");

    let mut tx = pool.begin().await?;

    let user = create_user_in_tx(&mut tx, "charlie@example.com", "Charlie").await?;
    println!("Created user in transaction: {}", user.name);

    // コミット
    tx.commit().await?;
    println!("Transaction committed");

    // コミット後にデータが存在することを確認
    let found = get_user_by_email(pool, "charlie@example.com").await;
    assert!(found.is_some(), "User should exist after commit");
    println!("User found after commit: {}", found.is_some());

    Ok(())
}

async fn demo_transaction_rollback(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Transaction Rollback Test ===");

    let mut tx = pool.begin().await?;

    let user = create_user_in_tx(&mut tx, "david@example.com", "David").await?;
    println!("Created user in transaction: {}", user.name);

    // ロールバック
    tx.rollback().await?;
    println!("Transaction rolled back");

    // ロールバック後にデータが存在しないことを確認
    let found = get_user_by_email(pool, "david@example.com").await;
    assert!(found.is_none(), "User should not exist after rollback");
    println!("User found after rollback: {}", found.is_some());

    Ok(())
}

async fn demo_savepoint(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Savepoint (Nested Transaction) Test ===");

    let mut tx = pool.begin().await?;

    // 最初のユーザーを作成
    let user1 = create_user_in_tx(&mut tx, "eve@example.com", "Eve").await?;
    println!("Created user1 in transaction: {}", user1.name);

    // セーブポイント（ネストトランザクション）を開始
    let mut savepoint = tx.begin().await?;

    // 2番目のユーザーを作成
    let user2 = create_user_in_tx(&mut savepoint, "frank@example.com", "Frank").await?;
    println!("Created user2 in savepoint: {}", user2.name);

    // セーブポイントをロールバック
    savepoint.rollback().await?;
    println!("Savepoint rolled back");

    // 外側のトランザクションをコミット
    tx.commit().await?;
    println!("Outer transaction committed");

    // user1は存在するがuser2は存在しない
    let found1 = get_user_by_email(pool, "eve@example.com").await;
    let found2 = get_user_by_email(pool, "frank@example.com").await;

    assert!(found1.is_some(), "user1 should exist");
    assert!(found2.is_none(), "user2 should not exist");

    println!("User1 (Eve) exists: {}", found1.is_some());
    println!("User2 (Frank) exists: {}", found2.is_some());

    Ok(())
}

async fn demo_null_handling(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: NULL Handling Test ===");

    // 電話番号なしでユーザーを作成
    let user_without_phone =
        create_user_with_phone(pool, "grace@example.com", "Grace", None).await?;

    // 電話番号ありでユーザーを作成
    let user_with_phone =
        create_user_with_phone(pool, "henry@example.com", "Henry", Some("090-1234-5678")).await?;

    println!(
        "User without phone: {} - {:?}",
        user_without_phone.name, user_without_phone.phone
    );
    println!(
        "User with phone: {} - {:?}",
        user_with_phone.name, user_with_phone.phone
    );

    assert!(user_without_phone.phone.is_none());
    assert_eq!(user_with_phone.phone, Some("090-1234-5678".to_string()));

    // データベースから再取得してNULL処理を確認
    let fetched = get_user_by_id(pool, user_without_phone.id).await.unwrap();
    assert!(fetched.phone.is_none(), "NULL should map to None in Rust");
    println!("NULL correctly mapped to Option::None");

    Ok(())
}

async fn demo_cascade_delete(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Cascade Delete Test ===");

    // ユーザーと注文を作成
    let user = create_user(pool, "ivan@example.com", "Ivan").await?;
    println!("Created user: {}", user.name);

    let order1 = create_order(pool, user.id, rust_decimal::Decimal::new(5000, 2)).await?;
    let order2 = create_order(pool, user.id, rust_decimal::Decimal::new(7500, 2)).await?;
    println!("Created orders: {}, {}", order1, order2);

    // 注文数を確認
    let order_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(pool)
        .await?;
    println!("Order count before delete: {}", order_count);
    assert_eq!(order_count, 2);

    // ユーザーを削除（CASCADE DELETE で注文も削除されるはず）
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user.id)
        .execute(pool)
        .await?;
    println!("Deleted user");

    // 注文も削除されたことを確認
    let order_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(pool)
        .await?;
    println!("Order count after delete: {}", order_count);
    assert_eq!(order_count, 0);

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

    demo_unique_constraint(&pool).await?;
    demo_foreign_key_constraint(&pool).await?;
    demo_transaction_commit(&pool).await?;
    demo_transaction_rollback(&pool).await?;
    demo_savepoint(&pool).await?;
    demo_null_handling(&pool).await?;
    demo_cascade_delete(&pool).await?;

    println!("\n=== All integration testing demos completed successfully! ===");
    Ok(())
}

// ================================
// テストモジュール（実際の統合テストはtests/ディレクトリに置く）
// ================================

#[cfg(test)]
mod tests {
    use super::*;

    // 注: 実際のsqlx::testを使うにはDATABASE_URLの設定とマイグレーションが必要
    // ここでは構造を示すためのサンプル

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_create_user() {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/antipattern".to_string()
        });

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .unwrap();

        // テスト用のユニークなメールアドレス
        let email = format!("test_{}@example.com", Uuid::new_v4());

        let user = create_user(&pool, &email, "Test User").await.unwrap();

        assert_eq!(user.email, email);
        assert_eq!(user.name, "Test User");
        assert!(user.id != Uuid::nil());
    }

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_duplicate_user_fails() {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/antipattern".to_string()
        });

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .unwrap();

        let email = format!("dup_{}@example.com", Uuid::new_v4());

        // 最初の作成は成功
        create_user(&pool, &email, "First").await.unwrap();

        // 2回目は失敗するはず
        let result = create_user(&pool, &email, "Second").await;
        assert!(result.is_err());
    }
}
