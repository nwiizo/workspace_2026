//! セキュリティパターンのデモ
//!
//! このデモでは以下を検証:
//! 1. パスワードのArgon2ハッシュ化
//! 2. 平文保存アンチパターン
//! 3. 機密データの分離
//! 4. Row Level Security (RLS)
//! 5. SQLインジェクション対策

use anyhow::Result;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ================================
// パスワードハッシュ関数
// ================================

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing error: {}", e))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Password hash parsing error: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ================================
// データ構造
// ================================

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct PublicUser {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserCredentials {
    id: Uuid,
    password_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct Post {
    id: Uuid,
    user_id: Uuid,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS posts_rls CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users_secure CASCADE")
        .execute(pool)
        .await?;

    // ユーザーテーブル（パスワードハッシュ付き）
    sqlx::query(
        r#"
        CREATE TABLE users_secure (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 投稿テーブル（RLS用）
    sqlx::query(
        r#"
        CREATE TABLE posts_rls (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users_secure(id) ON DELETE CASCADE,
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    println!("Tables created successfully");
    Ok(())
}

// ================================
// Demo: パスワードハッシュ
// ================================

async fn demo_password_hashing(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Password Hashing with Argon2 ===");

    let email = "alice@example.com";
    let password = "SecureP@ssw0rd123";

    // パスワードをハッシュ化
    let password_hash = hash_password(password)?;
    println!("Original password: {}", password);
    println!("Hashed password: {}...", &password_hash[..50]);

    // ユーザーを作成
    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users_secure (email, password_hash, name)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(&password_hash)
    .bind("Alice")
    .fetch_one(pool)
    .await?;

    println!("Created user: {}", user_id);

    // 認証のシミュレーション
    println!("\n--- Authentication Simulation ---");

    // 正しいパスワードで認証
    let user_creds: UserCredentials = sqlx::query_as(
        "SELECT id, password_hash FROM users_secure WHERE email = $1",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;

    let is_valid = verify_password(password, &user_creds.password_hash)?;
    println!("Correct password verification: {}", is_valid);

    // 間違ったパスワードで認証
    let is_invalid = verify_password("WrongPassword", &user_creds.password_hash)?;
    println!("Wrong password verification: {}", is_invalid);

    // ハッシュは毎回異なる（ソルトが異なる）
    let hash1 = hash_password("test")?;
    let hash2 = hash_password("test")?;
    println!("\nDifferent hashes for same password (due to salt):");
    println!("  Hash 1: {}...", &hash1[..40]);
    println!("  Hash 2: {}...", &hash2[..40]);
    println!("  Same? {}", hash1 == hash2);

    Ok(())
}

// ================================
// Demo: 機密データの分離
// ================================

async fn demo_data_separation(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Sensitive Data Separation ===");

    // 公開情報のみを取得（password_hashを含まない）
    let public_users: Vec<PublicUser> = sqlx::query_as(
        "SELECT id, name, created_at FROM users_secure ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    println!("Public user data (no password_hash):");
    for user in &public_users {
        println!("  - {} ({})", user.name, user.id);
    }

    // 内部用に全データを取得
    let internal_users: Vec<UserRow> = sqlx::query_as(
        "SELECT id, email, password_hash, name, created_at FROM users_secure",
    )
    .fetch_all(pool)
    .await?;

    println!("\nInternal user data (includes password_hash):");
    for user in &internal_users {
        println!("  - {} <{}> (hash: {}...)", user.name, user.email, &user.password_hash[..20]);
    }

    println!("\nBest practice: Use different structs for public vs internal data");

    Ok(())
}

// ================================
// Demo: Row Level Security
// ================================

async fn setup_rls(pool: &PgPool) -> Result<()> {
    println!("\n=== Setup: Row Level Security ===");

    // RLSを有効化
    sqlx::query("ALTER TABLE posts_rls ENABLE ROW LEVEL SECURITY")
        .execute(pool)
        .await?;

    // 既存のポリシーを削除
    let _ = sqlx::query("DROP POLICY IF EXISTS posts_owner_policy ON posts_rls")
        .execute(pool)
        .await;

    // ポリシーを作成
    sqlx::query(
        r#"
        CREATE POLICY posts_owner_policy ON posts_rls
            FOR ALL
            USING (user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::uuid)
        "#,
    )
    .execute(pool)
    .await?;

    println!("RLS enabled with owner policy");
    Ok(())
}

async fn demo_rls(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: Row Level Security ===");

    // 2人のユーザーを作成
    let user_a: Uuid = sqlx::query_scalar(
        "INSERT INTO users_secure (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("bob@example.com")
    .bind(hash_password("password")?)
    .bind("Bob")
    .fetch_one(pool)
    .await?;

    let user_b: Uuid = sqlx::query_scalar(
        "INSERT INTO users_secure (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("charlie@example.com")
    .bind(hash_password("password")?)
    .bind("Charlie")
    .fetch_one(pool)
    .await?;

    println!("Created users: Bob ({}), Charlie ({})", user_a, user_b);

    // 各ユーザーの投稿を作成
    sqlx::query("INSERT INTO posts_rls (user_id, title, content) VALUES ($1, $2, $3)")
        .bind(user_a)
        .bind("Bob's Post 1")
        .bind("Content from Bob")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO posts_rls (user_id, title, content) VALUES ($1, $2, $3)")
        .bind(user_a)
        .bind("Bob's Post 2")
        .bind("More content from Bob")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO posts_rls (user_id, title, content) VALUES ($1, $2, $3)")
        .bind(user_b)
        .bind("Charlie's Post")
        .bind("Content from Charlie")
        .execute(pool)
        .await?;

    // RLSなし（スーパーユーザー）での全件取得
    let all_posts: Vec<Post> = sqlx::query_as(
        "SELECT id, user_id, title, content, created_at FROM posts_rls",
    )
    .fetch_all(pool)
    .await?;
    println!("\nWithout RLS (superuser): {} posts", all_posts.len());

    // Bobとして投稿を取得
    println!("\n--- As Bob ---");
    {
        let mut tx = pool.begin().await?;
        sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_a))
            .execute(&mut *tx)
            .await?;

        let bob_posts: Vec<Post> = sqlx::query_as(
            "SELECT id, user_id, title, content, created_at FROM posts_rls",
        )
        .fetch_all(&mut *tx)
        .await?;

        println!("Posts visible to Bob: {}", bob_posts.len());
        for post in &bob_posts {
            println!("  - {}", post.title);
        }
    }

    // Charlieとして投稿を取得
    println!("\n--- As Charlie ---");
    {
        let mut tx = pool.begin().await?;
        sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_b))
            .execute(&mut *tx)
            .await?;

        let charlie_posts: Vec<Post> = sqlx::query_as(
            "SELECT id, user_id, title, content, created_at FROM posts_rls",
        )
        .fetch_all(&mut *tx)
        .await?;

        println!("Posts visible to Charlie: {}", charlie_posts.len());
        for post in &charlie_posts {
            println!("  - {}", post.title);
        }
    }

    // ユーザーコンテキストなし
    println!("\n--- Without user context ---");
    {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL app.current_user_id = ''")
            .execute(&mut *tx)
            .await?;

        let no_context_posts: Vec<Post> = sqlx::query_as(
            "SELECT id, user_id, title, content, created_at FROM posts_rls",
        )
        .fetch_all(&mut *tx)
        .await?;

        println!("Posts visible without context: {} (should be 0)", no_context_posts.len());
    }

    Ok(())
}

// ================================
// Demo: SQLインジェクション対策
// ================================

async fn demo_sql_injection_prevention(pool: &PgPool) -> Result<()> {
    println!("\n=== Demo: SQL Injection Prevention ===");

    // 危険な入力を準備
    let malicious_input = "'; DROP TABLE users_secure; --";

    // 安全: プレースホルダーを使用
    println!("\n--- Safe: Using placeholders ---");
    let safe_result: Vec<PublicUser> = sqlx::query_as(
        "SELECT id, name, created_at FROM users_secure WHERE name ILIKE '%' || $1 || '%'",
    )
    .bind(malicious_input)
    .fetch_all(pool)
    .await?;

    println!("Search with malicious input: {} results", safe_result.len());
    println!("Table still exists (attack failed)");

    // テーブルが存在することを確認
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users_secure')",
    )
    .fetch_one(pool)
    .await?;
    println!("users_secure table exists: {}", table_exists);

    // 安全なORDER BY: enumで制限
    println!("\n--- Safe ORDER BY with whitelist ---");

    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
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
        "SELECT id, name, created_at FROM users_secure ORDER BY {} ASC",
        sort_by.as_str()
    );

    let sorted_users: Vec<PublicUser> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await?;

    println!("Users sorted by {:?}:", sort_by);
    for user in &sorted_users {
        println!("  - {}", user.name);
    }

    Ok(())
}

// ================================
// Demo: パスワード強度検証
// ================================

struct PasswordPolicy {
    min_length: usize,
    require_uppercase: bool,
    require_lowercase: bool,
    require_digit: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
        }
    }
}

fn validate_password(password: &str, policy: &PasswordPolicy) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if password.len() < policy.min_length {
        errors.push(format!("Must be at least {} characters", policy.min_length));
    }

    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Must contain an uppercase letter".to_string());
    }

    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Must contain a lowercase letter".to_string());
    }

    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("Must contain a digit".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

async fn demo_password_policy() -> Result<()> {
    println!("\n=== Demo: Password Policy Validation ===");

    let policy = PasswordPolicy::default();

    let test_passwords = vec![
        ("short", "short"),
        ("nouppercase123", "nouppercase123"),
        ("NOLOWERCASE123", "NOLOWERCASE123"),
        ("NoDigitsHere", "NoDigitsHere"),
        ("ValidP@ss123", "ValidP@ss123"),
    ];

    for (name, password) in &test_passwords {
        match validate_password(password, &policy) {
            Ok(()) => println!("'{}': Valid", name),
            Err(errors) => {
                println!("'{}': Invalid", name);
                for e in errors {
                    println!("  - {}", e);
                }
            }
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

    setup_database(&pool).await?;

    demo_password_hashing(&pool).await?;
    demo_data_separation(&pool).await?;

    setup_rls(&pool).await?;
    demo_rls(&pool).await?;

    demo_sql_injection_prevention(&pool).await?;
    demo_password_policy().await?;

    println!("\n=== All security demos completed successfully! ===");
    Ok(())
}
