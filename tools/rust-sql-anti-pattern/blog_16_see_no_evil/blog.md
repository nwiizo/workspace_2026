# 深夜3時のpanic!：.unwrap()が招いた障害と5つの教訓

## 発端

深夜3時、PagerDutyのアラートで起こされた。

「サービスが断続的に停止しています」

ログを見ると、`panic: called 'Option::unwrap()' on a None value`。ユーザーが存在しない場合に`.unwrap()`が呼ばれてパニックしていた。

```rust
// 問題のコード
let user = get_user(pool, user_id).await.unwrap();  // None でパニック
```

存在しないユーザーIDでリクエストが来ると、プロセスがパニックしてクラッシュ。コンテナが再起動し、その間の他のリクエストも巻き添えになっていた。

## 教訓1：.unwrap()を本番コードで使わない

`.unwrap()`は「この値は絶対にSomeまたはOkである」という宣言だ。その仮定が崩れるとプロセス全体がクラッシュする。

```rust
// ❌ 本番でやってはいけない
let user = get_user(pool, user_id).await.unwrap();
let count: i64 = sqlx::query_scalar("SELECT COUNT(*)...").fetch_one(&pool).await.unwrap();

// ✅ エラーを伝播させる
let user = get_user(pool, user_id).await?;
let count: i64 = sqlx::query_scalar("SELECT COUNT(*)...").fetch_one(&pool).await?;
```

`?`演算子を使えば、エラーは呼び出し元に伝播する。パニックではなくエラーレスポンスとして処理できる。

## 教訓2：カスタムエラー型を定義する

sqlxのエラーをそのまま返すと、クライアントに不親切だ。

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found: {0}")]
    NotFound(Uuid),

    #[error("Email already exists: {0}")]
    EmailExists(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

```rust
async fn get_user(pool: &PgPool, id: Uuid) -> Result<User, UserError> {
    sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(UserError::NotFound(id))
}

async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<User, UserError> {
    let result = sqlx::query_as(
        "INSERT INTO users (email, name) VALUES ($1, $2) RETURNING *"
    )
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await;

    match result {
        Ok(user) => Ok(user),
        Err(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some("23505") =>
        {
            Err(UserError::EmailExists(email.to_string()))
        }
        Err(e) => Err(UserError::Database(e)),
    }
}
```

HTTPハンドラでは、エラー型に応じたステータスコードを返せる。

```rust
match get_user(&pool, user_id).await {
    Ok(user) => Ok(Json(user)),
    Err(UserError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
    Err(UserError::EmailExists(_)) => Err(StatusCode::CONFLICT),
    Err(UserError::Database(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
}
```

## 教訓3：PostgreSQLエラーコードを分類する

PostgreSQLは詳細なエラーコードを返す。これを活用して適切なエラーハンドリングをする。

```rust
mod pg_error_codes {
    pub const UNIQUE_VIOLATION: &str = "23505";
    pub const FOREIGN_KEY_VIOLATION: &str = "23503";
    pub const NOT_NULL_VIOLATION: &str = "23502";
    pub const CHECK_VIOLATION: &str = "23514";
    pub const SERIALIZATION_FAILURE: &str = "40001";
    pub const DEADLOCK_DETECTED: &str = "40P01";
}

fn classify_db_error(err: &sqlx::Error) -> DbError {
    if let sqlx::Error::Database(db_err) = err {
        if let Some(code) = db_err.code() {
            return match code.as_ref() {
                pg_error_codes::UNIQUE_VIOLATION =>
                    DbError::Duplicate(db_err.message().to_string()),
                pg_error_codes::FOREIGN_KEY_VIOLATION =>
                    DbError::ForeignKeyViolation,
                pg_error_codes::SERIALIZATION_FAILURE =>
                    DbError::SerializationFailure,
                pg_error_codes::DEADLOCK_DETECTED =>
                    DbError::Deadlock,
                _ => DbError::Other(db_err.message().to_string()),
            };
        }
    }
    DbError::Other(err.to_string())
}
```

## 教訓4：リトライ可能なエラーを区別する

デッドロックやシリアライゼーション失敗は、リトライで解決することが多い。

```rust
async fn execute_with_retry<T, F, Fut>(
    max_retries: u32,
    mut operation: F,
) -> Result<T, DbError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let mut attempts = 0;

    loop {
        attempts += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let db_err = classify_db_error(&e);

                // リトライ可能なエラーかチェック
                let should_retry = matches!(
                    db_err,
                    DbError::SerializationFailure | DbError::Deadlock
                );

                if should_retry && attempts < max_retries {
                    // エクスポネンシャルバックオフ
                    let delay = 10 * 2_u64.pow(attempts - 1);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    continue;
                }

                return Err(db_err);
            }
        }
    }
}
```

```rust
// 使用例
let result = execute_with_retry(3, || async {
    sqlx::query("UPDATE accounts SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(account_id)
        .execute(&pool)
        .await
}).await?;
```

## 教訓5：エラーを無視しない

`let _ =`でエラーを捨てるのは危険だ。

```rust
// ❌ エラーを無視
let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
    .bind(user_id)
    .execute(&pool)
    .await;

// ✅ エラーをログに記録、または伝播
if let Err(e) = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
    .bind(user_id)
    .execute(&pool)
    .await
{
    tracing::warn!("Failed to delete sessions: {}", e);
    // 致命的でなければ続行
}

// または単純に伝播
sqlx::query("DELETE FROM sessions WHERE user_id = $1")
    .bind(user_id)
    .execute(&pool)
    .await?;
```

## アンチパターン一覧

| パターン | 問題点 | 解決策 |
|---------|--------|--------|
| `.unwrap()` | パニックでクラッシュ | `?`で伝播 |
| `let _ =` | エラー無視 | ログ記録または伝播 |
| 汎用エラー | 原因特定が困難 | カスタムエラー型 |
| 無条件リトライ | 無限ループのリスク | エラー分類 + 回数制限 |

## 修正後のコード

冒頭の問題は、以下のように修正した。

```rust
// Before
let user = get_user(pool, user_id).await.unwrap();

// After
let user = match get_user(pool, user_id).await {
    Ok(user) => user,
    Err(UserError::NotFound(id)) => {
        return Err(ApiError::NotFound(format!("User {} not found", id)));
    }
    Err(e) => {
        tracing::error!("Database error: {}", e);
        return Err(ApiError::Internal);
    }
};
```

深夜3時のアラートは来なくなった。存在しないユーザーIDでリクエストが来ても、404を返すだけでプロセスはクラッシュしない。

## 結論

`.unwrap()`は「絶対に失敗しない」という強い主張だ。その仮定が崩れたとき、プロセス全体が道連れになる。

5つの教訓をまとめる。

1. **`.unwrap()`を本番で使わない**: `?`で伝播させる
2. **カスタムエラー型を定義**: 呼び出し元で適切に処理できる
3. **PostgreSQLエラーコードを活用**: 制約違反を区別
4. **リトライ可能なエラーを識別**: デッドロックは再試行
5. **エラーを無視しない**: 最低でもログに記録

エラーハンドリングは面倒に見えるが、深夜に起こされるよりはましだ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_16_see_no_evil
cargo run
```

## 参考資料

- [PostgreSQL - Error Codes](https://www.postgresql.org/docs/current/errcodes-appendix.html)
- [thiserror - docs.rs](https://docs.rs/thiserror/latest/thiserror/)
- [sqlx - Error handling](https://docs.rs/sqlx/latest/sqlx/struct.Error.html)
