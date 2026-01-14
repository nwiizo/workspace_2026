# sqlx派 vs Diesel派：7つの観点で決着をつける

## はじめに

「sqlxとDiesel、どっちを使うべきですか？」

Rustでデータベースを扱うとき、必ず出る質問だ。どちらも優れたライブラリだが、設計思想が異なる。sqlxはSQL直書き派、DieselはDSL（Domain Specific Language）派だ。

本記事では7つの観点から両者を比較する。最後に「こういう時はこっち」という判断基準を示す。

## 比較表

| 観点 | sqlx | Diesel |
|------|------|--------|
| クエリ記述 | SQLそのまま | Rust DSL |
| コンパイル時チェック | 要DB接続 | スキーマファイルから |
| async対応 | ネイティブ | diesel-async経由 |
| 学習コスト | SQL知識があれば低い | DSL習得が必要 |
| 複雑なクエリ | SQL直書きで柔軟 | DSLの制約あり |
| マイグレーション | sqlx-cli | diesel-cli |
| エコシステム | 新しいが活発 | 成熟している |

## 1. クエリ記述：SQL vs DSL

### sqlx：SQLをそのまま書く

```rust
let posts: Vec<Post> = sqlx::query_as(
    r#"
    SELECT p.id, p.title, p.content, p.created_at
    FROM posts p
    INNER JOIN post_tags pt ON p.id = pt.post_id
    INNER JOIN tags t ON pt.tag_id = t.id
    WHERE t.slug = $1
    ORDER BY p.created_at DESC
    "#
)
.bind("rust")
.fetch_all(&pool).await?;
```

SQL経験者にはそのまま読める。複雑なJOINやサブクエリもそのまま書ける。

### Diesel：Rust DSLで組み立てる

```rust
let posts = posts::table
    .inner_join(post_tags::table.inner_join(tags::table))
    .filter(tags::slug.eq("rust"))
    .order(posts::created_at.desc())
    .select(Post::as_select())
    .load(&mut conn)?;
```

Rustの型システムを活用。IDEの補完が効く。ただしDSLの文法を覚える必要がある。

### 判定

- **SQL習得済みで複雑なクエリが多い** → sqlx
- **型安全を重視、標準的なCRUDが中心** → Diesel

## 2. コンパイル時チェック

### sqlx：DB接続が必要

sqlxはコンパイル時に実際のDBに接続してクエリを検証する。

```rust
// query_as!マクロでコンパイル時チェック
let users = sqlx::query_as!(
    User,
    "SELECT id, name, email FROM users WHERE status = $1",
    "active"
)
.fetch_all(&pool).await?;
```

```sh
# オフラインモード用にクエリ情報を保存
cargo sqlx prepare
```

`cargo sqlx prepare`でクエリ情報をJSONファイルに保存すれば、CIでDB接続なしでもビルドできる。

### Diesel：スキーマファイルから生成

Dieselは`diesel print-schema`で生成したschema.rsからチェックする。

```rust
// schema.rs（自動生成）
table! {
    users (id) {
        id -> Uuid,
        name -> Varchar,
        email -> Varchar,
        status -> Varchar,
    }
}
```

DB接続は不要だが、スキーマ変更時は再生成が必要。

### 判定

- **CI/CDでDB接続が難しい** → Diesel
- **最新のスキーマとの整合性を保証したい** → sqlx（prepare使用）

## 3. 非同期（async）対応

### sqlx：ネイティブasync

```rust
// asyncが自然に書ける
let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts")
    .fetch_all(&pool).await?;

// ストリーミングも可能
let mut stream = sqlx::query_as::<_, Post>("SELECT * FROM posts")
    .fetch(&pool);

while let Some(post) = stream.try_next().await? {
    println!("{}", post.title);
}
```

### Diesel：diesel-asyncが必要

```rust
use diesel_async::RunQueryDsl;

let posts = posts::table
    .load::<Post>(&mut conn)
    .await?;
```

`diesel-async`クレートを使えばasyncに対応できるが、追加の依存が必要。

### 判定

- **Tokioベースのasyncアプリケーション** → sqlx
- **同期処理で十分、またはblocking使用** → Diesel

## 4. 複雑なクエリ

### sqlx：何でも書ける

再帰CTEもウィンドウ関数もそのまま書ける。

```rust
let categories: Vec<CategoryWithPath> = sqlx::query_as(
    r#"
    WITH RECURSIVE category_tree AS (
        SELECT id, name, 0 as depth, name as path
        FROM categories WHERE parent_id IS NULL

        UNION ALL

        SELECT c.id, c.name, ct.depth + 1, ct.path || ' > ' || c.name
        FROM categories c
        JOIN category_tree ct ON c.parent_id = ct.id
    )
    SELECT id, name, depth, path FROM category_tree ORDER BY path
    "#
)
.fetch_all(&pool).await?;
```

### Diesel：DSLの制約

標準のDSLでは表現できないクエリもある。`sql_query`で生SQLも書けるが、型安全性が下がる。

```rust
// Dieselで生SQL
let categories = diesel::sql_query(
    r#"WITH RECURSIVE category_tree AS (...) SELECT * FROM category_tree"#
)
.load::<CategoryWithPath>(&mut conn)?;
```

### 判定

- **CTEやウィンドウ関数を多用** → sqlx
- **標準的なCRUDが中心** → どちらでも

## 5. N+1問題の解決

### sqlx：JSON集約

PostgreSQL固有の機能を使って1クエリで取得。

```rust
#[derive(Debug, sqlx::FromRow)]
struct PostWithComments {
    id: Uuid,
    title: String,
    comments: Json<Vec<CommentData>>,
}

let posts: Vec<PostWithComments> = sqlx::query_as(
    r#"
    SELECT
        p.id, p.title,
        COALESCE(
            json_agg(json_build_object('id', c.id, 'body', c.body))
            FILTER (WHERE c.id IS NOT NULL),
            '[]'::json
        ) as comments
    FROM posts p
    LEFT JOIN comments c ON p.id = c.post_id
    GROUP BY p.id
    "#
)
.fetch_all(&pool).await?;
```

### Diesel：アソシエーション

DieselはBelongsTo/HasManyのようなアソシエーションを定義できる。

```rust
#[derive(Associations)]
#[diesel(belongs_to(Post))]
pub struct Comment { ... }

// Eager loading
let posts = posts::table.load::<Post>(&mut conn)?;
let comments = Comment::belonging_to(&posts)
    .load::<Comment>(&mut conn)?
    .grouped_by(&posts);
```

どちらも2クエリ必要だが、Dieselの方が宣言的。

### 判定

- **PostgreSQL固有機能をフル活用** → sqlx
- **ORMライクなアソシエーション** → Diesel

## 6. 動的クエリ構築

### sqlx：QueryBuilder

```rust
use sqlx::QueryBuilder;

let mut builder = QueryBuilder::new("SELECT * FROM users WHERE 1=1");

if let Some(name) = &filter.name {
    builder.push(" AND name ILIKE '%' || ");
    builder.push_bind(name);
    builder.push(" || '%'");
}

if let Some(status) = &filter.status {
    builder.push(" AND status = ");
    builder.push_bind(status);
}

let users = builder.build_query_as::<User>()
    .fetch_all(&pool).await?;
```

### Diesel：BoxedDsl

```rust
let mut query = users::table.into_boxed();

if let Some(name) = &filter.name {
    query = query.filter(users::name.ilike(format!("%{}%", name)));
}

if let Some(status) = &filter.status {
    query = query.filter(users::status.eq(status));
}

let results = query.load::<User>(&mut conn)?;
```

Dieselの方が型安全だが、`into_boxed()`が必要で若干冗長。

### 判定

- **シンプルな動的クエリ** → どちらでも
- **複雑な条件分岐** → sqlx（SQL直書きの柔軟性）

## 7. エコシステムと成熟度

### sqlx

- 2019年登場の比較的新しいライブラリ
- async-firstの設計
- 活発に開発中（2024年現在）
- PostgreSQL、MySQL、SQLiteをサポート

### Diesel

- 2015年登場の成熟したライブラリ
- 同期処理がベース（diesel-asyncで非同期対応）
- 安定版が長期間メンテナンスされている
- PostgreSQL、MySQL、SQLiteをサポート

### 判定

- **最新のasync/Tokioエコシステムとの統合** → sqlx
- **安定性と実績を重視** → Diesel

## 判断フローチャート

```
Rustでデータベースを使う
│
├─ SQLを直接書きたい / 複雑なクエリが多い
│   └─ sqlx
│
├─ Rustの型システムでクエリを構築したい
│   └─ Diesel
│
├─ async/Tokioが必須
│   ├─ 追加依存を避けたい → sqlx
│   └─ diesel-asyncでも可 → Diesel
│
├─ CIでDB接続なしでビルドしたい
│   ├─ sqlx prepare が使える → sqlx
│   └─ スキーマファイルで十分 → Diesel
│
└─ 迷ったら
    ├─ PostgreSQL中心でasync → sqlx
    └─ 複数DB対応でsync → Diesel
```

## 結論

sqlxとDieselは設計思想が異なる。どちらが優れているというより、プロジェクトの要件に合う方を選ぶ。

**sqlxを選ぶ理由**
- SQLに慣れている
- 複雑なクエリ（CTE、ウィンドウ関数）を多用
- async/Tokioファースト
- PostgreSQL固有機能を活用

**Dieselを選ぶ理由**
- Rustの型システムでクエリを構築したい
- スキーマファイルベースのコンパイル時チェック
- アソシエーション（BelongsTo/HasMany）が欲しい
- 同期処理で十分

個人的には、PostgreSQLを使うasyncアプリケーションではsqlxを選ぶことが多い。SQLを直接書ける柔軟性と、ネイティブasync対応が魅力だ。

どちらを選んでも、型安全なデータベースアクセスが実現できる。それがRustの良いところだ。

## 実行可能なデモコード

本記事のsqlxコードは以下で実行できる。

```sh
cd blog_10_diesel_comparison
cargo run
```

## 参考資料

- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
- [Diesel - Getting Started](https://diesel.rs/guides/getting-started)
- [diesel-async - GitHub](https://github.com/weiznich/diesel_async)
