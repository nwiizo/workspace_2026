# 27ms vs 1.5ms：N+1クエリ撲滅の実測データ

## はじめに

50件の記事を取得して著者名を表示する。それだけの処理に27msかかっていた。JOINに書き換えたら1.5msになった。18倍の差だ。

差の正体はN+1クエリ問題だった。記事を1回のクエリで取得した後、著者を記事ごとに1回ずつ取得していた。50件の記事なら51回のクエリが発行される。ネットワークラウンドトリップが50回余計に発生する。

本記事ではN+1問題の検出から解決までを実測データとともに比較する。JOINが常に最適とは限らない。状況に応じた選択肢を持っておくと役に立つ。

## 問題：N+1クエリとは何か

「1回のクエリで親データを取得し、N回のクエリで関連データを取得する」パターンをN+1クエリと呼ぶ。

```rust
// アンチパターン: 51回のクエリが発行される
let articles: Vec<Article> = sqlx::query_as("SELECT * FROM articles")
    .fetch_all(&pool).await?;  // 1回目

for article in &articles {
    // 50回のクエリ（記事ごとに1回）
    let author: Author = sqlx::query_as("SELECT * FROM authors WHERE id = $1")
        .bind(article.author_id)
        .fetch_one(&pool).await?;
    println!("{} by {}", article.title, author.name);
}
```

問題はクエリ数だけではない。各クエリにはネットワークラウンドトリップのオーバーヘッドがある。ローカル環境では0.5〜1ms、クラウド環境では2〜5msかかることもある。50回なら25〜250msのオーバーヘッドだ。

## 解決策の比較

| 方法 | クエリ数 | 実測時間 | 改善率 | 向いている場面 |
|------|---------|----------|--------|---------------|
| N+1（アンチパターン） | 51 | 27.95ms | - | なし |
| JOIN | 1 | 1.51ms | 18.5倍 | 1対1の関連 |
| IN句 + HashMap | 2 | 2.34ms | 11.9倍 | 1対多の関連 |
| DataLoader（キャッシュ） | 0 | 0.013ms | 2,150倍 | 同一リクエスト内で重複 |
| json_agg | 1 | 1.89ms | 14.8倍 | ネストしたJSONが欲しい |

実測環境: PostgreSQL 16, macOS, localhost接続

## 解決策1：JOIN

最もシンプルな解決策。1回のクエリで関連データを一括取得する。

```rust
// 1回のクエリで完結
let results: Vec<(i32, String, String)> = sqlx::query_as(
    r#"
    SELECT a.id, a.title, au.name as author_name
    FROM articles a
    JOIN authors au ON a.author_id = au.id
    "#
)
.fetch_all(&pool).await?;

for (id, title, author_name) in &results {
    println!("{}: {} by {}", id, title, author_name);
}
```

JOINの利点は明確だ。1回のクエリで済む。実行計画も最適化されやすい。

ただし注意点がある。1対多の関連をJOINすると、親データが重複して返される。記事1件にコメント10件なら、同じ記事が10行返される。データ転送量が増え、Rust側での重複排除も必要になる。

## 解決策2：IN句 + HashMap

1対多の関連には、IN句で一括取得してHashMapで紐づける方法が適している。

```rust
// 1. 記事を取得
let articles: Vec<Article> = sqlx::query_as("SELECT * FROM articles")
    .fetch_all(&pool).await?;

// 2. 著者IDを集める
let author_ids: Vec<i32> = articles.iter().map(|a| a.author_id).collect();

// 3. IN句で一括取得（ANY($1)はPostgreSQL固有の書き方）
let authors: Vec<(i32, String)> = sqlx::query_as(
    "SELECT id, name FROM authors WHERE id = ANY($1)"
)
.bind(&author_ids)
.fetch_all(&pool).await?;

// 4. HashMapで高速ルックアップ
let author_map: HashMap<i32, String> = authors.into_iter().collect();

for article in &articles {
    let author_name = author_map.get(&article.author_id).map(|s| s.as_str()).unwrap_or("unknown");
    println!("{} by {}", article.title, author_name);
}
```

クエリは2回だが、JOINより遅くなるのはなぜか。ネットワークラウンドトリップが1回増えるからだ。ただし1対多の関連では、JOINによるデータ重複を避けられるメリットがある。

コメントを取得する場合を考えてみる。

```rust
// 記事50件、各記事にコメント10件の場合

// JOINだと500行が返される（記事×コメント）
let rows = sqlx::query_as(
    "SELECT a.*, c.* FROM articles a LEFT JOIN comments c ON a.id = c.article_id"
).fetch_all(&pool).await?;  // 500行

// IN句なら記事50行 + コメント500行 = 550行（重複なし）
let articles = sqlx::query_as("SELECT * FROM articles").fetch_all(&pool).await?;  // 50行
let comments = sqlx::query_as("SELECT * FROM comments WHERE article_id = ANY($1)")
    .bind(&article_ids).fetch_all(&pool).await?;  // 500行
```

## 解決策3：DataLoader

GraphQLでよく使われるパターン。同一リクエスト内で同じデータを複数回参照する場合、キャッシュで重複リクエストを排除する。

```rust
struct AuthorLoader {
    pool: PgPool,
    cache: RwLock<HashMap<i32, String>>,
}

impl AuthorLoader {
    async fn load_many(&self, ids: &[i32]) -> Result<HashMap<i32, String>, sqlx::Error> {
        // キャッシュにないIDだけをDBから取得
        let cache = self.cache.read().await;
        let missing: Vec<i32> = ids.iter()
            .filter(|id| !cache.contains_key(id))
            .copied().collect();
        drop(cache);

        if !missing.is_empty() {
            let authors: Vec<(i32, String)> = sqlx::query_as(
                "SELECT id, name FROM authors WHERE id = ANY($1)"
            )
            .bind(&missing)
            .fetch_all(&self.pool).await?;

            let mut cache = self.cache.write().await;
            for (id, name) in authors {
                cache.insert(id, name);
            }
        }

        // キャッシュから結果を構築
        let cache = self.cache.read().await;
        Ok(ids.iter()
            .filter_map(|id| cache.get(id).map(|name| (*id, name.clone())))
            .collect())
    }
}
```

DataLoaderは2回目以降の呼び出しでキャッシュヒットする。同じ著者の記事を複数回参照する場合に効果的だ。ただし、リクエストをまたいでキャッシュを共有すると、データの古さが問題になる。通常はリクエストスコープでキャッシュをクリアする。

## 解決策4：json_agg

ネストしたJSONを返したい場合、PostgreSQLの`json_agg`を使うとクライアント側でのデータ整形が不要になる。

```rust
#[derive(Debug, Serialize, Deserialize)]
struct CommentJson {
    id: i32,
    body: String,
}

let results: Vec<(i32, String, serde_json::Value)> = sqlx::query_as(
    r#"
    SELECT
        a.id,
        a.title,
        COALESCE(
            json_agg(
                json_build_object('id', c.id, 'body', c.body)
                ORDER BY c.created_at DESC
            ) FILTER (WHERE c.id IS NOT NULL),
            '[]'
        ) as comments
    FROM articles a
    LEFT JOIN comments c ON a.id = c.article_id
    GROUP BY a.id, a.title
    "#
)
.fetch_all(&pool).await?;

for (id, title, comments_json) in &results {
    let comments: Vec<CommentJson> = serde_json::from_value(comments_json.clone())?;
    println!("記事 {}: {} ({} コメント)", id, title, comments.len());
}
```

`FILTER (WHERE c.id IS NOT NULL)`がないと、コメントがない記事で`[null]`が返される。`COALESCE(..., '[]')`で空配列にフォールバックしている。

## N+1の検出方法

問題を解決するには、まず検出できなければならない。

### 開発中：tracingでログ出力

sqlxはtracingクレートと統合されている。`RUST_LOG=sqlx=debug`で全クエリをログ出力できる。

```sh
RUST_LOG=sqlx=debug cargo run
```

同じパターンのクエリが大量に出力されていたらN+1の可能性がある。

### 本番環境：pg_stat_statements

PostgreSQLの`pg_stat_statements`拡張で、頻繁に実行されるクエリを特定できる。

```sql
SELECT calls, LEFT(query, 80) as query_preview
FROM pg_stat_statements
WHERE calls > 10
ORDER BY calls DESC
LIMIT 10;
```

同じパターンのクエリが1000回以上呼ばれていたら、N+1を疑う。

### コードレビュー：ループ内の.awaitを探す

最も確実な検出方法。ループ内で`.await`してDBアクセスしていたらN+1の可能性がある。

```rust
// ❌ 危険なパターン
for item in items {
    let x = sqlx::query!(...).fetch_one(&pool).await?;  // ループ内await
}

// ✅ 安全なパターン
let ids: Vec<_> = items.iter().map(|i| i.id).collect();
let results = sqlx::query!("... WHERE id = ANY($1)", &ids)
    .fetch_all(&pool).await?;
```

## どの解決策を選ぶべきか

```
1対1の関連？
├─ YES → JOIN
└─ NO → 1対多の関連？
         ├─ 同一リクエスト内で重複参照がある？
         │   ├─ YES → DataLoader
         │   └─ NO → IN句 + HashMap
         └─ ネストしたJSONが欲しい？
             ├─ YES → json_agg
             └─ NO → IN句 + HashMap
```

迷ったらIN句 + HashMapを選ぶ。汎用性が高く、理解しやすい。JOINは1対1の関連でのみ使い、1対多の場合はデータ重複に注意する。

## 補足：インデックスの基本

N+1を解決しても、インデックスがなければ遅い。IN句で50件のIDを検索するとき、インデックスがなければ50回のSeq Scan（全件走査）が発生する。

```sql
-- 外部キー列にはインデックスを作成
CREATE INDEX idx_articles_author_id ON articles(author_id);

-- 頻繁に使うWHERE条件には複合インデックス
CREATE INDEX idx_articles_author_status ON articles(author_id, status);

-- 一部のデータのみ検索する場合は部分インデックス
CREATE INDEX idx_articles_published ON articles(author_id) WHERE status = 'published';
```

PostgreSQLは外部キー列に自動でインデックスを作成しない。明示的に作成する必要がある。

## まとめ

冒頭の27msは、JOINに書き換えて1.5msになった。18倍の差は、51回のネットワークラウンドトリップが1回になった結果だ。

N+1問題の解決策をまとめる:

1. **JOIN**: 1対1の関連に最適。1対多ではデータ重複に注意
2. **IN句 + HashMap**: 1対多の関連に汎用的。2クエリで済む
3. **DataLoader**: 同一リクエスト内での重複参照を排除
4. **json_agg**: ネストしたJSONが欲しい場合。GROUP BYと併用

検出方法は3つ:
- 開発中: `RUST_LOG=sqlx=debug`
- 本番: `pg_stat_statements`
- コードレビュー: ループ内の`.await`を探す

どの解決策もトレードオフがある。JOINは1クエリだがデータが重複する。IN句は2クエリだが重複がない。状況に応じて選ぶ。迷ったらIN句 + HashMapから始めるのが無難だ。

## 実行可能なデモコード

本記事のコードは以下で実行できる:

```sh
cd blog_02_performance_optimization
cargo run
```

## 参考資料

- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
- [PostgreSQL - pg_stat_statements](https://www.postgresql.org/docs/current/pgstatstatements.html)
- [GraphQL DataLoader](https://github.com/graphql/dataloader)
