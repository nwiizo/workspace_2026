# デッドロックで15分間止まった話：ロック戦略の落とし穴

## 発端

金曜日の夕方、障害報告が飛んできた。

「決済処理が15分間止まりました」

ログを見ると、`ERROR 40P01 deadlock detected`。2つのトランザクションがお互いのロックを待って、身動きが取れなくなっていた。PostgreSQLがタイムアウトでデッドロックを検出し、片方をキャンセルするまで、他のトランザクションも巻き添えになっていた。

原因を調べると、送金処理で「送金元→送金先」の順でロックを取っていた。A→Bの送金とB→Aの送金が同時に来ると、互いにロックを待ってしまう。

## Lost Updateとは何か

デッドロックの前に、なぜロックが必要なのかを確認する。

```rust
// ❌ Lost Update が発生するコード
async fn withdraw(pool: &PgPool, account_id: Uuid, amount: Decimal) -> Result<()> {
    // 現在の残高を読む
    let balance: Decimal = sqlx::query_scalar(
        "SELECT balance FROM accounts WHERE id = $1"
    )
    .bind(account_id)
    .fetch_one(pool).await?;

    // アプリケーションで計算
    let new_balance = balance - amount;

    // 更新
    sqlx::query("UPDATE accounts SET balance = $1 WHERE id = $2")
        .bind(new_balance)
        .bind(account_id)
        .execute(pool).await?;

    Ok(())
}
```

2つのトランザクションが同時に実行されると。

```
TX1: SELECT balance → 1000
TX2: SELECT balance → 1000
TX1: UPDATE balance = 900  (1000 - 100)
TX2: UPDATE balance = 800  (1000 - 200)
-- 結果: 800（本来は700であるべき）
```

TX1の更新がTX2に上書きされ、100円が消えた。これがLost Updateだ。

## 解決策1：アトミック更新

最もシンプルな解決策。計算をSQLで行う。

```rust
// ✅ アトミック更新
async fn withdraw_atomic(pool: &PgPool, account_id: Uuid, amount: Decimal) -> Result<bool> {
    let rows = sqlx::query(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2 AND balance >= $1"
    )
    .bind(amount)
    .bind(account_id)
    .execute(pool).await?
    .rows_affected();

    Ok(rows > 0)
}
```

`balance = balance - $1`という形で書けば、読み取りと更新がアトミックに行われる。同時実行でも正しく動作する。

ただし、複雑なビジネスロジックがある場合は、アプリケーション側で計算が必要になる。そこでロックの出番だ。

## 解決策2：楽観的ロック

レコードにバージョン番号を持たせ、更新時に確認する。

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    stock INT NOT NULL DEFAULT 0,
    version INT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
async fn update_with_optimistic_lock(
    pool: &PgPool,
    product_id: Uuid,
    new_price: Decimal,
    expected_version: i32,
) -> Result<Product, LockError> {
    let result: Option<Product> = sqlx::query_as(
        r#"
        UPDATE products
        SET price = $1, version = version + 1, updated_at = NOW()
        WHERE id = $2 AND version = $3
        RETURNING id, name, price, stock, version, updated_at
        "#,
    )
    .bind(new_price)
    .bind(product_id)
    .bind(expected_version)
    .fetch_optional(pool).await?;

    match result {
        Some(product) => Ok(product),
        None => {
            // バージョン不一致 or レコード不存在
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM products WHERE id = $1)"
            )
            .bind(product_id)
            .fetch_one(pool).await?;

            if exists {
                Err(LockError::ConcurrentModification)
            } else {
                Err(LockError::NotFound)
            }
        }
    }
}
```

`WHERE version = $3`の条件により、他のトランザクションが先に更新していた場合は0行が返る。呼び出し元はリトライするか、ユーザーにエラーを返す。

### 楽観的ロックの特徴

- **長所**: ロックを取らないので、読み取りがブロックされない
- **短所**: 競合が多い場合、リトライが頻発する
- **適したケース**: 競合が少ない、ユーザーが編集して保存するフロー

## 解決策3：悲観的ロック（FOR UPDATE）

レコードを明示的にロックする。他のトランザクションはロックが解放されるまで待機する。

```rust
async fn transfer_with_pessimistic_lock(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    amount: Decimal,
) -> Result<(), LockError> {
    let mut tx = pool.begin().await?;

    // ロック順序を統一（IDでソート）してデッドロック防止
    let (first_id, second_id) = if from_id < to_id {
        (from_id, to_id)
    } else {
        (to_id, from_id)
    };

    // 最初のアカウントをロック
    let _first: Account = sqlx::query_as(
        "SELECT id, name, balance FROM accounts WHERE id = $1 FOR UPDATE"
    )
    .bind(first_id)
    .fetch_one(&mut *tx).await?;

    // 2番目のアカウントをロック
    let _second: Account = sqlx::query_as(
        "SELECT id, name, balance FROM accounts WHERE id = $1 FOR UPDATE"
    )
    .bind(second_id)
    .fetch_one(&mut *tx).await?;

    // 残高確認と送金処理
    let from_balance: Decimal = sqlx::query_scalar(
        "SELECT balance FROM accounts WHERE id = $1"
    )
    .bind(from_id)
    .fetch_one(&mut *tx).await?;

    if from_balance < amount {
        return Err(LockError::InsufficientStock);
    }

    sqlx::query("UPDATE accounts SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(from_id)
        .execute(&mut *tx).await?;

    sqlx::query("UPDATE accounts SET balance = balance + $1 WHERE id = $2")
        .bind(amount)
        .bind(to_id)
        .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}
```

**重要**: ロック順序を統一している。IDの小さい方から先にロックすることで、デッドロックを防ぐ。

## デッドロックが起きる仕組み

冒頭の障害は、ロック順序を統一していなかったために発生した。

```
TX1: A→Bへ送金
  Lock(A) ✓
  Lock(B) 待ち...

TX2: B→Aへ送金
  Lock(B) ✓
  Lock(A) 待ち...

→ 互いに相手のロック解放を待って永久に進めない
```

PostgreSQLは1秒でデッドロックを検出し、片方をキャンセルする。ただし、その間の他のトランザクションは待たされる。

### デッドロック防止策

1. **ロック順序の統一**: 必ずIDの小さい順などでロック
2. **タイムアウト設定**: `SET LOCAL lock_timeout = '5s'`
3. **NOWAIT**: 即座に失敗させる `FOR UPDATE NOWAIT`
4. **リトライ**: エラーコード40P01をキャッチして再試行

```rust
// NOWAIT: ロックが取れなければ即座に失敗
let result = sqlx::query_as::<_, Account>(
    "SELECT * FROM accounts WHERE id = $1 FOR UPDATE NOWAIT"
)
.bind(account_id)
.fetch_one(&mut *tx).await;

match result {
    Ok(account) => { /* 処理 */ }
    Err(e) if e.to_string().contains("could not obtain lock") => {
        // リトライまたはエラー返却
    }
    Err(e) => return Err(e.into()),
}
```

## FOR UPDATE のバリエーション

```sql
-- 基本: ロック取得まで待機
SELECT * FROM accounts WHERE id = $1 FOR UPDATE;

-- NOWAIT: ロック不可なら即座にエラー
SELECT * FROM accounts WHERE id = $1 FOR UPDATE NOWAIT;

-- SKIP LOCKED: ロック済みの行をスキップ（キュー処理向け）
SELECT * FROM tasks WHERE status = 'pending'
ORDER BY created_at
LIMIT 1
FOR UPDATE SKIP LOCKED;
```

`SKIP LOCKED`はジョブキューの実装で特に有用だ。複数のワーカーが同じキューから取り出しても、互いにブロックしない。

```rust
async fn claim_next_task(pool: &PgPool) -> Result<Option<Task>> {
    let mut tx = pool.begin().await?;

    let task: Option<Task> = sqlx::query_as(
        r#"
        SELECT id, payload, status
        FROM tasks
        WHERE status = 'pending'
        ORDER BY created_at
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        "#
    )
    .fetch_optional(&mut *tx).await?;

    if let Some(ref t) = task {
        sqlx::query("UPDATE tasks SET status = 'processing' WHERE id = $1")
            .bind(t.id)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(task)
}
```

## トランザクション分離レベル

PostgreSQLは3つの分離レベルをサポートする。

| レベル | 説明 | 用途 |
|--------|------|------|
| READ COMMITTED | コミット済みデータのみ表示（デフォルト）| 通常のトランザクション |
| REPEATABLE READ | トランザクション開始時点のスナップショット | レポート生成 |
| SERIALIZABLE | 完全な直列化 | 厳密な整合性が必要 |

```rust
let mut tx = pool.begin().await?;

// REPEATABLE READ に設定
sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
    .execute(&mut *tx).await?;

// このトランザクション内では、他のトランザクションの変更が見えない
let count1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
    .fetch_one(&mut *tx).await?;

// 他のトランザクションがINSERTしても...
// 同じクエリは同じ結果を返す
let count2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
    .fetch_one(&mut *tx).await?;

assert_eq!(count1, count2);  // 常に一致

tx.commit().await?;
```

### SERIALIZABLEの注意点

```rust
// SERIALIZABLEはシリアライゼーション失敗（40001）が起きうる
loop {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx).await?;

    match process(&mut tx).await {
        Ok(_) => {
            tx.commit().await?;
            break;
        }
        Err(e) if is_serialization_failure(&e) => {
            // リトライ
            tx.rollback().await?;
            continue;
        }
        Err(e) => return Err(e),
    }
}

fn is_serialization_failure(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = e {
        db_err.code().as_deref() == Some("40001")
    } else {
        false
    }
}
```

## ロック戦略の選択指針

```
どちらを選ぶ？
├─ 競合が稀 → 楽観的ロック
│   └─ versionカラムで検出、リトライで対応
├─ 競合が頻繁 → 悲観的ロック
│   └─ FOR UPDATEで確実にロック
├─ 単純なカウンタ更新 → アトミック更新
│   └─ SET count = count + 1
└─ キュー処理 → SKIP LOCKED
    └─ ワーカー間でブロックしない
```

## 冒頭の問題を振り返る

デッドロックで15分止まった問題は、ロック順序の統一で解決した。

```rust
// Before: ロック順序がランダム
let _from = lock(&from_id).await;  // A or B
let _to = lock(&to_id).await;      // B or A

// After: 必ずIDの小さい順でロック
let (first, second) = if from_id < to_id {
    (from_id, to_id)
} else {
    (to_id, from_id)
};
let _first = lock(&first).await;
let _second = lock(&second).await;
```

さらに、タイムアウトを設定して被害を限定するようにした。

```rust
sqlx::query("SET LOCAL lock_timeout = '5s'")
    .execute(&mut *tx).await?;
```

5秒でロックが取れなければエラーになる。エラーハンドリングでリトライするか、ユーザーに「しばらくしてから再試行してください」と返す。

## まとめ

ロック戦略を誤ると、デッドロックで本番が止まる。防止策は明確だ。

1. **アトミック更新を優先**: `SET x = x + 1` で済むなら使う
2. **楽観的ロック**: 競合が稀なら、バージョン番号で検出
3. **悲観的ロック**: 競合が頻繁なら、FOR UPDATEでロック
4. **ロック順序の統一**: デッドロック防止の基本
5. **タイムアウト設定**: 被害を限定する

金曜日の夕方に障害対応するのは避けたい。ロック戦略は設計時に決めておくものだ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_17_transaction_locking
cargo run
```

## 参考資料

- [PostgreSQL - Explicit Locking](https://www.postgresql.org/docs/current/explicit-locking.html)
- [PostgreSQL - Transaction Isolation](https://www.postgresql.org/docs/current/transaction-iso.html)
- [sqlx - Transactions](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html#method.begin)
