//! スケーラビリティ設計とパーティショニングのデモ
//!
//! このデモでは以下を検証:
//! 1. メタデータトリブル（年度別テーブル分割）アンチパターン
//! 2. PostgreSQL RANGEパーティショニング
//! 3. LISTパーティショニング
//! 4. HASHパーティショニング
//! 5. パーティションプルーニングの確認

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
struct Order {
    id: Uuid,
    user_id: Uuid,
    total: Decimal,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct User {
    id: Uuid,
    email: String,
    region: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct MonthlyRevenue {
    id: Uuid,
    year: i32,
    month: i32,
    revenue: Decimal,
}

// ================================
// アンチパターン: 年度別テーブル
// ================================

async fn demo_anti_pattern_yearly_tables(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Anti-pattern - Yearly Tables ===");

    // アンチパターン: 年ごとにテーブルを作成
    for year in 2022..=2024 {
        let table_name = format!("orders_{}", year);
        sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table_name))
            .execute(pool)
            .await?;

        sqlx::query(&format!(
            r#"
            CREATE TABLE {} (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL,
                total DECIMAL(10,2) NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
            table_name
        ))
        .execute(pool)
        .await?;
    }
    println!("Created yearly tables: orders_2022, orders_2023, orders_2024");

    // サンプルデータを挿入
    let user_id = Uuid::new_v4();
    for year in 2022..=2024 {
        let table_name = format!("orders_{}", year);
        for i in 1..=3 {
            let total = Decimal::new((year as i64 * 100 + i) * 100, 2);
            sqlx::query(&format!(
                "INSERT INTO {} (user_id, total, created_at) VALUES ($1, $2, $3)",
                table_name
            ))
            .bind(user_id)
            .bind(total)
            .bind(Utc::now())
            .execute(pool)
            .await?;
        }
    }

    // 問題: 全年度のデータを取得するにはUNION ALLが必要
    println!("\nProblem: Query requires UNION ALL for all years:");
    let orders: Vec<Order> = sqlx::query_as(
        r#"
        SELECT id, user_id, total, created_at FROM orders_2022 WHERE user_id = $1
        UNION ALL
        SELECT id, user_id, total, created_at FROM orders_2023 WHERE user_id = $1
        UNION ALL
        SELECT id, user_id, total, created_at FROM orders_2024 WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    println!("Total orders across all years: {}", orders.len());
    println!("This approach requires modifying queries each year!");

    Ok(())
}

// ================================
// 解決策: RANGEパーティショニング
// ================================

async fn setup_range_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Setup: RANGE Partitioning ===");

    // 親テーブルを作成（パーティション化）
    sqlx::query("DROP TABLE IF EXISTS orders_partitioned CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE orders_partitioned (
            id UUID NOT NULL DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            total DECIMAL(10,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            PRIMARY KEY (id, created_at)
        ) PARTITION BY RANGE (created_at)
        "#,
    )
    .execute(pool)
    .await?;

    // 年ごとのパーティションを作成
    for year in 2023..=2027 {
        let partition_name = format!("orders_partitioned_{}", year);
        let start_date = format!("{}-01-01", year);
        let end_date = format!("{}-01-01", year + 1);

        sqlx::query(&format!(
            "CREATE TABLE {} PARTITION OF orders_partitioned FOR VALUES FROM ('{}') TO ('{}')",
            partition_name, start_date, end_date
        ))
        .execute(pool)
        .await?;
    }

    // インデックスを作成（親テーブルに作成すると子テーブルに継承）
    sqlx::query("CREATE INDEX idx_orders_part_user_id ON orders_partitioned(user_id)")
        .execute(pool)
        .await?;

    println!("Created partitioned table with yearly partitions (2023-2027)");
    Ok(())
}

async fn demo_range_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: RANGE Partitioning ===");

    // サンプルデータを挿入
    let user_id = Uuid::new_v4();

    // 2024年のデータ
    for i in 1..=5 {
        let total = Decimal::new(i * 1000, 2);
        let created_at = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO orders_partitioned (user_id, total, created_at) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(total)
        .bind(created_at)
        .execute(pool)
        .await?;
    }

    // パーティショニングは透過的 - 通常のクエリで全データ取得
    let orders: Vec<Order> = sqlx::query_as(
        r#"
        SELECT id, user_id, total, created_at
        FROM orders_partitioned
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    println!("Fetched {} orders using single table query", orders.len());
    println!("Application code doesn't need to know about partitions!");

    // パーティションプルーニングを確認
    println!("\nChecking partition pruning with EXPLAIN:");
    let plan: Vec<(String,)> = sqlx::query_as(
        r#"
        EXPLAIN (FORMAT TEXT)
        SELECT * FROM orders_partitioned
        WHERE created_at >= '2024-01-01' AND created_at < '2024-07-01'
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (line,) in plan.iter().take(5) {
        println!("  {}", line);
    }

    Ok(())
}

// ================================
// LISTパーティショニング
// ================================

async fn setup_list_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Setup: LIST Partitioning ===");

    sqlx::query("DROP TABLE IF EXISTS users_by_region CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE users_by_region (
            id UUID NOT NULL DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL,
            region VARCHAR(10) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (id, region)
        ) PARTITION BY LIST (region)
        "#,
    )
    .execute(pool)
    .await?;

    // 地域別パーティション
    sqlx::query(
        r#"
        CREATE TABLE users_asia PARTITION OF users_by_region
        FOR VALUES IN ('JP', 'KR', 'CN', 'TW', 'SG')
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE users_europe PARTITION OF users_by_region
        FOR VALUES IN ('UK', 'DE', 'FR', 'IT', 'ES')
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE users_americas PARTITION OF users_by_region
        FOR VALUES IN ('US', 'CA', 'BR', 'MX')
        "#,
    )
    .execute(pool)
    .await?;

    // デフォルトパーティション
    sqlx::query(
        r#"
        CREATE TABLE users_other PARTITION OF users_by_region DEFAULT
        "#,
    )
    .execute(pool)
    .await?;

    println!("Created LIST partitioned table with regional partitions");
    Ok(())
}

async fn demo_list_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: LIST Partitioning ===");

    // 各地域にユーザーを追加
    let regions = vec!["JP", "US", "DE", "AU"];
    for region in &regions {
        sqlx::query("INSERT INTO users_by_region (email, region) VALUES ($1, $2)")
            .bind(format!("user@{}.example.com", region.to_lowercase()))
            .bind(*region)
            .execute(pool)
            .await?;
    }

    // アジア地域のユーザーを取得
    let asian_users: Vec<User> = sqlx::query_as(
        r#"
        SELECT id, email, region, created_at
        FROM users_by_region
        WHERE region IN ('JP', 'KR', 'CN', 'TW', 'SG')
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Asian users: {}", asian_users.len());

    // 全ユーザーを取得
    let all_users: Vec<User> = sqlx::query_as(
        "SELECT id, email, region, created_at FROM users_by_region ORDER BY region",
    )
    .fetch_all(pool)
    .await?;

    println!("Total users: {}", all_users.len());
    for user in &all_users {
        println!("  {} - {}", user.region, user.email);
    }

    Ok(())
}

// ================================
// HASHパーティショニング
// ================================

async fn setup_hash_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Setup: HASH Partitioning ===");

    sqlx::query("DROP TABLE IF EXISTS user_activities CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE user_activities (
            id UUID NOT NULL DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            activity_type VARCHAR(50) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (id, user_id)
        ) PARTITION BY HASH (user_id)
        "#,
    )
    .execute(pool)
    .await?;

    // 4つのパーティションに均等分散
    for i in 0..4 {
        sqlx::query(&format!(
            "CREATE TABLE user_activities_{} PARTITION OF user_activities FOR VALUES WITH (MODULUS 4, REMAINDER {})",
            i, i
        ))
        .execute(pool)
        .await?;
    }

    println!("Created HASH partitioned table with 4 partitions");
    Ok(())
}

async fn demo_hash_partitioning(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: HASH Partitioning ===");

    // 複数ユーザーのアクティビティを追加
    let user_ids: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();
    let activities = vec!["login", "view", "purchase", "logout"];

    for user_id in &user_ids {
        for activity in &activities {
            sqlx::query("INSERT INTO user_activities (user_id, activity_type) VALUES ($1, $2)")
                .bind(user_id)
                .bind(*activity)
                .execute(pool)
                .await?;
        }
    }

    println!("Inserted activities for 10 users");

    // 特定ユーザーのアクティビティを取得
    let user_id = user_ids[0];
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_activities WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    println!("Activities for user {}: {}", user_id, count);

    // パーティションごとの分散を確認
    println!("\nData distribution across partitions:");
    for i in 0..4 {
        let partition_count: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM user_activities_{}", i))
                .fetch_one(pool)
                .await?;
        println!("  Partition {}: {} rows", i, partition_count);
    }

    Ok(())
}

// ================================
// メタデータトリブル列の解決
// ================================

async fn demo_normalized_monthly_revenue(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Normalized Monthly Revenue ===");

    sqlx::query("DROP TABLE IF EXISTS monthly_revenue CASCADE")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE monthly_revenue (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            year INT NOT NULL,
            month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
            revenue DECIMAL(12,2) NOT NULL,
            UNIQUE (year, month)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // サンプルデータを挿入
    for month in 1..=12 {
        let revenue = Decimal::new(100000 + month as i64 * 5000, 2);
        sqlx::query("INSERT INTO monthly_revenue (year, month, revenue) VALUES ($1, $2, $3)")
            .bind(2024)
            .bind(month)
            .bind(revenue)
            .execute(pool)
            .await?;
    }

    // 特定月の収益を取得（シンプル）
    let month = 6;
    let revenue: Option<Decimal> =
        sqlx::query_scalar("SELECT revenue FROM monthly_revenue WHERE year = $1 AND month = $2")
            .bind(2024)
            .bind(month)
            .fetch_optional(pool)
            .await?;

    println!("Revenue for 2024-{}: {:?}", month, revenue);

    // 年間合計
    let total: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(revenue), 0) FROM monthly_revenue WHERE year = $1",
    )
    .bind(2024)
    .fetch_one(pool)
    .await?;

    println!("Total revenue for 2024: {}", total);

    // 月別一覧
    let revenues: Vec<MonthlyRevenue> = sqlx::query_as(
        "SELECT id, year, month, revenue FROM monthly_revenue WHERE year = $1 ORDER BY month",
    )
    .bind(2024)
    .fetch_all(pool)
    .await?;

    println!("\nMonthly breakdown:");
    for r in &revenues {
        println!("  {}-{:02}: {}", r.year, r.month, r.revenue);
    }

    Ok(())
}

// ================================
// パーティション管理の自動化
// ================================

async fn ensure_partitions_exist(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Auto Partition Management ===");

    let current_year = Utc::now().year();

    // 現在年と翌年のパーティションを確保
    for year in [current_year, current_year + 1] {
        let partition_name = format!("orders_partitioned_{}", year);

        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM pg_tables
                WHERE tablename = $1
            )
            "#,
        )
        .bind(&partition_name)
        .fetch_one(pool)
        .await?;

        if exists {
            println!("Partition {} already exists", partition_name);
        } else {
            let start_date = format!("{}-01-01", year);
            let end_date = format!("{}-01-01", year + 1);

            sqlx::query(&format!(
                "CREATE TABLE {} PARTITION OF orders_partitioned FOR VALUES FROM ('{}') TO ('{}')",
                partition_name, start_date, end_date
            ))
            .execute(pool)
            .await?;

            println!("Created partition: {}", partition_name);
        }
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

    // アンチパターンのデモ
    demo_anti_pattern_yearly_tables(&pool).await?;

    // 解決策: パーティショニング
    setup_range_partitioning(&pool).await?;
    demo_range_partitioning(&pool).await?;

    setup_list_partitioning(&pool).await?;
    demo_list_partitioning(&pool).await?;

    setup_hash_partitioning(&pool).await?;
    demo_hash_partitioning(&pool).await?;

    // 正規化されたメタデータ
    demo_normalized_monthly_revenue(&pool).await?;

    // パーティション自動管理
    ensure_partitions_exist(&pool).await?;

    println!("\n=== All scalability design demos completed successfully! ===");
    Ok(())
}
