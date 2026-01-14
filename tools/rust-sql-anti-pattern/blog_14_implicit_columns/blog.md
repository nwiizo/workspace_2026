# SELECT *は本当に悪なのか：明示的カラムの本当のメリット

## 常識への疑問

「SELECT *は使うな」

SQLのベストプラクティスとしてよく言われる。では、なぜ使ってはいけないのか。「パフォーマンスが悪い」「保守性が下がる」と言われるが、具体的に何が問題なのか。

本記事では「SELECT *は悪」という常識を一度疑い、本当のメリットとデメリットを明らかにする。結論を先に言うと、SELECT *は確かに避けるべきだが、その理由は単純なパフォーマンス問題ではない。

## SELECT *の何が問題か

### 問題1：機密データの意図しない取得

これが最も深刻な問題だ。

```sql
-- usersテーブルの定義
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    bio TEXT,
    password_hash VARCHAR(255) NOT NULL,  -- 機密
    api_secret VARCHAR(255),               -- 機密
    created_at TIMESTAMPTZ
);
```

```rust
// ❌ SELECT * でpassword_hashも取得してしまう
let users: Vec<UserRow> = sqlx::query_as("SELECT * FROM users")
    .fetch_all(&pool).await?;

// このusersをそのままJSONで返すと...
// password_hashがAPIレスポンスに含まれてしまう
```

SELECT *は「今あるカラムを全部取る」ではなく「将来追加されるカラムも全部取る」という意味だ。後から機密カラムが追加されたとき、既存のコードが脆弱性になる。

### 問題2：スキーマ変更での予期せぬ破壊

sqlxの`query_as`で構造体にマッピングする場合、カラムの数と順序が一致しないとランタイムエラーになる。

```rust
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

// テーブルにbioカラムが追加されると...
// SELECT * は id, email, name, bio, ... を返す
// 構造体とカラム数が合わずにエラー
```

明示的にカラムを指定すれば、テーブル構造が変わってもクエリは動き続ける。

### 問題3：JOINでのカラム名衝突

```sql
-- 両テーブルにid, created_atがある
SELECT *
FROM posts p
JOIN users u ON p.user_id = u.id
-- どっちのid？どっちのcreated_at？
```

```rust
// ❌ ambiguous
let results = sqlx::query("SELECT * FROM posts JOIN users ON ...")
    .fetch_all(&pool).await?;

// ✅ 明示的に指定
let results = sqlx::query_as::<_, PostWithAuthor>(
    r#"
    SELECT p.id as post_id, p.title, u.name as author_name
    FROM posts p JOIN users u ON p.user_id = u.id
    "#
)
.fetch_all(&pool).await?;
```

### 問題4：Index Only Scanの阻害

PostgreSQLはカバリングインデックスを使えば、テーブルにアクセスせずにクエリを完了できる。

```sql
CREATE INDEX idx_users_name ON users(name) INCLUDE (id);
```

```sql
-- ✅ Index Only Scan可能
SELECT id, name FROM users WHERE name LIKE 'A%';

-- ❌ テーブルアクセスが必要
SELECT * FROM users WHERE name LIKE 'A%';
```

SELECT *を使うと、必ずテーブルにアクセスするため、Index Only Scanの恩恵を受けられない。

## では、SELECT *はいつ使っていいのか

完全に禁止すべきかというと、そうでもない。

### 開発中のデバッグ

```sql
-- テーブル構造を確認したいとき
SELECT * FROM users LIMIT 5;
```

REPLや開発環境でデータを確認するときは便利だ。

### 管理ツールやETL

```rust
// バックアップやエクスポート用途
let rows = sqlx::query("SELECT * FROM users")
    .fetch_all(&pool).await?;

for row in rows {
    // 全カラムをダンプ
}
```

動的にスキーマを扱う必要がある場合は、SELECT *が適切なこともある。

### サブクエリ内

```sql
-- 外側のSELECTで必要なカラムだけ取得
SELECT id, name FROM (
    SELECT * FROM users WHERE status = 'active'
) AS active_users;
```

サブクエリ内では全カラムが必要なケースがある。ただし、CTEで書き換えた方が読みやすい。

## 用途別の構造体を定義する

明示的カラム指定の真のメリットは、用途に応じた構造体を定義できることだ。

```rust
// 一覧表示用（最小限）
#[derive(Debug, sqlx::FromRow)]
struct UserSummary {
    id: Uuid,
    name: String,
}

// 公開API用（機密なし）
#[derive(Debug, sqlx::FromRow)]
struct UserPublic {
    id: Uuid,
    name: String,
    bio: Option<String>,
}

// 内部処理用（メール含む）
#[derive(Debug, sqlx::FromRow)]
struct UserDetail {
    id: Uuid,
    email: String,
    name: String,
    bio: Option<String>,
    created_at: DateTime<Utc>,
}
```

```rust
// 一覧ページ
let users: Vec<UserSummary> = sqlx::query_as(
    "SELECT id, name FROM users ORDER BY name"
)
.fetch_all(&pool).await?;

// プロフィールAPI
let user: UserPublic = sqlx::query_as(
    "SELECT id, name, bio FROM users WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;

// 管理画面
let user: UserDetail = sqlx::query_as(
    "SELECT id, email, name, bio, created_at FROM users WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;
```

構造体を分けることで、どの情報がどこで使われるか明確になる。

## パフォーマンス比較

```rust
// 全カラム取得
let start = Instant::now();
let all: Vec<UserFull> = sqlx::query_as(
    "SELECT id, email, name, bio, avatar_url, password_hash, api_secret,
            last_login_at, created_at, updated_at FROM users"
)
.fetch_all(&pool).await?;
let all_time = start.elapsed();

// 最小限のカラム
let start = Instant::now();
let minimal: Vec<UserSummary> = sqlx::query_as(
    "SELECT id, name FROM users"
)
.fetch_all(&pool).await?;
let minimal_time = start.elapsed();
```

```
All columns: 15ms
Minimal: 8ms
```

データ転送量の差が実行時間に反映される。ただし、これは行数やネットワーク環境に依存する。小規模なデータでは差が出にくい。

## 結論：SELECT *を避ける本当の理由

パフォーマンスは副次的な理由だ。本当の理由は以下の3つ。

1. **セキュリティ**: 機密カラムを意図せず取得・露出するリスク
2. **保守性**: スキーマ変更で予期せぬ破壊が起きる
3. **明示性**: 何を取得しているか、コードを見ればわかる

「SELECT *は悪」という常識は正しい。ただし、その理由を理解した上で判断することが重要だ。開発中のデバッグや動的なツールでは使ってもいい。本番のアプリケーションコードでは避ける。

明示的なカラム指定は、面倒に見えて実は保守性を高める。用途に応じた構造体を定義することで、コードが自己文書化される。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_14_implicit_columns
cargo run
```

## 参考資料

- [PostgreSQL - SELECT](https://www.postgresql.org/docs/current/sql-select.html)
- [PostgreSQL - Index Only Scans](https://www.postgresql.org/docs/current/indexes-index-only-scans.html)
- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
