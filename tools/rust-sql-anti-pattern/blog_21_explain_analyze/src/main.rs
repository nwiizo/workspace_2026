//! EXPLAIN ANALYZEを読みこなすデモ
//!
//! このデモでは以下を検証:
//! 1. EXPLAIN vs EXPLAIN ANALYZE の違い
//! 2. スキャン方法（Seq Scan, Index Scan, Index Only Scan）
//! 3. JOIN方法（Nested Loop, Hash Join）
//! 4. JSON形式での実行計画取得
//! 5. クエリ問題の自動検出

use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// 実行計画の解析結果
// ================================

#[derive(Debug)]
#[allow(dead_code)]
struct QueryAnalysis {
    has_seq_scan: bool,
    has_disk_sort: bool,
    execution_time_ms: Option<f64>,
    planning_time_ms: Option<f64>,
}

// ================================
// JSON形式の実行計画構造体
// ================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExplainPlan {
    #[serde(rename = "Plan")]
    plan: PlanNode,
    #[serde(rename = "Planning Time")]
    planning_time: Option<f64>,
    #[serde(rename = "Execution Time")]
    execution_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PlanNode {
    #[serde(rename = "Node Type")]
    node_type: String,
    #[serde(rename = "Startup Cost")]
    startup_cost: Option<f64>,
    #[serde(rename = "Total Cost")]
    total_cost: Option<f64>,
    #[serde(rename = "Plan Rows")]
    plan_rows: Option<i64>,
    #[serde(rename = "Actual Rows")]
    actual_rows: Option<i64>,
    #[serde(rename = "Actual Total Time")]
    actual_total_time: Option<f64>,
    #[serde(rename = "Plans")]
    plans: Option<Vec<PlanNode>>,
    #[serde(rename = "Relation Name")]
    relation_name: Option<String>,
    #[serde(rename = "Index Name")]
    index_name: Option<String>,
    #[serde(rename = "Sort Method")]
    sort_method: Option<String>,
    #[serde(rename = "Join Type")]
    join_type: Option<String>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS order_items CASCADE")
        .execute(pool)
        .await?;
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
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
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
            status TEXT NOT NULL DEFAULT 'pending',
            total DECIMAL(12,2) NOT NULL DEFAULT 0,
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
            order_id UUID NOT NULL REFERENCES orders(id),
            product_name TEXT NOT NULL,
            quantity INT NOT NULL,
            price DECIMAL(10,2) NOT NULL
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
    let user_count = 100;
    for i in 0..user_count {
        sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
            .bind(format!("User {}", i))
            .bind(format!("user{}@example.com", i))
            .execute(pool)
            .await?;
    }
    println!("Created {} users", user_count);

    // 注文を作成
    let users: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(pool)
        .await?;

    let statuses = ["pending", "confirmed", "shipped", "delivered"];
    let mut order_count = 0;

    for (user_id,) in &users {
        let orders_per_user = 10;
        for j in 0..orders_per_user {
            let status = statuses[j % statuses.len()];
            let total = Decimal::new((j + 1) as i64 * 10000, 2);

            let order_id: Uuid = sqlx::query_scalar(
                "INSERT INTO orders (user_id, status, total) VALUES ($1, $2, $3) RETURNING id",
            )
            .bind(user_id)
            .bind(status)
            .bind(total)
            .fetch_one(pool)
            .await?;

            // 注文明細を追加
            for k in 0..3 {
                sqlx::query(
                    "INSERT INTO order_items (order_id, product_name, quantity, price) VALUES ($1, $2, $3, $4)"
                )
                .bind(order_id)
                .bind(format!("Product {}", k))
                .bind(k + 1)
                .bind(Decimal::new((k + 1) as i64 * 1000, 2))
                .execute(pool)
                .await?;
            }

            order_count += 1;
        }
    }
    println!("Created {} orders with items", order_count);

    // 統計情報を更新
    sqlx::query("ANALYZE users").execute(pool).await?;
    sqlx::query("ANALYZE orders").execute(pool).await?;
    sqlx::query("ANALYZE order_items").execute(pool).await?;
    println!("Updated table statistics");

    Ok(())
}

// ================================
// EXPLAIN取得関数
// ================================

async fn explain_query(pool: &PgPool, query: &str) -> Result<String> {
    let explain_query = format!("EXPLAIN {}", query);
    let rows: Vec<(String,)> = sqlx::query_as(&explain_query).fetch_all(pool).await?;
    let plan = rows
        .iter()
        .map(|(line,)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(plan)
}

async fn explain_analyze_query(pool: &PgPool, query: &str) -> Result<String> {
    let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", query);
    let rows: Vec<(String,)> = sqlx::query_as(&explain_query).fetch_all(pool).await?;
    let plan = rows
        .iter()
        .map(|(line,)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(plan)
}

async fn explain_analyze_json(pool: &PgPool, query: &str) -> Result<Vec<ExplainPlan>> {
    let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {}", query);
    let row: (serde_json::Value,) = sqlx::query_as(&explain_query).fetch_one(pool).await?;
    let plans: Vec<ExplainPlan> = serde_json::from_value(row.0)?;
    Ok(plans)
}

fn analyze_plan(plan: &str) -> QueryAnalysis {
    QueryAnalysis {
        has_seq_scan: plan.contains("Seq Scan"),
        has_disk_sort: plan.contains("external merge") || plan.contains("external sort"),
        execution_time_ms: extract_execution_time(plan),
        planning_time_ms: extract_planning_time(plan),
    }
}

fn extract_execution_time(plan: &str) -> Option<f64> {
    for line in plan.lines() {
        if line.contains("Execution Time:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let time_str = parts[1].trim().replace(" ms", "");
                return time_str.parse().ok();
            }
        }
    }
    None
}

fn extract_planning_time(plan: &str) -> Option<f64> {
    for line in plan.lines() {
        if line.contains("Planning Time:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let time_str = parts[1].trim().replace(" ms", "");
                return time_str.parse().ok();
            }
        }
    }
    None
}

// ================================
// デモ: EXPLAIN vs EXPLAIN ANALYZE
// ================================

async fn demo_explain_difference(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: EXPLAIN vs EXPLAIN ANALYZE ===");

    let query = "SELECT * FROM users WHERE email = 'user0@example.com'";

    // EXPLAIN（実行しない、推定のみ）
    let explain = explain_query(pool, query).await?;
    println!("\nEXPLAIN (estimates only, does not execute):");
    println!("{}", explain);

    // EXPLAIN ANALYZE（実際に実行して計測）
    let explain_analyze = explain_analyze_query(pool, query).await?;
    println!("\nEXPLAIN ANALYZE (actually executes and measures):");
    println!("{}", explain_analyze);

    Ok(())
}

// ================================
// デモ: Seq Scan vs Index Scan
// ================================

async fn demo_scan_methods(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Seq Scan vs Index Scan ===");

    // Seq Scan（インデックスなし）
    println!("\n--- Without Index (Seq Scan) ---");
    let query = "SELECT * FROM orders WHERE status = 'pending'";
    let plan = explain_analyze_query(pool, query).await?;
    println!("{}", plan);

    let analysis = analyze_plan(&plan);
    println!("\nAnalysis: has_seq_scan = {}", analysis.has_seq_scan);

    // インデックスを追加
    println!("\n--- Adding Index ---");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status)")
        .execute(pool)
        .await?;
    println!("Created index: idx_orders_status");

    // Index Scan
    println!("\n--- With Index (Index Scan) ---");
    let plan = explain_analyze_query(pool, query).await?;
    println!("{}", plan);

    let analysis = analyze_plan(&plan);
    println!("\nAnalysis: has_seq_scan = {}", analysis.has_seq_scan);

    Ok(())
}

// ================================
// デモ: Index Only Scan
// ================================

async fn demo_index_only_scan(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Index Only Scan ===");

    // カバリングインデックスを作成
    sqlx::query("DROP INDEX IF EXISTS idx_orders_status_total")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX idx_orders_status_total ON orders(status) INCLUDE (total)")
        .execute(pool)
        .await?;
    println!("Created covering index: idx_orders_status_total");

    // VACUUMでVisibility Mapを更新
    sqlx::query("VACUUM orders").execute(pool).await?;
    println!("Vacuumed orders table");

    // Index Only Scan
    let query = "SELECT status, total FROM orders WHERE status = 'pending'";
    let plan = explain_analyze_query(pool, query).await?;
    println!("\n{}", plan);

    if plan.contains("Index Only Scan") {
        println!("\nSuccess: Index Only Scan is being used!");
        println!("This means the query doesn't need to access the table heap.");
    }

    Ok(())
}

// ================================
// デモ: JOIN方法
// ================================

async fn demo_join_methods(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: JOIN Methods ===");

    // インデックスを確認
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_user_id ON orders(user_id)")
        .execute(pool)
        .await?;

    // Nested Loop（少ない行数のJOIN）
    println!("\n--- Nested Loop (small result set) ---");
    let query = r#"
        SELECT o.*, u.name
        FROM orders o
        JOIN users u ON o.user_id = u.id
        WHERE o.id = (SELECT id FROM orders LIMIT 1)
    "#;
    let plan = explain_analyze_query(pool, query).await?;
    println!("{}", plan);

    // Hash Join（大きい結果セット）
    println!("\n--- Hash Join (larger result set) ---");
    let query = r#"
        SELECT o.id, u.name
        FROM orders o
        JOIN users u ON o.user_id = u.id
        WHERE o.status = 'pending'
    "#;
    let plan = explain_analyze_query(pool, query).await?;
    println!("{}", plan);

    Ok(())
}

// ================================
// デモ: JSON形式での実行計画取得
// ================================

async fn demo_json_format(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: JSON Format Execution Plan ===");

    let query = "SELECT * FROM users WHERE email = 'user0@example.com'";
    let plans = explain_analyze_json(pool, query).await?;

    if let Some(plan) = plans.first() {
        println!("Node Type: {}", plan.plan.node_type);
        println!("Planning Time: {:?} ms", plan.planning_time);
        println!("Execution Time: {:?} ms", plan.execution_time);
        println!("Estimated Rows: {:?}", plan.plan.plan_rows);
        println!("Actual Rows: {:?}", plan.plan.actual_rows);
        println!("Relation: {:?}", plan.plan.relation_name);
        println!("Index: {:?}", plan.plan.index_name);
    }

    Ok(())
}

// ================================
// デモ: 推定行数と実際の行数の比較
// ================================

async fn demo_estimation_accuracy(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Estimation Accuracy ===");

    // 統計情報を更新
    sqlx::query("ANALYZE orders").execute(pool).await?;

    let queries = vec![
        (
            "High selectivity",
            "SELECT * FROM orders WHERE status = 'pending'",
        ),
        (
            "Unique value",
            "SELECT * FROM users WHERE email = 'user0@example.com'",
        ),
        ("Range query", "SELECT * FROM orders WHERE total > 50.00"),
    ];

    for (name, query) in queries {
        println!("\n--- {} ---", name);
        let plans = explain_analyze_json(pool, query).await?;

        if let Some(plan) = plans.first() {
            let estimated = plan.plan.plan_rows.unwrap_or(0);
            let actual = plan.plan.actual_rows.unwrap_or(0);
            let ratio = if estimated > 0 {
                actual as f64 / estimated as f64
            } else {
                0.0
            };

            println!("Query: {}", query);
            println!("Estimated rows: {}", estimated);
            println!("Actual rows: {}", actual);
            println!("Ratio (actual/estimated): {:.2}", ratio);

            if !(0.1..=10.0).contains(&ratio) {
                println!("WARNING: Large estimation error!");
            }
        }
    }

    Ok(())
}

// ================================
// デモ: 問題検出
// ================================

async fn demo_issue_detection(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Automatic Issue Detection ===");

    // インデックスなしのテーブルに対してクエリ
    sqlx::query("DROP INDEX IF EXISTS idx_orders_status")
        .execute(pool)
        .await?;

    let query = "SELECT * FROM orders WHERE status = 'pending' ORDER BY created_at";
    let plan = explain_analyze_query(pool, query).await?;

    let analysis = analyze_plan(&plan);
    println!("\nQuery: {}", query);
    println!("Analysis:");
    println!("  - Has Seq Scan: {}", analysis.has_seq_scan);
    println!("  - Has Disk Sort: {}", analysis.has_disk_sort);
    println!("  - Execution Time: {:?} ms", analysis.execution_time_ms);
    println!("  - Planning Time: {:?} ms", analysis.planning_time_ms);

    if analysis.has_seq_scan {
        println!("\nRecommendation: Consider adding an index on 'status' column");
    }

    // インデックスを追加して再テスト
    sqlx::query("CREATE INDEX idx_orders_status ON orders(status)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders(created_at)")
        .execute(pool)
        .await?;

    let plan = explain_analyze_query(pool, query).await?;
    let analysis = analyze_plan(&plan);

    println!("\nAfter adding indexes:");
    println!("  - Has Seq Scan: {}", analysis.has_seq_scan);
    println!("  - Execution Time: {:?} ms", analysis.execution_time_ms);

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

    demo_explain_difference(&pool).await?;
    demo_scan_methods(&pool).await?;
    demo_index_only_scan(&pool).await?;
    demo_join_methods(&pool).await?;
    demo_json_format(&pool).await?;
    demo_estimation_accuracy(&pool).await?;
    demo_issue_detection(&pool).await?;

    println!("\n=== All EXPLAIN ANALYZE demos completed successfully! ===");
    Ok(())
}
