//! # 論理削除を安全に実装する6つのパターン - 動作確認用コード
//!
//! このファイルは blog_12_soft_delete_patterns.md で解説している
//! 各パターンの動作確認用実装です。
//!
//! ## 実行方法
//! ```bash
//! # PostgreSQLを起動
//! docker-compose up -d
//!
//! # マイグレーション実行
//! psql postgres://postgres:postgres@localhost:5433/soft_delete_demo -f migrations/001_initial.sql
//!
//! # 実行
//! cargo run
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

// =============================================================================
// エラー型
// =============================================================================

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Record not found")]
    NotFound,
}

// =============================================================================
// パターン1: Newtype Pattern - 有効/削除済みを別の型として表現
// =============================================================================

/// 有効なユーザー（削除されていない）
#[derive(Debug, Clone, FromRow)]
pub struct ActiveUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 削除済みユーザー
#[derive(Debug, Clone, FromRow)]
pub struct DeletedUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub deleted_at: DateTime<Utc>,
}

/// ステータスを問わないユーザー（管理画面用など）
#[derive(Debug, Clone, FromRow)]
pub struct UserWithStatus {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl UserWithStatus {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl ActiveUser {
    /// 有効なユーザーを1件取得
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, name, email, created_at, updated_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 有効なユーザーを全件取得
    pub async fn all(pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, name, email, created_at, updated_at
            FROM users
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// 新規ユーザーを作成
    pub async fn create(pool: &PgPool, name: &str, email: &str) -> Result<Self, AppError> {
        let user = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO users (name, email)
            VALUES ($1, $2)
            RETURNING id, name, email, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(email)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// 論理削除を実行
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET deleted_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// 管理機能モジュール
pub mod admin {
    use super::*;

    /// 削除済みを含む全ユーザーを取得
    pub async fn all_users(pool: &PgPool) -> Result<Vec<UserWithStatus>, AppError> {
        let users = sqlx::query_as::<_, UserWithStatus>(
            r#"
            SELECT id, name, email, deleted_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// 削除済みユーザーのみ取得
    pub async fn deleted_users(pool: &PgPool) -> Result<Vec<DeletedUser>, AppError> {
        let users = sqlx::query_as::<_, DeletedUser>(
            r#"
            SELECT id, name, email, deleted_at
            FROM users
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// 削除済みユーザーを復元
    pub async fn restore_user(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET deleted_at = NULL, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

// =============================================================================
// パターン2: トレイトで共通インターフェースを定義
// =============================================================================

/// 論理削除可能なエンティティの基本トレイト
#[async_trait]
pub trait SoftDeletable: Sized {
    type Id: Send + Sync;

    /// 有効なレコードを1件取得
    async fn find_active(pool: &PgPool, id: Self::Id) -> Result<Option<Self>, AppError>;

    /// 有効なレコードを全件取得
    async fn all_active(pool: &PgPool) -> Result<Vec<Self>, AppError>;

    /// 論理削除を実行
    async fn soft_delete(pool: &PgPool, id: Self::Id) -> Result<bool, AppError>;

    /// 論理削除を取り消し（復元）
    async fn restore(pool: &PgPool, id: Self::Id) -> Result<bool, AppError>;
}

/// 投稿（トレイト実装の例）
#[derive(Debug, Clone, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
impl SoftDeletable for Post {
    type Id = Uuid;

    async fn find_active(pool: &PgPool, id: Self::Id) -> Result<Option<Self>, AppError> {
        let post = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, user_id, title, content, created_at, updated_at
            FROM posts
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(post)
    }

    async fn all_active(pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let posts = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, user_id, title, content, created_at, updated_at
            FROM posts
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    async fn soft_delete(pool: &PgPool, id: Self::Id) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE posts SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn restore(pool: &PgPool, id: Self::Id) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE posts SET deleted_at = NULL, updated_at = NOW() WHERE id = $1 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl Post {
    /// 新規投稿を作成
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        title: &str,
        content: &str,
    ) -> Result<Self, AppError> {
        let post = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO posts (user_id, title, content)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, title, content, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(content)
        .fetch_one(pool)
        .await?;

        Ok(post)
    }
}

// =============================================================================
// パターン3: ビューを活用（active_usersビューからの取得）
// =============================================================================

/// ビューからマッピングされる構造体
#[derive(Debug, Clone, FromRow)]
pub struct ActiveUserFromView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ActiveUserFromView {
    /// ビューから取得（削除済みは絶対に含まれない）
    pub async fn all(pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users = sqlx::query_as::<_, Self>(
            "SELECT id, name, email, created_at, updated_at FROM active_users",
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as::<_, Self>(
            "SELECT id, name, email, created_at, updated_at FROM active_users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as::<_, Self>(
            "SELECT id, name, email, created_at, updated_at FROM active_users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }
}

/// 投稿と著者を結合（両方とも有効なデータのみ）
#[derive(Debug, Clone, FromRow)]
pub struct PostWithAuthor {
    pub post_id: Uuid,
    pub title: String,
    pub author_name: String,
}

impl PostWithAuthor {
    pub async fn all(pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let posts = sqlx::query_as::<_, Self>(
            r#"
            SELECT
                p.id as post_id,
                p.title,
                u.name as author_name
            FROM active_posts p
            INNER JOIN active_users u ON p.user_id = u.id
            ORDER BY p.created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }
}

// =============================================================================
// パターン5: リポジトリパターンで抽象化
// =============================================================================

/// 読み取り専用リポジトリ（常に有効データのみ）
#[async_trait]
pub trait ReadRepository<T, Id>: Send + Sync {
    async fn find(&self, id: Id) -> Result<Option<T>, AppError>;
    async fn all(&self) -> Result<Vec<T>, AppError>;
    async fn exists(&self, id: Id) -> Result<bool, AppError>;
}

/// 書き込み可能リポジトリ
#[async_trait]
pub trait WriteRepository<T, Id>: ReadRepository<T, Id> {
    type CreateInput: Send + Sync;

    async fn create(&self, input: Self::CreateInput) -> Result<T, AppError>;
    async fn delete(&self, id: Id) -> Result<bool, AppError>; // 論理削除
}

/// ユーザー作成入力
pub struct CreateUserInput {
    pub name: String,
    pub email: String,
}

/// ユーザーリポジトリ実装
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadRepository<ActiveUser, Uuid> for UserRepository {
    async fn find(&self, id: Uuid) -> Result<Option<ActiveUser>, AppError> {
        ActiveUser::find_by_id(&self.pool, id).await
    }

    async fn all(&self) -> Result<Vec<ActiveUser>, AppError> {
        ActiveUser::all(&self.pool).await
    }

    async fn exists(&self, id: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL
            ) as exists
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<bool, _>("exists"))
    }
}

#[async_trait]
impl WriteRepository<ActiveUser, Uuid> for UserRepository {
    type CreateInput = CreateUserInput;

    async fn create(&self, input: Self::CreateInput) -> Result<ActiveUser, AppError> {
        ActiveUser::create(&self.pool, &input.name, &input.email).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, AppError> {
        ActiveUser::soft_delete(&self.pool, id).await
    }
}

// =============================================================================
// パターン6: マクロで定型コードを削減
// =============================================================================

/// 論理削除対応のメソッドを自動生成するマクロ
macro_rules! impl_soft_deletable_basic {
    ($struct:ident, $table:literal) => {
        impl $struct {
            /// 論理削除を実行
            pub async fn soft_delete_by_macro(
                pool: &::sqlx::PgPool,
                id: ::uuid::Uuid,
            ) -> Result<bool, AppError> {
                let result = ::sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                    $table
                ))
                .bind(id)
                .execute(pool)
                .await?;

                Ok(result.rows_affected() > 0)
            }

            /// 論理削除を取り消し
            pub async fn restore_by_macro(
                pool: &::sqlx::PgPool,
                id: ::uuid::Uuid,
            ) -> Result<bool, AppError> {
                let result = ::sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL",
                    $table
                ))
                .bind(id)
                .execute(pool)
                .await?;

                Ok(result.rows_affected() > 0)
            }
        }
    };
}

/// コメント
#[derive(Debug, Clone, FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl Comment {
    pub async fn create(
        pool: &PgPool,
        post_id: Uuid,
        user_id: Uuid,
        body: &str,
    ) -> Result<Self, AppError> {
        let comment = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO comments (post_id, user_id, body)
            VALUES ($1, $2, $3)
            RETURNING id, post_id, user_id, body, created_at
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .bind(body)
        .fetch_one(pool)
        .await?;

        Ok(comment)
    }

    pub async fn all_active(pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let comments = sqlx::query_as::<_, Self>(
            r#"
            SELECT id, post_id, user_id, body, created_at
            FROM comments
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(comments)
    }
}

// マクロを使用してsoft_delete_by_macroとrestore_by_macroを自動生成
impl_soft_deletable_basic!(Comment, "comments");

// =============================================================================
// データベースセットアップ
// =============================================================================

async fn setup_tables(pool: &PgPool) -> anyhow::Result<()> {
    // テーブルを削除（依存関係の順序で）
    sqlx::query("DROP TABLE IF EXISTS comments CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS posts CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS users CASCADE")
        .execute(pool)
        .await?;

    // usersテーブル
    sqlx::query(
        r#"
        CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(100) NOT NULL,
            email VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await?;

    // postsテーブル
    sqlx::query(
        r#"
        CREATE TABLE posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id),
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await?;

    // commentsテーブル
    sqlx::query(
        r#"
        CREATE TABLE comments (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL REFERENCES posts(id),
            user_id UUID NOT NULL REFERENCES users(id),
            body TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 部分インデックス（削除されていないレコード用）
    sqlx::query("CREATE INDEX idx_users_active ON users(id) WHERE deleted_at IS NULL")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX idx_posts_active ON posts(id) WHERE deleted_at IS NULL")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX idx_comments_active ON comments(id) WHERE deleted_at IS NULL")
        .execute(pool)
        .await?;

    // パターン3用のビュー
    sqlx::query(
        r#"
        CREATE VIEW active_users AS
        SELECT id, name, email, created_at, updated_at
        FROM users
        WHERE deleted_at IS NULL
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE VIEW active_posts AS
        SELECT id, user_id, title, content, created_at, updated_at
        FROM posts
        WHERE deleted_at IS NULL
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE VIEW active_comments AS
        SELECT id, post_id, user_id, body, created_at
        FROM comments
        WHERE deleted_at IS NULL
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// =============================================================================
// メイン関数 - 各パターンのデモ
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 論理削除パターン デモ ===\n");

    // データベース接続
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5433/soft_delete_demo".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("データベースに接続しました\n");

    // テーブルをセットアップ
    setup_tables(&pool).await?;
    println!("テーブルをセットアップしました\n");

    // ==========================================================================
    // パターン1: Newtype Pattern のデモ
    // ==========================================================================
    println!("--- パターン1: Newtype Pattern ---");

    // ユーザー作成
    let user1 = ActiveUser::create(&pool, "田中太郎", "tanaka@example.com").await?;
    let user2 = ActiveUser::create(&pool, "佐藤花子", "sato@example.com").await?;
    println!("ユーザーを作成: {} ({})", user1.name, user1.id);
    println!("ユーザーを作成: {} ({})", user2.name, user2.id);

    // 有効なユーザーを取得
    let active_users = ActiveUser::all(&pool).await?;
    println!("有効なユーザー数: {}", active_users.len());

    // ユーザーを論理削除
    ActiveUser::soft_delete(&pool, user2.id).await?;
    println!("ユーザーを論理削除: {}", user2.name);

    // 有効なユーザーを再取得（削除済みは含まれない）
    let active_users = ActiveUser::all(&pool).await?;
    println!("論理削除後の有効なユーザー数: {}", active_users.len());

    // 管理機能で削除済みを含めて取得
    let all_users = admin::all_users(&pool).await?;
    println!("全ユーザー数（削除済み含む）: {}", all_users.len());

    // 削除済みユーザーのみ取得
    let deleted_users = admin::deleted_users(&pool).await?;
    println!("削除済みユーザー数: {}", deleted_users.len());

    // 復元
    admin::restore_user(&pool, user2.id).await?;
    println!("ユーザーを復元: {}", user2.name);

    let active_users = ActiveUser::all(&pool).await?;
    println!("復元後の有効なユーザー数: {}\n", active_users.len());

    // ==========================================================================
    // パターン2: トレイト抽象化のデモ
    // ==========================================================================
    println!("--- パターン2: トレイト抽象化 ---");

    // 投稿作成
    let post1 = Post::create(&pool, user1.id, "Rustの論理削除パターン", "内容...").await?;
    let post2 = Post::create(&pool, user1.id, "型安全な設計", "内容...").await?;
    println!("投稿を作成: {}", post1.title);
    println!("投稿を作成: {}", post2.title);

    // SoftDeletableトレイトを使用
    let active_posts = Post::all_active(&pool).await?;
    println!("有効な投稿数: {}", active_posts.len());

    // トレイト経由で論理削除
    Post::soft_delete(&pool, post2.id).await?;
    println!("投稿を論理削除: {}", post2.title);

    let active_posts = Post::all_active(&pool).await?;
    println!("論理削除後の有効な投稿数: {}", active_posts.len());

    // 復元
    Post::restore(&pool, post2.id).await?;
    println!("投稿を復元: {}\n", post2.title);

    // ==========================================================================
    // パターン3: ビューを活用
    // ==========================================================================
    println!("--- パターン3: ビューを活用 ---");

    // ビューから取得（削除済みは含まれない保証）
    let users_from_view = ActiveUserFromView::all(&pool).await?;
    println!("ビューから取得したユーザー数: {}", users_from_view.len());

    // ビューを使ったJOIN
    let posts_with_authors = PostWithAuthor::all(&pool).await?;
    println!("投稿と著者の結合結果:");
    for p in &posts_with_authors {
        println!("  - 「{}」 by {}", p.title, p.author_name);
    }

    // ユーザーを論理削除すると、JOINの結果からも消える
    ActiveUser::soft_delete(&pool, user1.id).await?;
    let posts_with_authors = PostWithAuthor::all(&pool).await?;
    println!(
        "著者を論理削除後の投稿数（JOINでフィルタ）: {}\n",
        posts_with_authors.len()
    );

    // 復元
    admin::restore_user(&pool, user1.id).await?;

    // ==========================================================================
    // パターン5: リポジトリパターン
    // ==========================================================================
    println!("--- パターン5: リポジトリパターン ---");

    let user_repo = UserRepository::new(pool.clone());

    // リポジトリ経由でユーザー作成
    let user3 = user_repo
        .create(CreateUserInput {
            name: "山田次郎".to_string(),
            email: "yamada@example.com".to_string(),
        })
        .await?;
    println!("リポジトリ経由でユーザー作成: {}", user3.name);

    // 存在確認
    let exists = user_repo.exists(user3.id).await?;
    println!("ユーザー存在確認: {}", exists);

    // リポジトリ経由で論理削除
    user_repo.delete(user3.id).await?;
    let exists = user_repo.exists(user3.id).await?;
    println!("論理削除後の存在確認: {}\n", exists);

    // ==========================================================================
    // パターン6: マクロ
    // ==========================================================================
    println!("--- パターン6: マクロ ---");

    // コメント作成
    let comment = Comment::create(&pool, post1.id, user1.id, "素晴らしい記事です！").await?;
    println!("コメントを作成: {}", comment.body);

    let active_comments = Comment::all_active(&pool).await?;
    println!("有効なコメント数: {}", active_comments.len());

    // マクロで生成されたメソッドを使用
    Comment::soft_delete_by_macro(&pool, comment.id).await?;
    println!("マクロ経由で論理削除");

    let active_comments = Comment::all_active(&pool).await?;
    println!("論理削除後の有効なコメント数: {}", active_comments.len());

    Comment::restore_by_macro(&pool, comment.id).await?;
    println!("マクロ経由で復元");

    let active_comments = Comment::all_active(&pool).await?;
    println!("復元後の有効なコメント数: {}\n", active_comments.len());

    // ==========================================================================
    // まとめ
    // ==========================================================================
    println!("=== デモ完了 ===");
    println!("各パターンが正常に動作することを確認しました。");
    println!("\nパターン一覧:");
    println!("  1. Newtype Pattern - ActiveUser / DeletedUser で型を分離");
    println!("  2. トレイト抽象化 - SoftDeletable トレイトで共通化");
    println!("  3. ビュー - active_users ビューで安全なデフォルト");
    println!("  4. RLS - (PostgreSQL設定が必要なため省略)");
    println!("  5. リポジトリパターン - ReadRepository / WriteRepository");
    println!("  6. マクロ - impl_soft_deletable_basic! で自動生成");

    Ok(())
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5433/soft_delete_demo".to_string()
        });

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database")
    }

    #[tokio::test]
    async fn test_active_user_soft_delete() {
        let pool = setup_pool().await;

        // クリーンアップ
        sqlx::query("DELETE FROM comments")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM posts")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users")
            .execute(&pool)
            .await
            .unwrap();

        // ユーザー作成
        let user = ActiveUser::create(&pool, "テストユーザー", "test@example.com")
            .await
            .unwrap();

        // 作成直後は取得できる
        let found = ActiveUser::find_by_id(&pool, user.id).await.unwrap();
        assert!(found.is_some());

        // 論理削除
        let deleted = ActiveUser::soft_delete(&pool, user.id).await.unwrap();
        assert!(deleted);

        // 削除後は取得できない
        let found = ActiveUser::find_by_id(&pool, user.id).await.unwrap();
        assert!(found.is_none());

        // 管理機能では取得できる
        let deleted_users = admin::deleted_users(&pool).await.unwrap();
        assert!(!deleted_users.is_empty());

        // 復元
        let restored = admin::restore_user(&pool, user.id).await.unwrap();
        assert!(restored);

        // 復元後は取得できる
        let found = ActiveUser::find_by_id(&pool, user.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_view_excludes_deleted() {
        let pool = setup_pool().await;

        // クリーンアップ
        sqlx::query("DELETE FROM comments")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM posts")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users")
            .execute(&pool)
            .await
            .unwrap();

        // ユーザー作成
        let user = ActiveUser::create(&pool, "ビューテスト", "view_test@example.com")
            .await
            .unwrap();

        // ビューから取得できる
        let found = ActiveUserFromView::find_by_id(&pool, user.id)
            .await
            .unwrap();
        assert!(found.is_some());

        // 論理削除
        ActiveUser::soft_delete(&pool, user.id).await.unwrap();

        // ビューからは取得できない
        let found = ActiveUserFromView::find_by_id(&pool, user.id)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_repository_pattern() {
        let pool = setup_pool().await;

        // クリーンアップ
        sqlx::query("DELETE FROM comments")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM posts")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users")
            .execute(&pool)
            .await
            .unwrap();

        let repo = UserRepository::new(pool);

        // 作成
        let user = repo
            .create(CreateUserInput {
                name: "リポジトリテスト".to_string(),
                email: "repo_test@example.com".to_string(),
            })
            .await
            .unwrap();

        // 存在確認
        assert!(repo.exists(user.id).await.unwrap());

        // 削除
        repo.delete(user.id).await.unwrap();

        // 存在しなくなる
        assert!(!repo.exists(user.id).await.unwrap());
    }
}
