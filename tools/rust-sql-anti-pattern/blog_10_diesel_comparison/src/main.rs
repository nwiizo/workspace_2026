//! sqlxのパターンデモ（Dieselとの比較用）
//!
//! このデモでは以下を検証:
//! 1. SQLファーストなクエリ記述
//! 2. 複雑なJOIN
//! 3. 動的クエリ構築（QueryBuilder）
//! 4. CTEと再帰クエリ
//! 5. N+1問題の解決（JSON集約）

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Post {
    id: Uuid,
    user_id: Uuid,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Tag {
    id: Uuid,
    name: String,
    slug: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Product {
    id: Uuid,
    name: String,
    price: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Category {
    id: Uuid,
    name: String,
    parent_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CategoryWithPath {
    id: Uuid,
    name: String,
    depth: i32,
    path: String,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct CommentData {
    id: Uuid,
    body: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct PostWithComments {
    id: Uuid,
    title: String,
    comments: Json<Vec<CommentData>>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS post_tags CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS comments CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS posts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS tags CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS categories CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS demo_users CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE demo_users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(100) NOT NULL,
            email VARCHAR(255) NOT NULL UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 投稿テーブル
    sqlx::query(
        r#"
        CREATE TABLE posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES demo_users(id),
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // コメントテーブル
    sqlx::query(
        r#"
        CREATE TABLE comments (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL REFERENCES posts(id),
            body TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // タグテーブル
    sqlx::query(
        r#"
        CREATE TABLE tags (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(50) NOT NULL,
            slug VARCHAR(50) NOT NULL UNIQUE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 投稿-タグ中間テーブル
    sqlx::query(
        r#"
        CREATE TABLE post_tags (
            post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
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
            name VARCHAR(100) NOT NULL,
            parent_id UUID REFERENCES categories(id)
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
            name VARCHAR(200) NOT NULL,
            price DECIMAL(10,2) NOT NULL,
            category_id UUID REFERENCES categories(id)
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

    // ユーザー
    let user1: Uuid = sqlx::query_scalar(
        "INSERT INTO demo_users (name, email) VALUES ($1, $2) RETURNING id",
    )
    .bind("Alice")
    .bind("alice@example.com")
    .fetch_one(pool)
    .await?;

    let user2: Uuid = sqlx::query_scalar(
        "INSERT INTO demo_users (name, email) VALUES ($1, $2) RETURNING id",
    )
    .bind("Bob")
    .bind("bob@example.com")
    .fetch_one(pool)
    .await?;

    // 投稿
    let post1: Uuid = sqlx::query_scalar(
        "INSERT INTO posts (user_id, title, content) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user1)
    .bind("Introduction to Rust")
    .bind("Rust is a systems programming language...")
    .fetch_one(pool)
    .await?;

    let post2: Uuid = sqlx::query_scalar(
        "INSERT INTO posts (user_id, title, content) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user2)
    .bind("PostgreSQL Tips")
    .bind("Here are some PostgreSQL optimization tips...")
    .fetch_one(pool)
    .await?;

    // コメント
    for i in 1..=3 {
        sqlx::query("INSERT INTO comments (post_id, body) VALUES ($1, $2)")
            .bind(post1)
            .bind(format!("Great post! Comment {}", i))
            .execute(pool)
            .await?;
    }
    sqlx::query("INSERT INTO comments (post_id, body) VALUES ($1, $2)")
        .bind(post2)
        .bind("Thanks for the tips!")
        .execute(pool)
        .await?;

    // タグ
    let tag_rust: Uuid = sqlx::query_scalar(
        "INSERT INTO tags (name, slug) VALUES ($1, $2) RETURNING id",
    )
    .bind("Rust")
    .bind("rust")
    .fetch_one(pool)
    .await?;

    let tag_db: Uuid = sqlx::query_scalar(
        "INSERT INTO tags (name, slug) VALUES ($1, $2) RETURNING id",
    )
    .bind("Database")
    .bind("database")
    .fetch_one(pool)
    .await?;

    // 投稿-タグ関連
    sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)")
        .bind(post1)
        .bind(tag_rust)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)")
        .bind(post2)
        .bind(tag_db)
        .execute(pool)
        .await?;

    // カテゴリ階層
    let electronics: Uuid = sqlx::query_scalar(
        "INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id",
    )
    .bind("Electronics")
    .fetch_one(pool)
    .await?;

    let computers: Uuid = sqlx::query_scalar(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
    )
    .bind("Computers")
    .bind(electronics)
    .fetch_one(pool)
    .await?;

    let _laptops: Uuid = sqlx::query_scalar(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
    )
    .bind("Laptops")
    .bind(computers)
    .fetch_one(pool)
    .await?;

    println!("Sample data inserted");
    Ok(())
}

// ================================
// Demo: SQLファーストなクエリ
// ================================

async fn demo_sql_first(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: SQL-first queries ===");

    // sqlxはSQLをそのまま書ける
    let posts: Vec<Post> = sqlx::query_as(
        r#"
        SELECT id, user_id, title, content, created_at
        FROM posts
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Posts (SQL-first):");
    for post in &posts {
        println!("  - {} ({})", post.title, post.id);
    }

    Ok(())
}

// ================================
// Demo: 複雑なJOIN
// ================================

async fn demo_complex_join(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Complex JOIN ===");

    // タグで投稿を検索（多対多JOIN）
    let posts: Vec<Post> = sqlx::query_as(
        r#"
        SELECT p.id, p.user_id, p.title, p.content, p.created_at
        FROM posts p
        INNER JOIN post_tags pt ON p.id = pt.post_id
        INNER JOIN tags t ON pt.tag_id = t.id
        WHERE t.slug = $1
        "#,
    )
    .bind("rust")
    .fetch_all(pool)
    .await?;

    println!("Posts tagged with 'rust':");
    for post in &posts {
        println!("  - {}", post.title);
    }

    // 投稿と作者を一緒に取得
    #[derive(Debug, sqlx::FromRow)]
    struct PostWithAuthor {
        post_title: String,
        author_name: String,
    }

    let results: Vec<PostWithAuthor> = sqlx::query_as(
        r#"
        SELECT p.title as post_title, u.name as author_name
        FROM posts p
        JOIN demo_users u ON p.user_id = u.id
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\nPosts with authors:");
    for r in &results {
        println!("  {} by {}", r.post_title, r.author_name);
    }

    Ok(())
}

// ================================
// Demo: N+1問題の解決（JSON集約）
// ================================

async fn demo_json_aggregation(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: N+1 solution with JSON aggregation ===");

    // 1回のクエリで投稿とコメントを取得
    let posts_with_comments: Vec<PostWithComments> = sqlx::query_as(
        r#"
        SELECT
            p.id,
            p.title,
            COALESCE(
                json_agg(
                    json_build_object('id', c.id, 'body', c.body)
                ) FILTER (WHERE c.id IS NOT NULL),
                '[]'::json
            ) as comments
        FROM posts p
        LEFT JOIN comments c ON p.id = c.post_id
        GROUP BY p.id, p.title
        ORDER BY p.title
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("Posts with comments (single query):");
    for post in &posts_with_comments {
        println!("  {}: {} comments", post.title, post.comments.0.len());
        for comment in &post.comments.0 {
            println!("    - {}", comment.body);
        }
    }

    Ok(())
}

// ================================
// Demo: 再帰CTE（カテゴリ階層）
// ================================

async fn demo_recursive_cte(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Recursive CTE ===");

    // ルートカテゴリを取得
    let root_id: Uuid =
        sqlx::query_scalar("SELECT id FROM categories WHERE parent_id IS NULL LIMIT 1")
            .fetch_one(pool)
            .await?;

    let categories: Vec<CategoryWithPath> = sqlx::query_as(
        r#"
        WITH RECURSIVE category_tree AS (
            -- ベースケース
            SELECT
                id,
                name,
                0 as depth,
                name::text as path
            FROM categories
            WHERE id = $1

            UNION ALL

            -- 再帰ケース
            SELECT
                c.id,
                c.name,
                ct.depth + 1,
                (ct.path || ' > ' || c.name)::text
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

    println!("Category tree:");
    for cat in &categories {
        let indent = "  ".repeat(cat.depth as usize);
        println!("{}{}", indent, cat.name);
    }

    Ok(())
}

// ================================
// Demo: 動的クエリ（QueryBuilder）
// ================================

async fn demo_dynamic_query(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Dynamic query with QueryBuilder ===");

    // フィルター条件
    struct UserFilter {
        name_contains: Option<String>,
        email_domain: Option<String>,
    }

    let filter = UserFilter {
        name_contains: Some("li".to_string()),
        email_domain: None,
    };

    let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT id, name, email FROM demo_users WHERE 1=1");

    if let Some(name) = &filter.name_contains {
        builder.push(" AND name ILIKE '%' || ");
        builder.push_bind(name);
        builder.push(" || '%'");
    }

    if let Some(domain) = &filter.email_domain {
        builder.push(" AND email LIKE '%@' || ");
        builder.push_bind(domain);
    }

    builder.push(" ORDER BY name");

    let users: Vec<User> = builder.build_query_as().fetch_all(pool).await?;

    println!("Users matching filter:");
    for user in &users {
        println!("  {} <{}>", user.name, user.email);
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

    demo_sql_first(&pool).await?;
    demo_complex_join(&pool).await?;
    demo_json_aggregation(&pool).await?;
    demo_recursive_cte(&pool).await?;
    demo_dynamic_query(&pool).await?;

    println!("\n=== All sqlx pattern demos completed successfully! ===");
    Ok(())
}
