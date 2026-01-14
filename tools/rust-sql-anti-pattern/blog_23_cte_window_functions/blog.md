# ROW_NUMBER/RANK/DENSE_RANK完全理解：同点の扱いで迷わない

## 3つの関数の違い

ウィンドウ関数の`ROW_NUMBER`、`RANK`、`DENSE_RANK`は、全て「順番を付ける」関数だ。違いは「同点の扱い」にある。

```sql
SELECT
    name,
    score,
    ROW_NUMBER() OVER (ORDER BY score DESC) as row_num,
    RANK() OVER (ORDER BY score DESC) as rank,
    DENSE_RANK() OVER (ORDER BY score DESC) as dense_rank
FROM students;
```

| name | score | row_num | rank | dense_rank |
|------|-------|---------|------|------------|
| Alice | 100 | 1 | 1 | 1 |
| Bob | 90 | 2 | 2 | 2 |
| Charlie | 90 | 3 | 2 | 2 |
| David | 80 | 4 | 4 | 3 |

- **ROW_NUMBER**: 常に連番。同点でも異なる番号（どちらが先かは不定）
- **RANK**: 同点は同じ番号。次は飛ぶ（1,2,2,4）
- **DENSE_RANK**: 同点は同じ番号。次は連続（1,2,2,3）

## 使い分け

### ROW_NUMBER: 厳密に1件だけ欲しい

各ユーザーの最新注文を1件だけ取得する。

```rust
let latest_orders: Vec<Order> = sqlx::query_as(
    r#"
    WITH ranked AS (
        SELECT *,
            ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
        FROM orders
    )
    SELECT id, user_id, total, status, created_at
    FROM ranked
    WHERE rn = 1
    "#
)
.fetch_all(&pool).await?;
```

同じ`created_at`の注文が複数あっても、どれか1件だけが返る。

### RANK: 同順位を考慮したい

売上ランキングで同順位を表示する。

```rust
let ranking: Vec<(Uuid, Decimal, i64)> = sqlx::query_as(
    r#"
    SELECT
        user_id,
        SUM(total) as total_spent,
        RANK() OVER (ORDER BY SUM(total) DESC) as ranking
    FROM orders
    GROUP BY user_id
    "#
)
.fetch_all(&pool).await?;

// 出力:
// 1位: Alice (10000円)
// 2位: Bob (8000円)
// 2位: Charlie (8000円)
// 4位: David (5000円)
```

BobとCharlieが同点2位で、Davidは4位（3位は飛ぶ）。

### DENSE_RANK: 順位の数を知りたい

何種類の価格帯があるかを知りたい場合。

```rust
let price_tiers: Vec<(Decimal, i64)> = sqlx::query_as(
    r#"
    SELECT DISTINCT
        price,
        DENSE_RANK() OVER (ORDER BY price DESC) as tier
    FROM products
    "#
)
.fetch_all(&pool).await?;

// 出力:
// 1: 1000円
// 2: 800円
// 3: 500円
// 最大値 = 価格帯の数
```

## CTE（WITH句）の基本

ウィンドウ関数と組み合わせてよく使われるのがCTE（Common Table Expression）だ。

```rust
let high_value_customers: Vec<HighValueCustomer> = sqlx::query_as(
    r#"
    WITH completed_orders AS (
        SELECT * FROM orders WHERE status = 'completed'
    ),
    user_totals AS (
        SELECT user_id, SUM(total) as total_spent
        FROM completed_orders
        GROUP BY user_id
    )
    SELECT user_id, total_spent
    FROM user_totals
    WHERE total_spent > $1
    ORDER BY total_spent DESC
    "#
)
.bind(Decimal::new(50000, 2))
.fetch_all(&pool).await?;
```

CTEを使うと、複雑なクエリを段階的に読みやすくできる。

## 再帰CTE：階層データの取得

PostgreSQLは`WITH RECURSIVE`で再帰クエリをサポートする。カテゴリの階層構造を取得する例。

```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES categories(id)
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct CategoryWithPath {
    id: Uuid,
    name: String,
    depth: i32,
    path: String,
}

let categories: Vec<CategoryWithPath> = sqlx::query_as(
    r#"
    WITH RECURSIVE category_tree AS (
        -- ベースケース: ルートカテゴリ
        SELECT
            id,
            name,
            0 as depth,
            name as path
        FROM categories
        WHERE parent_id IS NULL

        UNION ALL

        -- 再帰ケース: 子カテゴリ
        SELECT
            c.id,
            c.name,
            ct.depth + 1,
            ct.path || ' > ' || c.name
        FROM categories c
        JOIN category_tree ct ON c.parent_id = ct.id
    )
    SELECT id, name, depth, path
    FROM category_tree
    ORDER BY path
    "#
)
.fetch_all(&pool).await?;

// 出力:
// Electronics (depth: 0)
// Electronics > Computers (depth: 1)
// Electronics > Computers > Laptops (depth: 2)
// Electronics > Computers > Desktops (depth: 2)
// Electronics > Phones (depth: 1)
// Electronics > Phones > Smartphones (depth: 2)
```

再帰CTEの構造。

1. **ベースケース**: `WHERE parent_id IS NULL`でルートを取得
2. **UNION ALL**: 結果を累積
3. **再帰ケース**: `JOIN category_tree`で親の結果を参照

## LAG/LEAD：前後の行を参照

前日・翌日の売上と比較する。

```rust
#[derive(Debug, sqlx::FromRow)]
struct SalesWithWindow {
    order_date: NaiveDate,
    daily_total: Decimal,
    running_total: Decimal,
    prev_day_total: Option<Decimal>,
    next_day_total: Option<Decimal>,
}

let sales: Vec<SalesWithWindow> = sqlx::query_as(
    r#"
    WITH daily_sales AS (
        SELECT
            created_at::DATE as order_date,
            SUM(total) as daily_total
        FROM orders
        GROUP BY created_at::DATE
    )
    SELECT
        order_date,
        daily_total,
        SUM(daily_total) OVER (ORDER BY order_date) as running_total,
        LAG(daily_total) OVER (ORDER BY order_date) as prev_day_total,
        LEAD(daily_total) OVER (ORDER BY order_date) as next_day_total
    FROM daily_sales
    ORDER BY order_date
    "#
)
.fetch_all(&pool).await?;

// 出力:
// 2024-01-01: 1000円, 累計: 1000円, 前日: None, 翌日: 1500円
// 2024-01-02: 1500円, 累計: 2500円, 前日: 1000円, 翌日: 800円
// 2024-01-03: 800円, 累計: 3300円, 前日: 1500円, 翌日: None
```

- **LAG(column)**: 前の行の値
- **LEAD(column)**: 次の行の値
- **SUM() OVER (ORDER BY ...)**: 累計

## ウィンドウフレーム：移動平均

3日間の移動平均を計算する。

```rust
let moving_avg: Vec<(NaiveDate, Decimal, Decimal)> = sqlx::query_as(
    r#"
    WITH daily_sales AS (
        SELECT
            created_at::DATE as order_date,
            SUM(total) as daily_total
        FROM orders
        GROUP BY created_at::DATE
    )
    SELECT
        order_date,
        daily_total,
        AVG(daily_total) OVER (
            ORDER BY order_date
            ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
        ) as moving_avg_3day
    FROM daily_sales
    ORDER BY order_date
    "#
)
.fetch_all(&pool).await?;
```

`ROWS BETWEEN 2 PRECEDING AND CURRENT ROW`は「現在の行から2行前まで」を指定する。

### フレーム指定のバリエーション

```sql
-- 現在行から2行前まで
ROWS BETWEEN 2 PRECEDING AND CURRENT ROW

-- 最初から現在行まで（累計）
ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW

-- 現在行の前後1行
ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING

-- パーティション全体
ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
```

## グループ別Top N

各ユーザーの上位2件の注文を取得する。

```rust
let top_orders: Vec<(Uuid, Uuid, Decimal, i64)> = sqlx::query_as(
    r#"
    WITH ranked_orders AS (
        SELECT
            id,
            user_id,
            total,
            ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY total DESC) as rn
        FROM orders
    )
    SELECT id, user_id, total, rn
    FROM ranked_orders
    WHERE rn <= 2
    ORDER BY user_id, rn
    "#
)
.fetch_all(&pool).await?;

// 出力:
// User A:
//   #1: Order X - 500円
//   #2: Order Y - 300円
// User B:
//   #1: Order Z - 800円
//   #2: Order W - 600円
```

`PARTITION BY user_id`でユーザーごとにグループ化し、`ROW_NUMBER()`で順位を付ける。

## DISTINCT ONとの比較

PostgreSQL固有の`DISTINCT ON`も「グループごとに1件」を取得できる。

```sql
-- DISTINCT ON（PostgreSQL固有）
SELECT DISTINCT ON (user_id) *
FROM orders
ORDER BY user_id, created_at DESC;

-- ROW_NUMBER（標準SQL）
WITH ranked AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
    FROM orders
)
SELECT * FROM ranked WHERE rn = 1;
```

| 方法 | N件取得 | 標準SQL | 可読性 |
|------|--------|---------|--------|
| DISTINCT ON | 1件のみ | No | 高い |
| ROW_NUMBER | N件可能 | Yes | 中程度 |

1件だけなら`DISTINCT ON`が簡潔。N件取得や標準SQL準拠が必要なら`ROW_NUMBER`を使う。

## パフォーマンス考慮

ウィンドウ関数はインデックスを活用できる。

```sql
-- ORDER BYに使うカラムにインデックスがあると効率的
CREATE INDEX idx_orders_user_created ON orders(user_id, created_at DESC);

-- このクエリでインデックスが活用される
SELECT *,
    ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
FROM orders;
```

ただし、大量データに対する`ORDER BY`は遅くなりうる。`WHERE`で絞り込んでから適用するのがベスト。

```sql
-- 先に絞り込む
WITH recent_orders AS (
    SELECT * FROM orders
    WHERE created_at > NOW() - INTERVAL '30 days'
)
SELECT *,
    ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
FROM recent_orders;
```

## まとめ

### 関数の使い分け

| 関数 | 同点の扱い | 用途 |
|------|-----------|------|
| ROW_NUMBER | 常に連番 | 厳密に1件 |
| RANK | 同点同番号、次は飛ぶ | ランキング表示 |
| DENSE_RANK | 同点同番号、次は連続 | 順位の種類を数える |

### よく使うパターン

1. **グループ別Top N**: `ROW_NUMBER() + PARTITION BY + WHERE rn <= N`
2. **累計**: `SUM() OVER (ORDER BY column)`
3. **前後比較**: `LAG(column) / LEAD(column)`
4. **移動平均**: `AVG() OVER (ROWS BETWEEN N PRECEDING AND CURRENT ROW)`
5. **階層取得**: `WITH RECURSIVE`

CTEとウィンドウ関数を組み合わせれば、複雑な集計もSQLだけで実現できる。アプリケーション側でループ処理するより効率的だ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_23_cte_window_functions
cargo run
```

## 参考資料

- [PostgreSQL - Window Functions](https://www.postgresql.org/docs/current/functions-window.html)
- [PostgreSQL - WITH Queries](https://www.postgresql.org/docs/current/queries-with.html)
- [PostgreSQL - DISTINCT ON](https://www.postgresql.org/docs/current/sql-select.html#SQL-DISTINCT)
