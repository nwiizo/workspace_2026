# pg_advisory_lockの全貌：分散ロックの落とし穴と回避策

## はじめに

「同じバッチ処理が複数インスタンスで同時に実行されている」

クラウド環境でアプリケーションをスケールアウトすると、この問題に遭遇する。クリーンアップジョブ、レポート生成、データ同期……1つのインスタンスだけで実行したい処理がある。

Redisを導入してもいいが、既にPostgreSQLを使っているなら`pg_advisory_lock`で分散ロックを実現できる。ただし、使い方を誤ると意図した排他制御ができない。本記事ではAdvisory Lockの内部動作を解説し、正しい使い方を示す。

## Advisory Lockとは

PostgreSQLの通常のロック（行ロック、テーブルロック）はSQLの実行に連動して自動で取得・解放される。一方、Advisory Lockはアプリケーションが明示的に取得・解放する。「アドバイザリ（助言的）」という名前の通り、PostgreSQL自体はロックの意味を知らない。

```sql
-- ロックを取得
SELECT pg_advisory_lock(12345);

-- 排他処理を実行
-- ...

-- ロックを解放
SELECT pg_advisory_unlock(12345);
```

ロックキーは64ビット整数。用途ごとに異なる数値を使う。

## 2種類のAdvisory Lock

### セッションレベルロック

デフォルトのAdvisory Lockはセッション（コネクション）に紐づく。明示的に解放するか、セッションが終了するまで保持される。

```rust
// セッションレベルロック
let lock_key: i64 = 12345;

// ノンブロッキングで取得を試みる
let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&pool).await?;

if acquired {
    // 排他処理
    do_exclusive_work().await?;

    // 明示的に解放
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_key)
        .execute(&pool).await?;
}
```

**特徴**
- 同一セッション内で複数回取得可能（再入可能）
- 取得した回数だけ解放が必要
- セッション終了時に自動解放

### トランザクションレベルロック

トランザクションに紐づくロック。コミットまたはロールバックで自動解放される。

```rust
// トランザクションレベルロック
let mut tx = pool.begin().await?;

sqlx::query("SELECT pg_advisory_xact_lock($1)")
    .bind(lock_key)
    .execute(&mut *tx).await?;

// トランザクション内の処理
do_work_in_transaction(&mut tx).await?;

// コミットでロックが自動解放
tx.commit().await?;
```

**特徴**
- トランザクション終了で自動解放（明示的なunlockは不要）
- 解放忘れがない
- トランザクション外では使えない

## ブロッキング vs ノンブロッキング

```sql
-- ブロッキング: ロックが取得できるまで待機
SELECT pg_advisory_lock(key);
SELECT pg_advisory_xact_lock(key);

-- ノンブロッキング: 即座にtrue/falseを返す
SELECT pg_try_advisory_lock(key);
SELECT pg_try_advisory_xact_lock(key);
```

バッチ処理では通常ノンブロッキングを使う。他のインスタンスが実行中なら、待機せずにスキップする方が効率的だ。

```rust
let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&pool).await?;

if !acquired {
    println!("Another instance is running, skipping");
    return Ok(());
}
```

## 落とし穴1：コネクションプールでのセッションリーク

sqlxのコネクションプールを使う場合、セッションレベルロックには注意が必要だ。

```rust
// ❌ 問題のあるコード
{
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(lock_key)
        .fetch_one(&pool)  // プールから取得したコネクション
        .await?;

    if acquired {
        do_work().await?;
        // unlockを忘れた！
    }
}  // コネクションはプールに返却されるが、ロックは残ったまま
```

コネクションがプールに返却されても、セッションは終了しない。ロックは保持されたままだ。次にこのコネクションを使った処理が、意図せずロックを保持することになる。

### 解決策1：必ずunlockする

```rust
let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&pool).await?;

if acquired {
    let result = do_work().await;

    // 成功・失敗に関わらずunlock
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_key)
        .execute(&pool).await?;

    result?;
}
```

### 解決策2：専用のコネクションを使う

```rust
let mut conn = pool.acquire().await?;

let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&mut *conn).await?;

if acquired {
    let result = do_work_with_conn(&mut conn).await;

    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_key)
        .execute(&mut *conn).await?;

    result?;
}
// connがdropされるとき、コネクションは閉じられる（max_lifetime経過後）
```

### 解決策3：トランザクションレベルロックを使う

```rust
let mut tx = pool.begin().await?;

let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
    .bind(lock_key)
    .fetch_one(&mut *tx).await?;

if acquired {
    do_work(&mut tx).await?;
    tx.commit().await?;  // ロックは自動解放
} else {
    tx.rollback().await?;
}
```

トランザクションレベルロックなら解放忘れはない。

## 落とし穴2：再入可能による意図しない動作

セッションレベルロックは再入可能だ。

```rust
// 1回目: true
let acquired1 = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&mut *conn).await?;

// 2回目: 同一セッションなのでtrue（カウントが2になる）
let acquired2 = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
    .bind(lock_key)
    .fetch_one(&mut *conn).await?;

// 1回目のunlock: まだロックは保持されている（カウントが1）
sqlx::query("SELECT pg_advisory_unlock($1)")
    .bind(lock_key)
    .execute(&mut *conn).await?;

// 2回目のunlock: これでやっと解放
sqlx::query("SELECT pg_advisory_unlock($1)")
    .bind(lock_key)
    .execute(&mut *conn).await?;
```

「2回ロックしたから2回解放が必要」という動作が、バグの原因になりやすい。

### 解決策：unlock_allを使う

```rust
// このセッションで保持している全Advisory Lockを解放
sqlx::query("SELECT pg_advisory_unlock_all()")
    .execute(&pool).await?;
```

ただしこれは他のAdvisory Lockも解放してしまう。用途ごとにコネクションを分けるか、トランザクションレベルロックを使う方が安全。

## 落とし穴3：ロックキーの衝突

ロックキーは64ビット整数だが、用途が異なるロックで同じキーを使ってしまうことがある。

```rust
// クリーンアップジョブ
const CLEANUP_LOCK: i64 = 1;

// レポート生成ジョブ（同じキーを使ってしまった）
const REPORT_LOCK: i64 = 1;  // 衝突！
```

### 解決策：名前空間を設ける

```rust
// 用途ごとに異なるキーレンジを使う
const LOCK_NAMESPACE_CLEANUP: i64 = 1_000_000;
const LOCK_NAMESPACE_REPORT: i64 = 2_000_000;

fn cleanup_lock_key(job_id: i64) -> i64 {
    LOCK_NAMESPACE_CLEANUP + job_id
}

fn report_lock_key(report_id: i64) -> i64 {
    LOCK_NAMESPACE_REPORT + report_id
}
```

または、2つの32ビット整数を使う方法もある。

```sql
-- 2つの32ビット整数でロック
SELECT pg_advisory_lock(1, 12345);  -- (namespace, key)
```

```rust
let namespace: i32 = 1;  // 用途を表す
let key: i32 = 12345;    // 具体的なID

sqlx::query("SELECT pg_try_advisory_lock($1, $2)")
    .bind(namespace)
    .bind(key)
    .fetch_one(&pool).await?;
```

## その他のPostgreSQL固有機能

Advisory Lockの解説に集中したが、本デモコードには他の機能も含まれている。簡潔に紹介する。

### JSONB

```rust
#[derive(Debug, Serialize, Deserialize)]
struct ProductAttributes {
    color: Option<String>,
    size: Option<String>,
    tags: Vec<String>,
}

// 挿入
sqlx::query("INSERT INTO products (name, attributes) VALUES ($1, $2)")
    .bind("T-Shirt")
    .bind(Json(&attrs))
    .execute(&pool).await?;

// 検索（->>'key'で値を取得）
let products: Vec<Product> = sqlx::query_as(
    "SELECT * FROM products WHERE attributes->>'color' = $1"
)
.bind("red")
.fetch_all(&pool).await?;

// 配列内検索（->'key' ? value）
let products: Vec<Product> = sqlx::query_as(
    "SELECT * FROM products WHERE attributes->'tags' ? $1"
)
.bind("sale")
.fetch_all(&pool).await?;
```

### DISTINCT ON

各グループの最初の行だけを取得。PostgreSQL固有の構文。

```rust
// 各ユーザーの最新注文を取得
let latest_orders: Vec<Order> = sqlx::query_as(
    r#"
    SELECT DISTINCT ON (user_id) *
    FROM orders
    ORDER BY user_id, created_at DESC
    "#
)
.fetch_all(&pool).await?;
```

### 生成列（Generated Columns）

カラム値を式から自動計算。

```sql
CREATE TABLE orders_with_tax (
    subtotal DECIMAL(10,2) NOT NULL,
    tax_rate DECIMAL(5,4) NOT NULL DEFAULT 0.10,
    tax_amount DECIMAL(10,2) GENERATED ALWAYS AS (subtotal * tax_rate) STORED,
    total DECIMAL(10,2) GENERATED ALWAYS AS (subtotal * (1 + tax_rate)) STORED
);
```

```rust
// subtotalとtax_rateだけ指定、tax_amountとtotalは自動計算
let order: OrderWithTax = sqlx::query_as(
    "INSERT INTO orders_with_tax (subtotal) VALUES ($1) RETURNING *"
)
.bind(Decimal::new(10000, 2))
.fetch_one(&pool).await?;

// tax_amount: 10.00, total: 110.00 が自動で入る
```

## まとめ

Advisory Lockは、PostgreSQLだけで分散ロックを実現できる便利な機能だ。ただし使い方を誤ると、意図した排他制御ができない。

**推奨パターン**

1. **バッチ処理**: ノンブロッキング + トランザクションレベルロック
2. **長時間処理**: 専用コネクション + セッションレベルロック + 確実なunlock
3. **ロックキー**: 名前空間を設けて衝突を防ぐ

**避けるべきパターン**

- コネクションプールでセッションレベルロックを使い、unlockを忘れる
- 同じキーを異なる用途で使う
- ブロッキングロックでタイムアウトなしに待機

Advisory Lockの動作を理解していれば、Redisなしで分散ロックを実現できる。既にPostgreSQLを使っているなら、まずはAdvisory Lockを検討してみる価値がある。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_11_postgresql_features
cargo run
```

## 参考資料

- [PostgreSQL - Advisory Locks](https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS)
- [PostgreSQL - JSON Functions](https://www.postgresql.org/docs/current/functions-json.html)
- [PostgreSQL - Generated Columns](https://www.postgresql.org/docs/current/ddl-generated-columns.html)
