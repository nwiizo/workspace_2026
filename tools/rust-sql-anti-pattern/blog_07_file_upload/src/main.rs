//! ファイルアップロード機能の正しい実装 - アンチパターン検証コード
//!
//! このコードは以下のアンチパターンとPostgreSQL/Rust固有の解決策を実演します:
//!
//! ## PostgreSQLの機能で防げるもの
//! - ENUM型: 不正なステータス値をDB側で防止
//! - CHECK制約: storage_type と inline_data/storage_key の整合性
//! - 外部キー + CASCADE: 親削除時の孤立レコード自動削除
//! - 部分インデックス: クリーンアップ検索の高速化
//! - FOR UPDATE SKIP LOCKED: 並行クリーンアップの競合防止
//! - LISTEN/NOTIFY + トリガー: 非同期クリーンアップ通知
//! - Advisory Lock: 分散環境でのクリーンアップジョブ重複防止
//! - 生成列（GENERATED ALWAYS AS）: storage_key の自動計算
//!
//! ## Rustの機能で防げるもの
//! - newtype パターン: FileId, TenantId の取り違え防止
//! - 型状態パターン（typestate）: 不正な状態遷移をコンパイル時に検出
//! - RAII / Drop: トランザクションの自動ロールバック
//! - Result + ?: エラー無視の防止
//! - sqlx::Type derive: DB ENUM との型安全な連携
//!
//! PostgreSQL 12+ の機能を活用

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

const DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/antipattern";

// =============================================================================
// Rust機能1: newtype パターン - IDの取り違えをコンパイル時に防止
// =============================================================================

/// テナントID（newtype wrapper）
/// UuidをそのままUserId.0やFileId.0と取り違えることを防ぐ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ファイルID（newtype wrapper）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct FileId(Uuid);

impl FileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FileId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// Rust機能2: sqlx::Type derive - PostgreSQL ENUMとの型安全な連携
// =============================================================================

/// ファイルステータス（PostgreSQL ENUMとRust enumのマッピング）
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "file_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileStatus {
    Uploading,
    Uploaded,
}

impl FileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileStatus::Uploading => "UPLOADING",
            FileStatus::Uploaded => "UPLOADED",
        }
    }
}

// =============================================================================
// Rust機能3: 型状態パターン（Typestate） - 不正な状態遷移をコンパイル時に防止
// =============================================================================

/// アップロード中状態（マーカー型）
pub struct Uploading;

/// アップロード完了状態（マーカー型）
pub struct Uploaded;

/// 型状態パターンを使ったファイル構造体
/// 状態によって利用可能なメソッドが異なる
#[derive(Debug)]
pub struct TypedFile<State> {
    pub file_id: FileId,
    pub tenant_id: TenantId,
    pub filename: String,
    pub content_type: String,
    pub file_size: Option<i64>,
    pub created_at: DateTime<Utc>,
    _state: PhantomData<State>,
}

impl TypedFile<Uploading> {
    /// 新しいアップロード中ファイルを作成
    pub fn new(tenant_id: TenantId, filename: String, content_type: String) -> Self {
        Self {
            file_id: FileId::new(),
            tenant_id,
            filename,
            content_type,
            file_size: None,
            created_at: Utc::now(),
            _state: PhantomData,
        }
    }

    /// アップロード完了に遷移（所有権を消費して新しい状態を返す）
    pub fn complete(self, file_size: i64) -> TypedFile<Uploaded> {
        TypedFile {
            file_id: self.file_id,
            tenant_id: self.tenant_id,
            filename: self.filename,
            content_type: self.content_type,
            file_size: Some(file_size),
            created_at: self.created_at,
            _state: PhantomData,
        }
    }

    // Note: Uploading状態ではget_download_url()は存在しない
    // → コンパイル時に不正な呼び出しを防止
}

impl TypedFile<Uploaded> {
    /// ダウンロードURLを取得（Uploaded状態でのみ利用可能）
    pub fn get_download_url(&self) -> String {
        format!(
            "https://storage.example.com/{}/{}",
            self.tenant_id, self.file_id
        )
    }

    /// ファイルサイズを取得（Uploaded状態では必ず存在）
    pub fn file_size(&self) -> i64 {
        self.file_size.unwrap() // Uploaded状態では常にSome
    }
}

// =============================================================================
// メイン関数
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    println!("=== ファイルアップロード機能 デモ (PostgreSQL + Rust) ===\n");

    // PostgreSQLバージョン確認
    let version: (String,) = sqlx::query_as("SELECT version()").fetch_one(&pool).await?;
    println!(
        "PostgreSQL: {}\n",
        version.0.split(',').next().unwrap_or(&version.0)
    );

    setup_tables(&pool).await?;

    // PostgreSQLの機能デモ
    println!("========================================");
    println!("  PostgreSQLの機能で防げるもの");
    println!("========================================\n");

    demo_pg_enum_type(&pool).await?;
    demo_pg_check_constraint(&pool).await?;
    demo_pg_cascade_delete(&pool).await?;
    demo_pg_partial_index(&pool).await?;
    demo_pg_skip_locked(&pool).await?;
    demo_pg_listen_notify(&pool).await?;
    demo_pg_advisory_lock(&pool).await?;
    demo_pg_generated_column(&pool).await?;

    // Rustの機能デモ
    println!("========================================");
    println!("  Rustの機能で防げるもの");
    println!("========================================\n");

    demo_rust_newtype().await?;
    demo_rust_typestate().await?;
    demo_rust_sqlx_type(&pool).await?;
    demo_rust_raii(&pool).await?;

    cleanup_tables(&pool).await?;

    Ok(())
}

// =============================================================================
// セットアップ
// =============================================================================

async fn setup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(pool)
        .await?;

    // 既存テーブルをクリーンアップ（テスト用）
    sqlx::query("DROP TABLE IF EXISTS attachments_hybrid CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS files CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS tenants CASCADE")
        .execute(pool)
        .await?;

    // PostgreSQL ENUM型
    sqlx::query("DROP TYPE IF EXISTS file_status CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("CREATE TYPE file_status AS ENUM ('UPLOADING', 'UPLOADED')")
        .execute(pool)
        .await?;

    // テナントテーブル
    sqlx::query(
        r#"
        CREATE TABLE tenants (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ファイルテーブル（全PostgreSQL機能を活用）
    sqlx::query(
        r#"
        CREATE TABLE files (
            file_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            filename VARCHAR(255) NOT NULL,
            content_type VARCHAR(100) NOT NULL,
            status file_status NOT NULL DEFAULT 'UPLOADING',
            file_size BIGINT,
            -- 生成列: storage_keyを自動計算
            storage_key TEXT GENERATED ALWAYS AS (
                'tenants/' || tenant_id::text || '/files/' || file_id::text
            ) STORED,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 部分インデックス（孤立ファイル検索用）
    sqlx::query(
        "CREATE INDEX idx_files_orphaned
         ON files(status, created_at)
         WHERE status = 'UPLOADING'",
    )
    .execute(pool)
    .await?;

    // ハイブリッドテーブル（CHECK制約デモ用）
    sqlx::query(
        r#"
        CREATE TABLE attachments_hybrid (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            file_name VARCHAR(255) NOT NULL,
            content_type VARCHAR(100) NOT NULL,
            file_size BIGINT NOT NULL,
            storage_type VARCHAR(20) NOT NULL,
            inline_data BYTEA,
            storage_key VARCHAR(500),
            checksum VARCHAR(64) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            -- CHECK制約: storage_typeに応じて適切なカラムが設定されていることを保証
            CONSTRAINT valid_storage_type CHECK (storage_type IN ('inline', 'external')),
            CONSTRAINT storage_data_check CHECK (
                (storage_type = 'inline' AND inline_data IS NOT NULL) OR
                (storage_type = 'external' AND storage_key IS NOT NULL)
            )
        )
        "#,
    )
    .execute(pool)
    .await?;

    // LISTEN/NOTIFY用トリガー関数
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION notify_file_cleanup()
        RETURNS TRIGGER AS $$
        BEGIN
            -- ファイルが削除されたときにクリーンアップチャンネルに通知
            PERFORM pg_notify('file_cleanup', json_build_object(
                'action', TG_OP,
                'file_id', OLD.file_id::text,
                'tenant_id', OLD.tenant_id::text,
                'storage_key', OLD.storage_key
            )::text);
            RETURN OLD;
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(pool)
    .await?;

    // 削除トリガー
    sqlx::query("DROP TRIGGER IF EXISTS file_cleanup_trigger ON files")
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        CREATE TRIGGER file_cleanup_trigger
        AFTER DELETE ON files
        FOR EACH ROW
        EXECUTE FUNCTION notify_file_cleanup()
        "#,
    )
    .execute(pool)
    .await?;

    // テストテナントを作成
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'Test Tenant') ON CONFLICT DO NOTHING")
        .bind(Uuid::nil())
        .execute(pool)
        .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TRIGGER IF EXISTS file_cleanup_trigger ON files")
        .execute(pool)
        .await?;
    sqlx::query("DROP FUNCTION IF EXISTS notify_file_cleanup()")
        .execute(pool)
        .await?;

    for table in ["attachments_hybrid", "files", "tenants"] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table))
            .execute(pool)
            .await?;
    }
    sqlx::query("DROP TYPE IF EXISTS file_status CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// PostgreSQL機能デモ
// =============================================================================

/// PostgreSQL ENUM型のデモ
async fn demo_pg_enum_type(pool: &PgPool) -> Result<()> {
    println!("--- PG機能1: ENUM型 ---\n");

    println!("  効果: 不正なステータス値をDB側で防止");
    println!("  定義: CREATE TYPE file_status AS ENUM ('UPLOADING', 'UPLOADED')\n");

    // 正常な値
    let file_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
         VALUES ($1, $2, 'test.pdf', 'application/pdf', 'UPLOADING')",
    )
    .bind(file_id)
    .bind(Uuid::nil())
    .execute(pool)
    .await?;
    println!("  ✓ 'UPLOADING' は有効な値");

    // 不正な値を試す
    let invalid_result = sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
         VALUES ($1, $2, 'test2.pdf', 'application/pdf', 'INVALID_STATUS')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::nil())
    .execute(pool)
    .await;

    match invalid_result {
        Err(e) => println!(
            "  ✓ 'INVALID_STATUS' は拒否: {}\n",
            e.to_string().lines().next().unwrap_or("")
        ),
        Ok(_) => println!("  ✗ 不正な値が受け入れられた（問題あり）\n"),
    }

    Ok(())
}

/// PostgreSQL CHECK制約のデモ
async fn demo_pg_check_constraint(pool: &PgPool) -> Result<()> {
    println!("--- PG機能2: CHECK制約 ---\n");

    println!("  効果: storage_type と inline_data/storage_key の整合性を保証");
    println!("  定義: CHECK ((storage_type = 'inline' AND inline_data IS NOT NULL) OR ...)\n");

    // 正常: inline + inline_data
    let result1 = sqlx::query(
        "INSERT INTO attachments_hybrid (id, tenant_id, file_name, content_type, file_size, storage_type, inline_data, checksum)
         VALUES ($1, $2, 'small.txt', 'text/plain', 100, 'inline', $3, 'checksum1')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::nil())
    .bind(b"small content".as_slice())
    .execute(pool)
    .await;
    println!(
        "  ✓ inline + inline_data: {}",
        if result1.is_ok() { "OK" } else { "NG" }
    );

    // 正常: external + storage_key
    let result2 = sqlx::query(
        "INSERT INTO attachments_hybrid (id, tenant_id, file_name, content_type, file_size, storage_type, storage_key, checksum)
         VALUES ($1, $2, 'large.pdf', 'application/pdf', 1000000, 'external', 'tenants/xxx/files/yyy', 'checksum2')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::nil())
    .execute(pool)
    .await;
    println!(
        "  ✓ external + storage_key: {}",
        if result2.is_ok() { "OK" } else { "NG" }
    );

    // 不正: inline なのに inline_data がない
    let result3 = sqlx::query(
        "INSERT INTO attachments_hybrid (id, tenant_id, file_name, content_type, file_size, storage_type, checksum)
         VALUES ($1, $2, 'invalid.txt', 'text/plain', 100, 'inline', 'checksum3')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::nil())
    .execute(pool)
    .await;
    println!(
        "  ✓ inline で inline_data なし: {} (CHECK制約違反)\n",
        if result3.is_err() { "拒否" } else { "受入" }
    );

    Ok(())
}

/// PostgreSQL CASCADE削除のデモ
async fn demo_pg_cascade_delete(pool: &PgPool) -> Result<()> {
    println!("--- PG機能3: 外部キー + CASCADE ---\n");

    println!("  効果: 親レコード削除時に子レコードを自動削除");
    println!("  定義: REFERENCES tenants(id) ON DELETE CASCADE\n");

    // テスト用テナントとファイルを作成
    let test_tenant_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'Cascade Test Tenant')")
        .bind(test_tenant_id)
        .execute(pool)
        .await?;

    for i in 1..=3 {
        sqlx::query(
            "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
             VALUES ($1, $2, $3, 'application/pdf', 'UPLOADED')",
        )
        .bind(Uuid::new_v4())
        .bind(test_tenant_id)
        .bind(format!("file{}.pdf", i))
        .execute(pool)
        .await?;
    }

    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files WHERE tenant_id = $1")
        .bind(test_tenant_id)
        .fetch_one(pool)
        .await?;
    println!("  削除前のファイル数: {}", before.0);

    // テナントを削除
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(test_tenant_id)
        .execute(pool)
        .await?;

    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files WHERE tenant_id = $1")
        .bind(test_tenant_id)
        .fetch_one(pool)
        .await?;
    println!("  削除後のファイル数: {} (CASCADEで自動削除)\n", after.0);

    Ok(())
}

/// PostgreSQL 部分インデックスのデモ
async fn demo_pg_partial_index(pool: &PgPool) -> Result<()> {
    println!("--- PG機能4: 部分インデックス ---\n");

    println!("  効果: 孤立ファイル検索を高速化（UPLOADINGのみインデックス）");
    println!("  定義: CREATE INDEX ... WHERE status = 'UPLOADING'\n");

    // インデックス情報を取得
    let index_info: Vec<(String, String)> = sqlx::query_as(
        "SELECT indexname, indexdef FROM pg_indexes WHERE tablename = 'files' AND indexname LIKE '%orphaned%'",
    )
    .fetch_all(pool)
    .await?;

    for (name, def) in &index_info {
        println!("  インデックス: {}", name);
        println!("  定義: {}\n", def);
    }

    // EXPLAINで確認
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT * FROM files WHERE status = 'UPLOADING' AND created_at < NOW() - INTERVAL '1 hour'",
    )
    .fetch_all(pool)
    .await?;

    println!("  クエリプラン:");
    for (plan,) in explain.iter().take(3) {
        println!("    {}", plan);
    }
    println!();

    Ok(())
}

/// PostgreSQL FOR UPDATE SKIP LOCKEDのデモ
async fn demo_pg_skip_locked(pool: &PgPool) -> Result<()> {
    println!("--- PG機能5: FOR UPDATE SKIP LOCKED ---\n");

    println!("  効果: 並行クリーンアップジョブの競合を防止");
    println!("  動作: ロック済みの行をスキップして処理を継続\n");

    // テスト用の古いファイルを作成
    for i in 1..=5 {
        sqlx::query(
            "INSERT INTO files (file_id, tenant_id, filename, content_type, status, created_at)
             VALUES ($1, $2, $3, 'application/pdf', 'UPLOADING', NOW() - INTERVAL '2 hours')",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::nil())
        .bind(format!("orphaned{}.pdf", i))
        .execute(pool)
        .await?;
    }

    // トランザクション1: 一部をロック
    let mut tx1 = pool.begin().await?;
    let locked: Vec<(Uuid,)> =
        sqlx::query_as("SELECT file_id FROM files WHERE status = 'UPLOADING' LIMIT 2 FOR UPDATE")
            .fetch_all(&mut *tx1)
            .await?;
    println!("  TX1: {} 件をロック中", locked.len());

    // トランザクション2: SKIP LOCKEDで残りを取得
    let skipped: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT file_id FROM files WHERE status = 'UPLOADING' FOR UPDATE SKIP LOCKED",
    )
    .fetch_all(pool)
    .await?;
    println!(
        "  TX2 (SKIP LOCKED): {} 件を取得（ロック済みをスキップ）",
        skipped.len()
    );

    tx1.rollback().await?;

    // クリーンアップ
    sqlx::query(
        "DELETE FROM files WHERE status = 'UPLOADING' AND created_at < NOW() - INTERVAL '1 hour'",
    )
    .execute(pool)
    .await?;

    println!();

    Ok(())
}

/// PostgreSQL LISTEN/NOTIFYのデモ
async fn demo_pg_listen_notify(pool: &PgPool) -> Result<()> {
    println!("--- PG機能6: LISTEN/NOTIFY + トリガー ---\n");

    println!("  効果: ファイル削除時にS3クリーンアップを非同期通知");
    println!("  動作: DELETEトリガー → pg_notify → リスナーがS3削除\n");

    println!("  トリガー定義:");
    println!("    CREATE TRIGGER file_cleanup_trigger");
    println!("    AFTER DELETE ON files");
    println!("    FOR EACH ROW EXECUTE FUNCTION notify_file_cleanup()\n");

    println!("  通知関数:");
    println!("    PERFORM pg_notify('file_cleanup', json_build_object(");
    println!("        'action', TG_OP,");
    println!("        'file_id', OLD.file_id::text,");
    println!("        'storage_key', OLD.storage_key");
    println!("    )::text);\n");

    // テスト用ファイルを作成して削除
    let test_file_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
         VALUES ($1, $2, 'notify_test.pdf', 'application/pdf', 'UPLOADED')",
    )
    .bind(test_file_id)
    .bind(Uuid::nil())
    .execute(pool)
    .await?;

    // 実際のリスナーは別プロセスで実行
    println!("  使用例（別プロセス）:");
    println!("    LISTEN file_cleanup;");
    println!("    -- 通知を受け取ったらS3からファイルを削除");
    println!("    -- {{\"action\":\"DELETE\",\"file_id\":\"...\",\"storage_key\":\"...\"}}\n");

    // 削除（トリガーが発火）
    sqlx::query("DELETE FROM files WHERE file_id = $1")
        .bind(test_file_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// PostgreSQL Advisory Lockのデモ
async fn demo_pg_advisory_lock(pool: &PgPool) -> Result<()> {
    println!("--- PG機能7: Advisory Lock ---\n");

    println!("  効果: 分散環境でクリーンアップジョブの重複実行を防止");
    println!("  動作: 同じロックキーを持つジョブは同時に1つだけ実行\n");

    // ロックキー（クリーンアップジョブ用）
    let lock_key: i64 = 12345;

    // 専用のコネクションを取得してロックを保持
    let mut conn1 = pool.acquire().await?;

    // コネクション1でロック取得
    let (acquired1,): (bool,) = sqlx::query_as("SELECT pg_try_advisory_lock($1)")
        .bind(lock_key)
        .fetch_one(&mut *conn1)
        .await?;

    println!(
        "  コネクション1: pg_try_advisory_lock({}) = {}",
        lock_key,
        if acquired1 {
            "true (取得成功)"
        } else {
            "false"
        }
    );

    if acquired1 {
        // 同じコネクションからの再取得（セッションレベルロックは再入可能）
        let (acquired1_again,): (bool,) = sqlx::query_as("SELECT pg_try_advisory_lock($1)")
            .bind(lock_key)
            .fetch_one(&mut *conn1)
            .await?;
        println!(
            "  コネクション1: 再取得 = {} (同一セッションは再入可能)",
            if acquired1_again { "true" } else { "false" }
        );

        // 別のコネクションからロック取得を試みる
        let mut conn2 = pool.acquire().await?;
        let (acquired2,): (bool,) = sqlx::query_as("SELECT pg_try_advisory_lock($1)")
            .bind(lock_key)
            .fetch_one(&mut *conn2)
            .await?;
        println!(
            "  コネクション2: pg_try_advisory_lock({}) = {} (別セッションはブロック)",
            lock_key,
            if acquired2 { "true" } else { "false" }
        );

        // ロック解放（2回取得したので2回解放）
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(lock_key)
            .execute(&mut *conn1)
            .await?;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(lock_key)
            .execute(&mut *conn1)
            .await?;
        println!("  コネクション1: ロック解放完了");
    }

    println!("\n  重要な特性:");
    println!("    - セッションレベルロック: 同一セッション内では再入可能");
    println!("    - 異なるセッション: ロック保持中は取得失敗（false）");
    println!("    - pg_try_advisory_lock: ノンブロッキング（即座にtrue/false）");
    println!("    - pg_advisory_lock: ブロッキング（取得まで待機）\n");

    println!("  分散環境での使用パターン:");
    println!("    SELECT pg_try_advisory_lock(CLEANUP_JOB_ID);");
    println!("    -- true: このインスタンスがジョブを実行");
    println!("    -- false: 別インスタンスが実行中、スキップ\n");

    Ok(())
}

/// PostgreSQL 生成列のデモ
async fn demo_pg_generated_column(pool: &PgPool) -> Result<()> {
    println!("--- PG機能8: 生成列（GENERATED ALWAYS AS） ---\n");

    println!("  効果: storage_key を自動計算、計算ミスを防止");
    println!(
        "  定義: storage_key TEXT GENERATED ALWAYS AS ('tenants/' || tenant_id::text || '/files/' || file_id::text) STORED\n"
    );

    // ファイルを作成（storage_keyは自動計算される）
    let file_id = Uuid::new_v4();
    let tenant_id = Uuid::nil();

    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
         VALUES ($1, $2, 'generated_test.pdf', 'application/pdf', 'UPLOADED')",
    )
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    // storage_keyを確認
    let (storage_key,): (String,) =
        sqlx::query_as("SELECT storage_key FROM files WHERE file_id = $1")
            .bind(file_id)
            .fetch_one(pool)
            .await?;

    println!("  file_id: {}", file_id);
    println!("  tenant_id: {}", tenant_id);
    println!("  storage_key (自動生成): {}", storage_key);

    // 期待値と比較
    let expected = format!("tenants/{}/files/{}", tenant_id, file_id);
    println!("  期待値: {}", expected);
    println!(
        "  一致: {}\n",
        if storage_key == expected {
            "✓"
        } else {
            "✗"
        }
    );

    // 直接更新を試みる（失敗するはず）
    let update_result = sqlx::query("UPDATE files SET storage_key = 'invalid' WHERE file_id = $1")
        .bind(file_id)
        .execute(pool)
        .await;

    match update_result {
        Err(e) => println!(
            "  直接更新の試み: 拒否 ({})\n",
            e.to_string().lines().next().unwrap_or("")
        ),
        Ok(_) => println!("  直接更新の試み: 成功（問題あり）\n"),
    }

    Ok(())
}

// =============================================================================
// Rust機能デモ
// =============================================================================

/// Rust newtype パターンのデモ
async fn demo_rust_newtype() -> Result<()> {
    println!("--- Rust機能1: newtype パターン ---\n");

    println!("  効果: FileId と TenantId の取り違えをコンパイル時に防止\n");

    let tenant_id = TenantId::new();
    let file_id = FileId::new();

    println!("  TenantId: {}", tenant_id);
    println!("  FileId: {}", file_id);

    println!("\n  コード例:");
    println!("    fn get_file(tenant_id: TenantId, file_id: FileId) {{ ... }}");
    println!("    ");
    println!("    // コンパイルエラー: 引数の型が違う");
    println!("    // get_file(file_id, tenant_id);  // ← 取り違えを防止");
    println!("    ");
    println!("    // 正しい呼び出し");
    println!("    get_file(tenant_id, file_id);  // ← OK\n");

    // 生のUuidとの比較
    println!("  生のUuidを使う場合（危険）:");
    println!("    fn get_file(tenant_id: Uuid, file_id: Uuid) {{ ... }}");
    println!("    get_file(file_id, tenant_id);  // ← コンパイル成功、実行時エラー\n");

    Ok(())
}

/// Rust 型状態パターンのデモ
async fn demo_rust_typestate() -> Result<()> {
    println!("--- Rust機能2: 型状態パターン（Typestate） ---\n");

    println!("  効果: 不正な状態遷移をコンパイル時に防止\n");

    // アップロード中ファイルを作成
    let uploading_file = TypedFile::<Uploading>::new(
        TenantId::nil(),
        "document.pdf".to_string(),
        "application/pdf".to_string(),
    );

    println!("  1. Uploading状態で作成:");
    println!("     file_id: {}", uploading_file.file_id);
    println!("     状態: Uploading");

    // Uploading状態では get_download_url() は呼べない
    println!("\n  2. Uploading状態では get_download_url() が存在しない:");
    println!("     // uploading_file.get_download_url();  // ← コンパイルエラー！");

    // 状態遷移（所有権が移動する）
    let uploaded_file = uploading_file.complete(1024);

    println!("\n  3. complete() で Uploaded 状態に遷移:");
    println!("     file_id: {}", uploaded_file.file_id);
    println!("     file_size: {} bytes", uploaded_file.file_size());
    println!("     状態: Uploaded");

    // Uploaded状態では get_download_url() が呼べる
    println!("\n  4. Uploaded状態では get_download_url() が利用可能:");
    println!("     download_url: {}", uploaded_file.get_download_url());

    // 古い変数は使えない
    println!("\n  5. 古いuploading_fileは所有権が移動して使用不可:");
    println!("     // uploading_file.complete(...)  // ← コンパイルエラー！");
    println!("     // (value moved)\n");

    Ok(())
}

/// Rust sqlx::Type deriveのデモ
async fn demo_rust_sqlx_type(pool: &PgPool) -> Result<()> {
    println!("--- Rust機能3: sqlx::Type derive ---\n");

    println!("  効果: PostgreSQL ENUMとRust enumの型安全な連携\n");

    #[derive(Debug, sqlx::FromRow)]
    struct FileRow {
        file_id: Uuid,
        status: FileStatus, // PostgreSQL ENUMがRust enumにマッピングされる
    }

    // UPLOADINGファイルを作成
    let file_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
         VALUES ($1, $2, 'sqlx_type_test.pdf', 'application/pdf', 'UPLOADING')",
    )
    .bind(file_id)
    .bind(Uuid::nil())
    .execute(pool)
    .await?;

    // 取得時にRust enumに自動変換される
    let file: FileRow = sqlx::query_as("SELECT file_id, status FROM files WHERE file_id = $1")
        .bind(file_id)
        .fetch_one(pool)
        .await?;

    println!("  取得結果:");
    println!("    file_id: {}", file.file_id);
    println!("    status (Rust enum): {:?}", file.status);

    // パターンマッチで安全に分岐
    match file.status {
        FileStatus::Uploading => println!("    → アップロード中"),
        FileStatus::Uploaded => println!("    → アップロード完了"),
    }

    println!("\n  derive定義:");
    println!("    #[derive(sqlx::Type)]");
    println!("    #[sqlx(type_name = \"file_status\", rename_all = \"SCREAMING_SNAKE_CASE\")]");
    println!("    pub enum FileStatus {{ Uploading, Uploaded }}\n");

    Ok(())
}

/// Rust RAII / Dropのデモ
async fn demo_rust_raii(pool: &PgPool) -> Result<()> {
    println!("--- Rust機能4: RAII / Drop トレイト ---\n");

    println!("  効果: トランザクションの自動ロールバック\n");

    let file_id = Uuid::new_v4();

    // スコープ内でトランザクションを開始
    {
        let mut tx = pool.begin().await?;

        sqlx::query(
            "INSERT INTO files (file_id, tenant_id, filename, content_type, status)
             VALUES ($1, $2, 'raii_test.pdf', 'application/pdf', 'UPLOADING')",
        )
        .bind(file_id)
        .bind(Uuid::nil())
        .execute(&mut *tx)
        .await?;

        println!("  1. トランザクション内でINSERT実行");

        // commit()を呼ばずにスコープを抜ける
        // → txがDropされるときに自動でROLLBACK
        println!("  2. commit()を呼ばずにスコープ終了");
        println!("     → Drop時に自動ROLLBACK");
    }

    // ファイルが存在しないことを確認
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM files WHERE file_id = $1)")
        .bind(file_id)
        .fetch_one(pool)
        .await?;

    println!(
        "  3. ファイル存在確認: {}",
        if exists.0 {
            "存在する（問題）"
        } else {
            "存在しない（ROLLBACK成功）"
        }
    );

    println!("\n  重要:");
    println!("    sqlx::Transaction は Drop 時に未コミットなら ROLLBACK");
    println!("    S3操作は ROLLBACK できないため、2フェーズアップロードが必要\n");

    Ok(())
}
