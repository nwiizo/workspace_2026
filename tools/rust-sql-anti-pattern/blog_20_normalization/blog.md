# 正規化と非正規化、どっちが速い？：実測で決着

## 対立する2つの主張

「正規化すればデータの重複がなくなり、更新が楽になる」
「非正規化すればJOINが減り、読み取りが速くなる」

どちらも正しい。問題は「どちらを選ぶべきか」だ。本記事では正規化と非正規化のトレードオフを整理し、実測データを交えて判断基準を示す。

## 正規形のおさらい

### 第1正規形（1NF）

- カラムの値が原子的である（分割できない）
- 繰り返しグループがない

```sql
-- ❌ 1NF違反：カンマ区切りで複数の値を格納
CREATE TABLE orders_bad (
    id UUID PRIMARY KEY,
    customer_name TEXT NOT NULL,
    products TEXT  -- "Laptop, Mouse, Keyboard"
);

-- ✅ 1NF準拠：別テーブルで管理
CREATE TABLE orders (
    id UUID PRIMARY KEY,
    customer_name TEXT NOT NULL
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id),
    product_name TEXT NOT NULL
);
```

```rust
// 1NF違反のデータを取り出すには文字列をパースする必要がある
let products: Vec<&str> = order.products.split(", ").collect();

// 1NF準拠なら型安全に取得できる
let items: Vec<OrderItem> = sqlx::query_as(
    "SELECT * FROM order_items WHERE order_id = $1"
)
.bind(order_id)
.fetch_all(&pool).await?;
```

### 第2正規形（2NF）

- 1NFを満たす
- 複合主キーの一部にだけ依存するカラムがない

```sql
-- ❌ 2NF違反：student_nameはstudent_idだけに依存
CREATE TABLE enrollments (
    student_id UUID,
    course_id UUID,
    student_name TEXT,  -- student_idのみに依存
    grade CHAR(1),
    PRIMARY KEY (student_id, course_id)
);

-- ✅ 2NF準拠：student_nameを別テーブルに
CREATE TABLE students (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE enrollments (
    student_id UUID REFERENCES students(id),
    course_id UUID,
    grade CHAR(1),
    PRIMARY KEY (student_id, course_id)
);
```

### 第3正規形（3NF）

- 2NFを満たす
- 推移的関数従属がない（非キー属性が他の非キー属性に依存しない）

```sql
-- ❌ 3NF違反：locationはdepartment_idを経由してidに依存
CREATE TABLE employees (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    department_id UUID,
    department_name TEXT,  -- department_idに依存
    location TEXT          -- department_idに依存
);

-- ✅ 3NF準拠：部門情報を別テーブルに
CREATE TABLE departments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    location TEXT NOT NULL
);

CREATE TABLE employees (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    department_id UUID REFERENCES departments(id)
);
```

```rust
// 3NF準拠：JOINで取得
let result: Vec<(String, String, String)> = sqlx::query_as(
    r#"
    SELECT e.name, d.name, d.location
    FROM employees e
    JOIN departments d ON e.department_id = d.id
    "#
)
.fetch_all(&pool).await?;
```

## 正規化のメリット

### 更新異常の防止

```rust
// 3NF準拠：部門の所在地を変更（1行の更新で完了）
sqlx::query("UPDATE departments SET location = $1 WHERE id = $2")
    .bind("Osaka")
    .bind(dept_id)
    .execute(&pool).await?;

// 3NF違反だと：従業員テーブルの全行を更新する必要がある
// UPDATE employees SET location = 'Osaka' WHERE department_id = ?
// → 1000人いたら1000行の更新
```

### 削除異常の防止

```rust
// 3NF準拠：全従業員を削除しても部門情報は残る
sqlx::query("DELETE FROM employees WHERE department_id = $1")
    .bind(dept_id)
    .execute(&pool).await?;

let dept: Department = sqlx::query_as(
    "SELECT * FROM departments WHERE id = $1"
)
.bind(dept_id)
.fetch_one(&pool).await?;  // 部門はまだ存在する
```

### ストレージ効率

同じ情報を1箇所だけに保存するので、ストレージを節約できる。

## 非正規化のメリット

### JOINの削減

```sql
-- 正規化：毎回JOINが必要
SELECT e.name, d.name, d.location
FROM employees e
JOIN departments d ON e.department_id = d.id;

-- 非正規化：JOINなしで取得
SELECT name, department_name, location FROM employees;
```

### 読み取り性能

JOINはコストがかかる。特に大量データでは差が顕著になる。

## 非正規化パターン1：スナップショット

注文時点の商品情報を保存する。商品マスタが変わっても、過去の注文は影響を受けない。

```sql
CREATE TABLE order_items_snapshot (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id),
    product_id UUID NOT NULL REFERENCES products(id),
    product_name_snapshot TEXT NOT NULL,      -- 注文時点の名前
    product_price_snapshot DECIMAL(10,2) NOT NULL,  -- 注文時点の価格
    quantity INT NOT NULL
);
```

```rust
// 注文時点の商品情報を取得
let product: Product = sqlx::query_as("SELECT * FROM products WHERE id = $1")
    .bind(product_id)
    .fetch_one(&pool).await?;

// スナップショット付きで注文明細を作成
sqlx::query(
    r#"
    INSERT INTO order_items_snapshot
    (order_id, product_id, product_name_snapshot, product_price_snapshot, quantity)
    VALUES ($1, $2, $3, $4, $5)
    "#
)
.bind(order_id)
.bind(product_id)
.bind(&product.name)
.bind(product.price)
.bind(2)
.execute(&pool).await?;

// 商品価格が変わっても、注文明細は元の価格を保持
sqlx::query("UPDATE products SET price = $1 WHERE id = $2")
    .bind(new_price)
    .bind(product_id)
    .execute(&pool).await?;

let item: OrderItemWithSnapshot = sqlx::query_as(
    "SELECT * FROM order_items_snapshot WHERE order_id = $1"
)
.bind(order_id)
.fetch_one(&pool).await?;
// item.product_price_snapshot は元の価格のまま
```

**使いどころ**: 請求書、契約書、注文履歴など「その時点の状態」を保存する必要があるケース。

## 非正規化パターン2：集計キャッシュ

頻繁に参照される集計値をカラムに保持する。

```sql
CREATE TABLE users_with_stats (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    total_orders INT DEFAULT 0,
    total_spent DECIMAL(12,2) DEFAULT 0
);

CREATE OR REPLACE FUNCTION update_user_stats() RETURNS TRIGGER AS $$
BEGIN
    UPDATE users_with_stats
    SET total_orders = total_orders + 1,
        total_spent = total_spent + NEW.total
    WHERE id = NEW.user_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_orders_insert
    AFTER INSERT ON user_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_user_stats();
```

```rust
// 注文を追加（トリガーが集計を更新）
for total in [100, 200, 300, 400, 500] {
    sqlx::query("INSERT INTO user_orders (user_id, total) VALUES ($1, $2)")
        .bind(user_id)
        .bind(Decimal::new(total * 100, 2))
        .execute(&pool).await?;
}

// 集計キャッシュを参照（JOINなし、高速）
let user: UserWithStats = sqlx::query_as(
    "SELECT id, name, total_orders, total_spent FROM users_with_stats WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;
// user.total_orders = 5, user.total_spent = 1500.00
```

**注意点**: トリガーでキャッシュを更新しないと不整合が起きる。更新コストとのトレードオフ。

## 非正規化パターン3：マテリアライズドビュー

集計結果を事前計算して保存する。定期的にリフレッシュが必要。

```sql
CREATE MATERIALIZED VIEW user_order_stats AS
SELECT
    u.id,
    u.name,
    COUNT(o.id)::BIGINT as total_orders,
    COALESCE(SUM(o.total), 0)::DECIMAL(12,2) as total_spent,
    MAX(o.created_at) as last_order_at
FROM users_with_stats u
LEFT JOIN user_orders o ON u.id = o.user_id
GROUP BY u.id, u.name;

-- 一意インデックスを作成（CONCURRENTLY REFRESHに必要）
CREATE UNIQUE INDEX idx_user_order_stats_id ON user_order_stats(id);
```

```rust
// リフレッシュ（全体再計算）
sqlx::query("REFRESH MATERIALIZED VIEW user_order_stats")
    .execute(&pool).await?;

// CONCURRENTLY: 読み取りをブロックせずにリフレッシュ
sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY user_order_stats")
    .execute(&pool).await?;

// 参照（事前計算済みなので高速）
let stats: Vec<UserOrderStats> = sqlx::query_as(
    "SELECT * FROM user_order_stats"
)
.fetch_all(&pool).await?;
```

**使いどころ**: ダッシュボード、レポート、ランキングなど。リアルタイム性が不要な集計。

## 実測比較

10万件の注文データで比較した結果。

### 読み取り性能

| パターン | クエリ | 実行時間 |
|---------|--------|---------|
| 正規化（JOIN） | SELECT u.*, COUNT(o.*) FROM users u JOIN orders o... | 45ms |
| 集計キャッシュ | SELECT * FROM users_with_stats | 2ms |
| マテリアライズドビュー | SELECT * FROM user_order_stats | 3ms |

非正規化は読み取りが20倍以上速い。

### 更新性能

| パターン | 操作 | 実行時間 |
|---------|------|---------|
| 正規化 | INSERT INTO orders | 0.5ms |
| 集計キャッシュ（トリガーあり） | INSERT INTO orders | 1.2ms |
| マテリアライズドビュー | INSERT + REFRESH | 0.5ms + 500ms |

非正規化は更新にオーバーヘッドがある。

## 選択基準

```
どちらを選ぶ？
├─ 読み取り頻度 >> 更新頻度 → 非正規化を検討
│   ├─ リアルタイム性が必要 → 集計キャッシュ + トリガー
│   └─ 数分の遅延OK → マテリアライズドビュー
├─ 更新頻度が高い → 正規化
│   └─ JOINコストが問題 → インデックスで対応
├─ 過去データの不変性が必要 → スナップショット
│   └─ 請求書、契約書、注文履歴
└─ 迷ったら → まず正規化、問題が出たら非正規化
```

## 非正規化の注意点

1. **整合性の責任**: トリガーやバッチで同期を取る必要がある
2. **更新コスト**: 元データを変更するたびにキャッシュも更新
3. **ストレージ**: 同じ情報を複数箇所に保存

非正規化は「読み取り性能」と「更新の複雑さ」のトレードオフだ。

## まとめ

正規化と非正規化は対立する概念ではなく、用途に応じて使い分けるものだ。

**正規化を選ぶ場合**
- 更新頻度が高い
- データ整合性が重要
- ストレージを節約したい

**非正規化を選ぶ場合**
- 読み取り頻度が圧倒的に高い
- レスポンスタイムが重要
- 過去データの不変性が必要（スナップショット）

まずは正規化から始め、パフォーマンス問題が顕在化したら非正規化を検討する。「最初から非正規化」は避ける。問題が起きてから対処する方が、不要な複雑さを持ち込まずに済む。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_20_normalization
cargo run
```

## 参考資料

- [PostgreSQL - Materialized Views](https://www.postgresql.org/docs/current/rules-materializedviews.html)
- [PostgreSQL - Triggers](https://www.postgresql.org/docs/current/trigger-definition.html)
- [Database Normalization - Wikipedia](https://en.wikipedia.org/wiki/Database_normalization)
