# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a blog series codebase demonstrating SQL anti-patterns using Rust + PostgreSQL (sqlx). Each `blog_XX_*` directory contains standalone Rust code that demonstrates specific anti-patterns from the book "SQL Antipatterns" with PostgreSQL-specific solutions.

## Build and Run Commands

```bash
# Build and run a specific blog's demo
cd blog_01_db_design_pitfalls && cargo run

# Run with quality checks (before commit)
cd blog_XX_* && cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Environment Setup

Requires a running PostgreSQL instance:

```bash
# Lima/Docker setup
limactl start docker
limactl shell docker nerdctl run -d \
  --name postgres-antipattern \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=antipattern \
  -p 5432:5432 \
  postgres:16

# Connection string used in all demos
DATABASE_URL=postgres://postgres:postgres@localhost:5432/antipattern
```

## Project Structure

- `blog_XX_*/` - Independent Rust crates, each runnable standalone
- `blog_XX_*.md` - Blog article content for each topic
- `SPECIFICATION.md` - Reference material from SQL Antipatterns book
- `index.md` - Table of contents for the book chapters

## Key Dependencies

All blog crates use:
- `sqlx` with features: `runtime-tokio`, `postgres`, `macros`, `chrono`, `uuid`, `rust_decimal`
- `tokio` with `full` features
- `rust_decimal` for monetary calculations (avoid FLOAT)
- `anyhow` for error handling

## Code Patterns

- Each `main.rs` sets up tables, runs demos, and cleans up
- SQL queries use `sqlx::query` / `sqlx::query_as` with bind parameters
- Error handling uses `Result<()>` with `?` propagation
- PostgreSQL-specific features (arrays, ENUM, generated columns) are demonstrated


## Article Writing Guidelines

### Article Structure Template

```markdown
# タイトル：問題を端的に表現

導入文（1-2文）
- この問題が金融だけでなく全ドメインに影響するなど、普遍性を示します
- 具体的な適用場面を列挙します

## 問題のあるスキーマ

問題のあるSQLコード例を示します。

## なぜ悪いのか

### 問題点1：具体的な問題点A
- コード例と問題の説明
- **問題点**: 箇条書きで明確に

### 問題点2：具体的な問題点B
（繰り返し）

## 解決策1：○○（推奨度の高い順）

### スキーマ設計
SQLコード例を示します。

### Rustでの使用例
sqlxを使ったコード例と型マッピングの説明をします。

### メリット・デメリット
特性を説明します。

## 解決策2：○○
（繰り返し）

## パフォーマンス比較（可能な場合）
- ベンチマーク条件
- 比較表

## どちらを選ぶべきか

### 選択基準
- 条件A → 解決策1
- 条件B → 解決策2

### 判断フローチャート
質問1？
├─ YES → 解決策A
└─ NO → 質問2？
        ├─ YES → 解決策B
        └─ NO → 解決策C

## チェックリスト

### 設計時
- [ ] 確認項目1
- [ ] 確認項目2

### Rust実装時
- [ ] 確認項目1
- [ ] 確認項目2

## 実行可能なデモコード
ファイルパスと実行コマンドを示します。

## 参考資料
- [リンクテキスト](URL)
```

### Article Content Depth Guidelines

記事を書く際は、以下の深さを目指す：

#### 1. 問題の普遍性を示す
- 金融だけでなく、科学計算、センサーデータ、座標など複数ドメインへの適用
- 「このアンチパターンは○○だけの問題ではない」という導入

#### 2. PostgreSQL固有の詳細を含める
- 内部動作（例：GINインデックスの構造、B-treeページ分割）
- パフォーマンス特性（例：検索は高速だが更新は遅い）
- バージョン固有の機能（例：PostgreSQL 18のUUIDv7）
- 演算子とその用途（例：`@>`, `&&`, `= ANY()`）

#### 3. Rust/sqlxの実装パターンを示す
- 型マッピング表（PostgreSQL型 → Rust型）
- エラーハンドリングパターン（FK違反、重複キーなど）
- N+1問題の回避方法
- Newtype Pattern、FromStr/Display実装

#### 4. 定量的な比較を含める
- パフォーマンスベンチマーク（クエリ時間、ストレージサイズ）
- トレードオフ表（メリット/デメリット）
- 選択基準のチェックリスト

#### 5. 判断フローチャートを提供
```
質問1？
├─ YES → 解決策A
└─ NO → 質問2？
        ├─ YES → 解決策B
        └─ NO → 解決策C
```

#### 6. 落とし穴と注意点を明記
- PostgreSQLがFK列にインデックスを自動作成しない
- 配列のインデックスアクセスがO(N²)
- UUIDv4のランダム挿入によるB-tree肥大化

#### 7. 参考資料は公式ドキュメント優先
- PostgreSQL公式ドキュメント
- sqlx/rust_decimal/uuid クレートのdocs.rs
- 信頼できる技術ブログ（Crunchy Data, Neon, pganalyzeなど）


### Code Sample Consistency Checklist

記事内のRustコードを校正する際は、以下を確認:

1. **構造体フィールドの一貫性**
   - 記事内で同じ構造体が複数回登場する場合、フィールドが一致しているか
   - SQLのSELECT句と構造体のフィールドが一致しているか
   ```rust
   // 構造体定義
   pub struct User { id, name, email, created_at, updated_at }

   // SQL（フィールドが一致していること）
   "SELECT id, name, email, created_at, updated_at FROM users"
   ```

2. **依存クレートのimport**
   - 使用している型のimportが記載されているか
   - `use sqlx::PgPool`, `use uuid::Uuid`, `use chrono::{DateTime, Utc}` など

3. **エラー型の定義**
   - カスタムエラー型を使用する場合、定義が記載されているか
   - `thiserror` の使用例を含めるか、省略を明記

4. **マクロ/トレイトの完全性**
   - マクロ定義が `// 実装` のまま省略されていないか
   - 省略する場合は「実装は省略」と明記

5. **async/await の一貫性**
   - `async fn` と `.await` の対応
   - `#[tokio::main]` or `#[tokio::test]` の有無

### Technical Accuracy Checklist

1. **SQL構文**
   - PostgreSQL固有の構文が正しいか（`gen_random_uuid()`, `TIMESTAMPTZ` など）
   - インデックス構文: `CREATE INDEX ... ON table(column) WHERE condition`

2. **sqlxマクロ**
   - `query_as!` でのカラム名指定: `"column_name!"` for non-null assertion
   - `query_scalar!` の戻り値型

3. **Rust型マッピング**
   - `DECIMAL` → `rust_decimal::Decimal`
   - `TIMESTAMPTZ` → `chrono::DateTime<Utc>`
   - `UUID` → `uuid::Uuid`
   - Nullable columns → `Option<T>`

### Japanese Writing Style

1. **見出しレベル**
   - `##` と `###` のみ使用（`####` は避ける）
   - 見出しは簡潔に

2. **文体**
   - 「です・ます」調で統一
   - 読者への呼びかけは適度に（「〜してみましょう」「〜を確認してください」）
   - 技術的な説明は丁寧に

3. **コードコメント**
   - 日本語コメントは簡潔に
   - 問題点には `// 危険！` `// 問題！` などのマーカー

4. **導入・結びの文例**
   - 導入: 「〜という問題に直面したことはありませんか？」「本記事では〜を解説します」
   - 結び: 「〜を活用してみてください」「〜の参考になれば幸いです」

5. **論理の流れを飛ばさない**
   - 自然に書き進めていれば生まれるはずの言葉の流れ、つまり論理の小さな階段を、一段も飛ばさないこと
   - 「なぜこうなるのか」→「だからこうする」→「その結果こうなる」という因果関係を丁寧に
   - 読者が「なぜ？」と思う箇所を先回りして説明する
   - 例: 「FLOATは誤差がある」だけでなく「なぜ誤差が生じるのか（2進数表現）」→「どの程度の誤差か」→「どの場面で問題になるか」と段階的に

### Pre-publish Checklist

```
□ 構造体とSQLのフィールドが一致している
□ importが揃っている（または省略を明記）
□ エラー型が定義されている（または省略を明記）
□ マクロ実装が完成している（または省略を明記）
□ 参考リンクが有効（404でない）
□ コードブロックの言語指定（```rust, ```sql）
□ 図表のASCII artが崩れていない
□ はてなブログの画像記法 [f:id:...] が正しい
```


## Anti-Patterns Covered

### Blog 01: DB Design Pitfalls (5 articles)

#### 1. Jaywalking (blog_01_01) - カンマ区切りでデータを格納

**Problem**: Comma-separated lists in VARCHAR columns (`tags VARCHAR(500)`)
- LIKE検索で誤検出（`%rust%`で`trustworthy`もヒット）
- インデックスが効かない（フルスキャン）
- 参照整合性がない
- 値の追加・削除が煩雑

**Solutions**:

1. **PostgreSQL配列型** - シンプルなケース向け
   ```sql
   CREATE TABLE posts (tags TEXT[] NOT NULL DEFAULT '{}');
   CREATE INDEX idx_posts_tags ON posts USING GIN(tags);
   ```
   - 演算子: `@>`（含む）, `&&`（共通要素）, `= ANY()`
   - GINインデックス: 検索は高速（~869μs）だが更新は遅い（B-treeの10倍）
   - **注意**: 可変長配列のインデックスアクセスはO(N²)、`unnest()`を使う

2. **交差テーブル** - メタデータ・正規化が必要な場合
   ```sql
   CREATE TABLE post_tags (
       post_id UUID REFERENCES posts(post_id) ON DELETE CASCADE,
       tag_id UUID REFERENCES tags(tag_id) ON DELETE CASCADE,
       PRIMARY KEY (post_id, tag_id)
   );
   CREATE INDEX idx_post_tags_tag_id ON post_tags(tag_id);  -- 逆方向検索用
   ```

3. **JSONB** - 柔軟なスキーマが必要な場合
   - ストレージは配列の2-3倍
   - クエリプランナーが統計を持たない

**Rust実装**:
```rust
// 配列型: Vec<T>にマッピング
let posts: Vec<(Uuid, Vec<String>)> = sqlx::query_as(
    "SELECT id, tags FROM posts WHERE $1 = ANY(tags)"
).bind(tag).fetch_all(pool).await?;

// N+1問題回避: array_aggを使用
sqlx::query_as!(
    PostWithTags,
    r#"SELECT post_id, title,
       COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as "tags!"
       FROM posts p LEFT JOIN post_tags pt USING (post_id)
       LEFT JOIN tags t USING (tag_id) GROUP BY post_id"#
)
```

**選択基準**:
- 配列型: タグ数<100、メタデータ不要、読み取り重視
- 交差テーブル: FK制約必要、タグ数>1000、更新頻繁


#### 2. ID Required (blog_01_02) - すべてのテーブルに「id」カラム

**Problem**: Unnecessary surrogate keys on junction tables (`id SERIAL PRIMARY KEY`)
- 重複を許可してしまう（`(post_id, tag_id)`の組み合わせ制約がない）
- 無駄なストレージ消費
- USING句が使えない

**Solutions**:

1. **複合主キー** - 交差テーブルの標準パターン
   ```sql
   CREATE TABLE post_tags (
       post_id UUID NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
       tag_id UUID NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
       PRIMARY KEY (post_id, tag_id)
   );
   ```

2. **意味のあるカラム名** - USING句を活用
   ```sql
   -- 良い例: USING句が使える
   SELECT * FROM users JOIN posts USING (user_id);

   -- 悪い例: 毎回テーブル指定が必要
   SELECT * FROM users u JOIN posts p ON u.id = p.user_id;
   ```

**UUID vs SERIAL/IDENTITY**:

| 特性 | BIGSERIAL/IDENTITY | UUIDv4 | UUIDv7 |
|-----|-------------------|--------|--------|
| サイズ | 8バイト | 16バイト | 16バイト |
| 分散生成 | 不可 | 可能 | 可能 |
| インデックス効率 | 最高 | 悪い（ランダム挿入） | 良好（時間順） |
| PostgreSQL 18+ | IDENTITY推奨 | 非推奨 | `uuidv7()`関数 |

**重要**: UUIDv4はランダムなためB-treeページ分割が頻発し、インデックスが肥大化。
**推奨**: PostgreSQL 18+ではUUIDv7、それ以外はBIGINT IDENTITY。

**Rust実装**:
```rust
// UUIDv7（推奨）
use uuid::Uuid;
let id = Uuid::now_v7();  // 時間順でソート可能

// Newtype Pattern（型安全）
#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(Uuid);

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct PostId(Uuid);

// 型が異なるため、コンパイル時にミスを検出
fn transfer(from: UserId, to: UserId, post: PostId) { ... }
```


#### 3. Keyless Entry (blog_01_03) - 外部キー制約の省略

**Problem**: Missing foreign key constraints
- 孤立データの発生（参照先が削除されても子レコードが残る）
- レースコンディション（存在確認→挿入の間に削除される）
- データ整合性の保証がない

**Solutions**:

1. **FK制約を常に宣言**
   ```sql
   CREATE TABLE comments (
       comment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
       post_id UUID NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
       user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE SET NULL
   );

   -- 重要: PostgreSQLはFK列にインデックスを自動作成しない
   CREATE INDEX idx_comments_post_id ON comments(post_id);
   CREATE INDEX idx_comments_user_id ON comments(user_id);
   ```

2. **ON DELETE オプション**
   - `CASCADE`: 親削除時に子も削除（コメント、ログ向け）
   - `SET NULL`: 親削除時にNULLに設定（履歴保持）
   - `RESTRICT`: 子がある場合は親削除を禁止

**Rust実装**:
```rust
// FK制約に任せてエラーをハンドリング（レースコンディション回避）
async fn add_comment(pool: &PgPool, post_id: Uuid, body: &str) -> Result<Uuid, AppError> {
    let comment_id = Uuid::new_v4();
    let result = sqlx::query!(
        "INSERT INTO comments (comment_id, post_id, body) VALUES ($1, $2, $3)",
        comment_id, post_id, body
    ).execute(pool).await;

    match result {
        Ok(_) => Ok(comment_id),
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
            Err(AppError::PostNotFound)
        }
        Err(e) => Err(AppError::Database(e)),
    }
}
```


#### 4. Rounding Error (blog_01_04) - FLOATの落とし穴

**Problem**: IEEE 754浮動小数点の本質的な問題（金融だけでなく全ドメインに影響）
- `0.1 + 0.2 != 0.3`（2進数で正確に表現できない）
- 累積誤差（1000回の0.01加算で誤差発生）
- 丸め方法の違い（NUMERIC vs FLOAT）

**PostgreSQL数値型**:

| 型 | ストレージ | 精度 | 用途 |
|---|----------|-----|-----|
| REAL | 4B | 6桁 | センサー、グラフィックス |
| DOUBLE PRECISION | 8B | 15桁 | 科学計算、座標 |
| NUMERIC(p,s) | 可変 | 131,072桁 | 正確な計算 |
| BIGINT | 8B | 19桁 | セント単位格納 |

**ドメイン別ガイド**:
- 金額・税率 → `NUMERIC`（必須）
- センサー/温度 → `DOUBLE PRECISION`（センサー誤差 > FLOAT誤差）
- 座標(GPS) → `DOUBLE PRECISION`（15桁で1mm精度可能）
- 統計・p値 → `NUMERIC`（有意性判定に正確さ必要）

**Rust実装**:
```rust
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

// 正確な計算
let price = dec!(19.99);  // f64から変換しない
let tax = (price * dec!(0.10))
    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);

// f64の比較にはapproxクレート
use approx::abs_diff_eq;
assert!(abs_diff_eq!(0.1 + 0.2, 0.3, epsilon = 1e-10));

// セント単位格納
#[derive(Debug, Clone, Copy)]
struct Cents(i64);
```

**精度選択**:
```sql
NUMERIC(10, 2)  -- 金額: -99,999,999.99 〜 99,999,999.99
NUMERIC(5, 4)   -- 税率: 0.0000 〜 0.9999
NUMERIC(16, 8)  -- 暗号通貨: 8桁の小数
```


#### 5. 31 Flavors (blog_01_05) - ENUM型の乱用

**Problem**: PostgreSQL ENUM type (hard to modify values)
- 値の追加は可能だが、削除・変更は困難（型の再作成が必要）
- トランザクション内で`ADD VALUE`できない
- マイグレーションが複雑

**Solutions**:

1. **参照テーブル** - 推奨
   ```sql
   CREATE TABLE post_statuses (
       status_id SERIAL PRIMARY KEY,
       code VARCHAR(50) NOT NULL UNIQUE,
       display_name VARCHAR(100) NOT NULL,
       sort_order INT NOT NULL DEFAULT 0,
       is_active BOOLEAN NOT NULL DEFAULT true
   );

   CREATE TABLE posts (
       post_id UUID PRIMARY KEY,
       status_code VARCHAR(50) REFERENCES post_statuses(code)
   );
   ```

2. **PostgreSQL ENUM** - 値が固定の場合のみ
   ```rust
   #[derive(Debug, sqlx::Type)]
   #[sqlx(type_name = "priority_level", rename_all = "lowercase")]
   enum Priority { Low, Medium, High, Critical }
   ```

**Rust実装**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostStatus {
    Draft, PendingReview, Published, Archived,
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
            _ => Err(AppError::InvalidStatus(s.to_string())),
        }
    }
}
```

**選択基準**:
- 参照テーブル: 値が変更される可能性、表示名/メタデータ必要、動的追加
- ENUM: 値が固定（曜日、優先度）、sqlx::Typeで直接マッピング


### Blog 02: Performance Optimization (6 articles)

#### 1. N+1 Query Problem (blog_02_01) - ループ内でクエリを発行

**Problem**: Fetching related data in a loop causes O(N+1) queries
- 100件のデータに対して101回のクエリが発行される
- ネットワークラウンドトリップのオーバーヘッドが積み重なる
- 1クエリあたり0.5-2msのオーバーヘッド × N回

**Solutions**:

1. **JOIN** - 1対1の関連を一括取得
   ```sql
   SELECT p.*, u.name as author_name
   FROM posts p JOIN users u ON p.user_id = u.user_id
   ```

2. **IN句 + HashMap** - 1対多の関連を効率的に取得
   ```rust
   let posts = sqlx::query_as!(Post, "SELECT * FROM posts LIMIT 100").fetch_all(pool).await?;
   let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
   let users = sqlx::query_as!(User, "SELECT * FROM users WHERE user_id = ANY($1)", &user_ids)
       .fetch_all(pool).await?;
   let user_map: HashMap<Uuid, User> = users.into_iter().map(|u| (u.user_id, u)).collect();
   ```

3. **array_agg** - 1対多を配列として取得
   ```sql
   SELECT p.post_id, p.title,
          COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') as "tags!"
   FROM posts p LEFT JOIN post_tags pt USING (post_id)
   LEFT JOIN tags t USING (tag_id) GROUP BY p.post_id
   ```

4. **json_agg** - 複雑なネスト構造を取得
   ```sql
   SELECT p.post_id, COALESCE(json_agg(json_build_object(
       'comment_id', c.comment_id, 'body', c.body
   )) FILTER (WHERE c.comment_id IS NOT NULL), '[]') as comments
   FROM posts p LEFT JOIN comments c USING (post_id) GROUP BY p.post_id
   ```

5. **DataLoader Pattern** - 自動バッチ化（GraphQL向け）
   - 一定時間内のリクエストをバッチ化
   - キャッシュで重複リクエストを回避

**Detection**:
- ループ内の `.await` でDBアクセスがないか確認
- pg_stat_statements で同一クエリの大量実行を検出

**pg_stat_statements Setup (Docker)**:
```bash
docker run -d --name postgres-app \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 postgres:17 \
  -c shared_preload_libraries=pg_stat_statements \
  -c pg_stat_statements.track=all

# Then: CREATE EXTENSION pg_stat_statements;
```

**実測パフォーマンス**:
| 方法 | クエリ数 | 実行時間 | 改善率 |
|------|---------|----------|--------|
| N+1パターン | 51回 | 27.95ms | - |
| JOIN | 1回 | 1.51ms | 18.5倍 |
| DataLoader（キャッシュ） | 0回 | 0.013ms | 2,150倍 |


#### 2. Index Design (blog_02_02) - インデックス設計とEXPLAIN ANALYZE

**Problem**: Missing or incorrect indexes cause full table scans

**EXPLAIN ANALYZE Output**:
```sql
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) SELECT ...
-- cost: 推定コスト, actual time: 実測時間(ms), rows: 行数, loops: 実行回数
-- Buffers: shared hit (キャッシュヒット) / read (ディスク読み取り)
```

**Scan Types** (速い順):
1. **Index Only Scan** - インデックスのみで完結（Heap Fetches: 0）
2. **Index Scan** - インデックス + テーブル参照
3. **Bitmap Index Scan** - 複数条件の組み合わせ
4. **Seq Scan** - 全件スキャン（インデックスなし）

**Index Types**:
- **B-tree** (default): 等価・範囲検索、ORDER BY
- **GIN**: 配列、JSONB、全文検索
- **BRIN**: 時系列データ（サイズが極小）
- **Partial Index**: `WHERE deleted_at IS NULL` など条件付き
- **Expression Index**: `LOWER(email)` など関数適用
- **Covering Index (INCLUDE)**: Index Only Scanを可能に

**Composite Index Rules**:
```
インデックス: (A, B, C)

使用可能:
✓ WHERE A = ?
✓ WHERE A = ? AND B = ?
✓ WHERE A = ? AND B = ? AND C = ?

使用不可能:
✗ WHERE B = ?           -- Aがない
✗ WHERE A = ? AND C = ? -- Bが抜けている

順序: 等価条件 → 範囲条件 → ORDER BY
```


#### 3. Spaghetti Query (blog_02_03) - 巨大SQLの分解

**Problem**: Monolithic SQL queries that are hard to maintain, debug, and optimize
- 7つのサブクエリを含む巨大なSELECT
- どの部分が遅いか特定困難
- 一部の更新でも全体を再取得

**Solutions**:

1. **分割 + 並列実行 (try_join!)**
   ```rust
   let (user, post_stats, social_stats) = tokio::try_join!(
       get_user(pool, user_id),
       get_post_stats(pool, user_id),
       get_social_stats(pool, user_id),
   )?;
   ```

2. **CTE (WITH句)** - 段階的にクエリを構築
   ```sql
   WITH recent_posts AS (...),
        post_engagement AS (...)
   SELECT ... FROM post_engagement
   ```
   - MATERIALIZED: 結果を一時テーブルに保存
   - NOT MATERIALIZED: インライン展開

3. **再帰CTE** - 階層構造の取得
   ```sql
   WITH RECURSIVE category_tree AS (
       SELECT ... WHERE parent_id IS NULL  -- Base case
       UNION ALL
       SELECT ... FROM categories c JOIN category_tree ct ON c.parent_id = ct.id
   )
   ```

4. **Materialized View** - 事前計算結果のキャッシュ
   ```sql
   CREATE MATERIALIZED VIEW user_stats AS SELECT ...;
   REFRESH MATERIALIZED VIEW CONCURRENTLY user_stats;
   ```


#### 4. Ambiguous Group (blog_02_04) - GROUP BYの正しい使い方

**Problem**: `SELECT title, MAX(created_at)` without `title` in GROUP BY

**Single-Value Rule**: GROUP BYを使う場合、SELECTできるのは:
1. GROUP BYに含まれる列
2. 集約関数の結果（COUNT, MAX, AVG等）

**Solutions**:

1. **DISTINCT ON** (PostgreSQL固有、最速)
   ```sql
   SELECT DISTINCT ON (user_id) * FROM posts ORDER BY user_id, created_at DESC
   ```
   - 制限: ORDER BYの最初の列と一致が必要

2. **ROW_NUMBER()** (標準SQL、N件取得可能)
   ```sql
   SELECT * FROM (
       SELECT *, ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
       FROM posts
   ) sub WHERE rn <= 3  -- 各ユーザーの最新3件
   ```

3. **LATERAL JOIN** (複雑なケース向け)
   ```sql
   SELECT u.*, latest.*
   FROM users u
   LEFT JOIN LATERAL (
       SELECT * FROM posts WHERE user_id = u.user_id ORDER BY created_at DESC LIMIT 3
   ) latest ON true
   ```

**ROW_NUMBER vs RANK vs DENSE_RANK**:
| 関数 | 同じ値の扱い | 用途 |
|------|-------------|------|
| ROW_NUMBER() | 連番（同値でも異なる番号）| 厳密に1件取得 |
| RANK() | 同じ番号、次は飛ばす | 同順位を考慮 |
| DENSE_RANK() | 同じ番号、次は連続 | 順位の数を知りたい |


#### 5. Random Selection (blog_02_05) - 効率的なランダム取得

**Problem**: `ORDER BY RANDOM()` causes O(n log n) sort on every query
- 100万行: 1-5秒かかる

**Solutions**:

| 方法 | 計算量 | 均一性 | 大規模対応 |
|------|--------|--------|----------|
| ORDER BY RANDOM() | O(n log n) | 均一 | 不可 |
| オフセット方式 | O(offset) | 均一 | 可能 |
| ID範囲方式 | O(1) | やや偏り | 可能 |
| TABLESAMPLE | O(1)〜O(n) | 偏りあり | 可能 |
| キャッシュ | O(1) | 均一 | 最適 |

1. **オフセット方式**
   ```rust
   let count = sqlx::query_scalar!("SELECT COUNT(*) FROM posts").fetch_one(pool).await?;
   let offset = rand::thread_rng().gen_range(0..count);
   sqlx::query!("SELECT * FROM posts ORDER BY id OFFSET $1 LIMIT 1", offset)
   ```

2. **TABLESAMPLE**
   ```sql
   SELECT * FROM posts TABLESAMPLE SYSTEM(1) LIMIT 5;   -- 高速だが偏りあり
   SELECT * FROM posts TABLESAMPLE BERNOULLI(0.1) LIMIT 5; -- 均一だが遅い
   ```

3. **キャッシュ方式** (推奨)
   ```rust
   pub struct RandomPostCache {
       post_ids: Arc<RwLock<Vec<Uuid>>>,
   }
   // 定期的にrefresh(), get_random()はO(1)
   ```


#### 6. Connection Pool (blog_02_06) - 接続プール設計

**Problem**: Creating new connections per request (4-10ms overhead each)

**PgPoolOptions Settings**:
```rust
PgPoolOptions::new()
    .max_connections(20)        // PostgreSQL側設定 / インスタンス数
    .min_connections(5)         // コールドスタート防止
    .acquire_timeout(Duration::from_secs(3))  // 接続取得タイムアウト
    .idle_timeout(Duration::from_secs(600))   // アイドル接続タイムアウト
    .max_lifetime(Duration::from_secs(1800))  // 接続の最大生存時間
    .test_before_acquire(true)  // 接続検証（本番推奨）
```

**max_connections計算**:
```
PostgreSQL max_connections = 100
監視・バックアップ用 = 10
アプリケーションインスタンス = 4
→ 各インスタンス = (100 - 10) / 4 = 22 → 安全マージンで20
```

**Monitoring**:
```rust
let size = pool.size();        // 現在の接続数
let idle = pool.num_idle();    // アイドル接続数
// usage > 80% で警告
```

**Best Practices**:
- 接続を長時間保持しない（トランザクション内で外部API呼び出し禁止）
- トランザクションは短く
- 大規模環境ではPgBouncer導入を検討


### Blog 06: Metadata Tribbles (Scalability Design)
- **Problem**: Year-based table splitting (`orders_2022`, `orders_2023`)
- **Solutions**:
  - PostgreSQL native partitioning: `PARTITION BY RANGE (created_at)`
  - Automatic partition pruning for queries
- **Key SQL**:
```sql
CREATE TABLE orders (...) PARTITION BY RANGE (created_at);
CREATE TABLE orders_2024 PARTITION OF orders FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');
```


### Blog 07: Orphaned Files (File Upload)
- **Problem**: DB records and file storage out of sync (orphaned files, dangling references)
- **Solutions**:
  - Two-phase upload: temp → confirmed status
  - Background cleanup job for orphaned files
  - Transactional outbox pattern for file operations


### Blog 08: Security
- **Problem**: Plaintext passwords, SQL injection, exposed sensitive data
- **Solutions**:
  - Argon2 password hashing: `argon2` crate
  - Parameterized queries (sqlx prevents injection by design)
  - Separate DTOs for API responses (exclude `password_hash`)
  - Row Level Security for data isolation


### Blog 09: Migration Strategy
- **Problem**: Manual schema changes, destructive migrations
- **Solutions**:
  - sqlx migrations: `sqlx migrate add`, `sqlx migrate run`
  - Zero-downtime patterns: add column → backfill → add constraint
  - Reversible migrations with `-- DOWN` section


### Blog 10: Diesel Comparison
- **Comparison**: sqlx (SQL-first) vs Diesel (query builder DSL)
- **sqlx**: Native async, raw SQL, compile-time checking via macros
- **Diesel**: Schema auto-generation, type-safe DSL, `diesel-async` for async


### Blog 11: PostgreSQL Features
- **LISTEN/NOTIFY**: Real-time notifications via `PgListener`
- **Advisory Locks**: Distributed locking with `pg_advisory_lock()`
- **RLS**: Row Level Security for tenant isolation
- **JSONB**: `serde_json::Value` mapping for flexible data


### Blog 12: Soft Delete Patterns
- **Problem**: `deleted_at` column causes WHERE clause hell, constraint issues
- **Solutions**:
  - **Newtype Pattern**: Separate `ActiveUser` / `DeletedUser` types
  - **Views**: `CREATE VIEW active_users AS SELECT * FROM users WHERE deleted_at IS NULL`
  - **RLS**: Database-enforced filtering with `current_setting('app.include_deleted')`
  - **Repository Pattern**: Encapsulate filter logic
  - **Macros**: `impl_soft_deletable!` for boilerplate reduction
- **Alternatives**: Archive tables, Temporal Tables, Event Sourcing
- **Best Practice**: Prefer `deleted_at TIMESTAMPTZ` over `is_deleted BOOLEAN`
- **Always**: Create partial index: `CREATE INDEX idx_active ON users(id) WHERE deleted_at IS NULL`


### Blog 13: Fear of the Unknown (NULL Handling)
- **Problem**: `NULL = NULL` returns UNKNOWN, not TRUE
- **Solutions**:
  - Use `IS NULL` / `IS NOT NULL` instead of `= NULL`
  - `COALESCE(value, default)` for fallbacks
  - Rust: `Option<T>` maps directly to nullable columns
- **Three-valued logic**: TRUE / FALSE / UNKNOWN in SQL


### Blog 14: Implicit Columns (SELECT *)
- **Problem**: `SELECT *` is fragile against schema changes
- **Solutions**:
  - Explicit column lists: `SELECT id, name, email FROM users`
  - sqlx `query_as!` enforces column matching at compile time


### Blog 15: Pseudokey Neat Freak
- **Problem**: Obsession with filling ID gaps, reusing deleted IDs
- **Solutions**:
  - Accept gaps as normal (rollbacks, deletes)
  - Use UUID for external/public identifiers
  - SERIAL/BIGSERIAL for internal IDs where sequence matters


### Blog 16: See No Evil (Error Handling)
- **Problem**: `.unwrap()`, ignoring errors, vague error messages
- **Solutions**:
  - Use `Result<T, E>` consistently
  - `thiserror` for custom error types
  - Pattern match on `sqlx::Error` variants (unique violation, FK violation, etc.)
  - Never use `.unwrap()` in production code


### Blog 17: Transaction & Locking
- **Problem**: Lost updates, deadlocks
- **Solutions**:
  - **Atomic updates**: `UPDATE ... SET count = count + 1`
  - **Optimistic locking**: `version` column with conditional update
  - **Pessimistic locking**: `SELECT ... FOR UPDATE`
  - **Deadlock prevention**: Consistent lock ordering


### Blog 18: Diplomatic Immunity
- **Problem**: SQL treated as second-class citizen (no review, no tests)
- **Solutions**:
  - Version control all migrations
  - Code review for schema changes
  - Integration tests for SQL logic
  - CI/CD pipeline for migrations


### Blog 19: Stored Procedures
- **Problem**: All logic in stored procedures OR avoiding them entirely
- **Solutions**:
  - Use for: data-intensive batch operations, complex constraints
  - Avoid for: business logic, validation, external API calls
  - Balance: Rust for business logic, PostgreSQL functions for data operations


### Blog 20: Normalization
- **1NF**: Atomic values, no repeating groups
- **2NF**: No partial dependencies on composite keys
- **3NF**: No transitive dependencies
- **When to denormalize**: Read-heavy queries, calculated aggregates, reporting tables


### Blog 21: EXPLAIN ANALYZE
- **Key metrics**: `actual time`, `rows`, `loops`, `Buffers`
- **Scan types**: Seq Scan (full table) → Index Scan → Index Only Scan
- **Red flags**: Large row estimates vs actual, many loops, missing index usage
- **Always use**: `EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)` for full details


### Blog 22: Integration Testing
- **Problem**: Mocks can't catch DB-specific bugs (constraints, transactions, NULL)
- **Solutions**:
  - `#[sqlx::test]` macro for isolated transactions
  - Fixtures for test data: `#[sqlx::test(fixtures("users"))]`
  - Testcontainers for CI/CD


### Blog 23: CTE & Window Functions
- **CTE**: `WITH ... AS` for readable complex queries, recursive queries for hierarchies
- **Window functions**:
  - `ROW_NUMBER()`, `RANK()`, `DENSE_RANK()` for ordering
  - `LAG()`, `LEAD()` for previous/next row access
  - `SUM() OVER (ORDER BY ...)` for running totals


### Blog 24: Multi-tenant Design
- **Approach 1**: Tenant ID column - simple, requires discipline in every query
- **Approach 2**: Schema separation - strong isolation, complex management
- **Approach 3**: RLS - database-enforced, transparent to application
- **Rust pattern**: `TenantContext` struct passed through request handlers
- **Index strategy**: Always include `tenant_id` in composite indexes
