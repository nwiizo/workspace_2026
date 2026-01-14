//! マルチテナント設計の3つのアプローチデモ
//!
//! このデモでは以下を検証:
//! 1. テナントID列によるデータ分離
//! 2. Row Level Security（RLS）によるデータ分離
//! 3. テナントコンテキストを使ったクエリ

use anyhow::Result;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// ================================
// データ構造
// ================================

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub subdomain: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub name: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Project {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
}

// ================================
// データベースセットアップ
// ================================

async fn setup_database(pool: &PgPool) -> Result<()> {
    // テーブル削除
    sqlx::query("DROP TABLE IF EXISTS projects CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS tenants CASCADE")
        .execute(pool)
        .await?;

    // テナントテーブル
    sqlx::query(
        r#"
        CREATE TABLE tenants (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            subdomain TEXT UNIQUE NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ユーザーテーブル
    sqlx::query(
        r#"
        CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (tenant_id, email)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // プロジェクトテーブル
    sqlx::query(
        r#"
        CREATE TABLE projects (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            owner_id UUID NOT NULL REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // インデックス
    sqlx::query("CREATE INDEX idx_users_tenant ON users(tenant_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX idx_projects_tenant ON projects(tenant_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX idx_projects_tenant_owner ON projects(tenant_id, owner_id)")
        .execute(pool)
        .await?;

    println!("Tables created successfully");
    Ok(())
}

async fn setup_rls(pool: &PgPool) -> Result<()> {
    println!("\n=== Setting up Row Level Security ===");

    // RLSを有効化
    sqlx::query("ALTER TABLE users ENABLE ROW LEVEL SECURITY")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE projects ENABLE ROW LEVEL SECURITY")
        .execute(pool)
        .await?;

    // 既存のポリシーを削除（エラーを無視）
    let _ = sqlx::query("DROP POLICY IF EXISTS tenant_isolation_users ON users")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP POLICY IF EXISTS tenant_isolation_projects ON projects")
        .execute(pool)
        .await;

    // テナント分離ポリシーを作成
    sqlx::query(
        r#"
        CREATE POLICY tenant_isolation_users ON users
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE POLICY tenant_isolation_projects ON projects
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID)
        "#,
    )
    .execute(pool)
    .await?;

    println!("RLS policies created");
    Ok(())
}

// ================================
// サンプルデータ作成
// ================================

async fn insert_sample_data(pool: &PgPool) -> Result<(Uuid, Uuid)> {
    println!("\n=== Inserting Sample Data ===");

    // テナントA
    let tenant_a: Uuid =
        sqlx::query_scalar("INSERT INTO tenants (name, subdomain) VALUES ($1, $2) RETURNING id")
            .bind("Company Alpha")
            .bind("alpha")
            .fetch_one(pool)
            .await?;
    println!("Created tenant A: {} (alpha)", tenant_a);

    // テナントB
    let tenant_b: Uuid =
        sqlx::query_scalar("INSERT INTO tenants (name, subdomain) VALUES ($1, $2) RETURNING id")
            .bind("Company Beta")
            .bind("beta")
            .fetch_one(pool)
            .await?;
    println!("Created tenant B: {} (beta)", tenant_b);

    // テナントAのユーザー
    let user_a1: Uuid = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, email, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_a)
    .bind("alice@alpha.com")
    .bind("Alice (Alpha)")
    .fetch_one(pool)
    .await?;

    let user_a2: Uuid = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, email, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_a)
    .bind("bob@alpha.com")
    .bind("Bob (Alpha)")
    .fetch_one(pool)
    .await?;
    println!("Created 2 users for tenant A");

    // テナントBのユーザー
    let user_b1: Uuid = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, email, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_b)
    .bind("charlie@beta.com")
    .bind("Charlie (Beta)")
    .fetch_one(pool)
    .await?;
    println!("Created 1 user for tenant B");

    // テナントAのプロジェクト
    sqlx::query("INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3)")
        .bind(tenant_a)
        .bind("Alpha Project 1")
        .bind(user_a1)
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3)")
        .bind(tenant_a)
        .bind("Alpha Project 2")
        .bind(user_a2)
        .execute(pool)
        .await?;
    println!("Created 2 projects for tenant A");

    // テナントBのプロジェクト
    sqlx::query("INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3)")
        .bind(tenant_b)
        .bind("Beta Project 1")
        .bind(user_b1)
        .execute(pool)
        .await?;
    println!("Created 1 project for tenant B");

    Ok((tenant_a, tenant_b))
}

// ================================
// アプローチ1: テナントID列
// ================================

async fn demo_tenant_id_column(pool: &PgPool, tenant_a: Uuid, tenant_b: Uuid) -> Result<()> {
    println!("\n=== Demo: Tenant ID Column Approach ===");

    // テナントAのコンテキスト
    let ctx_a = TenantContext {
        tenant_id: tenant_a,
        user_id: Uuid::new_v4(), // ダミー
    };

    // テナントAのユーザーを取得（WHERE tenant_id = $1 を常に付ける）
    let users_a: Vec<User> =
        sqlx::query_as("SELECT id, tenant_id, email, name FROM users WHERE tenant_id = $1")
            .bind(ctx_a.tenant_id)
            .fetch_all(pool)
            .await?;

    println!("\nTenant A users (using explicit WHERE clause):");
    for user in &users_a {
        println!("  - {} ({})", user.name, user.email);
    }

    // テナントBのコンテキスト
    let ctx_b = TenantContext {
        tenant_id: tenant_b,
        user_id: Uuid::new_v4(),
    };

    let users_b: Vec<User> =
        sqlx::query_as("SELECT id, tenant_id, email, name FROM users WHERE tenant_id = $1")
            .bind(ctx_b.tenant_id)
            .fetch_all(pool)
            .await?;

    println!("\nTenant B users (using explicit WHERE clause):");
    for user in &users_b {
        println!("  - {} ({})", user.name, user.email);
    }

    // クロステナントアクセスの防止デモ
    println!("\n--- Cross-tenant access prevention ---");
    let wrong_tenant_projects: Vec<Project> = sqlx::query_as(
        "SELECT id, tenant_id, name, owner_id FROM projects WHERE tenant_id = $1 AND owner_id = $2",
    )
    .bind(ctx_a.tenant_id)
    .bind(Uuid::new_v4()) // 存在しないユーザーID
    .fetch_all(pool)
    .await?;

    println!(
        "Projects found with wrong user ID: {} (should be 0 or own projects only)",
        wrong_tenant_projects.len()
    );

    Ok(())
}

// ================================
// アプローチ2: Row Level Security
// ================================

async fn demo_rls(pool: &PgPool, tenant_a: Uuid, tenant_b: Uuid) -> Result<()> {
    println!("\n=== Demo: Row Level Security (RLS) ===");

    // テナントAとして接続（セッション変数を設定）
    println!("\n--- As Tenant A ---");
    {
        let mut tx = pool.begin().await?;

        // セッション変数でテナントIDを設定
        sqlx::query(&format!("SET LOCAL app.current_tenant_id = '{}'", tenant_a))
            .execute(&mut *tx)
            .await?;

        // RLSにより自動的にテナントAのデータのみが取得される
        let users: Vec<User> = sqlx::query_as("SELECT id, tenant_id, email, name FROM users")
            .fetch_all(&mut *tx)
            .await?;

        println!("Users visible to Tenant A (via RLS):");
        for user in &users {
            println!("  - {} ({})", user.name, user.email);
        }

        let projects: Vec<Project> =
            sqlx::query_as("SELECT id, tenant_id, name, owner_id FROM projects")
                .fetch_all(&mut *tx)
                .await?;

        println!("Projects visible to Tenant A (via RLS):");
        for project in &projects {
            println!("  - {}", project.name);
        }

        // トランザクション終了（ロールバック）
    }

    // テナントBとして接続
    println!("\n--- As Tenant B ---");
    {
        let mut tx = pool.begin().await?;

        sqlx::query(&format!("SET LOCAL app.current_tenant_id = '{}'", tenant_b))
            .execute(&mut *tx)
            .await?;

        let users: Vec<User> = sqlx::query_as("SELECT id, tenant_id, email, name FROM users")
            .fetch_all(&mut *tx)
            .await?;

        println!("Users visible to Tenant B (via RLS):");
        for user in &users {
            println!("  - {} ({})", user.name, user.email);
        }

        let projects: Vec<Project> =
            sqlx::query_as("SELECT id, tenant_id, name, owner_id FROM projects")
                .fetch_all(&mut *tx)
                .await?;

        println!("Projects visible to Tenant B (via RLS):");
        for project in &projects {
            println!("  - {}", project.name);
        }
    }

    // テナントIDなしで接続（RLSにより何も見えない）
    println!("\n--- Without tenant context ---");
    {
        let mut tx = pool.begin().await?;

        // テナントIDを設定しない（空文字列）
        sqlx::query("SET LOCAL app.current_tenant_id = ''")
            .execute(&mut *tx)
            .await?;

        let users: Vec<User> = sqlx::query_as("SELECT id, tenant_id, email, name FROM users")
            .fetch_all(&mut *tx)
            .await?;

        println!(
            "Users visible without tenant context: {} (should be 0)",
            users.len()
        );
    }

    Ok(())
}

// ================================
// データ整合性チェック
// ================================

async fn demo_cross_tenant_protection(pool: &PgPool, tenant_a: Uuid, tenant_b: Uuid) -> Result<()> {
    println!("\n=== Demo: Cross-Tenant Data Protection ===");

    // テナントBのユーザーを取得
    let user_b: User =
        sqlx::query_as("SELECT id, tenant_id, email, name FROM users WHERE tenant_id = $1 LIMIT 1")
            .bind(tenant_b)
            .fetch_one(pool)
            .await?;

    // テナントAとしてテナントBのユーザーが所有するプロジェクトを作成しようとする
    // （外部キー制約で失敗するはず - ただしowner_idはusersテーブルを参照するだけ）
    println!("\nAttempting to create project in Tenant A with Tenant B's user as owner:");

    let result =
        sqlx::query("INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3)")
            .bind(tenant_a)
            .bind("Sneaky Project")
            .bind(user_b.id) // テナントBのユーザー
            .execute(pool)
            .await;

    match result {
        Ok(_) => {
            println!("  Warning: Insert succeeded (FK only checks user exists, not tenant)");
            println!("  This is why application-level tenant checks are important!");

            // クリーンアップ
            sqlx::query("DELETE FROM projects WHERE name = 'Sneaky Project'")
                .execute(pool)
                .await?;
        }
        Err(e) => {
            println!("  Insert failed (as expected): {}", e);
        }
    }

    // 正しい方法：CHECK制約またはトリガーで防ぐ
    println!("\nBetter approach: Add a CHECK constraint or use RLS with strict policies");

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
    setup_rls(&pool).await?;

    let (tenant_a, tenant_b) = insert_sample_data(&pool).await?;

    demo_tenant_id_column(&pool, tenant_a, tenant_b).await?;
    demo_rls(&pool, tenant_a, tenant_b).await?;
    demo_cross_tenant_protection(&pool, tenant_a, tenant_b).await?;

    println!("\n=== All multi-tenant demos completed successfully! ===");
    Ok(())
}
