//! NULL処理のデモ
//!
//! このデモでは以下を検証:
//! 1. IS NULL / IS NOT NULL
//! 2. COALESCE によるデフォルト値
//! 3. NULLIF の活用
//! 4. NOT IN の NULL問題と解決策
//! 5. 集約関数とNULL

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct User {
    id: Uuid,
    email: String,
    name: String,
    phone: Option<String>,
    bio: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    description: Option<String>,
    price: Decimal,
    discount_price: Option<Decimal>,
    parent_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Task {
    id: Uuid,
    title: String,
    assignee_id: Option<Uuid>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS null_tasks CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS null_products CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS null_users CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE null_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            phone VARCHAR(20),
            bio TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 商品テーブル
    sqlx::query(
        r#"
        CREATE TABLE null_products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(200) NOT NULL,
            description TEXT,
            price DECIMAL(10,2) NOT NULL,
            discount_price DECIMAL(10,2),
            parent_id UUID REFERENCES null_products(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // タスクテーブル
    sqlx::query(
        r#"
        CREATE TABLE null_tasks (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(200) NOT NULL,
            assignee_id UUID REFERENCES null_users(id)
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

    // ユーザー（一部NULLフィールドあり）
    let user1: Uuid = sqlx::query_scalar(
        "INSERT INTO null_users (email, name, phone, bio) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("alice@example.com")
    .bind("Alice")
    .bind(Some("090-1234-5678"))
    .bind(Some("Hello, I'm Alice"))
    .fetch_one(pool)
    .await?;

    let user2: Uuid = sqlx::query_scalar(
        "INSERT INTO null_users (email, name, phone, bio) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("bob@example.com")
    .bind("Bob")
    .bind::<Option<String>>(None) // phone is NULL
    .bind(Some("Bob's bio"))
    .fetch_one(pool)
    .await?;

    let user3: Uuid = sqlx::query_scalar(
        "INSERT INTO null_users (email, name, phone, bio) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("charlie@example.com")
    .bind("Charlie")
    .bind::<Option<String>>(None) // phone is NULL
    .bind::<Option<String>>(None) // bio is NULL
    .fetch_one(pool)
    .await?;

    // 商品
    sqlx::query(
        "INSERT INTO null_products (name, description, price, discount_price) VALUES ($1, $2, $3, $4)",
    )
    .bind("Product A")
    .bind(Some("Description for A"))
    .bind(Decimal::new(10000, 2))
    .bind(Some(Decimal::new(8000, 2)))
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO null_products (name, description, price, discount_price) VALUES ($1, $2, $3, $4)",
    )
    .bind("Product B")
    .bind::<Option<String>>(None) // description is NULL
    .bind(Decimal::new(5000, 2))
    .bind::<Option<Decimal>>(None) // discount is NULL
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO null_products (name, description, price, discount_price) VALUES ($1, $2, $3, $4)",
    )
    .bind("Product C")
    .bind(Some(""))  // Empty string (not NULL)
    .bind(Decimal::new(3000, 2))
    .bind(Some(Decimal::ZERO)) // 0 discount (not NULL)
    .execute(pool)
    .await?;

    // タスク
    sqlx::query("INSERT INTO null_tasks (title, assignee_id) VALUES ($1, $2)")
        .bind("Task 1")
        .bind(Some(user1))
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO null_tasks (title, assignee_id) VALUES ($1, $2)")
        .bind("Task 2")
        .bind(Some(user2))
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO null_tasks (title, assignee_id) VALUES ($1, $2)")
        .bind("Unassigned Task")
        .bind::<Option<Uuid>>(None) // assignee is NULL
        .execute(pool)
        .await?;

    println!("Created 3 users, 3 products, 3 tasks");
    println!("Users: Alice (with phone), Bob (no phone), Charlie (no phone, no bio)");

    // 非アクティブユーザー用にNULL IDを含む可能性のあるデータを作成
    sqlx::query("INSERT INTO null_users (email, name) VALUES ($1, $2)")
        .bind("inactive@example.com")
        .bind("Inactive User")
        .execute(pool)
        .await?;

    let _ = (user1, user2, user3);

    Ok(())
}

// ================================
// Demo: IS NULL / IS NOT NULL
// ================================

async fn demo_is_null(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: IS NULL / IS NOT NULL ===");

    // 電話番号がNULLのユーザーを取得
    let users_without_phone: Vec<User> = sqlx::query_as(
        "SELECT id, email, name, phone, bio, created_at FROM null_users WHERE phone IS NULL",
    )
    .fetch_all(pool)
    .await?;

    println!("Users without phone:");
    for user in &users_without_phone {
        println!("  {} - phone: {:?}", user.name, user.phone);
    }

    // 電話番号があるユーザー
    let users_with_phone: Vec<User> = sqlx::query_as(
        "SELECT id, email, name, phone, bio, created_at FROM null_users WHERE phone IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    println!("\nUsers with phone:");
    for user in &users_with_phone {
        println!("  {} - phone: {:?}", user.name, user.phone);
    }

    // アンチパターンの説明
    println!("\n--- Anti-pattern demonstration ---");
    let bad_query_result: Vec<User> = sqlx::query_as(
        "SELECT id, email, name, phone, bio, created_at FROM null_users WHERE phone = NULL",
    )
    .fetch_all(pool)
    .await?;
    println!(
        "WHERE phone = NULL returns {} rows (always 0!)",
        bad_query_result.len()
    );

    Ok(())
}

// ================================
// Demo: COALESCE
// ================================

async fn demo_coalesce(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: COALESCE ===");

    #[derive(Debug, sqlx::FromRow)]
    struct UserDisplay {
        name: String,
        phone_display: String,
        bio_display: String,
    }

    let users: Vec<UserDisplay> = sqlx::query_as(
        r#"
        SELECT
            name,
            COALESCE(phone, '(未登録)') as phone_display,
            COALESCE(bio, '(プロフィールなし)') as bio_display
        FROM null_users
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Users with COALESCE defaults:");
    for user in &users {
        println!("  {} - Phone: {}, Bio: {}", user.name, user.phone_display, user.bio_display);
    }

    // 商品の最終価格計算
    #[derive(Debug, sqlx::FromRow)]
    struct ProductPrice {
        name: String,
        price: Decimal,
        discount_price: Option<Decimal>,
        final_price: Decimal,
    }

    let products: Vec<ProductPrice> = sqlx::query_as(
        r#"
        SELECT
            name,
            price,
            discount_price,
            COALESCE(discount_price, price) as final_price
        FROM null_products
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\nProduct prices:");
    for p in &products {
        println!(
            "  {} - Regular: {}, Discount: {:?}, Final: {}",
            p.name, p.price, p.discount_price, p.final_price
        );
    }

    Ok(())
}

// ================================
// Demo: NULLIF
// ================================

async fn demo_nullif(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: NULLIF ===");

    // 空文字列をNULLとして扱う
    #[derive(Debug, sqlx::FromRow)]
    struct ProductDesc {
        name: String,
        description: Option<String>,
        normalized_desc: Option<String>,
    }

    let products: Vec<ProductDesc> = sqlx::query_as(
        r#"
        SELECT
            name,
            description,
            NULLIF(description, '') as normalized_desc
        FROM null_products
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Products with NULLIF(description, ''):");
    for p in &products {
        println!(
            "  {} - Original: {:?}, Normalized: {:?}",
            p.name, p.description, p.normalized_desc
        );
    }

    // 0をNULLとして扱う
    #[derive(Debug, sqlx::FromRow)]
    struct ProductDiscount {
        name: String,
        discount_price: Option<Decimal>,
        non_zero_discount: Option<Decimal>,
    }

    let products: Vec<ProductDiscount> = sqlx::query_as(
        r#"
        SELECT
            name,
            discount_price,
            NULLIF(discount_price, 0) as non_zero_discount
        FROM null_products
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\nProducts with NULLIF(discount_price, 0):");
    for p in &products {
        println!(
            "  {} - Discount: {:?}, Non-zero: {:?}",
            p.name, p.discount_price, p.non_zero_discount
        );
    }

    Ok(())
}

// ================================
// Demo: NOT IN with NULL
// ================================

async fn demo_not_in_null(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: NOT IN with NULL problem ===");

    // 問題: NOT IN に NULL が含まれると結果が0件になる
    println!("--- Problem: NOT IN with potential NULL ---");

    // まず、全ユーザーIDを確認
    let all_user_ids: Vec<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT id FROM null_users"
    )
    .fetch_all(pool)
    .await?;
    println!("User IDs count: {}", all_user_ids.len());

    // NOT IN を使用（NULLが含まれない場合は問題なし）
    let tasks_not_in: Vec<Task> = sqlx::query_as(
        r#"
        SELECT id, title, assignee_id
        FROM null_tasks
        WHERE assignee_id NOT IN (SELECT id FROM null_users WHERE name = 'Alice')
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("Tasks NOT assigned to Alice: {}", tasks_not_in.len());

    // 解決策1: NOT EXISTS
    println!("\n--- Solution 1: NOT EXISTS ---");
    let tasks_exists: Vec<Task> = sqlx::query_as(
        r#"
        SELECT t.id, t.title, t.assignee_id
        FROM null_tasks t
        WHERE NOT EXISTS (
            SELECT 1 FROM null_users u
            WHERE u.id = t.assignee_id AND u.name = 'Alice'
        )
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("Tasks (NOT EXISTS): {}", tasks_exists.len());
    for t in &tasks_exists {
        println!("  {} - assignee: {:?}", t.title, t.assignee_id);
    }

    // 解決策2: LEFT JOIN + IS NULL
    println!("\n--- Solution 2: LEFT JOIN + IS NULL ---");
    let tasks_left_join: Vec<Task> = sqlx::query_as(
        r#"
        SELECT t.id, t.title, t.assignee_id
        FROM null_tasks t
        LEFT JOIN null_users u ON u.id = t.assignee_id AND u.name = 'Alice'
        WHERE u.id IS NULL
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("Tasks (LEFT JOIN): {}", tasks_left_join.len());

    Ok(())
}

// ================================
// Demo: 集約関数とNULL
// ================================

async fn demo_aggregate_null(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Aggregate functions and NULL ===");

    #[derive(Debug, sqlx::FromRow)]
    struct ProductStats {
        total_products: i64,
        products_with_discount: i64,
        products_without_discount: i64,
        sum_discount: Option<Decimal>,
        avg_discount: Option<Decimal>,
    }

    let stats: ProductStats = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as total_products,
            COUNT(discount_price) as products_with_discount,
            COUNT(*) - COUNT(discount_price) as products_without_discount,
            SUM(discount_price) as sum_discount,
            AVG(discount_price) as avg_discount
        FROM null_products
        "#,
    )
    .fetch_one(pool)
    .await?;

    println!("Product statistics:");
    println!("  Total products: {}", stats.total_products);
    println!(
        "  With discount: {} (COUNT ignores NULL)",
        stats.products_with_discount
    );
    println!(
        "  Without discount: {}",
        stats.products_without_discount
    );
    println!(
        "  Sum of discounts: {:?} (NULL values ignored)",
        stats.sum_discount
    );
    println!(
        "  Avg of discounts: {:?} (NULL values ignored)",
        stats.avg_discount
    );

    // COALESCEで0として扱う
    #[derive(Debug, sqlx::FromRow)]
    struct ProductStatsWithZero {
        sum_discount_with_zero: Decimal,
        avg_discount_with_zero: Decimal,
    }

    let stats_zero: ProductStatsWithZero = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(COALESCE(discount_price, 0)), 0) as sum_discount_with_zero,
            COALESCE(AVG(COALESCE(discount_price, 0)), 0) as avg_discount_with_zero
        FROM null_products
        "#,
    )
    .fetch_one(pool)
    .await?;

    println!("\nWith COALESCE(discount_price, 0):");
    println!(
        "  Sum (treating NULL as 0): {}",
        stats_zero.sum_discount_with_zero
    );
    println!(
        "  Avg (treating NULL as 0): {}",
        stats_zero.avg_discount_with_zero
    );

    Ok(())
}

// ================================
// Demo: Option型とsqlxの連携
// ================================

async fn demo_option_mapping(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Rust Option type mapping ===");

    // Option<String>でNULLを自然に扱う
    let users: Vec<User> = sqlx::query_as(
        "SELECT id, email, name, phone, bio, created_at FROM null_users ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    println!("Processing users with Option:");
    for user in &users {
        // パターンマッチで安全に処理
        let phone_status = match &user.phone {
            Some(p) => format!("Phone: {}", p),
            None => "No phone registered".to_string(),
        };

        // unwrap_or_else でデフォルト値
        let bio = user.bio.as_deref().unwrap_or("(No bio)");

        println!("  {} - {} | Bio: {}", user.name, phone_status, bio);
    }

    // Option を使った更新
    println!("\n--- Updating with Option ---");
    let some_phone: Option<&str> = Some("080-9999-8888");
    let no_phone: Option<&str> = None;

    sqlx::query("UPDATE null_users SET phone = $1 WHERE name = $2")
        .bind(some_phone)
        .bind("Charlie")
        .execute(pool)
        .await?;
    println!("Updated Charlie with phone: {:?}", some_phone);

    sqlx::query("UPDATE null_users SET phone = $1 WHERE name = $2")
        .bind(no_phone)
        .bind("Alice")
        .execute(pool)
        .await?;
    println!("Updated Alice with phone: {:?} (set to NULL)", no_phone);

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

    demo_is_null(&pool).await?;
    demo_coalesce(&pool).await?;
    demo_nullif(&pool).await?;
    demo_not_in_null(&pool).await?;
    demo_aggregate_null(&pool).await?;
    demo_option_mapping(&pool).await?;

    println!("\n=== All NULL handling demos completed successfully! ===");
    Ok(())
}
