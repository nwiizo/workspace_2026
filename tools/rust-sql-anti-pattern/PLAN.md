# Rust + PostgreSQL アンチパターン ブログシリーズ計画

本ドキュメントは、SPECIFICATION.md を実務シナリオ別の5つのブログ記事に再編成する計画を示す。

---

## PostgreSQL固有の解決策：総論

PostgreSQLには他のRDBMSにはない強力な機能があり、多くのアンチパターンをDB側で解決できる：

### 配列型（TEXT[], INTEGER[]）
```sql
-- ジェイウォークの代替案（シンプルなケース向け）
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    tags TEXT[] DEFAULT '{}'  -- 配列型で格納
);

-- 検索はANY演算子で
SELECT * FROM posts WHERE 'rust' = ANY(tags);

-- GINインデックスで高速化
CREATE INDEX idx_posts_tags ON posts USING GIN(tags);
```

### JSONB型
```sql
-- 可変属性をJSONBで格納
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    attributes JSONB DEFAULT '{}'
);

-- JSONBインデックス
CREATE INDEX idx_products_attrs ON products USING GIN(attributes);

-- 属性での検索
SELECT * FROM products WHERE attributes->>'color' = 'red';
SELECT * FROM products WHERE attributes @> '{"size": "L"}';
```

### 生成列（Generated Columns）
```sql
-- 検索用ベクトルを自動生成
CREATE TABLE articles (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    search_vector TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', title || ' ' || body)
    ) STORED
);
```

### EXCLUDE制約
```sql
-- 時間範囲の重複を防ぐ
CREATE TABLE reservations (
    id SERIAL PRIMARY KEY,
    room_id INTEGER NOT NULL,
    period TSTZRANGE NOT NULL,
    EXCLUDE USING GIST (room_id WITH =, period WITH &&)
);
```

### 部分インデックス
```sql
-- アクティブなユーザーのみインデックス
CREATE INDEX idx_users_active_email ON users (email)
WHERE is_active = true;
```

---

## Rust特有の問題：総論

SQLアンチパターンをRustで扱う際には、以下のRust特有の観点を常に意識する必要がある：

### 1. 所有権とライフタイム
- コネクションプールの共有（`Arc<PgPool>`）
- クエリ結果のライフタイム（`'r` in `FromRow<'r, Row>`）
- 借用データを含む構造体の設計

### 2. 型システム
- `Option<T>` によるNULL表現
- カスタム型マッピング（`sqlx::Type`, `Encode`, `Decode`）
- `FromRow` derive マクロの制限
- ジェネリクスと型境界

### 3. 非同期処理
- `async/await` とコネクション管理
- トランザクションのライフタイム
- `tokio::try_join!` による並行クエリ
- ブロッキング処理の回避

### 4. エラーハンドリング
- `sqlx::Error` のバリアント
- DBエラーからドメインエラーへの変換
- `Result`/`Option` の適切な使用
- `.unwrap()` の回避

### 5. コンパイル時チェック
- `query!` vs `query_as!` マクロ
- オフラインモード（CI/CD対応）
- 型安全性とのトレードオフ

---

## ブログ一覧

| # | タイトル | 対象者 | Rust特有の焦点 |
|---|---------|--------|---------------|
| 1 | DB設計の落とし穴 | 新規構築する開発者 | 型マッピング、Option<T>、rust_decimal |
| 2 | パフォーマンス最適化 | 「遅い」に悩む開発者 | async並行処理、Stream、コネクションプール |
| 3 | 複雑なデータ構造 | ドメインモデル設計者 | enum活用、Box/Rc、serde_json |
| 4 | sqlxで安全なSQL | sqlx初心者〜中級者 | FromRow、Type trait、エラーハンドリング |
| 5 | PostgreSQL全文検索 | 検索機能実装者 | カスタム型、検索クエリビルダー |

---

## ブログ1: RustでWebサービスを作る前に知っておきたいDB設計の落とし穴

### メタ情報
- **想定読者**: RustでWebサービスを新規構築する開発者
- **Rust特有の焦点**: 型マッピング、Option<T>の活用、rust_decimalクレート

### Rust特有の問題

#### ジェイウォーク × Rust
```rust
// アンチパターン: String型でカンマ区切りを持つ
struct Post {
    tags: String,  // "rust,web,postgresql"
}

// 問題点:
// 1. パース処理が必要（split(',').collect()）
// 2. 型安全性がない（どんな文字列も入る）
// 3. 所有権の問題（&str vs String の変換コスト）

// 解決策: 交差テーブル + Vec<Tag>
struct PostWithTags {
    post: Post,
    tags: Vec<Tag>,  // 型安全、所有権明確
}
```

#### ラウンディングエラー × Rust
```rust
// アンチパターン: f64で金額を扱う
let price: f64 = 19.99;
let total = price * 3.0;  // 59.970000000000006 になる可能性

// Rust特有の解決策: rust_decimal クレート
use rust_decimal::Decimal;
use std::str::FromStr;

let price = Decimal::from_str("19.99").unwrap();
let total = price * Decimal::from(3);  // 正確に 59.97

// sqlxとの統合: features = ["rust_decimal"] が必要
```

#### サーティワンフレーバー × Rust
```rust
// Rust enumの強み: 網羅性チェック
#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "order_status", rename_all = "lowercase")]
enum OrderStatus {
    Pending,
    Processing,
    Shipped,
    Delivered,
}

// PostgreSQL ENUMとの対応
// CREATE TYPE order_status AS ENUM ('pending', 'processing', 'shipped', 'delivered');

// 問題: PostgreSQL ENUMは変更が困難
// 解決策: 参照テーブル + Rustでのマッピング
```

### 構成

#### 1. ジェイウォーク：カンマ区切りでデータを格納してはいけない
- **SQLの問題**: 検索困難、集約不可、参照整合性なし
- **Rustの問題**:
  - `String::split()` のコスト（アロケーション）
  - `&str` vs `String` の変換
  - パースエラーのハンドリング
- **解決策**: 交差テーブル + `Vec<T>` での表現

#### 2. IDリクワイアド：すべてのテーブルに「id」はいらない
- **Rustの視点**:
  - タプル型 `(PostId, TagId)` で複合キーを表現
  - newtypeパターン: `struct PostId(Uuid)`
  - Copyトレイトの活用

#### 3. キーレスエントリ：外部キー制約を省略してはいけない
- **Rustの視点**:
  - コンパイル時には参照整合性を検証できない
  - 実行時エラー（`sqlx::Error::Database`）の適切なハンドリング
  - 外部キー違反をドメインエラーに変換

#### 4. ラウンディングエラー：金額にFLOATを使ってはいけない
- **Rustの視点**:
  - `f64` の精度問題
  - `rust_decimal::Decimal` クレートの使用
  - sqlx features: `rust_decimal`
  - 整数型（cents単位）との比較

#### 5. サーティワンフレーバー：ENUMの値を後から変えられない
- **Rustの視点**:
  - `#[derive(sqlx::Type)]` でPostgreSQL ENUMにマッピング
  - `FromStr`/`Display` トレイトの実装
  - 参照テーブル方式との比較
  - `serde` との統合

---

## ブログ2: Rust製APIのパフォーマンスを10倍改善するDB最適化

### メタ情報
- **想定読者**: 「なぜか遅い」に悩む開発者
- **Rust特有の焦点**: async並行処理、Stream、コネクションプール管理

### Rust特有の問題

#### N+1問題 × Rust async
```rust
// アンチパターン: ループ内でawait
async fn get_posts_with_authors(pool: &PgPool) -> Result<Vec<PostWithAuthor>> {
    let posts = sqlx::query_as!(Post, "SELECT * FROM posts")
        .fetch_all(pool)
        .await?;

    let mut results = Vec::new();
    for post in posts {
        // 各投稿で1回のクエリ = N+1問題
        let author = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", post.user_id)
            .fetch_one(pool)
            .await?;
        results.push(PostWithAuthor { post, author });
    }
    Ok(results)
}

// 解決策1: JOINで一括取得
// 解決策2: IN句 + HashMap（後述）
// 解決策3: tokio::try_join! で並行実行（注意が必要）
```

#### コネクションプール × Rust
```rust
// コネクション枯渇の問題
let pool = PgPoolOptions::new()
    .max_connections(5)  // 小さすぎると並行処理でブロック
    .acquire_timeout(Duration::from_secs(3))
    .connect(&database_url)
    .await?;

// 長時間トランザクションの問題
let mut tx = pool.begin().await?;  // コネクションを専有
// ... 長い処理 ...
tx.commit().await?;
```

#### Stream vs collect
```rust
// メモリ効率: 大量データの場合
// collect(): 全データをメモリに載せる
let all_posts: Vec<Post> = sqlx::query_as!(Post, "SELECT * FROM posts")
    .fetch_all(pool)  // 100万件を一度にメモリへ
    .await?;

// Stream: 逐次処理（メモリ効率が良い）
use futures::TryStreamExt;
let mut stream = sqlx::query_as!(Post, "SELECT * FROM posts")
    .fetch(pool);  // Streamを返す

while let Some(post) = stream.try_next().await? {
    // 1件ずつ処理
}
```

### 構成

#### 1. N+1クエリ問題
- **SQLの問題**: クエリ数が O(N) になる
- **Rustの問題**:
  - `async` ループでの非効率
  - コネクションプールの枯渇リスク
- **解決策**:
  - JOINで一括取得
  - `HashMap` でのルックアップ
  - `tokio::try_join!` の適切な使用

#### 2. インデックスショットガン
- **Rustの視点**:
  - EXPLAINの結果をパース（`Row` から `String` へ）
  - パフォーマンス測定: `std::time::Instant`

#### 3. スパゲッティクエリ
- **Rustの問題**:
  - 複雑な型マッピング（多数のカラム）
  - `FromRow` の制限
- **解決策**:
  - クエリ分割 + `tokio::try_join!`
  - CTE（WITH句）の活用

#### 4. アンビギュアスグループ
- **Rustの視点**:
  - ウィンドウ関数の結果型マッピング
  - `DISTINCT ON` の活用

#### 5. ランダムセレクション
- **Rustの視点**:
  - `rand` クレートとの統合
  - TABLESAMPLE の型マッピング

---

## ブログ3: Rustで複雑なデータ構造を扱う：階層・ポリモーフィック・可変属性

### メタ情報
- **想定読者**: 「このデータ構造どう設計すれば？」と悩む開発者
- **Rust特有の焦点**: enum活用、Box/Rc、serde_json統合

### Rust特有の問題

#### 階層構造 × Rust
```rust
// Rustでの再帰的データ構造
struct Comment {
    id: i32,
    content: String,
    children: Vec<Comment>,  // 所有権が明確
}

// Box が必要なケース（自己参照型）
struct TreeNode {
    value: i32,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
}

// DBから取得したフラットデータを階層構造に変換
fn build_tree(flat: Vec<FlatComment>) -> Vec<Comment> {
    // parent_id でグループ化してツリーを構築
    // HashMap を使った効率的な実装
}
```

#### ポリモーフィック関連 × Rust enum
```rust
// Rustのenumでポリモーフィック関連を型安全に表現
#[derive(Debug)]
enum Commentable {
    Article(ArticleId),
    Video(VideoId),
    Product(ProductId),
}

struct Comment {
    id: CommentId,
    target: Commentable,  // 型安全！
    body: String,
}

// DBからの取得
impl Comment {
    async fn find_by_target(pool: &PgPool, target: &Commentable) -> Result<Vec<Self>> {
        match target {
            Commentable::Article(id) => /* article_comments テーブルから */,
            Commentable::Video(id) => /* video_comments テーブルから */,
            Commentable::Product(id) => /* product_comments テーブルから */,
        }
    }
}
```

#### EAV × serde_json
```rust
// アンチパターン: EAVテーブル
// attribute_name: "color", attribute_value: "red"
// → 型情報が失われる

// Rustでの解決策: serde_json::Value + 型変換
#[derive(Debug, Serialize, Deserialize)]
struct ProductAttributes {
    color: Option<String>,
    size: Option<String>,
    weight: Option<f64>,
}

struct Product {
    id: i32,
    name: String,
    attributes: ProductAttributes,  // JSONBからデシリアライズ
}

// sqlxでの取得
let product: Product = sqlx::query_as!(
    Product,
    r#"SELECT id, name, attributes as "attributes: Json<ProductAttributes>" FROM products"#
)
.fetch_one(pool)
.await?;
```

### 構成

#### 1. ナイーブツリー
- **SQLの問題**: 再帰クエリの必要性
- **Rustの問題**:
  - 再帰的データ構造（`Box<T>`, `Rc<T>`）
  - フラットデータからツリー構築のアルゴリズム
  - 所有権とライフタイムの管理
- **解決策**:
  - `WITH RECURSIVE` + Rustでの変換
  - 閉包テーブル方式

#### 2. ポリモーフィック関連
- **Rustの強み**: `enum` による型安全な表現
- **実装パターン**:
  - 参照先ごとの交差テーブル
  - 共通基底テーブル
  - Rust `enum` でのディスパッチ

#### 3. EAV
- **Rustの問題**:
  - 動的な属性 vs 静的型付け
  - `serde_json::Value` の型安全性
- **解決策**:
  - JSONB + 構造体へのデシリアライズ
  - バリデーション層の設計

#### 4. マルチカラムアトリビュート
- **Rustの視点**:
  - `Vec<T>` での表現
  - 正規化 vs 配列型（`Vec<String>` ↔ `TEXT[]`）

---

## ブログ4: sqlxで安全なSQLを書く：NULL・型・クエリの罠を避ける

### メタ情報
- **想定読者**: sqlxを使い始めた開発者
- **Rust特有の焦点**: FromRow、Type trait、エラーハンドリング

### Rust特有の問題

#### Option<T> と NULL
```rust
// Rustの強み: NULLは Option<T> で明示的に表現
#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
    email: Option<String>,  // NULLable
    bio: Option<String>,    // NULLable
}

// クエリパラメータでの NULL 処理
let email_filter: Option<&str> = Some("alice@example.com");
sqlx::query_as!(
    User,
    r#"SELECT * FROM users WHERE ($1::text IS NULL OR email = $1)"#,
    email_filter
)
```

#### FromRow の制限
```rust
// 問題: 複雑な型変換
#[derive(FromRow)]
struct Post {
    id: i32,
    title: String,
    // created_at: DateTime<Utc>,  // 特別な処理が必要
}

// 解決策: query_as! マクロで型を明示
let post = sqlx::query_as!(
    Post,
    r#"SELECT id, title, created_at as "created_at: DateTime<Utc>" FROM posts"#
)
.fetch_one(pool)
.await?;
```

#### エラーハンドリング
```rust
// アンチパターン: unwrap() の乱用
let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
    .fetch_one(pool)
    .await
    .unwrap();  // パニックする可能性

// 解決策: 適切なエラー変換
#[derive(Error, Debug)]
enum AppError {
    #[error("User not found: {0}")]
    UserNotFound(i32),
    #[error("Database error")]
    Database(#[from] sqlx::Error),
}

async fn get_user(pool: &PgPool, user_id: i32) -> Result<User, AppError> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::UserNotFound(user_id))
}
```

### 構成

#### 1. NULL処理
- **SQLの問題**: 三値論理、`= NULL` が動かない
- **Rustの強み**: `Option<T>` による明示的表現
- **実践**:
  - `COALESCE` の活用
  - 条件付きフィルタの設計パターン

#### 2. SELECT * を避ける
- **Rustの問題**:
  - `FromRow` がカラム順序に依存
  - スキーマ変更でコンパイルエラー
- **解決策**: 明示的なカラム指定

#### 3. SQLインジェクション対策
- **Rustの強み**: プリペアドステートメントがデフォルト
- **注意点**:
  - 動的なテーブル名/カラム名
  - ORDER BY の動的指定

#### 4. エラーハンドリング
- **Rustの強み**: `Result<T, E>` による明示的なエラー処理
- **実践**:
  - `sqlx::Error` のパターンマッチ
  - ドメインエラーへの変換
  - トランザクションのロールバック

#### 5. 型変換
- **sqlxの機能**:
  - `query!` vs `query_as!`
  - カスタム型の実装（`Type`, `Encode`, `Decode`）
- **具体例**:
  - `Uuid`
  - `DateTime<Utc>`
  - `Decimal`
  - `Json<T>`

---

## ブログ5: PostgreSQLの全文検索機能をRustで使いこなす

### メタ情報
- **想定読者**: 検索機能を実装したい開発者
- **Rust特有の焦点**: カスタム型、検索クエリビルダー、ランキング

### Rust特有の問題

#### tsvector型のRustでの扱い
```rust
// PostgreSQLのtsvector型をRustで扱う
// sqlxはtsvectorを直接サポートしていない

// 解決策1: 文字列として取得
let result: (String,) = sqlx::query_as(
    "SELECT search_vector::text FROM articles WHERE id = $1"
)
.bind(article_id)
.fetch_one(pool)
.await?;

// 解決策2: 検索時は直接使用（取得は不要）
let articles = sqlx::query_as!(
    Article,
    r#"
    SELECT id, title, body, ts_rank(search_vector, query) as rank
    FROM articles, plainto_tsquery('english', $1) query
    WHERE search_vector @@ query
    ORDER BY rank DESC
    "#,
    search_term
)
.fetch_all(pool)
.await?;
```

#### 検索クエリビルダー
```rust
// 型安全な検索クエリビルダー
struct SearchQuery {
    terms: Vec<String>,
    must_include: Vec<String>,
    must_exclude: Vec<String>,
}

impl SearchQuery {
    fn to_tsquery(&self) -> String {
        // 安全にtsqueryを構築
        let parts: Vec<String> = self.terms
            .iter()
            .map(|t| format!("{}:*", t))  // 前方一致
            .collect();
        parts.join(" | ")
    }
}
```

### 構成

#### 1. LIKEの限界
- **SQLの問題**: インデックスが効かない、ランキングなし
- **Rustの視点**: 文字列操作のコスト

#### 2. tsvector/tsquery
- **Rustでの課題**:
  - tsvector型のマッピング
  - 動的なtsqueryの安全な構築
- **実装パターン**:
  - search_vectorカラムの設計
  - ランキング結果の取得

#### 3. pg_trgm
- **Rustの視点**:
  - similarity関数の結果型
  - あいまい検索のしきい値設定

#### 4. 日本語検索
- **課題**: デフォルトパーサーの限界
- **解決策**:
  - pg_trgm（N-gram方式）
  - pg_bigm
  - 外部形態素解析

#### 5. 外部検索エンジン連携
- **Rustの視点**:
  - Meilisearch/Elasticsearch Rustクライアント
  - 非同期インデックス更新
  - PostgreSQLとのデータ同期

---

## まとめ: Rust特有の問題への対処法

| 問題 | 対処法 |
|-----|-------|
| NULL表現 | `Option<T>` を積極的に使う |
| 型マッピング | `query_as!` で型を明示 |
| エラー処理 | `Result` + カスタムエラー型 |
| 非同期処理 | `tokio::try_join!` でI/Oを並行化 |
| 大量データ | `Stream` で逐次処理 |
| コネクション管理 | プールサイズの適切な設定 |
| 型安全性 | `sqlx::Type` / `FromRow` の実装 |
| ポリモーフィズム | Rust `enum` で型安全に表現 |
