# DB設計の落とし穴：Rust/sqlxで学ぶ5つのアンチパターン

## はじめに

あるプロジェクトで、タグ検索機能のバグを調査していた。`WHERE tags LIKE '%rust%'`というクエリが、なぜか「trustworthy」という単語を含む投稿まで返してくる。タグは`rust,programming,systems`のようにカンマ区切りで格納されていた。

LIKEの部分一致は文字列のどこでもマッチする。`%rust%`は「rust」だけでなく「t**rust**worthy」にもヒットする。カンマ区切りでデータを格納した時点で、この問題は避けられなかった。

これは『SQLアンチパターン』で「Jaywalking（ジェイウォーク）」と呼ばれるパターンだ。本記事ではこれを含む5つのアンチパターンをRust + sqlxで検証する。どれもスキーマ設計時に一度は踏みそうな落とし穴であり、回避策を知っておくと設計判断が速くなる。

[asin:4814400748:detail]

## パターン1：Jaywalking（カンマ区切り格納）

### 問題のあるスキーマ

```sql
CREATE TABLE posts_bad (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    tags TEXT  -- 'rust,programming,systems'
);
```

```rust
// NG: LIKE検索で誤検出
let posts = sqlx::query_as::<_, (String, String)>(
    "SELECT title, tags FROM posts_bad WHERE tags LIKE '%rust%'"
)
.fetch_all(&pool).await?;
// → 'trustworthy' を含む投稿もマッチ
```

カンマ区切りには複数の問題がある。LIKE検索での誤検出に加えて、インデックスが効かない（全件スキャンになる）、タグの追加・削除がパース処理を伴う、参照整合性がないためタイポが検出できない。

### 解決策1：PostgreSQL配列型

タグのメタデータ管理が不要で、タグ数が100程度までなら配列型が適している。

```sql
CREATE TABLE posts_array (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    tags TEXT[] DEFAULT '{}'
);

-- GINインデックスで高速検索
CREATE INDEX idx_posts_tags ON posts_array USING GIN(tags);
```

```rust
// OK: ANY演算子で正確にマッチ
let posts = sqlx::query_as::<_, (String, Vec<String>)>(
    "SELECT title, tags FROM posts_array WHERE 'rust' = ANY(tags)"
)
.fetch_all(&pool).await?;
```

PostgreSQLの配列演算子は強力だ。`@>`は「すべて含む」、`&&`は「共通要素がある」を判定できる。

```sql
-- 両方のタグを持つ投稿
WHERE tags @> ARRAY['rust', 'programming']

-- いずれかのタグを持つ投稿
WHERE tags && ARRAY['rust', 'go']
```

ただし配列型には落とし穴がある。GINインデックスは検索は高速だが、更新はB-treeの約10倍遅い。読み取り重視のユースケースに向いている。

### 解決策2：交差テーブル

タグにメタデータ（作成日、同義語など）を持たせたい場合や、タグ数が1000を超える場合は交差テーブルを使う。

```sql
CREATE TABLE posts (
    post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(200) NOT NULL
);

CREATE TABLE tags (
    tag_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE post_tags (
    post_id UUID REFERENCES posts(post_id) ON DELETE CASCADE,
    tag_id UUID REFERENCES tags(tag_id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);
```

```rust
// JOINで検索
let posts: Vec<(Uuid, String)> = sqlx::query_as(
    "SELECT p.post_id, p.title FROM posts p
     INNER JOIN post_tags pt ON p.post_id = pt.post_id
     INNER JOIN tags t ON pt.tag_id = t.tag_id
     WHERE t.slug = $1"
)
.bind("rust")
.fetch_all(&pool).await?;
```

交差テーブルのメリットは参照整合性だ。存在しないタグを紐づけようとするとFK違反でエラーになる。タイポが本番で発覚する事態を防げる。

### 選択基準

| 観点 | 配列型 | 交差テーブル |
|------|--------|-------------|
| タグ数 | 〜100程度 | 1000以上可 |
| メタデータ | 不要 | 必要 |
| 参照整合性 | なし | あり |
| 更新頻度 | 低い | 高くてもOK |

迷ったら交差テーブルを選ぶ。後から配列型に戻すのは難しいが、逆は可能だ。

## パターン2：ID Required（不要な代理キー）

### 問題のあるスキーマ

```sql
CREATE TABLE user_roles_bad (
    id SERIAL PRIMARY KEY,  -- 不要
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL
);
```

交差テーブルにSERIAL主キーを追加すると、同じユーザーに同じロールを複数回付与できてしまう。複合主キーであれば重複挿入を防げる。

```rust
// 重複挿入が成功してしまう
sqlx::query("INSERT INTO user_roles_bad (user_id, role_id) VALUES (1, 1)")
    .execute(&pool).await?;
sqlx::query("INSERT INTO user_roles_bad (user_id, role_id) VALUES (1, 1)")
    .execute(&pool).await?;  // エラーにならない
```

### 解決策：複合主キー

```sql
CREATE TABLE user_roles (
    user_id UUID REFERENCES users(user_id) ON DELETE CASCADE,
    role_id UUID REFERENCES roles(role_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);
```

### 意味のあるカラム名

全テーブルで`id`という名前を使うと、JOINで混乱する。`user_id`、`post_id`のように役割を示す名前にすると、USING句が使える。

```sql
-- 良い例: USING句で簡潔に書ける
SELECT * FROM users JOIN posts USING (user_id);

-- 悪い例: 毎回テーブル指定が必要
SELECT * FROM users u JOIN posts p ON u.id = p.user_id;
```

### UUID vs SERIAL

| 特性 | SERIAL/IDENTITY | UUIDv4 | UUIDv7 |
|------|-----------------|--------|--------|
| サイズ | 4-8バイト | 16バイト | 16バイト |
| 分散生成 | 不可 | 可能 | 可能 |
| インデックス効率 | 最高 | 悪い | 良好 |
| PostgreSQL 18+ | IDENTITY推奨 | - | uuidv7()関数 |

UUIDv4はランダムなためB-treeページ分割が頻発し、インデックスが肥大化する。UUIDv7は時間順でソート可能なため、この問題を緩和できる。

```rust
use uuid::Uuid;

// UUIDv7（推奨）
let id = Uuid::now_v7();  // 時間順でソート可能

// Newtype Patternで型安全性を確保
#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(Uuid);

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct PostId(Uuid);
```

Newtype Patternを使えば、`UserId`と`PostId`を取り違えるとコンパイルエラーになる。

## パターン3：Keyless Entry（外部キー制約なし）

### 問題

FK制約を省略すると、存在しない親レコードを参照できてしまう。

```sql
CREATE TABLE comments_bad (
    id SERIAL PRIMARY KEY,
    post_id INTEGER,  -- FK制約なし
    body TEXT NOT NULL
);
```

```rust
// 存在しないpost_idでも挿入成功
sqlx::query("INSERT INTO comments_bad (post_id, body) VALUES (9999, 'コメント')")
    .execute(&pool).await?;  // エラーにならない
```

アプリケーション側で存在チェックを行う方法もあるが、レースコンディションに弱い。チェックから挿入の間に親レコードが削除される可能性がある。

### 解決策：FK制約 + ON DELETEオプション

```sql
CREATE TABLE comments (
    comment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    body TEXT NOT NULL
);

-- 重要: PostgreSQLはFK列に自動でインデックスを作成しない
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);
```

ON DELETEオプションの選択基準:
- **CASCADE**: 親削除時に子も削除（コメント、ログなど）
- **SET NULL**: 親削除時にNULLに設定（ユーザー削除後も履歴を残したい場合）
- **RESTRICT**: 子がある場合は親削除を禁止

### レースコンディション対策

FK制約に任せて、違反時にアプリケーションエラーに変換する。

```rust
async fn add_comment(pool: &PgPool, post_id: Uuid, body: &str) -> Result<Uuid, AppError> {
    let comment_id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO comments (comment_id, post_id, body) VALUES ($1, $2, $3)"
    )
    .bind(comment_id)
    .bind(post_id)
    .bind(body)
    .execute(pool).await;

    match result {
        Ok(_) => Ok(comment_id),
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
            Err(AppError::PostNotFound)
        }
        Err(e) => Err(AppError::Database(e)),
    }
}
```

この方法なら、存在チェックと挿入の間に親が削除されても正しくエラーハンドリングできる。

## パターン4：Rounding Error（FLOATの落とし穴）

### 問題の本質

IEEE 754浮動小数点は2進数表現のため、0.1や0.2のような10進数を正確に表現できない。

```rust
let a = 0.1_f64;
let b = 0.2_f64;
println!("{}", a + b);       // 0.30000000000000004
println!("{}", a + b == 0.3); // false
```

これは金額計算だけの問題ではない。税率、割引率、統計のp値など、正確な10進数計算が必要な場面は多い。

### 解決策1：DECIMAL型 + rust_decimal

```sql
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    -- Generated Columnで税込価格を自動計算
    price_with_tax DECIMAL(10, 2) GENERATED ALWAYS AS (price * 1.10) STORED
);
```

```rust
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

// 文字列から生成（f64から変換しない）
let price = dec!(19.99);
let tax = (price * dec!(0.10))
    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);

println!("税込: {}", price + tax);  // 21.99
```

`rust_decimal`の`round_dp_with_strategy`で丸め戦略を指定できる。金融向けには`MidpointAwayFromZero`（四捨五入）を使うことが多い。

### 解決策2：セント単位格納

小数を避け、整数で格納する方法もある。

```sql
CREATE TABLE products_cents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    price_cents INTEGER NOT NULL  -- $19.99 → 1999
);
```

```rust
fn format_price(cents: i32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

let total_cents: i64 = sqlx::query_scalar("SELECT SUM(price_cents) FROM products_cents")
    .fetch_one(&pool).await?;
println!("合計: {}", format_price(total_cents as i32));
```

### FLOATが許容される場面

すべての数値にDECIMALを使う必要はない。測定誤差が浮動小数点誤差より大きい場合は、DOUBLE PRECISIONで十分だ。

```sql
-- センサーデータ: 測定誤差 > FLOAT誤差
CREATE TABLE sensor_readings (
    id SERIAL PRIMARY KEY,
    temperature DOUBLE PRECISION NOT NULL,
    humidity DOUBLE PRECISION NOT NULL
);

-- GPS座標: 15桁精度で1mm単位まで表現可能
CREATE TABLE locations (
    id SERIAL PRIMARY KEY,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL
);
```

| 用途 | 推奨型 | 理由 |
|------|--------|------|
| 金額・税率 | DECIMAL | 1円の誤差も許されない |
| センサー値 | DOUBLE PRECISION | 測定誤差 > 計算誤差 |
| GPS座標 | DOUBLE PRECISION | 6桁で約11cm精度、十分 |
| 統計p値 | DECIMAL | 0.05の閾値判定に正確さ必要 |

## パターン5：31 Flavors（ENUM乱用）

### 問題

PostgreSQLのENUM型は値の追加は可能だが、削除・名前変更には型の再作成が必要になる。

```sql
CREATE TYPE priority_level AS ENUM ('low', 'medium', 'high', 'critical');

-- 値の追加は可能
ALTER TYPE priority_level ADD VALUE 'urgent' AFTER 'critical';

-- 値の削除は直接できない
-- 型を作り直す必要がある
```

### 解決策1：参照テーブル

値が変更される可能性がある場合、参照テーブルを使う。

```sql
CREATE TABLE post_statuses (
    status_id SERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    display_name VARCHAR(100) NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE TABLE posts (
    post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    status_code VARCHAR(50) REFERENCES post_statuses(code)
);
```

参照テーブルなら、値の追加・削除・表示名変更がすべてINSERT/UPDATE/DELETEで完結する。マイグレーション不要だ。

### Rustでの型安全な扱い

参照テーブル方式では、Rust側でenumを定義してFromStr/Displayを実装する。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostStatus {
    Draft,
    PendingReview,
    Published,
    Archived,
}

impl PostStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for PostStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "pending_review" => Ok(Self::PendingReview),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            _ => Err(AppError::InvalidStatus(s.to_string())),
        }
    }
}
```

```rust
// DBから取得した文字列をenumに変換
let row: (String,) = sqlx::query_as("SELECT status_code FROM posts WHERE post_id = $1")
    .bind(post_id)
    .fetch_one(&pool).await?;
let status: PostStatus = row.0.parse()?;
```

### 解決策2：sqlx::Type（ENUMが適切な場合）

曜日や優先度のように値が固定されている場合は、PostgreSQL ENUMを使ってもよい。sqlx::Typeで直接マッピングできる。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "priority_level", rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}
```

```rust
// 直接バインド可能
sqlx::query("INSERT INTO tasks (title, priority) VALUES ($1, $2)")
    .bind("重要タスク")
    .bind(Priority::High)
    .execute(&pool).await?;

// 直接取得可能
let task: (String, Priority) = sqlx::query_as("SELECT title, priority FROM tasks LIMIT 1")
    .fetch_one(&pool).await?;
```

### 選択基準

| 観点 | ENUM | 参照テーブル |
|------|------|-------------|
| 値の追加 | 可能 | 可能 |
| 値の削除 | 困難 | 可能 |
| 値の名前変更 | 困難 | 可能 |
| メタデータ | 不可 | 可能 |
| sqlx::Type | 使用可 | FromStr実装 |

値が変わる可能性があるなら参照テーブル、曜日のように変わらないならENUMでよい。

## まとめ

冒頭の`LIKE '%rust%'`問題は、配列型に移行して`'rust' = ANY(tags)`に書き換えることで解決した。PostgreSQL配列とGINインデックスの組み合わせは、この手の検索には最適だ。

5つのアンチパターンの回避策を整理する:

1. **Jaywalking**: カンマ区切り → 配列型または交差テーブル
2. **ID Required**: 全テーブルにid → 交差テーブルは複合主キー
3. **Keyless Entry**: FK制約省略 → FK制約 + ON DELETE + インデックス
4. **Rounding Error**: FLOAT → 金額にはDECIMAL、センサーはFLOATでよい
5. **31 Flavors**: ENUM乱用 → 参照テーブル（値が変わらないならENUM可）

どれも「とりあえず動く」設計から「正しく動く」設計への移行パターンだ。設計時に判断できれば、後からのマイグレーションを避けられる。

## 実行可能なデモコード

本記事のコードは以下で実行できる:

```sh
cd blog_01_db_design_pitfalls
cargo run
```

PostgreSQLが`localhost:5432`で動作している必要がある。

## 参考資料

- [SQL Antipatterns](https://pragprog.com/titles/bksqla/sql-antipatterns/)
- [PostgreSQL - Array Functions](https://www.postgresql.org/docs/current/functions-array.html)
- [rust_decimal - docs.rs](https://docs.rs/rust_decimal/latest/rust_decimal/)
- [sqlx - Type derive](https://docs.rs/sqlx/latest/sqlx/trait.Type.html)
