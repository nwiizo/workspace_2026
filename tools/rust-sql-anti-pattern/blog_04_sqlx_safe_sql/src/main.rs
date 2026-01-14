//! sqlxで安全なSQL - アンチパターン検証コード（詳細版）
//!
//! このコードは以下のアンチパターンと解決策を実演します:
//! 1. フィア・オブ・ジ・アンノウン（NULL処理）
//!    → Option<T> / IS NULL / COALESCE / NULLIF / NULL順序
//! 2. インプリシットカラム（SELECT *）
//!    → 明示的カラム指定 / sqlx::FromRow
//! 3. SQLインジェクション
//!    → プリペアドステートメント / QueryBuilder / 動的IN句
//! 4. シー・ノー・エビル（エラー無視）
//!    → 制約違反検出 / リトライ戦略 / コネクションプール管理
//! 5. 型変換のベストプラクティス
//!    → rust_decimal / UUID / sqlx::Type / カスタム型

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

const DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/antipattern";

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    println!("=== sqlxで安全なSQL デモ ===\n");

    setup_tables(&pool).await?;
    insert_sample_data(&pool).await?;

    demo_null_handling(&pool).await?;
    demo_null_advanced(&pool).await?;
    demo_implicit_columns(&pool).await?;
    demo_sql_injection(&pool).await?;
    demo_query_builder(&pool).await?;
    demo_error_handling(&pool).await?;
    demo_error_advanced(&pool).await?;
    demo_type_conversion(&pool).await?;
    demo_uuid_handling(&pool).await?;

    cleanup_tables(&pool).await?;

    Ok(())
}

async fn setup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users_safe (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            bio TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products_typed (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            price DECIMAL(10, 2) NOT NULL,
            metadata JSONB DEFAULT '{}',
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS categories (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_categories (
            product_id INTEGER REFERENCES products_typed(id) ON DELETE CASCADE,
            category_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
            PRIMARY KEY (product_id, category_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "INSERT INTO users_safe (name, email, bio) VALUES ('Alice', 'alice@example.com', 'Developer')",
    )
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO users_safe (name, email, bio) VALUES ('Bob', NULL, NULL)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO users_safe (name, email, bio) VALUES ('Charlie', 'charlie@example.com', NULL)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"INSERT INTO products_typed (name, price, metadata) VALUES ('商品A', 1999.99, '{"category": "electronics", "weight": 0.5}')"#,
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO categories (id, name) VALUES (1, 'Electronics')")
        .execute(pool)
        .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS product_categories CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS categories CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS products_typed CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users_safe CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}

/// 1. フィア・オブ・ジ・アンノウン: NULLを正しく扱う
async fn demo_null_handling(pool: &PgPool) -> Result<()> {
    println!("--- 1. NULL処理 ---");

    // アンチパターン: WHERE column = NULL
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users_safe WHERE email = NULL")
        .fetch_one(pool)
        .await?;
    println!("  WHERE email = NULL: {} 件 (常に0)", count.0);

    // 正解: IS NULL
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users_safe WHERE email IS NULL")
        .fetch_one(pool)
        .await?;
    println!("  WHERE email IS NULL: {} 件", count.0);

    // RustのOption型との対応
    #[derive(Debug)]
    struct User {
        name: String,
        email: Option<String>,
        bio: Option<String>,
    }

    let users: Vec<(String, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT name, email, bio FROM users_safe")
            .fetch_all(pool)
            .await?;

    println!("\n  RustのOption型でNULLを表現:");
    for (name, email, bio) in &users {
        let user = User {
            name: name.clone(),
            email: email.clone(),
            bio: bio.clone(),
        };
        println!(
            "    {} - email: {:?}, bio: {:?}",
            user.name, user.email, user.bio
        );
    }

    // COALESCEの活用
    let users: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, COALESCE(email, 'no-email@example.com') FROM users_safe",
    )
    .fetch_all(pool)
    .await?;

    println!("\n  COALESCE: NULLにデフォルト値:");
    for (name, email) in &users {
        println!("    {} - {}", name, email);
    }

    // 条件付きNULLパラメータ
    let search_email: Option<&str> = Some("alice@example.com");
    let users: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM users_safe WHERE ($1::text IS NULL OR email = $1)",
    )
    .bind(search_email)
    .fetch_all(pool)
    .await?;
    println!("\n  条件付きNULLパラメータ: {:?}", users);

    println!();
    Ok(())
}

/// 2. インプリシットカラム: SELECT * を避ける理由
async fn demo_implicit_columns(pool: &PgPool) -> Result<()> {
    println!("--- 2. SELECT * を避ける ---");

    // アンチパターン: SELECT *
    println!("  SELECT *の問題:");
    println!("    - スキーマ変更で壊れる可能性");
    println!("    - 不要なデータを転送");
    println!("    - 型推論が困難");

    // 正解: 明示的なカラム指定
    #[derive(Debug)]
    struct UserSummary {
        id: i32,
        name: String,
    }

    let users: Vec<(i32, String)> = sqlx::query_as("SELECT id, name FROM users_safe")
        .fetch_all(pool)
        .await?;

    println!("\n  明示的なカラム指定:");
    for (id, name) in &users {
        let user = UserSummary {
            id: *id,
            name: name.clone(),
        };
        println!("    {:?}", user);
    }

    // 必要な列だけ取得することでパフォーマンス向上
    println!("\n  メリット:");
    println!("    - 必要なデータのみ転送");
    println!("    - 型安全性が向上");
    println!("    - sqlxのコンパイル時チェックが効く");

    println!();
    Ok(())
}

/// 3. SQLインジェクション: 動的SQLの安全な書き方
async fn demo_sql_injection(pool: &PgPool) -> Result<()> {
    println!("--- 3. SQLインジェクション対策 ---");

    // アンチパターン（コメントアウト - 危険）
    // let user_input = "'; DROP TABLE users_safe; --";
    // let query = format!("SELECT * FROM users_safe WHERE name = '{}'", user_input);

    println!("  アンチパターン: 文字列結合によるクエリ構築");
    println!("    危険: format!(\"SELECT * FROM users WHERE name = '{{}}'\", user_input)");

    // 正解: プリペアドステートメント
    let user_input = "Alice";
    let users: Vec<(String,)> = sqlx::query_as("SELECT name FROM users_safe WHERE name = $1")
        .bind(user_input)
        .fetch_all(pool)
        .await?;
    println!("\n  プリペアドステートメント: {:?}", users);

    // 動的なWHERE句の安全な構築
    println!("\n  動的WHERE句の安全な構築:");

    struct SearchParams {
        name: Option<String>,
        has_email: Option<bool>,
    }

    let params = SearchParams {
        name: Some("Alice".to_string()),
        has_email: Some(true),
    };

    let users: Vec<(String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT name, email FROM users_safe
        WHERE ($1::text IS NULL OR name = $1)
          AND ($2::boolean IS NULL OR (email IS NOT NULL) = $2)
        "#,
    )
    .bind(&params.name)
    .bind(&params.has_email)
    .fetch_all(pool)
    .await?;

    for (name, email) in &users {
        println!("    {} - {:?}", name, email);
    }

    // 動的なORDER BY（ホワイトリスト方式）
    println!("\n  動的ORDER BY（ホワイトリスト方式）:");

    enum SortColumn {
        Name,
        CreatedAt,
    }

    impl SortColumn {
        fn as_str(&self) -> &'static str {
            match self {
                SortColumn::Name => "name",
                SortColumn::CreatedAt => "created_at",
            }
        }
    }

    let sort_by = SortColumn::Name;
    let query = format!(
        "SELECT name FROM users_safe ORDER BY {} LIMIT 3",
        sort_by.as_str()
    );
    let users: Vec<(String,)> = sqlx::query_as(&query).fetch_all(pool).await?;
    println!("    ソート結果: {:?}", users);

    println!();
    Ok(())
}

/// 4. シー・ノー・エビル: エラーを無視しない
async fn demo_error_handling(pool: &PgPool) -> Result<()> {
    println!("--- 4. エラーハンドリング ---");

    // カスタムエラー型
    #[derive(Error, Debug)]
    enum AppError {
        #[error("Database error: {0}")]
        Database(#[from] sqlx::Error),
        #[error("User not found: {0}")]
        UserNotFound(String),
        #[error("Category already exists: {0}")]
        DuplicateCategory(String),
        #[error("Foreign key violation")]
        ForeignKeyViolation,
    }

    // 制約違反の検出
    println!("  制約違反の検出:");

    let result = sqlx::query("INSERT INTO categories (id, name) VALUES (1, 'Electronics')")
        .execute(pool)
        .await;

    match result {
        Ok(_) => println!("    挿入成功"),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_unique_violation() {
                println!("    ユニーク制約違反: {:?}", db_err.message());
            } else if db_err.is_foreign_key_violation() {
                println!("    外部キー制約違反: {:?}", db_err.message());
            } else {
                println!("    その他のDBエラー: {:?}", db_err.message());
            }
        }
        Err(e) => println!("    エラー: {:?}", e),
    }

    // 外部キー違反の検出
    println!("\n  外部キー違反の検出:");
    let result = sqlx::query("INSERT INTO product_categories (product_id, category_id) VALUES (999, 1)")
        .execute(pool)
        .await;

    match result {
        Ok(_) => println!("    挿入成功"),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_foreign_key_violation() {
                println!("    外部キー制約違反: 存在しない商品ID");
            }
        }
        Err(e) => println!("    エラー: {:?}", e),
    }

    // トランザクションのエラーハンドリング
    println!("\n  トランザクション:");
    let mut tx = pool.begin().await?;

    let result: Result<(), sqlx::Error> = async {
        sqlx::query("INSERT INTO categories (name) VALUES ('New Category')")
            .execute(&mut *tx)
            .await?;

        // 意図的なエラー
        sqlx::query("INSERT INTO categories (name) VALUES ('Electronics')") // 重複
            .execute(&mut *tx)
            .await?;

        Ok(())
    }
    .await;

    match result {
        Ok(_) => {
            tx.commit().await?;
            println!("    コミット成功");
        }
        Err(e) => {
            tx.rollback().await?;
            println!("    ロールバック: {:?}", e);
        }
    }

    println!();
    Ok(())
}

/// 5. 型変換のベストプラクティス
async fn demo_type_conversion(pool: &PgPool) -> Result<()> {
    println!("--- 5. 型変換 ---");

    // DECIMAL型とrust_decimal
    println!("  DECIMAL + rust_decimal:");
    let price = Decimal::from_str("1999.99")?;
    let result: (Decimal,) =
        sqlx::query_as("SELECT price FROM products_typed WHERE id = 1")
            .fetch_one(pool)
            .await?;
    println!("    価格: {}", result.0);
    println!("    計算: {} * 3 = {}", result.0, result.0 * Decimal::from(3));

    // JSONB型とserde_json
    println!("\n  JSONB + serde:");

    #[derive(Debug, Serialize, Deserialize)]
    struct ProductMetadata {
        category: String,
        weight: f64,
    }

    let result: (serde_json::Value,) =
        sqlx::query_as("SELECT metadata FROM products_typed WHERE id = 1")
            .fetch_one(pool)
            .await?;

    let metadata: ProductMetadata = serde_json::from_value(result.0)?;
    println!("    メタデータ: {:?}", metadata);

    // TIMESTAMPTZ型とchrono
    println!("\n  TIMESTAMPTZ + chrono:");
    let result: (DateTime<Utc>,) =
        sqlx::query_as("SELECT created_at FROM products_typed WHERE id = 1")
            .fetch_one(pool)
            .await?;
    println!("    作成日時: {}", result.0);
    println!("    フォーマット: {}", result.0.format("%Y-%m-%d %H:%M:%S"));

    // カスタム型のマッピング
    println!("\n  カスタム型:");

    #[derive(Debug)]
    struct Product {
        id: i32,
        name: String,
        price: Decimal,
        metadata: ProductMetadata,
        created_at: DateTime<Utc>,
    }

    let row: (i32, String, Decimal, serde_json::Value, DateTime<Utc>) =
        sqlx::query_as("SELECT id, name, price, metadata, created_at FROM products_typed WHERE id = 1")
            .fetch_one(pool)
            .await?;

    let product = Product {
        id: row.0,
        name: row.1,
        price: row.2,
        metadata: serde_json::from_value(row.3)?,
        created_at: row.4,
    };
    println!("    {:?}", product);

    println!();
    Ok(())
}

/// 1b. NULL処理の高度なパターン
async fn demo_null_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 1b. NULL処理（高度なパターン） ---");

    // NULLIF: 特定の値をNULLに変換
    println!("  NULLIF（特定値をNULLに変換）:");
    let result: (Option<String>,) = sqlx::query_as(
        "SELECT NULLIF(email, 'no-email@example.com') FROM users_safe WHERE name = 'Bob'"
    )
    .fetch_one(pool)
    .await?;
    println!("    NULLIF(email, 'no-email@example.com'): {:?}", result.0);

    // NULL順序制御
    println!("\n  ORDER BY での NULL 順序:");
    let result: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, email FROM users_safe ORDER BY email NULLS FIRST"
    )
    .fetch_all(pool)
    .await?;
    println!("    NULLS FIRST:");
    for (name, email) in result.iter().take(3) {
        println!("      {} - {:?}", name, email);
    }

    let result: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, email FROM users_safe ORDER BY email NULLS LAST"
    )
    .fetch_all(pool)
    .await?;
    println!("    NULLS LAST:");
    for (name, email) in result.iter().take(3) {
        println!("      {} - {:?}", name, email);
    }

    // IS DISTINCT FROM（NULLを含む比較）
    println!("\n  IS DISTINCT FROM（NULL安全な比較）:");
    println!("    NULL = NULL     → NULL（不定）");
    println!("    NULL IS DISTINCT FROM NULL → false（同じと判定）");
    println!("    'a' IS DISTINCT FROM NULL  → true");

    let result: (bool,) = sqlx::query_as(
        "SELECT NULL IS DISTINCT FROM NULL"
    )
    .fetch_one(pool)
    .await?;
    println!("    結果: {}", result.0);

    // Rustでの null 処理パターン
    println!("\n  Rustでのパターン:");
    println!("    Option<T>.unwrap_or(default)  → NULL時にデフォルト値");
    println!("    Option<T>.map(|v| ...)        → 値がある場合のみ変換");
    println!("    Option<T>.and_then(|v| ...)   → ネストしたOptionの平坦化");
    println!();

    Ok(())
}

/// 3b. QueryBuilder による動的クエリ構築
async fn demo_query_builder(pool: &PgPool) -> Result<()> {
    println!("--- 3b. QueryBuilder（動的クエリ構築） ---");

    use sqlx::QueryBuilder;

    // 動的なIN句
    println!("  動的IN句:");
    let names = vec!["Alice", "Bob"];
    let mut builder = QueryBuilder::new("SELECT name, email FROM users_safe WHERE name IN (");

    let mut separated = builder.separated(", ");
    for name in &names {
        separated.push_bind(*name);
    }
    separated.push_unseparated(")");

    let query = builder.build_query_as::<(String, Option<String>)>();
    let results = query.fetch_all(pool).await?;
    for (name, email) in &results {
        println!("    {} - {:?}", name, email);
    }

    // 動的なWHERE条件
    println!("\n  動的WHERE条件:");

    #[derive(Default)]
    struct UserFilter {
        name: Option<String>,
        has_email: Option<bool>,
    }

    let filter = UserFilter {
        name: Some("Alice".to_string()),
        has_email: Some(true),
    };

    let mut builder = QueryBuilder::new("SELECT name, email FROM users_safe WHERE 1=1");

    if let Some(ref name) = filter.name {
        builder.push(" AND name = ");
        builder.push_bind(name);
    }

    if let Some(has_email) = filter.has_email {
        if has_email {
            builder.push(" AND email IS NOT NULL");
        } else {
            builder.push(" AND email IS NULL");
        }
    }

    let query = builder.build_query_as::<(String, Option<String>)>();
    let results = query.fetch_all(pool).await?;
    println!("    フィルタ結果:");
    for (name, email) in &results {
        println!("      {} - {:?}", name, email);
    }

    // 動的なUPSERT（INSERT ... ON CONFLICT）
    println!("\n  動的UPSERT:");
    println!("    INSERT INTO ... ON CONFLICT (key) DO UPDATE SET ...");
    println!("    → QueryBuilder で動的にカラムを構築");

    // バルクインサート
    println!("\n  バルクインサート:");
    let new_users = vec![
        ("NewUser1", "new1@example.com"),
        ("NewUser2", "new2@example.com"),
    ];

    let mut builder = QueryBuilder::new(
        "INSERT INTO users_safe (name, email) "
    );

    builder.push_values(new_users.iter(), |mut b, (name, email)| {
        b.push_bind(*name).push_bind(*email);
    });

    builder.push(" ON CONFLICT DO NOTHING");

    let query = builder.build();
    let result = query.execute(pool).await?;
    println!("    {} 件挿入", result.rows_affected());
    println!();

    Ok(())
}

/// 4b. エラーハンドリング（高度なパターン）
async fn demo_error_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 4b. エラーハンドリング（高度なパターン） ---");

    // リトライ戦略
    println!("  リトライ戦略:");
    println!("    一時的なエラー（ネットワーク障害など）はリトライ可能");

    async fn with_retry<F, Fut, T>(max_retries: u32, mut f: F) -> Result<T, sqlx::Error>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let mut attempt = 0;
        loop {
            match f().await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    attempt += 1;
                    if attempt >= max_retries {
                        return Err(e);
                    }
                    // 一時的なエラーのみリトライ
                    match &e {
                        sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut => {
                            println!("      リトライ {}/{}", attempt, max_retries);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        _ => return Err(e),
                    }
                }
            }
        }
    }

    println!("    リトライ可能: Io, PoolTimedOut");
    println!("    リトライ不可: Database（制約違反など）");

    // コネクションプール設定
    println!("\n  コネクションプール設定:");
    println!("    PgPoolOptions::new()");
    println!("        .max_connections(10)        // 最大接続数");
    println!("        .min_connections(2)         // 最小接続数");
    println!("        .acquire_timeout(Duration::from_secs(5))  // 取得タイムアウト");
    println!("        .idle_timeout(Duration::from_secs(60))    // アイドルタイムアウト");
    println!("        .max_lifetime(Duration::from_secs(1800))  // 最大生存時間");

    // デッドロック検出
    println!("\n  デッドロック検出:");
    println!("    sqlx::Error::Database(db_err) => {{");
    println!("        if db_err.code() == Some(Cow::Borrowed(\"40P01\")) {{");
    println!("            // deadlock_detected");
    println!("        }}");
    println!("    }}");

    // カスタムエラー型への変換
    println!("\n  カスタムエラー型:");
    #[derive(Debug)]
    enum AppDbError {
        NotFound,
        DuplicateKey(String),
        ForeignKeyViolation(String),
        Other(sqlx::Error),
    }

    impl From<sqlx::Error> for AppDbError {
        fn from(e: sqlx::Error) -> Self {
            match &e {
                sqlx::Error::RowNotFound => AppDbError::NotFound,
                sqlx::Error::Database(db_err) => {
                    if db_err.is_unique_violation() {
                        AppDbError::DuplicateKey(db_err.message().to_string())
                    } else if db_err.is_foreign_key_violation() {
                        AppDbError::ForeignKeyViolation(db_err.message().to_string())
                    } else {
                        AppDbError::Other(e)
                    }
                }
                _ => AppDbError::Other(e),
            }
        }
    }
    println!("    sqlx::Error → AppDbError に変換して適切に処理");
    println!();

    Ok(())
}

/// 5b. UUID処理
async fn demo_uuid_handling(pool: &PgPool) -> Result<()> {
    println!("--- 5b. UUID処理 ---");

    // UUID拡張機能
    sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
        .execute(pool)
        .await?;

    // UUIDテーブル作成
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS entities (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            name TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await?;

    // Rust側でUUID生成して挿入
    println!("  Rust側でUUID生成:");
    let rust_uuid = Uuid::new_v4();
    sqlx::query("INSERT INTO entities (id, name) VALUES ($1, $2)")
        .bind(rust_uuid)
        .bind("Rust生成")
        .execute(pool)
        .await?;
    println!("    生成したUUID: {}", rust_uuid);

    // PostgreSQL側でUUID生成
    println!("\n  PostgreSQL側でUUID生成:");
    let result: (Uuid, String) = sqlx::query_as(
        "INSERT INTO entities (name) VALUES ('PostgreSQL生成') RETURNING id, name"
    )
    .fetch_one(pool)
    .await?;
    println!("    生成されたUUID: {}", result.0);

    // UUID検索
    println!("\n  UUID検索:");
    let entity: (Uuid, String) = sqlx::query_as(
        "SELECT id, name FROM entities WHERE id = $1"
    )
    .bind(rust_uuid)
    .fetch_one(pool)
    .await?;
    println!("    検索結果: {} - {}", entity.0, entity.1);

    // UUIDの利点と欠点
    println!("\n  UUID vs SERIAL:");
    println!("    ┌────────────┬───────────────────┬───────────────────┐");
    println!("    │ 観点       │ UUID              │ SERIAL/BIGSERIAL  │");
    println!("    ├────────────┼───────────────────┼───────────────────┤");
    println!("    │ 衝突       │ ◎ 実質なし       │ △ シーケンス依存 │");
    println!("    │ 分散生成   │ ◎ クライアント可 │ × DB必須         │");
    println!("    │ サイズ     │ △ 16バイト       │ ◎ 4-8バイト      │");
    println!("    │ 順序性     │ × ランダム       │ ◎ 順序あり       │");
    println!("    │ インデックス│ △ 断片化しやすい │ ◎ 効率的         │");
    println!("    └────────────┴───────────────────┴───────────────────┘");

    println!("\n  UUIDv7（時刻順ソート可能）:");
    println!("    uuid クレートでは uuid::Uuid::now_v7() で生成可能（要 v7 feature）");
    println!("    → インデックス断片化の問題を解決");

    // クリーンアップ
    sqlx::query("DROP TABLE IF EXISTS entities CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
