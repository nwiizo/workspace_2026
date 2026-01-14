# EXPLAIN ANALYZEの読み方：Seq Scanが消える瞬間を見る

## 仮説

「インデックスを追加すればSeq Scanが消えて速くなるはず」

これは正しいこともあれば、正しくないこともある。本記事ではEXPLAIN ANALYZEの出力を読み解き、クエリのボトルネックを特定する方法を実験形式で示す。

## 実験環境

- PostgreSQL 16
- 100ユーザー × 10注文 = 1,000注文
- 各注文に3つの明細 = 3,000明細

```rust
// サンプルデータの投入
for i in 0..100 {
    sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
        .bind(format!("User {}", i))
        .bind(format!("user{}@example.com", i))
        .execute(&pool).await?;
}

// 統計情報を更新（重要）
sqlx::query("ANALYZE users").execute(&pool).await?;
sqlx::query("ANALYZE orders").execute(&pool).await?;
sqlx::query("ANALYZE order_items").execute(&pool).await?;
```

## 実験1：EXPLAIN vs EXPLAIN ANALYZE

まず2つのコマンドの違いを確認する。

```sql
-- EXPLAIN：推定のみ（実行しない）
EXPLAIN SELECT * FROM users WHERE email = 'user0@example.com';
```

```
Index Scan using users_email_key on users  (cost=0.14..8.16 rows=1 width=64)
  Index Cond: (email = 'user0@example.com'::text)
```

`cost=0.14..8.16`は推定コスト。`rows=1`は推定行数。実際の実行時間は不明。

```sql
-- EXPLAIN ANALYZE：実際に実行して計測
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT * FROM users WHERE email = 'user0@example.com';
```

```
Index Scan using users_email_key on users  (cost=0.14..8.16 rows=1 width=64)
                                           (actual time=0.015..0.016 rows=1 loops=1)
  Index Cond: (email = 'user0@example.com'::text)
  Buffers: shared hit=2
Planning Time: 0.045 ms
Execution Time: 0.027 ms
```

`actual time=0.015..0.016`が実測時間（ミリ秒）。`rows=1`が実際の行数。`loops=1`は実行回数。`Buffers: shared hit=2`はメモリから読み取ったページ数。

```rust
async fn explain_analyze_query(pool: &PgPool, query: &str) -> Result<String> {
    let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", query);
    let rows: Vec<(String,)> = sqlx::query_as(&explain_query)
        .fetch_all(pool).await?;
    let plan = rows.iter()
        .map(|(line,)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(plan)
}
```

## 実験2：Seq Scan vs Index Scan

### インデックスなしの状態

```sql
-- インデックスがない
SELECT * FROM orders WHERE status = 'pending';
```

```
Seq Scan on orders  (cost=0.00..22.00 rows=250 width=80)
                    (actual time=0.008..0.125 rows=250 loops=1)
  Filter: (status = 'pending'::text)
  Rows Removed by Filter: 750
  Buffers: shared hit=12
```

`Seq Scan`（順次スキャン）でテーブル全体を読み取り、`Filter`で絞り込んでいる。`Rows Removed by Filter: 750`は捨てられた行数。

### インデックスを追加

```sql
CREATE INDEX idx_orders_status ON orders(status);
```

```
Index Scan using idx_orders_status on orders  (cost=0.28..12.68 rows=250 width=80)
                                              (actual time=0.015..0.065 rows=250 loops=1)
  Index Cond: (status = 'pending'::text)
  Buffers: shared hit=5
```

`Index Scan`に変わり、`Buffers`が12→5に減少。必要なページだけを読み取るようになった。

```rust
// インデックス追加前後の比較
let plan_before = explain_analyze_query(&pool, query).await?;
let has_seq_scan_before = plan_before.contains("Seq Scan");

sqlx::query("CREATE INDEX idx_orders_status ON orders(status)")
    .execute(&pool).await?;

let plan_after = explain_analyze_query(&pool, query).await?;
let has_seq_scan_after = plan_after.contains("Seq Scan");

println!("Before: Seq Scan = {}", has_seq_scan_before);  // true
println!("After: Seq Scan = {}", has_seq_scan_after);    // false
```

## 実験3：Index Only Scan

Index ScanよりさらにIndex Only Scanが速い。テーブルにアクセスせず、インデックスだけで完結する。

### 条件

1. SELECTするカラムが全てインデックスに含まれている
2. Visibility Mapが更新されている（VACUUM後）

```sql
-- カバリングインデックスを作成
CREATE INDEX idx_orders_status_total ON orders(status) INCLUDE (total);

-- VACUUMでVisibility Mapを更新
VACUUM orders;
```

```sql
SELECT status, total FROM orders WHERE status = 'pending';
```

```
Index Only Scan using idx_orders_status_total on orders
  (cost=0.28..12.68 rows=250 width=16)
  (actual time=0.012..0.045 rows=250 loops=1)
  Index Cond: (status = 'pending'::text)
  Heap Fetches: 0
  Buffers: shared hit=3
```

`Index Only Scan`になり、`Heap Fetches: 0`でテーブルへのアクセスがゼロ。`Buffers`も5→3に減少。

```rust
sqlx::query("CREATE INDEX idx_orders_status_total ON orders(status) INCLUDE (total)")
    .execute(&pool).await?;

sqlx::query("VACUUM orders")
    .execute(&pool).await?;

let plan = explain_analyze_query(
    &pool,
    "SELECT status, total FROM orders WHERE status = 'pending'"
).await?;

if plan.contains("Index Only Scan") {
    println!("Success: Index Only Scan is being used!");
}
```

## 実験4：JOIN方式の違い

PostgreSQLは状況に応じてJOIN方式を選択する。

### Nested Loop

少ない行数をJOINする場合に選ばれる。

```sql
SELECT o.*, u.name
FROM orders o
JOIN users u ON o.user_id = u.id
WHERE o.id = (SELECT id FROM orders LIMIT 1);
```

```
Nested Loop  (cost=0.57..16.61 rows=1 width=96)
             (actual time=0.025..0.028 rows=1 loops=1)
  ->  Index Scan using orders_pkey on orders o  (cost=0.28..8.30 rows=1 width=80)
        Index Cond: (id = $0)
  ->  Index Scan using users_pkey on users u  (cost=0.14..8.16 rows=1 width=16)
        Index Cond: (id = o.user_id)
```

外側のループ（orders）から1行ずつ取り出し、内側（users）をインデックスで検索する。

### Hash Join

大きい結果セットをJOINする場合に選ばれる。

```sql
SELECT o.id, u.name
FROM orders o
JOIN users u ON o.user_id = u.id
WHERE o.status = 'pending';
```

```
Hash Join  (cost=3.25..27.43 rows=250 width=32)
           (actual time=0.065..0.180 rows=250 loops=1)
  Hash Cond: (o.user_id = u.id)
  ->  Index Scan using idx_orders_status on orders o  (cost=0.28..19.18 rows=250 width=32)
        Index Cond: (status = 'pending'::text)
  ->  Hash  (cost=1.97..1.97 rows=100 width=16)
        Buckets: 1024  Batches: 1  Memory Usage: 13kB
        ->  Seq Scan on users u  (cost=0.00..1.97 rows=100 width=16)
```

usersテーブル全体をハッシュテーブルに読み込み、ordersを走査しながらハッシュで結合する。

## 実験5：推定行数と実際の行数

PostgreSQLは統計情報を使って行数を推定する。推定が大きくずれると、非効率なプランが選ばれる。

```rust
async fn check_estimation_accuracy(pool: &PgPool, query: &str) -> Result<()> {
    let plans = explain_analyze_json(pool, query).await?;

    if let Some(plan) = plans.first() {
        let estimated = plan.plan.plan_rows.unwrap_or(0);
        let actual = plan.plan.actual_rows.unwrap_or(0);
        let ratio = if estimated > 0 {
            actual as f64 / estimated as f64
        } else {
            0.0
        };

        println!("Estimated rows: {}", estimated);
        println!("Actual rows: {}", actual);
        println!("Ratio: {:.2}", ratio);

        if !(0.1..=10.0).contains(&ratio) {
            println!("WARNING: Large estimation error!");
        }
    }
    Ok(())
}
```

推定と実際の比率が0.1〜10倍の範囲を外れたら警告。大きくずれている場合は`ANALYZE`を実行して統計情報を更新する。

```sql
-- 統計情報を更新
ANALYZE orders;
```

## 実験6：JSON形式での解析

プログラムで実行計画を解析するにはJSON形式が便利。

```rust
#[derive(Debug, Deserialize)]
struct ExplainPlan {
    #[serde(rename = "Plan")]
    plan: PlanNode,
    #[serde(rename = "Planning Time")]
    planning_time: Option<f64>,
    #[serde(rename = "Execution Time")]
    execution_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct PlanNode {
    #[serde(rename = "Node Type")]
    node_type: String,
    #[serde(rename = "Actual Rows")]
    actual_rows: Option<i64>,
    #[serde(rename = "Actual Total Time")]
    actual_total_time: Option<f64>,
    #[serde(rename = "Plans")]
    plans: Option<Vec<PlanNode>>,
}

async fn explain_analyze_json(pool: &PgPool, query: &str) -> Result<Vec<ExplainPlan>> {
    let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {}", query);
    let row: (serde_json::Value,) = sqlx::query_as(&explain_query)
        .fetch_one(pool).await?;
    let plans: Vec<ExplainPlan> = serde_json::from_value(row.0)?;
    Ok(plans)
}
```

```rust
let plans = explain_analyze_json(&pool, query).await?;
if let Some(plan) = plans.first() {
    println!("Node Type: {}", plan.plan.node_type);
    println!("Execution Time: {:?} ms", plan.execution_time);
    println!("Actual Rows: {:?}", plan.plan.actual_rows);
}
```

## 実験7：問題の自動検出

実行計画から問題を自動検出する関数を作る。

```rust
#[derive(Debug)]
struct QueryAnalysis {
    has_seq_scan: bool,
    has_disk_sort: bool,
    execution_time_ms: Option<f64>,
    planning_time_ms: Option<f64>,
}

fn analyze_plan(plan: &str) -> QueryAnalysis {
    QueryAnalysis {
        has_seq_scan: plan.contains("Seq Scan"),
        has_disk_sort: plan.contains("external merge") || plan.contains("external sort"),
        execution_time_ms: extract_execution_time(plan),
        planning_time_ms: extract_planning_time(plan),
    }
}

fn extract_execution_time(plan: &str) -> Option<f64> {
    for line in plan.lines() {
        if line.contains("Execution Time:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let time_str = parts[1].trim().replace(" ms", "");
                return time_str.parse().ok();
            }
        }
    }
    None
}
```

```rust
let plan = explain_analyze_query(&pool, query).await?;
let analysis = analyze_plan(&plan);

if analysis.has_seq_scan {
    println!("Recommendation: Consider adding an index");
}

if analysis.has_disk_sort {
    println!("Recommendation: Increase work_mem or add index for ORDER BY");
}
```

## 読み方のチェックリスト

EXPLAIN ANALYZE出力を見るときのポイント。

| 項目 | 良い兆候 | 悪い兆候 |
|------|---------|---------|
| スキャン方式 | Index Scan, Index Only Scan | Seq Scan（大きいテーブルで） |
| 行数推定 | actual ≒ estimated | 10倍以上のずれ |
| Buffers | shared hit のみ | read（ディスクI/O）が多い |
| Sort | メモリ内 | external merge/sort |
| Heap Fetches | 0（Index Only Scan） | 多い |

## まとめ

EXPLAIN ANALYZEはクエリのボトルネックを特定する最も確実な方法だ。

1. **EXPLAINだけでなくANALYZEをつける**: 推定ではなく実測値を見る
2. **Seq Scanを疑う**: 大きいテーブルでSeq Scanは遅い
3. **推定と実際の差を確認**: 大きくずれていたらANALYZEを実行
4. **Index Only Scanを狙う**: カバリングインデックス + VACUUM
5. **BUFFERSオプション**: ディスクI/Oの有無を確認

インデックスを追加してもSeq Scanが消えないこともある。行数が少ない場合、PostgreSQLはSeq Scanの方が効率的と判断することがある。EXPLAIN ANALYZEで確認しながら最適化を進める。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_21_explain_analyze
cargo run
```

## 参考資料

- [PostgreSQL - EXPLAIN](https://www.postgresql.org/docs/current/sql-explain.html)
- [PostgreSQL - Using EXPLAIN](https://www.postgresql.org/docs/current/using-explain.html)
- [PostgreSQL - Index Only Scans](https://www.postgresql.org/docs/current/indexes-index-only-scans.html)
