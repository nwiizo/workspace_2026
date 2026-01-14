//! CTEとWindow関数で複雑なクエリを制するデモ
//!
//! このデモでは以下を検証:
//! 1. CTE（WITH句）による読みやすいクエリ
//! 2. 再帰CTEによる階層データの取得
//! 3. Window関数（ROW_NUMBER, RANK, LAG, LEAD, SUM OVER）
//! 4. 累計・移動平均の計算

use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct HighValueCustomer {
    user_id: Uuid,
    total_spent: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryWithPath {
    id: Uuid,
    name: String,
    depth: i32,
    path: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct OrderWithRank {
    id: Uuid,
    user_id: Uuid,
    total: Decimal,
    row_num: i64,
    user_rank: i64,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct SalesWithWindow {
    order_date: NaiveDate,
    daily_total: Decimal,
    running_total: Decimal,
    prev_day_total: Option<Decimal>,
    next_day_total: Option<Decimal>,
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
    sqlx::query("DROP TABLE IF EXISTS categories CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true,
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
            total DECIMAL(12,2) NOT NULL,
            status TEXT NOT NULL DEFAULT 'completed',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // カテゴリテーブル（階層構造）
    sqlx::query(
        r#"
        CREATE TABLE categories (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            parent_id UUID REFERENCES categories(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    println!("\n=== Inserting Sample Data ===");

    // ユーザーを作成
    let users = vec!["Alice", "Bob", "Charlie", "David", "Eve"];
    let mut user_ids: Vec<Uuid> = Vec::new();

    for name in users {
        let id: Uuid = sqlx::query_scalar("INSERT INTO users (name) VALUES ($1) RETURNING id")
            .bind(name)
            .fetch_one(pool)
            .await?;
        user_ids.push(id);
    }
    println!("Created {} users", user_ids.len());

    // 注文を作成（日付を分散させる）
    let base_date = chrono::Utc::now();
    for (i, user_id) in user_ids.iter().enumerate() {
        for j in 0..5 {
            let total = Decimal::new(((i + 1) * 100 + j * 50) as i64 * 100, 2);
            let days_ago = (i * 5 + j) as i64;
            let created_at = base_date - chrono::Duration::days(days_ago);

            sqlx::query("INSERT INTO orders (user_id, total, created_at) VALUES ($1, $2, $3)")
                .bind(user_id)
                .bind(total)
                .bind(created_at)
                .execute(pool)
                .await?;
        }
    }
    println!("Created 25 orders");

    // カテゴリの階層構造を作成
    let electronics: Uuid = sqlx::query_scalar(
        "INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id",
    )
    .bind("Electronics")
    .fetch_one(pool)
    .await?;

    let computers: Uuid =
        sqlx::query_scalar("INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id")
            .bind("Computers")
            .bind(electronics)
            .fetch_one(pool)
            .await?;

    let phones: Uuid =
        sqlx::query_scalar("INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id")
            .bind("Phones")
            .bind(electronics)
            .fetch_one(pool)
            .await?;

    sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, $2)")
        .bind("Laptops")
        .bind(computers)
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, $2)")
        .bind("Desktops")
        .bind(computers)
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, $2)")
        .bind("Smartphones")
        .bind(phones)
        .execute(pool)
        .await?;

    println!("Created category hierarchy");
    println!("Root category ID: {}", electronics);

    Ok(())
}

// ================================
// CTEデモ
// ================================

async fn demo_basic_cte(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Basic CTE (WITH clause) ===");

    let min_spent = Decimal::new(50000, 2); // 500.00

    let customers: Vec<HighValueCustomer> = sqlx::query_as(
        r#"
        WITH completed_orders AS (
            SELECT * FROM orders WHERE status = 'completed'
        ),
        user_totals AS (
            SELECT user_id, SUM(total) as total_spent
            FROM completed_orders
            GROUP BY user_id
        )
        SELECT user_id, total_spent
        FROM user_totals
        WHERE total_spent > $1
        ORDER BY total_spent DESC
        "#,
    )
    .bind(min_spent)
    .fetch_all(pool)
    .await?;

    println!("High value customers (spent > {}):", min_spent);
    for customer in &customers {
        println!(
            "  User {} - Total: {}",
            customer.user_id, customer.total_spent
        );
    }

    Ok(())
}

// ================================
// 再帰CTEデモ
// ================================

async fn demo_recursive_cte(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Recursive CTE (Category Tree) ===");

    // ルートカテゴリIDを取得
    let root_id: Uuid =
        sqlx::query_scalar("SELECT id FROM categories WHERE parent_id IS NULL LIMIT 1")
            .fetch_one(pool)
            .await?;

    let categories: Vec<CategoryWithPath> = sqlx::query_as(
        r#"
        WITH RECURSIVE category_tree AS (
            -- ベースケース: ルートカテゴリ
            SELECT
                id,
                name,
                0 as depth,
                name as path
            FROM categories
            WHERE id = $1

            UNION ALL

            -- 再帰ケース: 子カテゴリ
            SELECT
                c.id,
                c.name,
                ct.depth + 1,
                ct.path || ' > ' || c.name
            FROM categories c
            JOIN category_tree ct ON c.parent_id = ct.id
        )
        SELECT id, name, depth, path
        FROM category_tree
        ORDER BY path
        "#,
    )
    .bind(root_id)
    .fetch_all(pool)
    .await?;

    println!("Category tree from root {}:", root_id);
    for cat in &categories {
        let indent = "  ".repeat(cat.depth as usize);
        println!("{}{} (depth: {})", indent, cat.name, cat.depth);
    }
    println!("\nFull paths:");
    for cat in &categories {
        println!("  {}", cat.path);
    }

    Ok(())
}

// ================================
// Window関数デモ
// ================================

async fn demo_row_number_rank(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Window Functions (ROW_NUMBER, RANK) ===");

    let orders: Vec<OrderWithRank> = sqlx::query_as(
        r#"
        SELECT
            id,
            user_id,
            total,
            ROW_NUMBER() OVER (ORDER BY total DESC) as row_num,
            RANK() OVER (PARTITION BY user_id ORDER BY total DESC) as user_rank
        FROM orders
        ORDER BY total DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Top 10 orders with rankings:");
    for order in &orders {
        println!(
            "  Order {} - Total: {} (Global #{}. User's #{})",
            order.id, order.total, order.row_num, order.user_rank
        );
    }

    Ok(())
}

async fn demo_lag_lead(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Window Functions (LAG, LEAD, Running Total) ===");

    let sales: Vec<SalesWithWindow> = sqlx::query_as(
        r#"
        WITH daily_sales AS (
            SELECT
                created_at::DATE as order_date,
                SUM(total) as daily_total
            FROM orders
            GROUP BY created_at::DATE
        )
        SELECT
            order_date,
            daily_total,
            SUM(daily_total) OVER (ORDER BY order_date) as running_total,
            LAG(daily_total) OVER (ORDER BY order_date) as prev_day_total,
            LEAD(daily_total) OVER (ORDER BY order_date) as next_day_total
        FROM daily_sales
        ORDER BY order_date
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Daily sales with running total and comparison:");
    for sale in &sales {
        println!(
            "  {} - Daily: {}, Running: {}, Prev: {:?}, Next: {:?}",
            sale.order_date,
            sale.daily_total,
            sale.running_total,
            sale.prev_day_total,
            sale.next_day_total
        );
    }

    Ok(())
}

async fn demo_moving_average(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Moving Average with Window Frame ===");

    let result: Vec<(NaiveDate, Decimal, Decimal)> = sqlx::query_as(
        r#"
        WITH daily_sales AS (
            SELECT
                created_at::DATE as order_date,
                SUM(total) as daily_total
            FROM orders
            GROUP BY created_at::DATE
        )
        SELECT
            order_date,
            daily_total,
            AVG(daily_total) OVER (
                ORDER BY order_date
                ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
            ) as moving_avg_3day
        FROM daily_sales
        ORDER BY order_date
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Daily sales with 3-day moving average:");
    for (date, daily, moving_avg) in &result {
        println!("  {} - Daily: {}, 3-day MA: {}", date, daily, moving_avg);
    }

    Ok(())
}

async fn demo_top_n_per_group(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Top N per Group ===");

    // 各ユーザーのトップ2注文を取得
    let result: Vec<(Uuid, Uuid, Decimal, i64)> = sqlx::query_as(
        r#"
        WITH ranked_orders AS (
            SELECT
                id,
                user_id,
                total,
                ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY total DESC) as rn
            FROM orders
        )
        SELECT id, user_id, total, rn
        FROM ranked_orders
        WHERE rn <= 2
        ORDER BY user_id, rn
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Top 2 orders per user:");
    let mut current_user: Option<Uuid> = None;
    for (order_id, user_id, total, rank) in &result {
        if current_user != Some(*user_id) {
            println!("\n  User {}:", user_id);
            current_user = Some(*user_id);
        }
        println!("    #{}: Order {} - {}", rank, order_id, total);
    }

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
    insert_sample_data(&pool).await?;

    demo_basic_cte(&pool).await?;
    demo_recursive_cte(&pool).await?;
    demo_row_number_rank(&pool).await?;
    demo_lag_lead(&pool).await?;
    demo_moving_average(&pool).await?;
    demo_top_n_per_group(&pool).await?;

    println!("\n=== All CTE and Window function demos completed successfully! ===");
    Ok(())
}
