# S3に残った1万ファイルの謎：孤立ファイル事件簿

## 発端

「S3のストレージ料金が先月の3倍になっています」

Slackに流れてきた経理からのメッセージだった。ファイルアップロード機能をリリースしてから1ヶ月。ユーザー数は増えていたが、3倍は異常だ。

調べてみると、DBには存在しないファイルがS3に1万件以上残っていた。孤立ファイル（orphaned files）だ。DBのレコードは消えているのに、S3のオブジェクトだけが残っている。毎日増え続けていた。

## 失敗1：アップロード途中で失敗するケース

最初に疑ったのはアップロード処理の途中失敗だ。

```rust
// 最初の実装（問題あり）
async fn upload_file(pool: &PgPool, s3: &S3Client, file: &[u8]) -> Result<FileId> {
    let file_id = FileId::new();

    // 1. S3にアップロード
    s3.put_object(file_id.to_string(), file).await?;

    // 2. DBにレコード作成
    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, status)
         VALUES ($1, $2, $3, 'UPLOADED')"
    )
    .bind(file_id)
    .bind(tenant_id)
    .bind(filename)
    .execute(pool).await?;  // ここで失敗するとS3だけ残る

    Ok(file_id)
}
```

S3へのアップロードは成功、その後のDB操作で失敗。よくあるパターンだ。ネットワーク障害、制約違反、デッドロック、何でも起きうる。

S3にはロールバック機能がない。DBトランザクションをロールバックしても、S3のファイルは消えない。

## 失敗2：削除時のS3消し忘れ

次に見つかったのは削除処理の問題だった。

```rust
// 削除処理（問題あり）
async fn delete_file(pool: &PgPool, file_id: FileId) -> Result<()> {
    // DBレコードを削除
    sqlx::query("DELETE FROM files WHERE file_id = $1")
        .bind(file_id)
        .execute(pool).await?;

    // S3のファイル削除を忘れていた！
    // s3.delete_object(...).await?;

    Ok(())
}
```

DBのレコードを消したら満足してしまい、S3の削除を忘れていた。テストでは気づかなかった。DBの行が消えればテストはパスするからだ。

## 失敗3：CASCADE削除で発火しないトリガー

外部キー制約のCASCADE削除を導入した。テナントを削除すると、そのテナントのファイルも自動で消える。便利だと思った。

```sql
CREATE TABLE files (
    file_id UUID PRIMARY KEY,
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    ...
);
```

```rust
// テナント削除
sqlx::query("DELETE FROM tenants WHERE id = $1")
    .bind(tenant_id)
    .execute(pool).await?;
// filesテーブルの行は自動削除される
// しかしS3のファイルは残ったまま
```

DBのCASCADE削除はS3を知らない。アプリケーションコードを経由しないので、S3削除のロジックが走らない。

## 原因の整理

3つの失敗に共通する原因がある。

1. **DBとS3は別システム**: トランザクションで一括管理できない
2. **S3は冪等じゃない**: 同じ操作を何度やっても同じ結果になるとは限らない
3. **テストで検知できない**: DBだけ見ていると気づかない

分散システムにおける一貫性の問題だ。2フェーズコミットのような仕組みがなければ、DBとS3の状態はずれうる。

## 解決策1：2フェーズアップロード

アップロードを「仮登録」と「本登録」の2段階に分ける。

```sql
CREATE TYPE file_status AS ENUM ('UPLOADING', 'UPLOADED');

CREATE TABLE files (
    file_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    status file_status NOT NULL DEFAULT 'UPLOADING',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 孤立ファイル検索用の部分インデックス
CREATE INDEX idx_files_orphaned ON files(status, created_at)
WHERE status = 'UPLOADING';
```

```rust
// 2フェーズアップロード
async fn upload_file_safe(
    pool: &PgPool,
    s3: &S3Client,
    tenant_id: TenantId,
    filename: &str,
    file: &[u8],
) -> Result<FileId> {
    // Phase 1: DBに仮登録（UPLOADING状態）
    let file_id = FileId::new();
    sqlx::query(
        "INSERT INTO files (file_id, tenant_id, filename, status)
         VALUES ($1, $2, $3, 'UPLOADING')"
    )
    .bind(file_id)
    .bind(tenant_id)
    .bind(filename)
    .execute(pool).await?;

    // Phase 2: S3にアップロード
    match s3.put_object(file_id.to_string(), file).await {
        Ok(_) => {
            // Phase 3: 本登録（UPLOADED状態）
            sqlx::query(
                "UPDATE files SET status = 'UPLOADED' WHERE file_id = $1"
            )
            .bind(file_id)
            .execute(pool).await?;
            Ok(file_id)
        }
        Err(e) => {
            // 失敗時はDBレコードを削除
            sqlx::query("DELETE FROM files WHERE file_id = $1")
                .bind(file_id)
                .execute(pool).await?;
            Err(e.into())
        }
    }
}
```

`UPLOADING`状態のまま残ったファイルは、バックグラウンドジョブで定期的に削除する。

## 解決策2：LISTEN/NOTIFYでS3削除を通知

CASCADE削除でもS3を消すために、トリガーでLISTEN/NOTIFYを使う。

```sql
CREATE FUNCTION notify_file_cleanup() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('file_cleanup', json_build_object(
        'action', TG_OP,
        'file_id', OLD.file_id::text,
        'storage_key', OLD.storage_key
    )::text);
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER file_cleanup_trigger
AFTER DELETE ON files
FOR EACH ROW EXECUTE FUNCTION notify_file_cleanup();
```

別プロセスで通知をリッスンし、S3を削除する。

```rust
// 別プロセスで実行
async fn listen_for_cleanup(pool: &PgPool, s3: &S3Client) -> Result<()> {
    let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await?;
    listener.listen("file_cleanup").await?;

    loop {
        let notification = listener.recv().await?;
        let payload: serde_json::Value = serde_json::from_str(notification.payload())?;

        if let Some(storage_key) = payload.get("storage_key").and_then(|v| v.as_str()) {
            // S3からファイルを削除
            s3.delete_object(storage_key).await?;
        }
    }
}
```

## 解決策3：定期クリーンアップジョブ

念のため、定期的に孤立ファイルを探して削除するジョブも用意する。

```rust
async fn cleanup_orphaned_files(pool: &PgPool, s3: &S3Client) -> Result<u64> {
    // Advisory Lockで重複実行を防止
    let lock_key: i64 = 12345;
    let (acquired,): (bool,) = sqlx::query_as(
        "SELECT pg_try_advisory_lock($1)"
    )
    .bind(lock_key)
    .fetch_one(pool).await?;

    if !acquired {
        // 他のインスタンスが実行中
        return Ok(0);
    }

    // 1時間以上UPLOADINGのままのファイルを取得
    // FOR UPDATE SKIP LOCKEDで競合を回避
    let orphaned: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT file_id, storage_key
        FROM files
        WHERE status = 'UPLOADING'
          AND created_at < NOW() - INTERVAL '1 hour'
        FOR UPDATE SKIP LOCKED
        LIMIT 100
        "#
    )
    .fetch_all(pool).await?;

    let mut deleted = 0;
    for (file_id, storage_key) in orphaned {
        // S3から削除
        if let Err(e) = s3.delete_object(&storage_key).await {
            eprintln!("Failed to delete S3 object {}: {}", storage_key, e);
            continue;
        }

        // DBから削除
        sqlx::query("DELETE FROM files WHERE file_id = $1")
            .bind(file_id)
            .execute(pool).await?;

        deleted += 1;
    }

    // ロック解放
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_key)
        .execute(pool).await?;

    Ok(deleted)
}
```

`FOR UPDATE SKIP LOCKED`で、既にロックされている行はスキップする。複数のワーカーが同時に動いても競合しない。

## 解決策4：Rustの型でミスを防ぐ

ここまでPostgreSQLの機能を活用してきたが、Rust側でも防御を固める。

### newtype パターン

IDの取り違えをコンパイル時に防ぐ。

```rust
#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct TenantId(Uuid);

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct FileId(Uuid);

// これはコンパイルエラー
// fn get_file(file_id: FileId, tenant_id: TenantId) -> ...
// get_file(tenant_id, file_id);  // 引数の順序が違う
```

### 型状態パターン

状態遷移をコンパイル時に検証する。

```rust
pub struct Uploading;
pub struct Uploaded;

pub struct File<State> {
    file_id: FileId,
    _state: PhantomData<State>,
}

impl File<Uploading> {
    pub fn complete(self, size: i64) -> File<Uploaded> {
        // 所有権を消費して新しい状態を返す
        File { file_id: self.file_id, _state: PhantomData }
    }

    // Uploading状態では get_download_url() は存在しない
}

impl File<Uploaded> {
    pub fn get_download_url(&self) -> String {
        // Uploaded状態でのみ呼び出し可能
        format!("https://storage.example.com/{}", self.file_id)
    }
}
```

`File<Uploading>`に対して`get_download_url()`を呼ぼうとすると、コンパイルエラーになる。

## 今はこうしている

1. **2フェーズアップロード**: UPLOADING → UPLOADED の明示的な状態遷移
2. **LISTEN/NOTIFY**: CASCADE削除でもS3削除を実行
3. **定期クリーンアップ**: 1時間ごとに孤立ファイルを削除
4. **部分インデックス**: `WHERE status = 'UPLOADING'`で孤立ファイル検索を高速化
5. **Advisory Lock**: クリーンアップジョブの重複実行を防止
6. **型状態パターン**: 不正な状態遷移をコンパイル時に検出

冒頭の「S3に残った1万ファイル」は、クリーンアップジョブを3回実行して解消した。ストレージ料金は通常に戻った。

分散システムでは「DBが成功 ≠ 全体が成功」だ。外部ストレージとDBの一貫性は、アプリケーション側で保証する必要がある。その仕組みを最初から設計に組み込んでおけば、深夜の緊急対応は避けられた。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_07_file_upload
cargo run
```

## 参考資料

- [PostgreSQL - LISTEN/NOTIFY](https://www.postgresql.org/docs/current/sql-listen.html)
- [PostgreSQL - Advisory Locks](https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS)
- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
