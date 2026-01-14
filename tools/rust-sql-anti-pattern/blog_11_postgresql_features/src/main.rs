//! PostgreSQL固有機能のデモ
//!
//! このデモでは以下を検証:
//! 1. LISTEN/NOTIFY（簡易版）
//! 2. Advisory Locks（分散ロック）
//! 3. JSONB操作
//! 4. DISTINCT ON
//! 5. 生成列（Generated Columns）

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, Serialize, Deserialize)]
struct ProductAttributes {
    color: Option<String>,
    size: Option<String>,
    weight: Option<f64>,
    tags: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    attributes: Json<ProductAttributes>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Order {
    id: Uuid,
    user_id: Uuid,
    status: String,
    total: Decimal,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderWithTax {
    id: Uuid,
    subtotal: Decimal,
    tax_rate: Decimal,
    tax_amount: Decimal,
    total: Decimal,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS jsonb_products CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders_with_tax CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS user_orders CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS feature_users CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE feature_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 注文テーブル
    sqlx::query(
        r#"
        CREATE TABLE user_orders (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES feature_users(id),
            status VARCHAR(20) NOT NULL DEFAULT 'pending',
            total DECIMAL(10,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // JSONBプロダクトテーブル
    sqlx::query(
        r#"
        CREATE TABLE jsonb_products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(200) NOT NULL,
            attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // GINインデックス
    sqlx::query("CREATE INDEX idx_products_attributes ON jsonb_products USING GIN (attributes)")
        .execute(pool)
        .await?;

    // 生成列を持つテーブル
    sqlx::query(
        r#"
        CREATE TABLE orders_with_tax (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            subtotal DECIMAL(10,2) NOT NULL,
            tax_rate DECIMAL(5,4) NOT NULL DEFAULT 0.10,
            tax_amount DECIMAL(10,2) GENERATED ALWAYS AS (subtotal * tax_rate) STORED,
            total DECIMAL(10,2) GENERATED ALWAYS AS (subtotal * (1 + tax_rate)) STORED
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

// ================================
// Demo: Advisory Locks
// ================================

async fn demo_advisory_locks(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Advisory Locks ===");

    let lock_id: i64 = 12345;

    // ノンブロッキングでロック取得
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(lock_id)
        .fetch_one(pool)
        .await?;

    println!("Lock {} acquired: {}", lock_id, acquired);

    if acquired {
        println!("Performing exclusive operation...");

        // 排他処理をシミュレート
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // ロックを解放
        let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
            .bind(lock_id)
            .fetch_one(pool)
            .await?;

        println!("Lock {} released: {}", lock_id, released);
    }

    // トランザクションレベルのロック
    println!("\n--- Transaction-level lock ---");
    {
        let mut tx = pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(lock_id)
            .execute(&mut *tx)
            .await?;
        println!("Transaction lock acquired");

        // トランザクション内での処理
        println!("Performing transaction work...");

        tx.commit().await?;
        println!("Transaction committed, lock auto-released");
    }

    Ok(())
}

// ================================
// Demo: JSONB操作
// ================================

async fn demo_jsonb(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: JSONB Operations ===");

    // JSONB商品を作成
    let attrs = ProductAttributes {
        color: Some("red".to_string()),
        size: Some("large".to_string()),
        weight: Some(1.5),
        tags: vec!["sale".to_string(), "new".to_string()],
    };

    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO jsonb_products (name, attributes) VALUES ($1, $2) RETURNING id",
    )
    .bind("T-Shirt")
    .bind(Json(&attrs))
    .fetch_one(pool)
    .await?;

    println!("Created product: {}", product_id);

    // 別の商品も追加
    let attrs2 = ProductAttributes {
        color: Some("blue".to_string()),
        size: Some("medium".to_string()),
        weight: Some(0.8),
        tags: vec!["sale".to_string()],
    };

    sqlx::query("INSERT INTO jsonb_products (name, attributes) VALUES ($1, $2)")
        .bind("Pants")
        .bind(Json(&attrs2))
        .execute(pool)
        .await?;

    // JSONB内のフィールドで検索
    println!("\n--- Search by color ---");
    let red_products: Vec<Product> = sqlx::query_as(
        r#"
        SELECT id, name, attributes
        FROM jsonb_products
        WHERE attributes->>'color' = $1
        "#,
    )
    .bind("red")
    .fetch_all(pool)
    .await?;

    for p in &red_products {
        println!("  {} - color: {:?}", p.name, p.attributes.0.color);
    }

    // JSONB配列内を検索
    println!("\n--- Search by tag ---");
    let sale_products: Vec<Product> = sqlx::query_as(
        r#"
        SELECT id, name, attributes
        FROM jsonb_products
        WHERE attributes->'tags' ? $1
        "#,
    )
    .bind("sale")
    .fetch_all(pool)
    .await?;

    println!("Products with 'sale' tag: {}", sale_products.len());
    for p in &sale_products {
        println!("  {} - tags: {:?}", p.name, p.attributes.0.tags);
    }

    // JSONB更新
    println!("\n--- Update JSONB field ---");
    sqlx::query(
        r#"
        UPDATE jsonb_products
        SET attributes = jsonb_set(attributes, '{color}', '"green"'::jsonb)
        WHERE id = $1
        "#,
    )
    .bind(product_id)
    .execute(pool)
    .await?;

    let updated: Product = sqlx::query_as(
        "SELECT id, name, attributes FROM jsonb_products WHERE id = $1",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;

    println!("Updated color: {:?}", updated.attributes.0.color);

    // タグを追加
    sqlx::query(
        r#"
        UPDATE jsonb_products
        SET attributes = jsonb_set(
            attributes,
            '{tags}',
            COALESCE(attributes->'tags', '[]'::jsonb) || '["featured"]'::jsonb
        )
        WHERE id = $1
        "#,
    )
    .bind(product_id)
    .execute(pool)
    .await?;

    let with_new_tag: Product = sqlx::query_as(
        "SELECT id, name, attributes FROM jsonb_products WHERE id = $1",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await?;

    println!("Tags after addition: {:?}", with_new_tag.attributes.0.tags);

    Ok(())
}

// ================================
// Demo: DISTINCT ON
// ================================

async fn demo_distinct_on(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: DISTINCT ON ===");

    // サンプルデータ作成
    let user1: Uuid = sqlx::query_scalar(
        "INSERT INTO feature_users (name) VALUES ($1) RETURNING id",
    )
    .bind("Alice")
    .fetch_one(pool)
    .await?;

    let user2: Uuid = sqlx::query_scalar(
        "INSERT INTO feature_users (name) VALUES ($1) RETURNING id",
    )
    .bind("Bob")
    .fetch_one(pool)
    .await?;

    // 各ユーザーに複数の注文
    for (user_id, amounts) in [
        (user1, vec![100, 200, 150]),
        (user2, vec![50, 300]),
    ] {
        for amount in amounts {
            sqlx::query("INSERT INTO user_orders (user_id, total) VALUES ($1, $2)")
                .bind(user_id)
                .bind(Decimal::new(amount * 100, 2))
                .execute(pool)
                .await?;
            // 時間差をつける
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    // 各ユーザーの最新注文を取得
    let latest_orders: Vec<Order> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (user_id)
            id, user_id, status, total, created_at
        FROM user_orders
        ORDER BY user_id, created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Latest order per user:");
    for order in &latest_orders {
        println!("  User {} - {} ({})", order.user_id, order.total, order.status);
    }

    Ok(())
}

// ================================
// Demo: Generated Columns
// ================================

async fn demo_generated_columns(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Generated Columns ===");

    // 生成列を持つテーブルにデータ挿入
    let order: OrderWithTax = sqlx::query_as(
        r#"
        INSERT INTO orders_with_tax (subtotal)
        VALUES ($1)
        RETURNING id, subtotal, tax_rate, tax_amount, total
        "#,
    )
    .bind(Decimal::new(10000, 2)) // 100.00
    .fetch_one(pool)
    .await?;

    println!("Order with auto-calculated tax:");
    println!("  Subtotal: {}", order.subtotal);
    println!("  Tax rate: {}", order.tax_rate);
    println!("  Tax amount: {} (auto-calculated)", order.tax_amount);
    println!("  Total: {} (auto-calculated)", order.total);

    // 異なる税率で作成
    let order2: OrderWithTax = sqlx::query_as(
        r#"
        INSERT INTO orders_with_tax (subtotal, tax_rate)
        VALUES ($1, $2)
        RETURNING id, subtotal, tax_rate, tax_amount, total
        "#,
    )
    .bind(Decimal::new(20000, 2)) // 200.00
    .bind(Decimal::new(800, 4))   // 0.08 = 8%
    .fetch_one(pool)
    .await?;

    println!("\nOrder with 8% tax:");
    println!("  Subtotal: {}", order2.subtotal);
    println!("  Tax rate: {}", order2.tax_rate);
    println!("  Tax amount: {} (auto-calculated)", order2.tax_amount);
    println!("  Total: {} (auto-calculated)", order2.total);

    Ok(())
}

// ================================
// Demo: NOTIFY/LISTEN（簡易版）
// ================================

async fn demo_notify(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: NOTIFY (sending) ===");

    // 通知を送信
    let payload = serde_json::json!({
        "event_type": "order_created",
        "order_id": Uuid::new_v4(),
        "timestamp": Utc::now().to_rfc3339()
    });

    sqlx::query("SELECT pg_notify($1, $2)")
        .bind("orders")
        .bind(payload.to_string())
        .execute(pool)
        .await?;

    println!("Notification sent to 'orders' channel");
    println!("Payload: {}", payload);
    println!("\nNote: To receive notifications, use PgListener in a separate task");

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

    demo_advisory_locks(&pool).await?;
    demo_jsonb(&pool).await?;
    demo_distinct_on(&pool).await?;
    demo_generated_columns(&pool).await?;
    demo_notify(&pool).await?;

    println!("\n=== All PostgreSQL feature demos completed successfully! ===");
    Ok(())
}
