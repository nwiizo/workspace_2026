//! ストアドプロシージャとの正しい付き合い方のデモ
//!
//! このデモでは以下を検証:
//! 1. PostgreSQL FUNCTION の作成と呼び出し
//! 2. PostgreSQL PROCEDURE の作成と呼び出し
//! 3. トリガーによるデータ整合性の保証
//! 4. テーブルを返す関数（RETURNS TABLE）
//! 5. OUTパラメータを持つ関数
//! 6. 監査ログトリガー

use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// データ型定義
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct SalesReport {
    product_id: Uuid,
    product_name: String,
    total_quantity: i64,
    total_revenue: Decimal,
    avg_price: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct UserStats {
    total_orders: i32,
    total_spent: Decimal,
    last_order_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PriceUpdate {
    product_id: Uuid,
    new_price: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct AuditLog {
    id: i64,
    table_name: String,
    record_id: Uuid,
    action: String,
    old_values: Option<serde_json::Value>,
    new_values: Option<serde_json::Value>,
    changed_by: Option<Uuid>,
    changed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    price: Decimal,
    stock: i32,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS audit_log CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS order_items CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS orders_archive CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TYPE IF EXISTS user_tier CASCADE")
        .execute(pool)
        .await?;

    // ユーザーティア ENUM
    sqlx::query("CREATE TYPE user_tier AS ENUM ('bronze', 'silver', 'gold')")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            tier user_tier NOT NULL DEFAULT 'bronze',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 商品テーブル
    sqlx::query(
        r#"
        CREATE TABLE products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            price DECIMAL(10,2) NOT NULL,
            stock INT NOT NULL DEFAULT 0,
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
            user_id UUID NOT NULL REFERENCES users(id),
            total DECIMAL(12,2) NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 注文明細テーブル
    sqlx::query(
        r#"
        CREATE TABLE order_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
            product_id UUID NOT NULL REFERENCES products(id),
            quantity INT NOT NULL,
            price DECIMAL(10,2) NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // アーカイブテーブル
    sqlx::query(
        r#"
        CREATE TABLE orders_archive (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL,
            total DECIMAL(12,2) NOT NULL,
            status TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 監査ログテーブル
    sqlx::query(
        r#"
        CREATE TABLE audit_log (
            id BIGSERIAL PRIMARY KEY,
            table_name TEXT NOT NULL,
            record_id UUID NOT NULL,
            action TEXT NOT NULL,
            old_values JSONB,
            new_values JSONB,
            changed_by UUID,
            changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

async fn setup_functions(pool: &PgPool) -> Result<()> {
    // 1. 単純な FUNCTION
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION add_numbers(a INT, b INT) RETURNS INT AS $$
        BEGIN
            RETURN a + b;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: add_numbers function");

    // 2. 在庫チェックトリガー
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION enforce_positive_stock() RETURNS TRIGGER AS $$
        BEGIN
            IF NEW.stock < 0 THEN
                RAISE EXCEPTION 'Stock cannot be negative: got %', NEW.stock;
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TRIGGER IF EXISTS check_stock_trigger ON products")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER check_stock_trigger
            BEFORE UPDATE ON products
            FOR EACH ROW
            EXECUTE FUNCTION enforce_positive_stock()
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: enforce_positive_stock trigger");

    // 3. 注文合計自動更新トリガー
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_order_total() RETURNS TRIGGER AS $$
        BEGIN
            UPDATE orders
            SET total = (
                SELECT COALESCE(SUM(price * quantity), 0)
                FROM order_items
                WHERE order_id = COALESCE(NEW.order_id, OLD.order_id)
            )
            WHERE id = COALESCE(NEW.order_id, OLD.order_id);
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TRIGGER IF EXISTS order_items_changed ON order_items")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER order_items_changed
            AFTER INSERT OR UPDATE OR DELETE ON order_items
            FOR EACH ROW
            EXECUTE FUNCTION update_order_total()
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: update_order_total trigger");

    // 4. 売上レポート関数（RETURNS TABLE）
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION get_sales_report(
            start_date DATE,
            end_date DATE
        ) RETURNS TABLE (
            product_id UUID,
            product_name TEXT,
            total_quantity BIGINT,
            total_revenue DECIMAL(12,2),
            avg_price DECIMAL(10,2)
        ) AS $$
        BEGIN
            RETURN QUERY
            SELECT
                p.id,
                p.name,
                COALESCE(SUM(oi.quantity)::BIGINT, 0::BIGINT),
                COALESCE(SUM(oi.price * oi.quantity), 0::DECIMAL(12,2)),
                COALESCE(AVG(oi.price), 0::DECIMAL(10,2))
            FROM products p
            LEFT JOIN order_items oi ON oi.product_id = p.id
            LEFT JOIN orders o ON oi.order_id = o.id
                AND o.created_at::DATE BETWEEN start_date AND end_date
            GROUP BY p.id, p.name
            ORDER BY COALESCE(SUM(oi.price * oi.quantity), 0) DESC;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: get_sales_report function");

    // 5. ユーザー統計関数（OUT パラメータ）
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION get_user_stats(
            p_user_id UUID,
            OUT total_orders INT,
            OUT total_spent DECIMAL(12,2),
            OUT last_order_date DATE
        ) AS $$
        BEGIN
            SELECT
                COUNT(*)::INT,
                COALESCE(SUM(total), 0),
                MAX(created_at)::DATE
            INTO total_orders, total_spent, last_order_date
            FROM orders
            WHERE user_id = p_user_id;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: get_user_stats function");

    // 6. バルク価格更新プロシージャ
    sqlx::query(
        r#"
        CREATE OR REPLACE PROCEDURE bulk_update_prices(updates JSONB) AS $$
        DECLARE
            item JSONB;
        BEGIN
            FOR item IN SELECT * FROM jsonb_array_elements(updates) LOOP
                UPDATE products
                SET price = (item->>'new_price')::DECIMAL
                WHERE id = (item->>'product_id')::UUID;
            END LOOP;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: bulk_update_prices procedure");

    // 7. 古い注文アーカイブプロシージャ
    sqlx::query(
        r#"
        CREATE OR REPLACE PROCEDURE archive_old_orders(cutoff_date DATE) AS $$
        BEGIN
            INSERT INTO orders_archive (id, user_id, total, status, created_at)
            SELECT id, user_id, total, status, created_at
            FROM orders
            WHERE created_at::DATE < cutoff_date;

            DELETE FROM orders WHERE created_at::DATE < cutoff_date;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: archive_old_orders procedure");

    // 8. 監査ログトリガー
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION audit_trigger() RETURNS TRIGGER AS $$
        BEGIN
            INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, changed_by)
            VALUES (
                TG_TABLE_NAME,
                COALESCE(NEW.id, OLD.id),
                TG_OP,
                CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN to_jsonb(OLD) END,
                CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN to_jsonb(NEW) END,
                NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
            );
            RETURN COALESCE(NEW, OLD);
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TRIGGER IF EXISTS orders_audit ON orders")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER orders_audit
            AFTER INSERT OR UPDATE OR DELETE ON orders
            FOR EACH ROW EXECUTE FUNCTION audit_trigger()
        "#,
    )
    .execute(pool)
    .await?;
    println!("Created: audit_trigger");

    Ok(())
}

// ================================
// FUNCTION呼び出しデモ
// ================================

async fn demo_simple_function(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Simple Function (add_numbers) ===");

    let result: i32 = sqlx::query_scalar("SELECT add_numbers(10, 20)")
        .fetch_one(pool)
        .await?;

    println!("add_numbers(10, 20) = {}", result);
    assert_eq!(result, 30);

    Ok(())
}

async fn demo_sales_report(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Table-Returning Function (get_sales_report) ===");

    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();

    let reports: Vec<SalesReport> = sqlx::query_as(
        r#"
        SELECT
            product_id,
            product_name,
            total_quantity,
            total_revenue,
            avg_price
        FROM get_sales_report($1, $2)
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    println!("Sales Report ({} to {}):", start_date, end_date);
    for report in &reports {
        println!(
            "  {} - Qty: {}, Revenue: {}, Avg Price: {}",
            report.product_name, report.total_quantity, report.total_revenue, report.avg_price
        );
    }

    Ok(())
}

async fn demo_user_stats(pool: &PgPool, user_id: Uuid) -> Result<()> {
    println!("\n=== Demo: OUT Parameter Function (get_user_stats) ===");

    let stats: UserStats = sqlx::query_as("SELECT * FROM get_user_stats($1)")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    println!("User Stats for {}:", user_id);
    println!("  Total Orders: {}", stats.total_orders);
    println!("  Total Spent: {}", stats.total_spent);
    println!(
        "  Last Order Date: {}",
        stats
            .last_order_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    );

    Ok(())
}

// ================================
// PROCEDURE呼び出しデモ
// ================================

async fn demo_bulk_update_prices(pool: &PgPool, updates: &[PriceUpdate]) -> Result<()> {
    println!("\n=== Demo: Procedure (bulk_update_prices) ===");

    let updates_json = serde_json::to_value(updates)?;
    println!("Updating prices with: {}", updates_json);

    sqlx::query("CALL bulk_update_prices($1::jsonb)")
        .bind(updates_json)
        .execute(pool)
        .await?;

    println!("Prices updated successfully");

    // 確認
    let products: Vec<Product> = sqlx::query_as("SELECT id, name, price, stock FROM products")
        .fetch_all(pool)
        .await?;

    for p in &products {
        println!("  {} - Price: {}", p.name, p.price);
    }

    Ok(())
}

async fn demo_archive_orders(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Procedure (archive_old_orders) ===");

    let cutoff_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    println!("Archiving orders before {}...", cutoff_date);

    sqlx::query("CALL archive_old_orders($1)")
        .bind(cutoff_date)
        .execute(pool)
        .await?;

    let archived_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders_archive")
        .fetch_one(pool)
        .await?;

    println!("Archived {} orders", archived_count);

    Ok(())
}

// ================================
// トリガーデモ
// ================================

async fn demo_stock_trigger(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Stock Enforcement Trigger ===");

    // 商品追加
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (name, price, stock) VALUES ('Test Product', 100, 10) RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    println!("Created product with stock = 10");

    // 正常な更新
    sqlx::query("UPDATE products SET stock = 5 WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await?;
    println!("Updated stock to 5: OK");

    // 在庫をマイナスにしようとする（失敗するはず）
    let result = sqlx::query("UPDATE products SET stock = -1 WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await;

    match result {
        Ok(_) => println!("ERROR: Should have failed but succeeded!"),
        Err(e) => {
            println!("Expected error caught: Stock cannot be negative");
            assert!(e.to_string().contains("Stock cannot be negative"));
        }
    }

    Ok(())
}

async fn demo_order_total_trigger(pool: &PgPool, user_id: Uuid) -> Result<()> {
    println!("\n=== Demo: Order Total Auto-Update Trigger ===");

    // 商品を取得
    let products: Vec<Product> = sqlx::query_as("SELECT id, name, price, stock FROM products")
        .fetch_all(pool)
        .await?;

    if products.is_empty() {
        println!("No products found, skipping demo");
        return Ok(());
    }

    // 注文作成（total は 0 で開始）
    let order_id: Uuid =
        sqlx::query_scalar("INSERT INTO orders (user_id, total) VALUES ($1, 0) RETURNING id")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    println!("Created order {} (initial total: 0)", order_id);

    // 注文明細を追加（トリガーが合計を更新するはず）
    for (i, product) in products.iter().take(2).enumerate() {
        let qty = (i + 1) as i32;
        sqlx::query(
            "INSERT INTO order_items (order_id, product_id, quantity, price) VALUES ($1, $2, $3, $4)",
        )
        .bind(order_id)
        .bind(product.id)
        .bind(qty)
        .bind(product.price)
        .execute(pool)
        .await?;

        println!(
            "  Added item: {} x {} @ {}",
            product.name, qty, product.price
        );

        // 合計を確認
        let total: Decimal = sqlx::query_scalar("SELECT total FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(pool)
            .await?;

        println!("  Current order total: {}", total);
    }

    Ok(())
}

// ================================
// 監査ログデモ
// ================================

async fn demo_audit_log(pool: &PgPool, user_id: Uuid) -> Result<()> {
    println!("\n=== Demo: Audit Log Trigger ===");

    // セッション変数でユーザーIDを設定
    let mut tx = pool.begin().await?;

    sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_id))
        .execute(&mut *tx)
        .await?;

    // 注文を作成（INSERT が記録される）
    let order_id: Uuid =
        sqlx::query_scalar("INSERT INTO orders (user_id, total) VALUES ($1, 100) RETURNING id")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    println!("Created order {}", order_id);

    // 注文を更新（UPDATE が記録される）
    sqlx::query("UPDATE orders SET total = 200 WHERE id = $1")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

    println!("Updated order total to 200");

    tx.commit().await?;

    // 監査ログを確認
    let logs: Vec<AuditLog> = sqlx::query_as(
        r#"
        SELECT id, table_name, record_id, action, old_values, new_values, changed_by, changed_at
        FROM audit_log
        WHERE record_id = $1
        ORDER BY id
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    println!("\nAudit Log Entries:");
    for log in &logs {
        println!(
            "  ID: {}, Action: {}, Table: {}",
            log.id, log.action, log.table_name
        );
        println!("    Changed by: {:?}", log.changed_by);
        if let Some(old) = &log.old_values {
            println!("    Old: {}", old);
        }
        if let Some(new) = &log.new_values {
            println!("    New: {}", new);
        }
    }

    assert_eq!(logs.len(), 2); // INSERT + UPDATE
    assert_eq!(logs[0].action, "INSERT");
    assert_eq!(logs[1].action, "UPDATE");

    Ok(())
}

// ================================
// サンプルデータ作成
// ================================

async fn create_sample_data(pool: &PgPool) -> Result<Uuid> {
    println!("\n=== Creating Sample Data ===");

    // ユーザー作成
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (name, email, tier) VALUES ('Test User', 'test@example.com', 'gold') RETURNING id",
    )
    .fetch_one(pool)
    .await?;
    println!("Created user: {}", user_id);

    // 商品作成
    let products = vec![
        ("Laptop", Decimal::new(99999, 2), 50),     // 999.99
        ("Mouse", Decimal::new(2999, 2), 100),      // 29.99
        ("Keyboard", Decimal::new(7999, 2), 75),    // 79.99
        ("Monitor", Decimal::new(29999, 2), 30),    // 299.99
        ("Headphones", Decimal::new(14999, 2), 60), // 149.99
    ];

    for (name, price, stock) in products {
        sqlx::query("INSERT INTO products (name, price, stock) VALUES ($1, $2, $3)")
            .bind(name)
            .bind(price)
            .bind(stock)
            .execute(pool)
            .await?;
        println!("Created product: {} @ {} (stock: {})", name, price, stock);
    }

    // 過去の注文を作成（アーカイブテスト用）
    sqlx::query(
        r#"
        INSERT INTO orders (user_id, total, created_at)
        VALUES
            ($1, 100.00, '2024-01-15'::TIMESTAMPTZ),
            ($1, 200.00, '2024-03-20'::TIMESTAMPTZ),
            ($1, 150.00, '2024-05-10'::TIMESTAMPTZ),
            ($1, 300.00, '2024-07-25'::TIMESTAMPTZ),
            ($1, 250.00, '2024-11-05'::TIMESTAMPTZ)
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    println!("Created 5 sample orders");

    Ok(user_id)
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

    // セットアップ
    setup_database(&pool).await?;
    setup_functions(&pool).await?;

    // サンプルデータ作成
    let user_id = create_sample_data(&pool).await?;

    // デモ実行
    demo_simple_function(&pool).await?;
    demo_sales_report(&pool).await?;
    demo_user_stats(&pool, user_id).await?;
    demo_stock_trigger(&pool).await?;
    demo_order_total_trigger(&pool, user_id).await?;
    demo_audit_log(&pool, user_id).await?;

    // バルク価格更新
    let products: Vec<Product> =
        sqlx::query_as("SELECT id, name, price, stock FROM products LIMIT 2")
            .fetch_all(&pool)
            .await?;

    if products.len() >= 2 {
        let updates: Vec<PriceUpdate> = products
            .iter()
            .map(|p| PriceUpdate {
                product_id: p.id,
                new_price: p.price * Decimal::new(110, 2), // 10% 値上げ
            })
            .collect();

        demo_bulk_update_prices(&pool, &updates).await?;
    }

    // 古い注文のアーカイブ
    demo_archive_orders(&pool).await?;

    println!("\n=== All demos completed successfully! ===");
    Ok(())
}
