//! 正規化と非正規化の判断基準デモ
//!
//! このデモでは以下を検証:
//! 1. 第1正規形（1NF）- 原子値、繰り返しグループなし
//! 2. 第2正規形（2NF）- 部分関数従属なし
//! 3. 第3正規形（3NF）- 推移的関数従属なし
//! 4. 非正規化パターン - スナップショット、集計キャッシュ、マテリアライズドビュー

use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// 1NF: 第1正規形の構造体
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderBad {
    id: Uuid,
    customer_name: String,
    products: String, // カンマ区切り - 1NF違反
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Order {
    id: Uuid,
    customer_name: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderItem {
    id: Uuid,
    order_id: Uuid,
    product_name: String,
}

// ================================
// 3NF: 第3正規形の構造体
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Department {
    id: Uuid,
    name: String,
    location: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Employee {
    id: Uuid,
    name: String,
    department_id: Uuid,
}

// ================================
// 非正規化: スナップショット付き注文明細
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    price: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderItemWithSnapshot {
    id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
    product_name_snapshot: String,
    product_price_snapshot: Decimal,
    quantity: i32,
}

// ================================
// 非正規化: 集計キャッシュ付きユーザー
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserWithStats {
    id: Uuid,
    name: String,
    total_orders: i32,
    total_spent: Decimal,
}

// ================================
// マテリアライズドビュー用構造体
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserOrderStats {
    id: Uuid,
    name: String,
    total_orders: i64,
    total_spent: Decimal,
    last_order_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP MATERIALIZED VIEW IF EXISTS user_order_stats CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS order_items_snapshot CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS user_orders CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users_with_stats CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS order_items CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders_bad CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS employees CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS departments CASCADE")
        .execute(pool)
        .await?;

    // === 1NF違反テーブル ===
    sqlx::query(
        r#"
        CREATE TABLE orders_bad (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            customer_name TEXT NOT NULL,
            products TEXT  -- "商品A, 商品B, 商品C" - 1NF違反
        )
        "#,
    )
    .execute(pool)
    .await?;

    // === 1NF準拠テーブル ===
    sqlx::query(
        r#"
        CREATE TABLE orders (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            customer_name TEXT NOT NULL,
            total DECIMAL(12,2) DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE order_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
            product_name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // === 3NF: 部門と従業員 ===
    sqlx::query(
        r#"
        CREATE TABLE departments (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            location TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE employees (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            department_id UUID NOT NULL REFERENCES departments(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // === 非正規化: 商品とスナップショット付き注文明細 ===
    sqlx::query(
        r#"
        CREATE TABLE products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            price DECIMAL(10,2) NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE order_items_snapshot (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
            product_id UUID NOT NULL REFERENCES products(id),
            product_name_snapshot TEXT NOT NULL,
            product_price_snapshot DECIMAL(10,2) NOT NULL,
            quantity INT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // === 非正規化: 集計キャッシュ付きユーザー ===
    sqlx::query(
        r#"
        CREATE TABLE users_with_stats (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            total_orders INT DEFAULT 0,
            total_spent DECIMAL(12,2) DEFAULT 0
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE user_orders (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users_with_stats(id),
            total DECIMAL(12,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 集計キャッシュ更新トリガー
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_user_stats() RETURNS TRIGGER AS $$
        BEGIN
            UPDATE users_with_stats
            SET total_orders = total_orders + 1,
                total_spent = total_spent + NEW.total
            WHERE id = NEW.user_id;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TRIGGER IF EXISTS user_orders_insert ON user_orders")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER user_orders_insert
            AFTER INSERT ON user_orders
            FOR EACH ROW
            EXECUTE FUNCTION update_user_stats()
        "#,
    )
    .execute(pool)
    .await?;

    // === マテリアライズドビュー ===
    sqlx::query(
        r#"
        CREATE MATERIALIZED VIEW user_order_stats AS
        SELECT
            u.id,
            u.name,
            COUNT(o.id)::BIGINT as total_orders,
            COALESCE(SUM(o.total), 0)::DECIMAL(12,2) as total_spent,
            MAX(o.created_at) as last_order_at
        FROM users_with_stats u
        LEFT JOIN user_orders o ON u.id = o.user_id
        GROUP BY u.id, u.name
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE UNIQUE INDEX idx_user_order_stats_id ON user_order_stats(id)")
        .execute(pool)
        .await?;

    println!("Database setup completed");
    Ok(())
}

// ================================
// 1NFデモ
// ================================

async fn demo_1nf(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: First Normal Form (1NF) ===");

    // 1NF違反: カンマ区切りで商品を格納
    sqlx::query("INSERT INTO orders_bad (customer_name, products) VALUES ($1, $2)")
        .bind("田中太郎")
        .bind("Laptop, Mouse, Keyboard") // 1NF違反
        .execute(pool)
        .await?;

    let bad_order: OrderBad =
        sqlx::query_as("SELECT id, customer_name, products FROM orders_bad LIMIT 1")
            .fetch_one(pool)
            .await?;

    println!("1NF Violation:");
    println!("  Order: {:?}", bad_order);
    println!("  Problem: Products stored as comma-separated string");

    // カンマ区切りをパースする必要がある（面倒）
    let products: Vec<&str> = bad_order.products.split(", ").collect();
    println!("  Parsed products: {:?}", products);

    // 1NF準拠: 別テーブルで管理
    let order_id: Uuid =
        sqlx::query_scalar("INSERT INTO orders (customer_name) VALUES ($1) RETURNING id")
            .bind("鈴木花子")
            .fetch_one(pool)
            .await?;

    for product_name in &["Laptop", "Mouse", "Keyboard"] {
        sqlx::query("INSERT INTO order_items (order_id, product_name) VALUES ($1, $2)")
            .bind(order_id)
            .bind(*product_name)
            .execute(pool)
            .await?;
    }

    let items: Vec<OrderItem> =
        sqlx::query_as("SELECT id, order_id, product_name FROM order_items WHERE order_id = $1")
            .bind(order_id)
            .fetch_all(pool)
            .await?;

    println!("\n1NF Compliant:");
    println!("  Order ID: {}", order_id);
    println!("  Items:");
    for item in &items {
        println!("    - {}", item.product_name);
    }

    Ok(())
}

// ================================
// 3NFデモ
// ================================

async fn demo_3nf(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Third Normal Form (3NF) ===");

    // 部門を作成
    let dept_id: Uuid =
        sqlx::query_scalar("INSERT INTO departments (name, location) VALUES ($1, $2) RETURNING id")
            .bind("Engineering")
            .bind("Tokyo")
            .fetch_one(pool)
            .await?;

    println!("Created department: Engineering (Tokyo) - {}", dept_id);

    // 従業員を作成
    for name in &["Alice", "Bob", "Charlie"] {
        sqlx::query("INSERT INTO employees (name, department_id) VALUES ($1, $2)")
            .bind(*name)
            .bind(dept_id)
            .execute(pool)
            .await?;
    }

    println!("Created 3 employees in Engineering department");

    // 3NF: 部門情報を変更しても1行の更新で済む
    sqlx::query("UPDATE departments SET location = $1 WHERE id = $2")
        .bind("Osaka")
        .bind(dept_id)
        .execute(pool)
        .await?;

    println!("Updated department location to Osaka (1 row update)");

    // JOINで従業員と部門情報を取得
    let result: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT e.name, d.name, d.location
        FROM employees e
        JOIN departments d ON e.department_id = d.id
        WHERE d.id = $1
        "#,
    )
    .bind(dept_id)
    .fetch_all(pool)
    .await?;

    println!("\nEmployees with department info (via JOIN):");
    for (emp_name, dept_name, location) in &result {
        println!("  {} - {} ({})", emp_name, dept_name, location);
    }

    // 削除異常の防止: 従業員を全員削除しても部門は残る
    sqlx::query("DELETE FROM employees WHERE department_id = $1")
        .bind(dept_id)
        .execute(pool)
        .await?;

    let dept: Department =
        sqlx::query_as("SELECT id, name, location FROM departments WHERE id = $1")
            .bind(dept_id)
            .fetch_one(pool)
            .await?;

    println!("\nAfter deleting all employees, department still exists:");
    println!("  {:?}", dept);

    Ok(())
}

// ================================
// 非正規化: スナップショットデモ
// ================================

async fn demo_snapshot(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Denormalization - Snapshot Pattern ===");

    // 商品作成
    let product_id: Uuid =
        sqlx::query_scalar("INSERT INTO products (name, price) VALUES ($1, $2) RETURNING id")
            .bind("Premium Laptop")
            .bind(Decimal::new(99999, 2)) // 999.99
            .fetch_one(pool)
            .await?;

    println!("Created product: Premium Laptop @ 999.99");

    // 注文作成
    let order_id: Uuid =
        sqlx::query_scalar("INSERT INTO orders (customer_name) VALUES ($1) RETURNING id")
            .bind("Customer A")
            .fetch_one(pool)
            .await?;

    // 注文時点の商品情報を取得
    let product: Product = sqlx::query_as("SELECT id, name, price FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await?;

    // スナップショット付きで注文明細を作成
    sqlx::query(
        r#"
        INSERT INTO order_items_snapshot
        (order_id, product_id, product_name_snapshot, product_price_snapshot, quantity)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(order_id)
    .bind(product_id)
    .bind(&product.name)
    .bind(product.price)
    .bind(2)
    .execute(pool)
    .await?;

    println!(
        "Created order item with snapshot: {} @ {}",
        product.name, product.price
    );

    // 商品価格を変更
    sqlx::query("UPDATE products SET price = $1 WHERE id = $2")
        .bind(Decimal::new(89999, 2)) // 899.99
        .bind(product_id)
        .execute(pool)
        .await?;

    println!("Updated product price to 899.99");

    // 注文明細は元の価格を保持
    let order_item: OrderItemWithSnapshot = sqlx::query_as(
        r#"
        SELECT id, order_id, product_id, product_name_snapshot, product_price_snapshot, quantity
        FROM order_items_snapshot
        WHERE order_id = $1
        "#,
    )
    .bind(order_id)
    .fetch_one(pool)
    .await?;

    println!("\nOrder item still shows original price:");
    println!(
        "  Product: {} @ {} (qty: {})",
        order_item.product_name_snapshot, order_item.product_price_snapshot, order_item.quantity
    );

    let current_product: Product =
        sqlx::query_as("SELECT id, name, price FROM products WHERE id = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await?;

    println!("  Current product price: {}", current_product.price);

    Ok(())
}

// ================================
// 非正規化: 集計キャッシュデモ
// ================================

async fn demo_aggregate_cache(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Denormalization - Aggregate Cache ===");

    // ユーザー作成（集計キャッシュ付き）
    let user_id: Uuid =
        sqlx::query_scalar("INSERT INTO users_with_stats (name) VALUES ($1) RETURNING id")
            .bind("Heavy Shopper")
            .fetch_one(pool)
            .await?;

    println!("Created user: Heavy Shopper");

    // 注文を追加（トリガーが集計を更新）
    for i in 1..=5 {
        let total = Decimal::new(i * 10000, 2); // 100.00, 200.00, ...
        sqlx::query("INSERT INTO user_orders (user_id, total) VALUES ($1, $2)")
            .bind(user_id)
            .bind(total)
            .execute(pool)
            .await?;
        println!("  Order {}: {}", i, total);
    }

    // 集計キャッシュを確認（JOINなしで高速に取得）
    let user: UserWithStats = sqlx::query_as(
        "SELECT id, name, total_orders, total_spent FROM users_with_stats WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    println!("\nUser stats (from cache, no JOIN needed):");
    println!("  Name: {}", user.name);
    println!("  Total Orders: {}", user.total_orders);
    println!("  Total Spent: {}", user.total_spent);

    // 実際の集計と比較
    let actual: (i64, Decimal) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT, COALESCE(SUM(total), 0) FROM user_orders WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    println!("\nActual stats (from aggregation):");
    println!("  Total Orders: {}", actual.0);
    println!("  Total Spent: {}", actual.1);

    assert_eq!(user.total_orders as i64, actual.0);
    assert_eq!(user.total_spent, actual.1);
    println!("\nCache is consistent with actual data!");

    Ok(())
}

// ================================
// マテリアライズドビューデモ
// ================================

async fn demo_materialized_view(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Denormalization - Materialized View ===");

    // マテリアライズドビューをリフレッシュ
    sqlx::query("REFRESH MATERIALIZED VIEW user_order_stats")
        .execute(pool)
        .await?;

    println!("Refreshed materialized view");

    // マテリアライズドビューから取得（高速）
    let stats: Vec<UserOrderStats> = sqlx::query_as(
        "SELECT id, name, total_orders, total_spent, last_order_at FROM user_order_stats",
    )
    .fetch_all(pool)
    .await?;

    println!("\nUser order stats (from materialized view):");
    for stat in &stats {
        println!(
            "  {} - Orders: {}, Spent: {}, Last: {:?}",
            stat.name, stat.total_orders, stat.total_spent, stat.last_order_at
        );
    }

    // CONCURRENTLYでリフレッシュ（読み取りをブロックしない）
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY user_order_stats")
        .execute(pool)
        .await?;

    println!("\nRefreshed materialized view CONCURRENTLY (no read blocking)");

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

    demo_1nf(&pool).await?;
    demo_3nf(&pool).await?;
    demo_snapshot(&pool).await?;
    demo_aggregate_cache(&pool).await?;
    demo_materialized_view(&pool).await?;

    println!("\n=== All normalization demos completed successfully! ===");
    Ok(())
}
