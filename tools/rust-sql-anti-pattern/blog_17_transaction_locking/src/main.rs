//! トランザクションとロック戦略のデモ
//!
//! このデモでは以下を検証:
//! 1. 楽観的ロック（バージョン番号）
//! 2. 悲観的ロック（FOR UPDATE）
//! 3. デッドロック回避
//! 4. トランザクション分離レベル

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
pub enum LockError {
    #[error("Concurrent modification detected")]
    ConcurrentModification,

    #[error("Record not found")]
    NotFound,

    #[error("Insufficient stock")]
    InsufficientStock,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// ================================
// データ構造
// ================================

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    price: Decimal,
    stock: i32,
    version: i32,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Account {
    id: Uuid,
    name: String,
    balance: Decimal,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS lock_accounts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS lock_products CASCADE")
        .execute(pool)
        .await?;

    // 楽観的ロック用のプロダクトテーブル
    sqlx::query(
        r#"
        CREATE TABLE lock_products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(200) NOT NULL,
            price DECIMAL(10,2) NOT NULL,
            stock INT NOT NULL DEFAULT 0,
            version INT NOT NULL DEFAULT 1,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 悲観的ロック用のアカウントテーブル
    sqlx::query(
        r#"
        CREATE TABLE lock_accounts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(100) NOT NULL,
            balance DECIMAL(10,2) NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<(Uuid, Uuid, Uuid)> {
    println!("\n=== Inserting Sample Data ===");

    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO lock_products (name, price, stock) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("Widget")
    .bind(Decimal::new(1000, 2))
    .bind(100)
    .fetch_one(pool)
    .await?;

    let account_a: Uuid = sqlx::query_scalar(
        "INSERT INTO lock_accounts (name, balance) VALUES ($1, $2) RETURNING id",
    )
    .bind("Account A")
    .bind(Decimal::new(100000, 2))
    .fetch_one(pool)
    .await?;

    let account_b: Uuid = sqlx::query_scalar(
        "INSERT INTO lock_accounts (name, balance) VALUES ($1, $2) RETURNING id",
    )
    .bind("Account B")
    .bind(Decimal::new(50000, 2))
    .fetch_one(pool)
    .await?;

    println!("Created product and 2 accounts");
    Ok((product_id, account_a, account_b))
}

// ================================
// Demo: アトミック更新
// ================================

async fn demo_atomic_update(pool: &PgPool, product_id: Uuid) -> Result<()> {
    println!("\n=== Demo: Atomic Update ===");

    // 在庫を確実に減らす（Lost Update を防ぐ）
    let rows_affected = sqlx::query(
        "UPDATE lock_products SET stock = stock - 1, updated_at = NOW() WHERE id = $1 AND stock > 0",
    )
    .bind(product_id)
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected > 0 {
        println!("Stock decremented atomically (1 row affected)");
    } else {
        println!("Stock not available");
    }

    // 現在の在庫を確認
    let stock: i32 =
        sqlx::query_scalar("SELECT stock FROM lock_products WHERE id = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await?;

    println!("Current stock: {}", stock);
    Ok(())
}

// ================================
// Demo: 楽観的ロック
// ================================

async fn update_with_optimistic_lock(
    pool: &PgPool,
    product_id: Uuid,
    new_price: Decimal,
    expected_version: i32,
) -> Result<Product, LockError> {
    let result: Option<Product> = sqlx::query_as(
        r#"
        UPDATE lock_products
        SET price = $1, version = version + 1, updated_at = NOW()
        WHERE id = $2 AND version = $3
        RETURNING id, name, price, stock, version, updated_at
        "#,
    )
    .bind(new_price)
    .bind(product_id)
    .bind(expected_version)
    .fetch_optional(pool)
    .await?;

    match result {
        Some(product) => Ok(product),
        None => {
            // レコードが存在するか確認
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM lock_products WHERE id = $1)",
            )
            .bind(product_id)
            .fetch_one(pool)
            .await?;

            if exists {
                Err(LockError::ConcurrentModification)
            } else {
                Err(LockError::NotFound)
            }
        }
    }
}

async fn demo_optimistic_locking(pool: &PgPool, product_id: Uuid) -> Result<()> {
    println!("\n=== Demo: Optimistic Locking ===");

    // 現在の製品情報を取得
    let product: Product = sqlx::query_as(
        "SELECT id, name, price, stock, version, updated_at FROM lock_products WHERE id = $1",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;

    println!(
        "Current: {} @ {} (version {})",
        product.name, product.price, product.version
    );

    // 価格を更新（正しいバージョン）
    let new_price = Decimal::new(1200, 2);
    match update_with_optimistic_lock(pool, product_id, new_price, product.version).await {
        Ok(updated) => {
            println!(
                "Updated: {} @ {} (version {})",
                updated.name, updated.price, updated.version
            );
        }
        Err(e) => println!("Update failed: {}", e),
    }

    // 古いバージョンで更新を試みる（失敗するはず）
    let another_price = Decimal::new(1500, 2);
    match update_with_optimistic_lock(pool, product_id, another_price, product.version).await {
        Ok(_) => println!("Unexpected success"),
        Err(LockError::ConcurrentModification) => {
            println!("Correctly detected: Concurrent modification (stale version)");
        }
        Err(e) => println!("Unexpected error: {}", e),
    }

    Ok(())
}

// ================================
// Demo: 悲観的ロック
// ================================

async fn transfer_with_pessimistic_lock(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    amount: Decimal,
) -> Result<(), LockError> {
    let mut tx = pool.begin().await?;

    // ロック順序を統一（IDでソート）してデッドロック防止
    let (first_id, second_id) = if from_id < to_id {
        (from_id, to_id)
    } else {
        (to_id, from_id)
    };

    // 最初のアカウントをロック
    let _first: Account = sqlx::query_as(
        "SELECT id, name, balance FROM lock_accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(first_id)
    .fetch_one(&mut *tx)
    .await?;

    // 2番目のアカウントをロック
    let _second: Account = sqlx::query_as(
        "SELECT id, name, balance FROM lock_accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(second_id)
    .fetch_one(&mut *tx)
    .await?;

    // 残高確認
    let from_balance: Decimal = sqlx::query_scalar(
        "SELECT balance FROM lock_accounts WHERE id = $1",
    )
    .bind(from_id)
    .fetch_one(&mut *tx)
    .await?;

    if from_balance < amount {
        return Err(LockError::InsufficientStock);
    }

    // 送金元から引き落とし
    sqlx::query("UPDATE lock_accounts SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // 送金先に入金
    sqlx::query("UPDATE lock_accounts SET balance = balance + $1 WHERE id = $2")
        .bind(amount)
        .bind(to_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

async fn demo_pessimistic_locking(pool: &PgPool, account_a: Uuid, account_b: Uuid) -> Result<()> {
    println!("\n=== Demo: Pessimistic Locking ===");

    // 転送前の残高を確認
    let balance_a: Decimal =
        sqlx::query_scalar("SELECT balance FROM lock_accounts WHERE id = $1")
            .bind(account_a)
            .fetch_one(pool)
            .await?;
    let balance_b: Decimal =
        sqlx::query_scalar("SELECT balance FROM lock_accounts WHERE id = $1")
            .bind(account_b)
            .fetch_one(pool)
            .await?;

    println!("Before transfer:");
    println!("  Account A: {}", balance_a);
    println!("  Account B: {}", balance_b);

    // 送金
    let amount = Decimal::new(25000, 2);
    transfer_with_pessimistic_lock(pool, account_a, account_b, amount).await?;

    // 転送後の残高を確認
    let balance_a: Decimal =
        sqlx::query_scalar("SELECT balance FROM lock_accounts WHERE id = $1")
            .bind(account_a)
            .fetch_one(pool)
            .await?;
    let balance_b: Decimal =
        sqlx::query_scalar("SELECT balance FROM lock_accounts WHERE id = $1")
            .bind(account_b)
            .fetch_one(pool)
            .await?;

    println!("\nAfter transfer of {}:", amount);
    println!("  Account A: {}", balance_a);
    println!("  Account B: {}", balance_b);

    Ok(())
}

// ================================
// Demo: FOR UPDATE NOWAIT / SKIP LOCKED
// ================================

async fn demo_lock_options(pool: &PgPool, product_id: Uuid) -> Result<()> {
    println!("\n=== Demo: FOR UPDATE options ===");

    println!("FOR UPDATE: Blocks until lock is available");
    println!("FOR UPDATE NOWAIT: Fails immediately if locked");
    println!("FOR UPDATE SKIP LOCKED: Skips locked rows (good for queues)");

    // SKIP LOCKED のデモ
    let mut tx = pool.begin().await?;

    let product: Option<Product> = sqlx::query_as(
        r#"
        SELECT id, name, price, stock, version, updated_at
        FROM lock_products
        WHERE stock > 0
        ORDER BY id
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .fetch_optional(&mut *tx)
    .await?;

    match product {
        Some(p) => println!("Got product with SKIP LOCKED: {} (stock: {})", p.name, p.stock),
        None => println!("No unlocked products available"),
    }

    tx.rollback().await?;

    let _ = product_id;
    Ok(())
}

// ================================
// Demo: トランザクション分離レベル
// ================================

async fn demo_isolation_levels(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Transaction Isolation Levels ===");

    println!("PostgreSQL supports:");
    println!("  1. READ COMMITTED (default)");
    println!("     - Sees only committed data");
    println!("     - May see different data in same transaction");

    println!("\n  2. REPEATABLE READ");
    println!("     - Snapshot at transaction start");
    println!("     - Same query returns same results");

    println!("\n  3. SERIALIZABLE");
    println!("     - Full isolation");
    println!("     - May fail with serialization error (40001)");

    // REPEATABLE READ のデモ
    let mut tx = pool.begin().await?;

    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;

    let count1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lock_products")
        .fetch_one(&mut *tx)
        .await?;

    println!("\nIn REPEATABLE READ transaction:");
    println!("  First count: {}", count1);
    println!("  (Even if rows are added externally, this count won't change)");

    tx.rollback().await?;

    Ok(())
}

// ================================
// Demo: デッドロック回避
// ================================

async fn demo_deadlock_prevention(_pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Deadlock Prevention ===");

    println!("Deadlock scenario:");
    println!("  TX1: Lock(A), then Lock(B)");
    println!("  TX2: Lock(B), then Lock(A)");
    println!("  → Both wait forever!");

    println!("\nPrevention strategies:");
    println!("  1. Lock ordering: Always lock in consistent order (e.g., by ID)");
    println!("  2. Lock timeout: SET LOCAL lock_timeout = '5s'");
    println!("  3. NOWAIT: Fail immediately if locked");
    println!("  4. Retry with backoff: Catch 40P01 and retry");

    // ロック順序の統一を示す
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let (first, second) = if id1 < id2 { (id1, id2) } else { (id2, id1) };
    println!("\nOrdering example:");
    println!("  ID1: {}", id1);
    println!("  ID2: {}", id2);
    println!("  Lock order: {} then {}", first, second);

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
    let (product_id, account_a, account_b) = insert_sample_data(&pool).await?;

    demo_atomic_update(&pool, product_id).await?;
    demo_optimistic_locking(&pool, product_id).await?;
    demo_pessimistic_locking(&pool, account_a, account_b).await?;
    demo_lock_options(&pool, product_id).await?;
    demo_isolation_levels(&pool).await?;
    demo_deadlock_prevention(&pool).await?;

    println!("\n=== All transaction/locking demos completed successfully! ===");
    Ok(())
}
