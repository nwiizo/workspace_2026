//! 疑似キー潔癖症のデモ
//!
//! このデモでは以下を検証:
//! 1. SERIAL の欠番は正常動作
//! 2. UUID による ID 管理
//! 3. 表示用番号の分離
//! 4. カーソルベースページネーション

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderSerial {
    id: i32,
    customer_name: String,
    total: Decimal,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderUuid {
    id: Uuid,
    order_number: String,
    customer_name: String,
    total: Decimal,
    created_at: DateTime<Utc>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS orders_serial CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders_uuid CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP SEQUENCE IF EXISTS order_number_seq CASCADE")
        .execute(pool)
        .await?;

    // SERIALを使ったテーブル
    sqlx::query(
        r#"
        CREATE TABLE orders_serial (
            id SERIAL PRIMARY KEY,
            customer_name VARCHAR(100) NOT NULL,
            total DECIMAL(10,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // UUIDを使ったテーブル（表示用番号は別管理）
    sqlx::query(
        r#"
        CREATE TABLE orders_uuid (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            order_number VARCHAR(20) NOT NULL UNIQUE,
            customer_name VARCHAR(100) NOT NULL,
            total DECIMAL(10,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 表示用番号のシーケンス
    sqlx::query("CREATE SEQUENCE order_number_seq START 1")
        .execute(pool)
        .await?;

    // インデックス
    sqlx::query("CREATE INDEX idx_orders_uuid_created ON orders_uuid(created_at)")
        .execute(pool)
        .await?;

    println!("Tables created successfully");
    Ok(())
}

// ================================
// Demo: SERIAL の欠番
// ================================

async fn demo_serial_gaps(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: SERIAL gaps are normal ===");

    // 注文を作成
    sqlx::query("INSERT INTO orders_serial (customer_name, total) VALUES ($1, $2)")
        .bind("Customer A")
        .bind(Decimal::new(10000, 2))
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO orders_serial (customer_name, total) VALUES ($1, $2)")
        .bind("Customer B")
        .bind(Decimal::new(20000, 2))
        .execute(pool)
        .await?;

    // ID=2 の注文を削除
    sqlx::query("DELETE FROM orders_serial WHERE id = 2")
        .execute(pool)
        .await?;

    // 新しい注文を作成
    sqlx::query("INSERT INTO orders_serial (customer_name, total) VALUES ($1, $2)")
        .bind("Customer C")
        .bind(Decimal::new(30000, 2))
        .execute(pool)
        .await?;

    // 現在の注文を確認
    let orders: Vec<OrderSerial> = sqlx::query_as(
        "SELECT id, customer_name, total, created_at FROM orders_serial ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    println!("Orders (note the gap at ID=2):");
    for order in &orders {
        println!("  ID {} - {} ({})", order.id, order.customer_name, order.total);
    }

    println!("\nID=2 was deleted but NOT reused - this is correct behavior!");
    println!("Gaps can also occur from:");
    println!("  - Rolled back transactions");
    println!("  - Sequence cache on server restart");
    println!("  - Bulk insert failures");

    Ok(())
}

// ================================
// Demo: UUID による ID 管理
// ================================

async fn generate_order_number(pool: &PgPool) -> Result<String> {
    let now = Utc::now();
    let seq: i64 = sqlx::query_scalar("SELECT nextval('order_number_seq')")
        .fetch_one(pool)
        .await?;

    Ok(format!("ORD-{}{:02}-{:06}", now.year(), now.month(), seq))
}

async fn demo_uuid_with_display_number(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: UUID with display order number ===");

    // 注文を作成（UUIDは内部ID、order_numberは表示用）
    for i in 1..=3 {
        let order_number = generate_order_number(pool).await?;
        let customer_name = format!("Customer {}", i);

        sqlx::query(
            "INSERT INTO orders_uuid (order_number, customer_name, total) VALUES ($1, $2, $3)",
        )
        .bind(&order_number)
        .bind(&customer_name)
        .bind(Decimal::new(i * 10000, 2))
        .execute(pool)
        .await?;

        println!("Created order: {} for {}", order_number, customer_name);
    }

    // 注文を確認
    let orders: Vec<OrderUuid> = sqlx::query_as(
        "SELECT id, order_number, customer_name, total, created_at FROM orders_uuid ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    println!("\nOrders with UUID and display number:");
    for order in &orders {
        println!(
            "  {} (ID: {}) - {} {}",
            order.order_number, order.id, order.customer_name, order.total
        );
    }

    println!("\nBenefits of UUID:");
    println!("  - No gap concerns (there's no sequence)");
    println!("  - Can generate on client side");
    println!("  - Harder to guess/enumerate");
    println!("  - Safe for distributed systems");

    Ok(())
}

// ================================
// Demo: カーソルベースページネーション
// ================================

async fn demo_cursor_pagination(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Cursor-based pagination ===");

    // より多くの注文を追加
    for i in 4..=10 {
        let order_number = generate_order_number(pool).await?;
        sqlx::query(
            "INSERT INTO orders_uuid (order_number, customer_name, total) VALUES ($1, $2, $3)",
        )
        .bind(&order_number)
        .bind(format!("Customer {}", i))
        .bind(Decimal::new(i * 10000, 2))
        .execute(pool)
        .await?;

        // 時間差をつける
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Page 1: 最初の3件
    let page1: Vec<OrderUuid> = sqlx::query_as(
        r#"
        SELECT id, order_number, customer_name, total, created_at
        FROM orders_uuid
        ORDER BY created_at DESC
        LIMIT 3
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Page 1:");
    for order in &page1 {
        println!("  {} - {}", order.order_number, order.customer_name);
    }

    // Page 2: カーソル（最後のcreated_at）以降の3件
    if let Some(last) = page1.last() {
        let page2: Vec<OrderUuid> = sqlx::query_as(
            r#"
            SELECT id, order_number, customer_name, total, created_at
            FROM orders_uuid
            WHERE created_at < $1
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(last.created_at)
        .fetch_all(pool)
        .await?;

        println!("\nPage 2 (after cursor):");
        for order in &page2 {
            println!("  {} - {}", order.order_number, order.customer_name);
        }
    }

    println!("\nCursor pagination advantages:");
    println!("  - No offset performance issue");
    println!("  - Works with large datasets");
    println!("  - Stable results with concurrent inserts");
    println!("  - ID gaps don't matter");

    Ok(())
}

// ================================
// Demo: アンチパターン警告
// ================================

async fn demo_anti_pattern_warning(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Anti-pattern warnings ===");

    println!("Never do these:");
    println!("  1. Search for gaps to reuse IDs");
    println!("     - Causes race conditions");
    println!("     - Breaks external references");
    println!("     - Confuses audit logs");

    println!("\n  2. Renumber all IDs to fill gaps");
    println!("     - Breaks foreign keys");
    println!("     - Long table locks");
    println!("     - Invalidates caches");

    println!("\n  3. Use ID for pagination range");
    println!("     - WHERE id BETWEEN 1 AND 100 returns inconsistent counts");

    // ID範囲ページネーションの問題を示す
    let count_by_range: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders_serial WHERE id BETWEEN 1 AND 3")
            .fetch_one(pool)
            .await?;

    println!(
        "\n  Example: ID range 1-3 returns {} rows (expected 3)",
        count_by_range
    );

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

    demo_serial_gaps(&pool).await?;
    demo_uuid_with_display_number(&pool).await?;
    demo_cursor_pagination(&pool).await?;
    demo_anti_pattern_warning(&pool).await?;

    println!("\n=== All pseudokey demos completed successfully! ===");
    Ok(())
}
