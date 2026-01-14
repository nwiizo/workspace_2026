# IDに欠番があっても慌てない：ギャップ恐怖症からの卒業

## 常識を疑う

「IDに欠番があります。データがおかしくないですか？」

こういう問い合わせを受けることがある。ID 1, 2, 4, 5 と続いていて、3がない。「データが消えた」「バグがある」と心配される。

結論から言うと、SERIALやIDENTITYのギャップは正常動作だ。むしろ、ギャップを埋めようとする方が危険だ。

## なぜギャップが発生するか

PostgreSQLのSERIALは内部的にシーケンスを使う。シーケンスは「次の値を発行する」だけで、使われたかどうかは追跡しない。

### 原因1：ロールバック

```sql
BEGIN;
INSERT INTO orders (customer) VALUES ('Alice');  -- ID=1が発行
-- 何らかの理由でロールバック
ROLLBACK;

INSERT INTO orders (customer) VALUES ('Bob');    -- ID=2が発行
-- ID=1は使われていない
```

### 原因2：削除

```rust
// ID=1, 2, 3 の注文を作成
// ID=2 を削除
sqlx::query("DELETE FROM orders WHERE id = 2")
    .execute(&pool).await?;

// 新しい注文は ID=4 になる（2は再利用されない）
```

### 原因3：シーケンスキャッシュ

PostgreSQLはパフォーマンスのためにシーケンス値をキャッシュする。サーバー再起動でキャッシュが失われ、ギャップが生じる。

### 原因4：バルクインサートの失敗

```sql
INSERT INTO orders (customer) VALUES ('A'), ('B'), ('C'), ('D');
-- 途中で制約違反が起きると、シーケンス値は消費されたまま
```

## ギャップを埋めようとするとどうなるか

「空いているIDを見つけて再利用しよう」という発想は危険だ。

```sql
-- ❌ アンチパターン：空きIDを探す
SELECT id + 1 FROM orders o1
WHERE NOT EXISTS (SELECT 1 FROM orders o2 WHERE o2.id = o1.id + 1)
ORDER BY id LIMIT 1;
```

### 問題1：レースコンディション

2つのトランザクションが同時に空きIDを見つけ、同じIDを使おうとする。

```
TX1: 空きID = 3 を発見
TX2: 空きID = 3 を発見（同時）
TX1: ID=3 でINSERT → 成功
TX2: ID=3 でINSERT → 重複エラー
```

### 問題2：外部参照の破壊

以前ID=3だった注文を参照している外部システム、ログ、キャッシュがあるかもしれない。新しいデータが古いIDを使うと混乱が起きる。

### 問題3：監査ログの混乱

「注文ID=3」という記録があったとき、それが古いデータなのか新しいデータなのかわからなくなる。

## 正しいアプローチ

### アプローチ1：ギャップを受け入れる

```rust
// IDはただの識別子。欠番は気にしない
let orders: Vec<Order> = sqlx::query_as(
    "SELECT id, customer, total FROM orders ORDER BY id"
)
.fetch_all(&pool).await?;

// ID: 1, 2, 4, 5（3は欠番、問題なし）
```

### アプローチ2：表示用番号を分離する

IDとは別に、顧客に見せる番号を管理する。

```sql
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),  -- 内部ID
    order_number VARCHAR(20) NOT NULL UNIQUE,       -- 表示用番号
    customer_name VARCHAR(100) NOT NULL,
    total DECIMAL(10,2) NOT NULL
);
```

```rust
fn generate_order_number() -> String {
    let now = Utc::now();
    format!("ORD-{}{:02}-{:06}",
        now.year(), now.month(),
        SEQUENCE.fetch_add(1, Ordering::SeqCst))
}

// ORD-202401-000001, ORD-202401-000002, ...
```

- 内部ID（UUID）: システム間の参照、外部に公開しない
- 表示用番号: 顧客に見せる、連番である必要はない

### アプローチ3：UUIDを使う

UUIDならそもそもギャップという概念がない。

```rust
let order_id = Uuid::new_v4();  // ランダムな識別子
// 順序も連続性もない → ギャップの概念がない
```

ただしUUIDv4はランダムなため、B-treeインデックスの効率が悪い。UUIDv7（時間順ソート可能）を検討する。

## ページネーションとギャップ

「ID 1〜100を取得」というページネーションは、ギャップがあると問題になる。

```sql
-- ❌ 問題あり：ギャップがあると件数が減る
SELECT * FROM orders WHERE id BETWEEN 1 AND 100;
-- ID 2, 15, 87 が欠番なら97件しか返らない

-- ✅ カーソルベースページネーション
SELECT * FROM orders WHERE created_at < $1 ORDER BY created_at DESC LIMIT 100;
```

### カーソルベースの利点

```rust
// Page 1
let page1: Vec<Order> = sqlx::query_as(
    "SELECT * FROM orders ORDER BY created_at DESC LIMIT 20"
)
.fetch_all(&pool).await?;

// Page 2（前ページの最後のcreated_atをカーソルとして使用）
if let Some(last) = page1.last() {
    let page2: Vec<Order> = sqlx::query_as(
        "SELECT * FROM orders WHERE created_at < $1 ORDER BY created_at DESC LIMIT 20"
    )
    .bind(last.created_at)
    .fetch_all(&pool).await?;
}
```

- ギャップの影響を受けない
- 大量データでもOFFSETのパフォーマンス問題がない
- 並行挿入でも安定した結果

## やってはいけないこと

1. **ギャップを探して再利用**: レースコンディション、参照破壊
2. **全IDを振り直す**: FK違反、長時間ロック、キャッシュ無効化
3. **ID範囲でページネーション**: 件数が不安定

## 結論

「IDに欠番がある」は異常ではなく正常だ。SERIALやIDENTITYは「ユニークな値を発行する」機能であり、「連続した値を発行する」機能ではない。

ギャップ恐怖症から卒業するためのポイント。

1. **IDは識別子**: 順序や連続性に意味を持たせない
2. **表示用番号は分離**: 顧客に見せる番号は別管理
3. **カーソルベースページネーション**: ID範囲に依存しない
4. **UUIDの検討**: そもそもギャップという概念がない

IDの欠番を見つけても慌てない。それは設計上正常な動作だ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_15_pseudokey_neat_freak
cargo run
```

## 参考資料

- [PostgreSQL - Sequence Manipulation Functions](https://www.postgresql.org/docs/current/functions-sequence.html)
- [PostgreSQL - IDENTITY Columns](https://www.postgresql.org/docs/current/sql-createtable.html)
- [uuid - docs.rs](https://docs.rs/uuid/latest/uuid/)
