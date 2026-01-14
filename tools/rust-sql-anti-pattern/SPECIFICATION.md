# Rust + PostgreSQL Webサービス開発 アンチパターン集

本ドキュメントは、RustとPostgreSQLを用いたWebサービス開発において頻出するアンチパターンをまとめたものである。

---

## 第I部 導入

### 1章 アンチパターンとは何か?

アンチパターンとは、一見すると合理的に見えるが、実際には問題を引き起こす設計や実装のパターンである。多くの開発者が同じ落とし穴にはまることから、これらのパターンには名前が付けられ、文書化されてきた。

RustとPostgreSQLの組み合わせは、型安全性とパフォーマンスの両立を可能にする強力な選択肢である。しかし、この組み合わせにおいても、データベース設計やアプリケーション実装において陥りやすいアンチパターンが存在する。

#### 1.1 アンチパターンのタイプ

本ドキュメントでは、以下の4つのカテゴリでアンチパターンを分類する：

| カテゴリ | 説明 |
|---------|------|
| **データベース論理設計** | テーブル構造、リレーションシップ、制約の設計に関する問題 |
| **データベース物理設計** | データ型、インデックス、ストレージに関する問題 |
| **クエリ設計** | SQLクエリの記述方法に関する問題 |
| **アプリケーション開発** | Rustコードとデータベースの統合に関する問題 |

#### 1.2 各章の構成

各アンチパターンは以下の構成で解説する：

1. **目的**: 開発者が達成しようとしていること
2. **アンチパターン**: 問題のあるアプローチとその弊害
3. **見つけ方**: コードレビューや設計レビューでの発見方法
4. **使ってもよい場合**: 例外的に許容されるケース
5. **解決策**: 推奨されるアプローチ

#### 1.3 Rust + PostgreSQL環境の前提

本ドキュメントでは、以下の技術スタックを前提とする：

```toml
# Cargo.toml の典型的な依存関係
[dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
# または
diesel = { version = "2", features = ["postgres"] }
# Webフレームワーク
axum = "0.7"
# または
actix-web = "4"
```

#### 1.4 サンプルデータベース

本ドキュメントで使用するサンプルデータベースのER図：

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    users     │     │    posts     │     │   comments   │
├──────────────┤     ├──────────────┤     ├──────────────┤
│ id (PK)      │────<│ id (PK)      │────<│ id (PK)      │
│ email        │     │ user_id (FK) │     │ post_id (FK) │
│ name         │     │ title        │     │ user_id (FK) │
│ created_at   │     │ content      │     │ body         │
│ updated_at   │     │ status       │     │ created_at   │
└──────────────┘     │ created_at   │     └──────────────┘
                     │ updated_at   │
                     └──────────────┘

┌──────────────┐     ┌──────────────┐
│    tags      │     │  post_tags   │
├──────────────┤     ├──────────────┤
│ id (PK)      │────<│ post_id (FK) │
│ name         │     │ tag_id (FK)  │
│ slug         │     │ (複合PK)     │
└──────────────┘     └──────────────┘
```

対応するマイグレーション：

```sql
-- migrations/001_initial.sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE post_tags (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- インデックス
CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);
```

---

## 第II部 データベース論理設計のアンチパターン

### 2章 ジェイウォーク（信号無視）

#### 2.1 目的：複数の値を持つ属性を格納する

投稿に複数のタグを関連付けたい、ユーザーに複数のロールを割り当てたいなど、1つのエンティティに対して複数の値を持つ属性を格納したいケースは頻繁に発生する。

#### 2.2 アンチパターン：カンマ区切りフォーマットのリストを格納する

**問題のあるスキーマ：**

```sql
-- アンチパターン: タグをカンマ区切りで格納
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,
    tags VARCHAR(500)  -- "rust,postgresql,web" のように格納
);
```

**問題のあるRustコード：**

```rust
#[derive(Debug, sqlx::FromRow)]
struct Post {
    id: Uuid,
    title: String,
    content: String,
    tags: Option<String>,  // カンマ区切り文字列
}

impl Post {
    // タグをパースする必要がある
    fn get_tags(&self) -> Vec<&str> {
        self.tags
            .as_ref()
            .map(|t| t.split(',').collect())
            .unwrap_or_default()
    }
}
```

##### 2.2.1 特定のタグを持つ投稿の検索が困難

```rust
// アンチパターン: LIKEを使った検索
async fn find_posts_by_tag(pool: &PgPool, tag: &str) -> Result<Vec<Post>, sqlx::Error> {
    // "rust" を検索すると "trustworthy" もマッチしてしまう
    sqlx::query_as!(
        Post,
        r#"SELECT * FROM posts WHERE tags LIKE '%' || $1 || '%'"#,
        tag
    )
    .fetch_all(pool)
    .await
}
```

##### 2.2.2 集約クエリの作成が複雑

```rust
// タグごとの投稿数を取得したい場合、純粋なSQLでは困難
// アプリケーション側で処理する必要がある
async fn count_posts_by_tag(pool: &PgPool) -> Result<HashMap<String, i64>, sqlx::Error> {
    let posts: Vec<Post> = sqlx::query_as("SELECT * FROM posts")
        .fetch_all(pool)
        .await?;

    let mut counts: HashMap<String, i64> = HashMap::new();
    for post in posts {
        for tag in post.get_tags() {
            *counts.entry(tag.to_string()).or_insert(0) += 1;
        }
    }
    Ok(counts)
}
// 問題: 全件取得が必要でパフォーマンスが悪い
```

##### 2.2.3 タグの追加・削除が煩雑

```rust
// アンチパターン: 文字列操作でタグを追加
async fn add_tag_to_post(
    pool: &PgPool,
    post_id: Uuid,
    new_tag: &str,
) -> Result<(), sqlx::Error> {
    // 既存のタグを取得
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;

    // 文字列操作でタグを追加
    let new_tags = match post.tags {
        Some(tags) => format!("{},{}", tags, new_tag),
        None => new_tag.to_string(),
    };

    // 更新
    sqlx::query("UPDATE posts SET tags = $1 WHERE id = $2")
        .bind(new_tags)
        .bind(post_id)
        .execute(pool)
        .await?;

    Ok(())
}
// 問題: レースコンディション、重複チェックなし
```

##### 2.2.4 参照整合性が保証できない

```rust
// 存在しないタグ名も格納できてしまう
// タグのマスタ管理が不可能
sqlx::query("UPDATE posts SET tags = 'rust,nonexistent_tag,typo_tga' WHERE id = $1")
    .bind(post_id)
    .execute(pool)
    .await?;
```

#### 2.3 アンチパターンの見つけ方

以下のような兆候がある場合、ジェイウォークの可能性がある：

- `VARCHAR`や`TEXT`カラムに対して`LIKE '%keyword%'`検索を頻繁に行っている
- アプリケーションコードで文字列の`split()`や`join()`を多用している
- カラム名が複数形になっている（`tags`, `roles`, `categories`）
- 「このカラムの最大長をいくつにすべきか」という議論がある

**コードレビューでの発見：**

```rust
// このパターンを見たら警戒
struct Entity {
    // 複数形の名前 + String型 = 危険信号
    tags: String,
    categories: String,
    permissions: Option<String>,
}
```

#### 2.4 アンチパターンを用いてもよい場合

以下の条件を**すべて**満たす場合のみ、カンマ区切りが許容される：

1. データの検索や集約を行わない（表示のみ）
2. 個別の値に対する更新が発生しない
3. 参照整合性が不要
4. パフォーマンスが極めて重要で、JOINのコストを避けたい

実際にはこれらの条件を満たすケースは稀である。

#### 2.5 解決策：交差テーブルを作成する

**正しいスキーマ：**

```sql
-- タグのマスタテーブル
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) NOT NULL UNIQUE
);

-- 交差テーブル（多対多のリレーション）
CREATE TABLE post_tags (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- インデックス（逆方向の検索用）
CREATE INDEX idx_post_tags_tag_id ON post_tags(tag_id);
```

**正しいRustコード：**

```rust
#[derive(Debug, sqlx::FromRow)]
struct Post {
    id: Uuid,
    title: String,
    content: String,
}

#[derive(Debug, sqlx::FromRow)]
struct Tag {
    id: Uuid,
    name: String,
    slug: String,
}

#[derive(Debug)]
struct PostWithTags {
    post: Post,
    tags: Vec<Tag>,
}
```

##### 2.5.1 特定のタグを持つ投稿の検索

```rust
async fn find_posts_by_tag(pool: &PgPool, tag_slug: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT p.id, p.title, p.content
        FROM posts p
        INNER JOIN post_tags pt ON p.id = pt.post_id
        INNER JOIN tags t ON pt.tag_id = t.id
        WHERE t.slug = $1
        "#,
        tag_slug
    )
    .fetch_all(pool)
    .await
}
```

##### 2.5.2 集約クエリの作成

```rust
#[derive(Debug, sqlx::FromRow)]
struct TagCount {
    tag_name: String,
    post_count: i64,
}

async fn count_posts_by_tag(pool: &PgPool) -> Result<Vec<TagCount>, sqlx::Error> {
    sqlx::query_as!(
        TagCount,
        r#"
        SELECT t.name as tag_name, COUNT(pt.post_id) as "post_count!"
        FROM tags t
        LEFT JOIN post_tags pt ON t.id = pt.tag_id
        GROUP BY t.id, t.name
        ORDER BY post_count DESC
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 2.5.3 タグの追加・削除

```rust
async fn add_tag_to_post(
    pool: &PgPool,
    post_id: Uuid,
    tag_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO post_tags (post_id, tag_id)
        VALUES ($1, $2)
        ON CONFLICT (post_id, tag_id) DO NOTHING
        "#,
        post_id,
        tag_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn remove_tag_from_post(
    pool: &PgPool,
    post_id: Uuid,
    tag_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM post_tags WHERE post_id = $1 AND tag_id = $2",
        post_id,
        tag_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

##### 2.5.4 投稿とタグを一緒に取得

```rust
async fn get_post_with_tags(pool: &PgPool, post_id: Uuid) -> Result<PostWithTags, sqlx::Error> {
    let post = sqlx::query_as!(Post, "SELECT id, title, content FROM posts WHERE id = $1", post_id)
        .fetch_one(pool)
        .await?;

    let tags = sqlx::query_as!(
        Tag,
        r#"
        SELECT t.id, t.name, t.slug
        FROM tags t
        INNER JOIN post_tags pt ON t.id = pt.tag_id
        WHERE pt.post_id = $1
        "#,
        post_id
    )
    .fetch_all(pool)
    .await?;

    Ok(PostWithTags { post, tags })
}
```

#### 2.6 PostgreSQL配列型の検討

PostgreSQLには配列型があり、これを使う選択肢もある：

```sql
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    tags TEXT[] DEFAULT '{}'
);

-- GINインデックスで検索を高速化
CREATE INDEX idx_posts_tags ON posts USING GIN(tags);
```

```rust
async fn find_posts_by_tag(pool: &PgPool, tag: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE $1 = ANY(tags)",
        tag
    )
    .fetch_all(pool)
    .await
}
```

**配列型の利点と欠点：**

| 利点 | 欠点 |
|------|------|
| JOINが不要でクエリがシンプル | 参照整合性が保証できない |
| 単一テーブルで完結 | タグのメタデータ（作成日など）を持てない |
| GINインデックスで検索可能 | タグの一覧取得に`UNNEST`が必要 |

配列型は、参照整合性が不要で、値のメタデータが必要ない場合に検討する価値がある。

---

### 3章 ナイーブツリー（素朴な木）

#### 3.1 目的：階層構造を格納し、クエリを実行する

コメントへの返信、組織図、カテゴリの親子関係など、階層構造（ツリー構造）をデータベースに格納し、効率的にクエリを実行したいケースは多い。

#### 3.2 アンチパターン：常に親のみに依存する（隣接リスト）

最もシンプルで直感的なアプローチは、各行に親への参照を持たせる「隣接リスト」である。

**問題のあるスキーマ：**

```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id),
    parent_id UUID REFERENCES comments(id),  -- 親コメントへの参照
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**問題のあるRustコード：**

```rust
#[derive(Debug, sqlx::FromRow)]
struct Comment {
    id: Uuid,
    post_id: Uuid,
    parent_id: Option<Uuid>,
    user_id: Uuid,
    body: String,
    created_at: DateTime<Utc>,
}
```

##### 3.2.1 子孫の取得が困難

```rust
// アンチパターン: 再帰的にクエリを実行
async fn get_all_descendants(
    pool: &PgPool,
    comment_id: Uuid,
) -> Result<Vec<Comment>, sqlx::Error> {
    let mut all_descendants = Vec::new();
    let mut current_ids = vec![comment_id];

    // 深さごとにクエリを実行する必要がある
    while !current_ids.is_empty() {
        let children: Vec<Comment> = sqlx::query_as(
            "SELECT * FROM comments WHERE parent_id = ANY($1)"
        )
        .bind(&current_ids)
        .fetch_all(pool)
        .await?;

        current_ids = children.iter().map(|c| c.id).collect();
        all_descendants.extend(children);
    }

    Ok(all_descendants)
}
// 問題: N+1クエリ、ツリーの深さ分のラウンドトリップが発生
```

##### 3.2.2 ツリー全体の取得とメモリでの構築

```rust
// アンチパターン: 全件取得してメモリで構築
#[derive(Debug)]
struct CommentTree {
    comment: Comment,
    children: Vec<CommentTree>,
}

async fn build_comment_tree(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<CommentTree>, sqlx::Error> {
    // 全コメントを取得
    let comments: Vec<Comment> = sqlx::query_as(
        "SELECT * FROM comments WHERE post_id = $1"
    )
    .bind(post_id)
    .fetch_all(pool)
    .await?;

    // メモリ上でツリーを構築（複雑なロジック）
    fn build_tree(comments: &[Comment], parent_id: Option<Uuid>) -> Vec<CommentTree> {
        comments
            .iter()
            .filter(|c| c.parent_id == parent_id)
            .map(|c| CommentTree {
                comment: c.clone(),
                children: build_tree(comments, Some(c.id)),
            })
            .collect()
    }

    Ok(build_tree(&comments, None))
}
// 問題: 大量のコメントがある場合にメモリを圧迫
```

##### 3.2.3 サブツリーの削除が複雑

```rust
// アンチパターン: 再帰的に削除
async fn delete_comment_tree(
    pool: &PgPool,
    comment_id: Uuid,
) -> Result<(), sqlx::Error> {
    // まず子孫を全て取得
    let descendants = get_all_descendants(pool, comment_id).await?;

    // 逆順で削除（葉から）
    for comment in descendants.into_iter().rev() {
        sqlx::query("DELETE FROM comments WHERE id = $1")
            .bind(comment.id)
            .execute(pool)
            .await?;
    }

    // 最後に自分自身を削除
    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(pool)
        .await?;

    Ok(())
}
// 注: ON DELETE CASCADEを使えば不要だが、ソフトデリートでは問題になる
```

#### 3.3 アンチパターンの見つけ方

- `parent_id`カラムのみで階層を表現している
- 「全ての子孫を取得する」クエリがアプリケーションコードで再帰的に実装されている
- ツリーの深さに制限を設けている（「最大5階層まで」など）
- 階層データの表示が遅い

#### 3.4 アンチパターンを用いてもよい場合

以下の場合は隣接リストで十分：

1. 直接の親子関係のみを取得する
2. ツリーの深さが浅い（2-3階層）
3. PostgreSQLの再帰CTE（`WITH RECURSIVE`）を使用できる

#### 3.5 解決策：代替ツリーモデルを使用する

##### 3.5.1 再帰CTE（PostgreSQLの機能）

PostgreSQLの`WITH RECURSIVE`を使えば、隣接リストでも効率的にツリーを取得できる：

```rust
async fn get_comment_tree_with_depth(
    pool: &PgPool,
    root_comment_id: Uuid,
) -> Result<Vec<CommentWithDepth>, sqlx::Error> {
    sqlx::query_as!(
        CommentWithDepth,
        r#"
        WITH RECURSIVE comment_tree AS (
            -- ベースケース: ルートコメント
            SELECT id, parent_id, body, user_id, created_at, 0 as depth
            FROM comments
            WHERE id = $1

            UNION ALL

            -- 再帰ケース: 子コメント
            SELECT c.id, c.parent_id, c.body, c.user_id, c.created_at, ct.depth + 1
            FROM comments c
            INNER JOIN comment_tree ct ON c.parent_id = ct.id
        )
        SELECT id, parent_id, body, user_id, created_at, depth as "depth!"
        FROM comment_tree
        ORDER BY depth, created_at
        "#,
        root_comment_id
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
struct CommentWithDepth {
    id: Uuid,
    parent_id: Option<Uuid>,
    body: String,
    user_id: Uuid,
    created_at: DateTime<Utc>,
    depth: i32,
}
```

##### 3.5.2 経路列挙（Path Enumeration）

各ノードに、ルートからのパスを格納する：

```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id),
    path TEXT NOT NULL,  -- '/uuid1/uuid2/uuid3' のような形式
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comments_path ON comments USING btree(path text_pattern_ops);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct Comment {
    id: Uuid,
    post_id: Uuid,
    path: String,
    user_id: Uuid,
    body: String,
    created_at: DateTime<Utc>,
}

impl Comment {
    fn depth(&self) -> usize {
        self.path.matches('/').count() - 1
    }

    fn parent_id(&self) -> Option<Uuid> {
        let parts: Vec<&str> = self.path.trim_matches('/').split('/').collect();
        if parts.len() > 1 {
            Uuid::parse_str(parts[parts.len() - 2]).ok()
        } else {
            None
        }
    }
}

// 子孫の取得が簡単
async fn get_descendants(pool: &PgPool, comment: &Comment) -> Result<Vec<Comment>, sqlx::Error> {
    sqlx::query_as!(
        Comment,
        "SELECT * FROM comments WHERE path LIKE $1 || '%' AND id != $2",
        comment.path,
        comment.id
    )
    .fetch_all(pool)
    .await
}

// コメント作成時にパスを設定
async fn create_reply(
    pool: &PgPool,
    parent: &Comment,
    user_id: Uuid,
    body: &str,
) -> Result<Comment, sqlx::Error> {
    let new_id = Uuid::new_v4();
    let new_path = format!("{}{}/", parent.path, new_id);

    sqlx::query_as!(
        Comment,
        r#"
        INSERT INTO comments (id, post_id, path, user_id, body)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
        new_id,
        parent.post_id,
        new_path,
        user_id,
        body
    )
    .fetch_one(pool)
    .await
}
```

##### 3.5.3 閉包テーブル（Closure Table）

全ての祖先-子孫関係を別テーブルに格納する：

```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id),
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 閉包テーブル
CREATE TABLE comment_tree_paths (
    ancestor_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    descendant_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    depth INT NOT NULL DEFAULT 0,
    PRIMARY KEY (ancestor_id, descendant_id)
);

CREATE INDEX idx_ctp_descendant ON comment_tree_paths(descendant_id);
```

```rust
// 子孫の取得
async fn get_descendants(
    pool: &PgPool,
    comment_id: Uuid,
) -> Result<Vec<CommentWithDepth>, sqlx::Error> {
    sqlx::query_as!(
        CommentWithDepth,
        r#"
        SELECT c.*, ctp.depth as "depth!"
        FROM comments c
        INNER JOIN comment_tree_paths ctp ON c.id = ctp.descendant_id
        WHERE ctp.ancestor_id = $1 AND ctp.depth > 0
        ORDER BY ctp.depth
        "#,
        comment_id
    )
    .fetch_all(pool)
    .await
}

// 新しいコメントの追加
async fn create_reply(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    // コメントを作成
    let new_id: Uuid = sqlx::query_scalar(
        "INSERT INTO comments (post_id, user_id, body) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(post_id)
    .bind(user_id)
    .bind(body)
    .fetch_one(&mut **tx)
    .await?;

    // 閉包テーブルに関係を追加
    sqlx::query(
        r#"
        INSERT INTO comment_tree_paths (ancestor_id, descendant_id, depth)
        SELECT ancestor_id, $1, depth + 1
        FROM comment_tree_paths
        WHERE descendant_id = $2
        UNION ALL
        SELECT $1, $1, 0
        "#
    )
    .bind(new_id)
    .bind(parent_id)
    .execute(&mut **tx)
    .await?;

    Ok(new_id)
}
```

##### 3.5.4 どの設計を使うべきか

| モデル | 子孫取得 | 祖先取得 | 挿入 | 削除 | 参照整合性 |
|--------|---------|---------|------|------|-----------|
| 隣接リスト + 再帰CTE | ○ | ○ | ◎ | ○ | ◎ |
| 経路列挙 | ◎ | ○ | ○ | △ | △ |
| 閉包テーブル | ◎ | ◎ | ○ | ○ | ◎ |

**推奨：**
- 読み取りが多い場合: **閉包テーブル**
- 書き込みが多い場合: **隣接リスト + 再帰CTE**
- シンプルさを優先: **経路列挙**

---

### 4章 IDリクワイアド（とりあえずID）

#### 4.1 目的：主キーの規約を確立する

すべてのテーブルに主キーを持たせ、行を一意に識別できるようにしたい。これはリレーショナルデータベースの基本原則である。

#### 4.2 アンチパターン：すべてのテーブルに「id」列を用いる

「すべてのテーブルには`id`という名前の自動採番主キーを持たせる」という規約を盲目的に適用してしまうケース。

**問題のあるスキーマ：**

```sql
-- アンチパターン: 交差テーブルにも不要なidを追加
CREATE TABLE post_tags (
    id SERIAL PRIMARY KEY,  -- 不要
    post_id UUID NOT NULL REFERENCES posts(id),
    tag_id UUID NOT NULL REFERENCES tags(id)
);

-- 本来は複合主キーで十分
-- PRIMARY KEY (post_id, tag_id)
```

##### 4.2.1 冗長なキーが作成されてしまう

```rust
#[derive(Debug, sqlx::FromRow)]
struct PostTag {
    id: i32,          // このidは使われない
    post_id: Uuid,
    tag_id: Uuid,
}

// 実際のクエリでは複合キーで操作する
async fn remove_tag(pool: &PgPool, post_id: Uuid, tag_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM post_tags WHERE post_id = $1 AND tag_id = $2")
        .bind(post_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}
// idは一度も使われていない
```

##### 4.2.2 重複行を許可してしまう

```sql
-- id があると、同じ post_id と tag_id の組み合わせが複数回挿入できてしまう
INSERT INTO post_tags (post_id, tag_id) VALUES ('uuid1', 'uuid2');
INSERT INTO post_tags (post_id, tag_id) VALUES ('uuid1', 'uuid2');  -- 許可されてしまう！
```

```rust
// UNIQUEインデックスを別途追加する必要がある
// CREATE UNIQUE INDEX idx_post_tags_unique ON post_tags(post_id, tag_id);
```

##### 4.2.3 キーの意味がわかりにくくなる

```rust
// アンチパターン: すべてのテーブルで "id" を使用
async fn get_user_posts(pool: &PgPool, user_id: Uuid) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        // どのテーブルの id かわかりにくい
        "SELECT p.id, p.title FROM posts p WHERE p.id IN (
            SELECT post_id FROM user_favorites WHERE user_id = $1
        )",
        user_id
    )
    .fetch_all(pool)
    .await
}
```

##### 4.2.4 USINGを使用できない

```sql
-- "id" という名前だと USING が使えない
-- これはエラーになる
SELECT * FROM posts JOIN users USING (id);

-- 正しい列名なら USING が使える
SELECT * FROM posts JOIN users USING (user_id);
```

##### 4.2.5 自然キーの無視

```sql
-- アンチパターン: 自然キーがあるのにサロゲートキーを追加
CREATE TABLE countries (
    id SERIAL PRIMARY KEY,      -- 不要
    country_code CHAR(2) UNIQUE NOT NULL,  -- これで十分
    name VARCHAR(100) NOT NULL
);

-- 正しいアプローチ
CREATE TABLE countries (
    country_code CHAR(2) PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);
```

#### 4.3 アンチパターンの見つけ方

- すべてのテーブルに`id`という名前のカラムがある
- 交差テーブル（多対多の中間テーブル）にもサロゲートキーがある
- `UNIQUE`制約を別途追加している交差テーブルがある
- 自然キー（ISBN、国コード、メールアドレスなど）があるのにサロゲートキーを使用している

#### 4.4 アンチパターンを用いてもよい場合

1. **ORMの制約**: 一部のORMは単一の主キーを要求する
2. **将来の柔軟性**: 複合キーの構成が変わる可能性がある場合
3. **外部キーの簡素化**: 多くのテーブルから参照される場合

#### 4.5 解決策：状況に応じて適切に調整する

##### 4.5.1 わかりやすい列名にする

```sql
-- 推奨: テーブル名を接頭辞として使用
CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE
);

CREATE TABLE posts (
    post_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id),
    title VARCHAR(200) NOT NULL
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct User {
    user_id: Uuid,
    email: String,
}

#[derive(Debug, sqlx::FromRow)]
struct Post {
    post_id: Uuid,
    user_id: Uuid,  // どのテーブルの外部キーか明確
    title: String,
}

// JOINもわかりやすい
async fn get_posts_with_users(pool: &PgPool) -> Result<Vec<PostWithUser>, sqlx::Error> {
    sqlx::query_as!(
        PostWithUser,
        r#"
        SELECT p.post_id, p.title, u.user_id, u.email
        FROM posts p
        JOIN users u ON p.user_id = u.user_id
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 4.5.2 複合キーを活用する

```sql
-- 交差テーブルは複合主キーを使用
CREATE TABLE post_tags (
    post_id UUID NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (post_id, tag_id)
);
```

```rust
// Rustでは複合キーをタプルで表現
async fn add_tag(
    pool: &PgPool,
    post_id: Uuid,
    tag_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO post_tags (post_id, tag_id)
        VALUES ($1, $2)
        ON CONFLICT (post_id, tag_id) DO NOTHING
        "#,
        post_id,
        tag_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

##### 4.5.3 自然キーの活用

```rust
// 国コードは自然キー
#[derive(Debug, sqlx::FromRow)]
struct Country {
    country_code: String,  // "JP", "US" など
    name: String,
}

// 外部キーとして使用
#[derive(Debug, sqlx::FromRow)]
struct Address {
    address_id: Uuid,
    country_code: String,  // 外部キー
    city: String,
}
```

##### 4.5.4 UUIDとSERIALの使い分け

```rust
// UUID: 分散システム、外部公開するID
// SERIAL/BIGSERIAL: 内部的なID、順序が意味を持つ場合

// UUID推奨のケース
#[derive(Debug, sqlx::FromRow)]
struct User {
    user_id: Uuid,  // APIで公開される
}

// SERIAL推奨のケース
#[derive(Debug, sqlx::FromRow)]
struct AuditLog {
    log_id: i64,  // 順序が重要、内部使用のみ
    action: String,
    created_at: DateTime<Utc>,
}
```

#### 4.6 Rust/sqlxでの実装パターン

```rust
// 複合主キーを持つエンティティの操作
#[derive(Debug)]
struct PostTagKey {
    post_id: Uuid,
    tag_id: Uuid,
}

impl PostTagKey {
    async fn exists(&self, pool: &PgPool) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM post_tags WHERE post_id = $1 AND tag_id = $2)",
            self.post_id,
            self.tag_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    async fn delete(&self, pool: &PgPool) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM post_tags WHERE post_id = $1 AND tag_id = $2",
            self.post_id,
            self.tag_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
```

---

### 5章 キーレスエントリ（外部キー嫌い）

#### 5.1 目的：データベースのアーキテクチャを単純化する

パフォーマンスの向上やスキーマの柔軟性を求めて、データベースの制約を減らしたいと考えることがある。

#### 5.2 アンチパターン：外部キー制約を使用しない

「外部キー制約はパフォーマンスに悪影響を与える」「アプリケーション側で整合性を管理すればよい」という誤った信念から、外部キー制約を省略してしまうケース。

**問題のあるスキーマ：**

```sql
-- アンチパターン: 外部キー制約なし
CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,  -- REFERENCES users(id) がない
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL
);

CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL,  -- REFERENCES posts(id) がない
    user_id UUID NOT NULL,  -- REFERENCES users(id) がない
    body TEXT NOT NULL
);
```

##### 5.2.1 孤立データの発生

```rust
// ユーザーを削除しても、投稿は残り続ける
async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
// 問題: このユーザーの投稿やコメントが孤立データとして残る
```

```rust
// 存在しないユーザーの投稿を作成できてしまう
async fn create_post(
    pool: &PgPool,
    user_id: Uuid,  // 存在確認なし
    title: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO posts (user_id, title, content) VALUES ($1, $2, '') RETURNING id",
        user_id,
        title
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}
// 問題: user_id が存在しなくても挿入できてしまう
```

##### 5.2.2 アプリケーション側での整合性チェックが必要

```rust
// アンチパターン: アプリケーション側で参照整合性を確認
async fn create_post_with_validation(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    content: &str,
) -> Result<Uuid, anyhow::Error> {
    // まずユーザーの存在確認
    let user_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
        user_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    if !user_exists {
        return Err(anyhow::anyhow!("User not found"));
    }

    // 投稿を作成
    let post_id = sqlx::query_scalar!(
        "INSERT INTO posts (user_id, title, content) VALUES ($1, $2, $3) RETURNING id",
        user_id,
        title,
        content
    )
    .fetch_one(pool)
    .await?;

    Ok(post_id)
}
// 問題:
// 1. レースコンディション（確認後にユーザーが削除される可能性）
// 2. 全ての挿入箇所でチェックが必要（漏れの可能性）
// 3. パフォーマンス低下（追加のクエリ）
```

##### 5.2.3 クリーンアップスクリプトが必要になる

```rust
// アンチパターン: 定期的に孤立データを削除
async fn cleanup_orphaned_data(pool: &PgPool) -> Result<CleanupResult, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 孤立した投稿を削除
    let orphaned_posts = sqlx::query!(
        "DELETE FROM posts WHERE user_id NOT IN (SELECT id FROM users) RETURNING id"
    )
    .fetch_all(&mut *tx)
    .await?;

    // 孤立したコメントを削除
    let orphaned_comments = sqlx::query!(
        r#"
        DELETE FROM comments
        WHERE post_id NOT IN (SELECT id FROM posts)
           OR user_id NOT IN (SELECT id FROM users)
        RETURNING id
        "#
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(CleanupResult {
        posts_deleted: orphaned_posts.len(),
        comments_deleted: orphaned_comments.len(),
    })
}
// 問題: 孤立データが発生してから削除されるまでの間、不整合な状態が続く
```

##### 5.2.4 デバッグの困難さ

```rust
// 孤立データによる謎のエラー
async fn get_post_with_author(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<PostWithAuthor, sqlx::Error> {
    // 投稿は見つかるが...
    let post = sqlx::query_as!(Post, "SELECT * FROM posts WHERE id = $1", post_id)
        .fetch_one(pool)
        .await?;

    // ユーザーが見つからない！
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", post.user_id)
        .fetch_optional(pool)
        .await?;

    match user {
        Some(u) => Ok(PostWithAuthor { post, author: u }),
        None => {
            // なぜユーザーがいないのかわからない
            // 外部キー制約があればこの状態は発生しない
            Err(sqlx::Error::RowNotFound)
        }
    }
}
```

#### 5.3 アンチパターンの見つけ方

- スキーマに`REFERENCES`や`FOREIGN KEY`が見当たらない
- 「孤立データ」「orphan」「cleanup」といったコードやスクリプトがある
- 挿入前に「存在確認」のクエリを実行している
- JOINの結果で片方のデータが`NULL`になることがある（INNER JOINで結果が減る）

#### 5.4 アンチパターンを用いてもよい場合

1. **極端なパフォーマンス要件**: 書き込みが極めて多く、外部キーチェックのオーバーヘッドが許容できない
2. **分散データベース**: 一部の分散DBは外部キーをサポートしない
3. **アーカイブテーブル**: 参照先が削除されても保持したい履歴データ
4. **一時テーブル**: バッチ処理用の一時的なテーブル

ただし、これらのケースでも代替の整合性確保手段を検討すべきである。

#### 5.5 解決策：外部キー制約を宣言する

##### 5.5.1 基本的な外部キー制約

```sql
CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- コメントテーブル
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id),
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

##### 5.5.2 ON DELETE オプションの活用

```sql
-- CASCADE: 親が削除されたら子も削除
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL
);

-- SET NULL: 親が削除されたらNULLに設定
CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,  -- NULLableに変更
    title VARCHAR(200) NOT NULL
);

-- RESTRICT: 子が存在する場合は親の削除を禁止（デフォルト）
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);

CREATE TABLE products (
    id UUID PRIMARY KEY,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE RESTRICT
);
```

```rust
// CASCADEを使えば、削除が簡単に
async fn delete_post(pool: &PgPool, post_id: Uuid) -> Result<(), sqlx::Error> {
    // コメントも自動的に削除される
    sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;
    Ok(())
}

// RESTRICTでは、子が存在するとエラーになる
async fn delete_category(pool: &PgPool, category_id: Uuid) -> Result<(), sqlx::Error> {
    match sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(category_id)
        .execute(pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
            Err(sqlx::Error::Database(e))
            // または適切なビジネスエラーに変換
        }
        Err(e) => Err(e),
    }
}
```

##### 5.5.3 遅延制約（DEFERRABLE）

```sql
-- トランザクション終了時に制約をチェック
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) DEFERRABLE INITIALLY DEFERRED
);
```

```rust
// 順序を気にせず挿入できる
async fn create_user_with_post(
    pool: &PgPool,
    user: NewUser,
    post: NewPost,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 遅延制約なら、先にpostを挿入することも可能
    // （通常は制約エラーになる）
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *tx)
        .await?;

    let user_id = Uuid::new_v4();

    // 先にpostを挿入
    sqlx::query("INSERT INTO posts (id, user_id, title) VALUES ($1, $2, $3)")
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&post.title)
        .execute(&mut *tx)
        .await?;

    // 後からuserを挿入
    sqlx::query("INSERT INTO users (id, email, name) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&user.email)
        .bind(&user.name)
        .execute(&mut *tx)
        .await?;

    // トランザクションコミット時に制約チェック
    tx.commit().await?;
    Ok(())
}
```

##### 5.5.4 外部キー制約のインデックス

```sql
-- 外部キーには必ずインデックスを作成する
CREATE TABLE comments (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL
);

-- PostgreSQLは外部キーに自動でインデックスを作成しない
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);
```

#### 5.6 外部キー制約のパフォーマンス

外部キー制約のオーバーヘッドは通常無視できるレベルである：

| 操作 | オーバーヘッド | 備考 |
|------|--------------|------|
| INSERT | 親の存在確認（インデックス参照） | 数マイクロ秒 |
| UPDATE（FK列） | 新しい親の存在確認 | 数マイクロ秒 |
| DELETE | 子の存在確認 | インデックスがあれば高速 |

インデックスが適切に設定されていれば、外部キー制約による遅延は問題にならない。

---

### 6章 EAV（エンティティ・アトリビュート・バリュー）

#### 6.1 目的：可変属性をサポートする

製品カタログのように、カテゴリごとに異なる属性を持つデータを格納したい。例えば、書籍には「著者」「ISBN」があり、電子機器には「消費電力」「保証期間」がある。

#### 6.2 アンチパターン：汎用的な属性テーブルを使用する

Entity-Attribute-Value（EAV）パターンは、属性名と値をキーバリュー形式で格納する設計である。

**問題のあるスキーマ：**

```sql
-- アンチパターン: EAVテーブル
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL
);

CREATE TABLE product_attributes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    attribute_name VARCHAR(100) NOT NULL,
    attribute_value TEXT,
    UNIQUE (product_id, attribute_name)
);
```

**問題のあるRustコード：**

```rust
#[derive(Debug, sqlx::FromRow)]
struct ProductAttribute {
    id: Uuid,
    product_id: Uuid,
    attribute_name: String,
    attribute_value: Option<String>,
}

// 製品の属性を取得
async fn get_product_attributes(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<HashMap<String, String>, sqlx::Error> {
    let attrs: Vec<ProductAttribute> = sqlx::query_as(
        "SELECT * FROM product_attributes WHERE product_id = $1"
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;

    Ok(attrs
        .into_iter()
        .filter_map(|a| a.attribute_value.map(|v| (a.attribute_name, v)))
        .collect())
}
```

##### 6.2.1 データ型の整合性がない

```rust
// アンチパターン: すべての値がTEXT型
async fn set_product_price(
    pool: &PgPool,
    product_id: Uuid,
    price: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO product_attributes (product_id, attribute_name, attribute_value)
        VALUES ($1, 'price', $2)
        ON CONFLICT (product_id, attribute_name) DO UPDATE SET attribute_value = $2
        "#
    )
    .bind(product_id)
    .bind(price.to_string())  // 数値を文字列に変換
    .execute(pool)
    .await?;
    Ok(())
}

// 問題: 不正な値も格納できてしまう
// "not_a_number" も価格として保存できる
```

```rust
// 取得時に型変換が必要
async fn get_product_price(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<Option<f64>, anyhow::Error> {
    let result: Option<String> = sqlx::query_scalar(
        "SELECT attribute_value FROM product_attributes
         WHERE product_id = $1 AND attribute_name = 'price'"
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some(s) => Ok(Some(s.parse::<f64>()?)),  // パースエラーの可能性
        None => Ok(None),
    }
}
```

##### 6.2.2 必須属性の強制ができない

```sql
-- EAVでは NOT NULL 制約を属性ごとに設定できない
-- 必須属性がなくても製品を作成できてしまう
INSERT INTO products (id, name) VALUES (gen_random_uuid(), 'Unknown Product');
-- price属性がなくても問題なく挿入される
```

##### 6.2.3 行の再構築が複雑

```rust
// アンチパターン: ピボットクエリ
async fn get_products_with_attrs(pool: &PgPool) -> Result<Vec<ProductRow>, sqlx::Error> {
    sqlx::query_as!(
        ProductRow,
        r#"
        SELECT
            p.id,
            p.name,
            MAX(CASE WHEN pa.attribute_name = 'price' THEN pa.attribute_value END) as price,
            MAX(CASE WHEN pa.attribute_name = 'weight' THEN pa.attribute_value END) as weight,
            MAX(CASE WHEN pa.attribute_name = 'color' THEN pa.attribute_value END) as color
        FROM products p
        LEFT JOIN product_attributes pa ON p.id = pa.product_id
        GROUP BY p.id, p.name
        "#
    )
    .fetch_all(pool)
    .await
}
// 問題:
// 1. 新しい属性を追加するたびにクエリを修正する必要がある
// 2. パフォーマンスが悪い（属性数に比例してJOINが遅くなる）
```

##### 6.2.4 集約クエリが困難

```rust
// 価格の平均を計算（EAVでは複雑）
async fn get_average_price(pool: &PgPool) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT AVG(attribute_value::NUMERIC) as "avg!"
        FROM product_attributes
        WHERE attribute_name = 'price'
          AND attribute_value ~ '^[0-9]+\.?[0-9]*$'  -- 数値のみフィルタ
        "#
    )
    .fetch_one(pool)
    .await
}
// 問題: 型変換のオーバーヘッド、不正なデータのスキップ
```

#### 6.3 アンチパターンの見つけ方

- `attribute_name`と`attribute_value`のようなカラムがある
- 値の型変換を頻繁に行っている
- CASE式やピボットクエリが多用されている
- 「どんな属性でも追加できる」という要件がある

#### 6.4 アンチパターンを用いてもよい場合

1. **属性が本当に動的**: ユーザーが自由に属性を定義できるシステム
2. **属性の検索が不要**: 単純な表示のみで、属性での検索やフィルタリングがない
3. **プロトタイピング**: 要件が固まっていない初期段階

#### 6.5 解決策：サブタイプのモデリングを行う

##### 6.5.1 シングルテーブル継承（STI）

すべてのサブタイプを1つのテーブルに格納し、タイプ識別列で区別する：

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_type VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    -- 書籍用
    author VARCHAR(100),
    isbn VARCHAR(13),
    page_count INT,
    -- 電子機器用
    power_consumption INT,
    warranty_months INT,
    -- 衣類用
    size VARCHAR(10),
    color VARCHAR(50),
    material VARCHAR(100)
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    product_type: String,
    name: String,
    price: BigDecimal,
    // 書籍用
    author: Option<String>,
    isbn: Option<String>,
    page_count: Option<i32>,
    // 電子機器用
    power_consumption: Option<i32>,
    warranty_months: Option<i32>,
    // 衣類用
    size: Option<String>,
    color: Option<String>,
    material: Option<String>,
}

// 型安全な列挙型に変換
enum Product {
    Book(Book),
    Electronics(Electronics),
    Clothing(Clothing),
}

impl TryFrom<ProductRow> for Product {
    type Error = anyhow::Error;

    fn try_from(row: ProductRow) -> Result<Self, Self::Error> {
        match row.product_type.as_str() {
            "book" => Ok(Product::Book(Book {
                id: row.id,
                name: row.name,
                price: row.price,
                author: row.author.ok_or_else(|| anyhow::anyhow!("Missing author"))?,
                isbn: row.isbn,
                page_count: row.page_count,
            })),
            "electronics" => Ok(Product::Electronics(Electronics {
                id: row.id,
                name: row.name,
                price: row.price,
                power_consumption: row.power_consumption,
                warranty_months: row.warranty_months.unwrap_or(12),
            })),
            _ => Err(anyhow::anyhow!("Unknown product type")),
        }
    }
}
```

##### 6.5.2 具象テーブル継承（CTI）

サブタイプごとに完全なテーブルを作成する：

```sql
CREATE TABLE books (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    author VARCHAR(100) NOT NULL,
    isbn VARCHAR(13),
    page_count INT
);

CREATE TABLE electronics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    power_consumption INT,
    warranty_months INT NOT NULL DEFAULT 12
);

CREATE TABLE clothing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    size VARCHAR(10) NOT NULL,
    color VARCHAR(50),
    material VARCHAR(100)
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct Book {
    id: Uuid,
    name: String,
    price: BigDecimal,
    author: String,
    isbn: Option<String>,
    page_count: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
struct Electronics {
    id: Uuid,
    name: String,
    price: BigDecimal,
    power_consumption: Option<i32>,
    warranty_months: i32,
}

// すべての製品を検索する場合はUNIONが必要
async fn search_all_products(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<ProductSummary>, sqlx::Error> {
    sqlx::query_as!(
        ProductSummary,
        r#"
        SELECT id, name, price, 'book' as "product_type!" FROM books WHERE name ILIKE $1
        UNION ALL
        SELECT id, name, price, 'electronics' as "product_type!" FROM electronics WHERE name ILIKE $1
        UNION ALL
        SELECT id, name, price, 'clothing' as "product_type!" FROM clothing WHERE name ILIKE $1
        "#,
        format!("%{}%", query)
    )
    .fetch_all(pool)
    .await
}
```

##### 6.5.3 クラステーブル継承

共通属性を基底テーブルに、サブタイプ固有の属性を別テーブルに格納する：

```sql
-- 基底テーブル
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_type VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- サブタイプテーブル
CREATE TABLE book_details (
    product_id UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    author VARCHAR(100) NOT NULL,
    isbn VARCHAR(13),
    page_count INT
);

CREATE TABLE electronics_details (
    product_id UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    power_consumption INT,
    warranty_months INT NOT NULL DEFAULT 12
);
```

```rust
// 書籍の取得
async fn get_book(pool: &PgPool, product_id: Uuid) -> Result<BookProduct, sqlx::Error> {
    sqlx::query_as!(
        BookProduct,
        r#"
        SELECT
            p.id, p.name, p.price, p.created_at,
            bd.author, bd.isbn, bd.page_count
        FROM products p
        INNER JOIN book_details bd ON p.id = bd.product_id
        WHERE p.id = $1
        "#,
        product_id
    )
    .fetch_one(pool)
    .await
}

// 書籍の作成（トランザクション使用）
async fn create_book(
    pool: &PgPool,
    book: NewBook,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let product_id = sqlx::query_scalar!(
        r#"
        INSERT INTO products (product_type, name, price)
        VALUES ('book', $1, $2)
        RETURNING id
        "#,
        book.name,
        book.price
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO book_details (product_id, author, isbn, page_count)
        VALUES ($1, $2, $3, $4)
        "#,
        product_id,
        book.author,
        book.isbn,
        book.page_count
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(product_id)
}
```

##### 6.5.4 PostgreSQLのJSONB型を活用する

半構造化データにはJSONB型が有効：

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_type VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- JSONB用のGINインデックス
CREATE INDEX idx_products_attributes ON products USING GIN(attributes);
```

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, sqlx::FromRow)]
struct Product {
    id: Uuid,
    product_type: String,
    name: String,
    price: BigDecimal,
    attributes: Value,  // serde_json::Value
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BookAttributes {
    author: String,
    isbn: Option<String>,
    page_count: Option<i32>,
}

// 書籍の作成
async fn create_book(pool: &PgPool, book: NewBook) -> Result<Uuid, sqlx::Error> {
    let attributes = serde_json::to_value(BookAttributes {
        author: book.author,
        isbn: book.isbn,
        page_count: book.page_count,
    })?;

    sqlx::query_scalar!(
        r#"
        INSERT INTO products (product_type, name, price, attributes)
        VALUES ('book', $1, $2, $3)
        RETURNING id
        "#,
        book.name,
        book.price,
        attributes
    )
    .fetch_one(pool)
    .await
}

// 著者で検索
async fn find_books_by_author(
    pool: &PgPool,
    author: &str,
) -> Result<Vec<Product>, sqlx::Error> {
    sqlx::query_as!(
        Product,
        r#"
        SELECT * FROM products
        WHERE product_type = 'book'
          AND attributes->>'author' ILIKE $1
        "#,
        format!("%{}%", author)
    )
    .fetch_all(pool)
    .await
}
```

##### 6.5.5 どの設計を選ぶべきか

| 設計 | 長所 | 短所 | 適用場面 |
|------|------|------|----------|
| シングルテーブル継承 | シンプル、JOIN不要 | NULL列が多い | サブタイプが少ない |
| 具象テーブル継承 | 型安全、NULL列なし | 共通検索が複雑 | サブタイプが独立 |
| クラステーブル継承 | 柔軟、正規化 | JOINが必要 | 共通属性が多い |
| JSONB | 最も柔軟 | 型安全性が弱い | 属性が動的 |

---

### 7章 ポリモーフィック関連

#### 7.1 目的：複数の親テーブルを参照する

コメントを投稿にも、画像にも、動画にも付けられるようにしたい。つまり、1つのテーブルが複数の異なる親テーブルを参照する設計が必要になる。

#### 7.2 アンチパターン：二重目的の外部キーを使用する

親の種類を示す列と親のIDを示す列を組み合わせて、複数の親テーブルを参照しようとするパターン。

**問題のあるスキーマ：**

```sql
-- アンチパターン: ポリモーフィック関連
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 参照先のタイプを文字列で指定
    commentable_type VARCHAR(50) NOT NULL,  -- 'Post', 'Image', 'Video' など
    commentable_id UUID NOT NULL,            -- 参照先のID
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 問題: commentable_id に外部キー制約を設定できない
```

**問題のあるRustコード：**

```rust
#[derive(Debug, sqlx::FromRow)]
struct Comment {
    id: Uuid,
    commentable_type: String,
    commentable_id: Uuid,
    user_id: Uuid,
    body: String,
    created_at: DateTime<Utc>,
}

// コメント先を取得（型によって異なるクエリが必要）
async fn get_commentable(
    pool: &PgPool,
    comment: &Comment,
) -> Result<Commentable, anyhow::Error> {
    match comment.commentable_type.as_str() {
        "Post" => {
            let post = sqlx::query_as!(Post, "SELECT * FROM posts WHERE id = $1", comment.commentable_id)
                .fetch_one(pool)
                .await?;
            Ok(Commentable::Post(post))
        }
        "Image" => {
            let image = sqlx::query_as!(Image, "SELECT * FROM images WHERE id = $1", comment.commentable_id)
                .fetch_one(pool)
                .await?;
            Ok(Commentable::Image(image))
        }
        "Video" => {
            let video = sqlx::query_as!(Video, "SELECT * FROM videos WHERE id = $1", comment.commentable_id)
                .fetch_one(pool)
                .await?;
            Ok(Commentable::Video(video))
        }
        _ => Err(anyhow::anyhow!("Unknown commentable type")),
    }
}
```

##### 7.2.1 外部キー制約が使えない

```sql
-- これは不可能
-- commentable_id は複数のテーブルを参照するため、
-- 単一の REFERENCES 句を書けない
ALTER TABLE comments
ADD CONSTRAINT fk_commentable
FOREIGN KEY (commentable_id) REFERENCES ???(id);  -- どのテーブル？
```

##### 7.2.2 孤立データが発生する

```rust
// 投稿を削除しても、コメントは残り続ける
async fn delete_post(pool: &PgPool, post_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;
    // commentsテーブルのcommentable_type='Post', commentable_id=post_id の行は孤立
    Ok(())
}
```

##### 7.2.3 JOINが複雑になる

```rust
// 投稿とそのコメントを取得（JOINが使いにくい）
async fn get_post_with_comments(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<PostWithComments, sqlx::Error> {
    let post = sqlx::query_as!(Post, "SELECT * FROM posts WHERE id = $1", post_id)
        .fetch_one(pool)
        .await?;

    // JOINではなく、別クエリで取得する必要がある
    let comments = sqlx::query_as!(
        Comment,
        r#"
        SELECT * FROM comments
        WHERE commentable_type = 'Post' AND commentable_id = $1
        ORDER BY created_at
        "#,
        post_id
    )
    .fetch_all(pool)
    .await?;

    Ok(PostWithComments { post, comments })
}
```

##### 7.2.4 タイプの不一致

```rust
// タイプ名のタイプミスを検出できない
async fn create_comment(
    pool: &PgPool,
    commentable_type: &str,  // 文字列なので何でも入る
    commentable_id: Uuid,
    user_id: Uuid,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO comments (commentable_type, commentable_id, user_id, body)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        commentable_type,  // "Psot" というタイプミスも受け入れる
        commentable_id,
        user_id,
        body
    )
    .fetch_one(pool)
    .await
}
```

#### 7.3 アンチパターンの見つけ方

- `*_type`と`*_id`のペアでカラムがある（`commentable_type`, `commentable_id`）
- 外部キー制約がない`*_id`カラムがある
- クエリに`WHERE type = 'Post'`のような条件が頻出する
- JOINを避けて別クエリで関連データを取得している

#### 7.4 アンチパターンを用いてもよい場合

1. **参照先のテーブルが非常に多い**: 数十以上のテーブルを参照する可能性がある
2. **ActiveRecordパターン**: RailsなどのORMでポリモーフィック関連がサポートされている
3. **柔軟性が最優先**: 新しい参照先を頻繁に追加する必要がある

ただし、Rustではこのパターンは推奨されない。型安全性を活かすべきである。

#### 7.5 解決策：関連を単純化する

##### 7.5.1 交差テーブルを参照先ごとに作成

```sql
-- 参照先ごとに交差テーブルを作成
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 投稿へのコメント
CREATE TABLE post_comments (
    comment_id UUID PRIMARY KEY REFERENCES comments(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE
);

-- 画像へのコメント
CREATE TABLE image_comments (
    comment_id UUID PRIMARY KEY REFERENCES comments(id) ON DELETE CASCADE,
    image_id UUID NOT NULL REFERENCES images(id) ON DELETE CASCADE
);

-- 動画へのコメント
CREATE TABLE video_comments (
    comment_id UUID PRIMARY KEY REFERENCES comments(id) ON DELETE CASCADE,
    video_id UUID NOT NULL REFERENCES videos(id) ON DELETE CASCADE
);
```

```rust
// 投稿のコメントを取得（JOINが簡単）
async fn get_post_comments(pool: &PgPool, post_id: Uuid) -> Result<Vec<Comment>, sqlx::Error> {
    sqlx::query_as!(
        Comment,
        r#"
        SELECT c.*
        FROM comments c
        INNER JOIN post_comments pc ON c.id = pc.comment_id
        WHERE pc.post_id = $1
        ORDER BY c.created_at
        "#,
        post_id
    )
    .fetch_all(pool)
    .await
}

// 投稿にコメントを追加（トランザクション使用）
async fn add_comment_to_post(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let comment_id = sqlx::query_scalar!(
        "INSERT INTO comments (user_id, body) VALUES ($1, $2) RETURNING id",
        user_id,
        body
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO post_comments (comment_id, post_id) VALUES ($1, $2)",
        comment_id,
        post_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(comment_id)
}
```

##### 7.5.2 共通の基底テーブルを作成

参照先のテーブルが共通のスーパータイプを持つ場合：

```sql
-- コンテンツの基底テーブル
CREATE TABLE contents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_type VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 各コンテンツタイプ
CREATE TABLE posts (
    content_id UUID PRIMARY KEY REFERENCES contents(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    body TEXT NOT NULL
);

CREATE TABLE images (
    content_id UUID PRIMARY KEY REFERENCES contents(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    alt_text VARCHAR(200)
);

CREATE TABLE videos (
    content_id UUID PRIMARY KEY REFERENCES contents(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    duration_seconds INT NOT NULL
);

-- コメントは共通の基底テーブルを参照
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id UUID NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
// 投稿の作成（コンテンツテーブル経由）
async fn create_post(
    pool: &PgPool,
    title: &str,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 基底テーブルに挿入
    let content_id = sqlx::query_scalar!(
        "INSERT INTO contents (content_type) VALUES ('post') RETURNING id"
    )
    .fetch_one(&mut *tx)
    .await?;

    // 投稿テーブルに挿入
    sqlx::query!(
        "INSERT INTO posts (content_id, title, body) VALUES ($1, $2, $3)",
        content_id,
        title,
        body
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(content_id)
}

// コメントを追加（外部キー制約が効く）
async fn add_comment(
    pool: &PgPool,
    content_id: Uuid,
    user_id: Uuid,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO comments (content_id, user_id, body)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        content_id,
        user_id,
        body
    )
    .fetch_one(pool)
    .await
}

// 任意のコンテンツのコメントを取得
async fn get_comments(pool: &PgPool, content_id: Uuid) -> Result<Vec<Comment>, sqlx::Error> {
    sqlx::query_as!(
        Comment,
        "SELECT * FROM comments WHERE content_id = $1 ORDER BY created_at",
        content_id
    )
    .fetch_all(pool)
    .await
}
```

##### 7.5.3 Rustの型システムを活用

```rust
// 型安全なコンテンツ参照
#[derive(Debug, Clone)]
enum ContentRef {
    Post(Uuid),
    Image(Uuid),
    Video(Uuid),
}

impl ContentRef {
    fn content_id(&self) -> Uuid {
        match self {
            ContentRef::Post(id) | ContentRef::Image(id) | ContentRef::Video(id) => *id,
        }
    }

    fn content_type(&self) -> &'static str {
        match self {
            ContentRef::Post(_) => "post",
            ContentRef::Image(_) => "image",
            ContentRef::Video(_) => "video",
        }
    }
}

// 型安全なコメント作成
async fn add_comment(
    pool: &PgPool,
    content: ContentRef,
    user_id: Uuid,
    body: &str,
) -> Result<Uuid, sqlx::Error> {
    // content_id は必ず存在するコンテンツを参照する（外部キー制約）
    sqlx::query_scalar!(
        "INSERT INTO comments (content_id, user_id, body) VALUES ($1, $2, $3) RETURNING id",
        content.content_id(),
        user_id,
        body
    )
    .fetch_one(pool)
    .await
}
```

#### 7.6 設計の比較

| 設計 | 外部キー | 拡張性 | クエリの複雑さ |
|------|---------|--------|---------------|
| ポリモーフィック関連 | × | ◎ | 高 |
| 交差テーブル | ◎ | ○ | 中 |
| 共通基底テーブル | ◎ | ○ | 低 |

**推奨**: 参照先が3-5個程度なら交差テーブル、それ以上なら共通基底テーブルを検討する。

---

### 8章 マルチカラムアトリビュート（複数列属性）

#### 8.1 目的：複数の値を持つ属性を格納する

ユーザーが複数の電話番号を持てるようにしたい、商品に複数のタグを付けたいなど、1つのエンティティに対して複数の値を持つ属性を格納したい。

#### 8.2 アンチパターン：複数の列を定義する

**問題のあるスキーマ：**

```sql
-- アンチパターン: 電話番号用に複数の列を定義
CREATE TABLE users (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    phone1 VARCHAR(20),
    phone2 VARCHAR(20),
    phone3 VARCHAR(20)
);

-- アンチパターン: タグ用に複数の列を定義
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    tag1 VARCHAR(50),
    tag2 VARCHAR(50),
    tag3 VARCHAR(50),
    tag4 VARCHAR(50),
    tag5 VARCHAR(50)
);
```

##### 8.2.1 値の検索が困難

```rust
// アンチパターン: 全ての列を検索する必要がある
async fn find_user_by_phone(pool: &PgPool, phone: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT * FROM users
        WHERE phone1 = $1 OR phone2 = $1 OR phone3 = $1
        "#,
        phone
    )
    .fetch_optional(pool)
    .await
}
// 問題: 列が増えるたびにクエリを修正する必要がある
```

##### 8.2.2 値の追加と削除が煩雑

```rust
// アンチパターン: 空いている列を探して追加
async fn add_phone(pool: &PgPool, user_id: Uuid, phone: &str) -> Result<(), anyhow::Error> {
    let user: User = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await?;

    // 空いている列を探す
    let update_query = if user.phone1.is_none() {
        "UPDATE users SET phone1 = $1 WHERE id = $2"
    } else if user.phone2.is_none() {
        "UPDATE users SET phone2 = $1 WHERE id = $2"
    } else if user.phone3.is_none() {
        "UPDATE users SET phone3 = $1 WHERE id = $2"
    } else {
        return Err(anyhow::anyhow!("No more phone slots available"));
    };

    sqlx::query(update_query)
        .bind(phone)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}
```

##### 8.2.3 一意性の保証が困難

```sql
-- 同じ電話番号が異なる列に入る可能性がある
INSERT INTO users (id, name, phone1, phone2)
VALUES (gen_random_uuid(), 'Alice', '090-1234-5678', '090-1234-5678');
-- phone1とphone2に同じ値が入っても制約違反にならない
```

##### 8.2.4 増加する値の処理

```sql
-- 3つでは足りなくなった場合...
ALTER TABLE users ADD COLUMN phone4 VARCHAR(20);
ALTER TABLE users ADD COLUMN phone5 VARCHAR(20);
-- すべてのクエリを修正する必要がある
```

#### 8.3 アンチパターンの見つけ方

- `column1`, `column2`, `column3`のような連番の列名がある
- `OR column1 = ? OR column2 = ? OR column3 = ?`のようなクエリがある
- 「列が足りない」という要望が発生する
- 多くの列がNULLになっている

#### 8.4 アンチパターンを用いてもよい場合

1. **値の数が固定**: 住所の（都道府県、市区町村、番地）のように必ず同じ数
2. **値に順序や意味がある**: 第1連絡先、第2連絡先のように優先順位がある
3. **クエリパターンが限定的**: 全列を同時に取得するのみで、検索しない

#### 8.5 解決策：従属テーブルを作成する

```sql
-- 正しいスキーマ: 従属テーブル
CREATE TABLE users (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);

CREATE TABLE user_phones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    phone VARCHAR(20) NOT NULL,
    phone_type VARCHAR(20) DEFAULT 'mobile',  -- 'mobile', 'home', 'work'
    is_primary BOOLEAN DEFAULT false,
    UNIQUE (user_id, phone)
);

CREATE INDEX idx_user_phones_user_id ON user_phones(user_id);
CREATE INDEX idx_user_phones_phone ON user_phones(phone);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct UserPhone {
    id: Uuid,
    user_id: Uuid,
    phone: String,
    phone_type: Option<String>,
    is_primary: bool,
}

// 電話番号で検索（簡単！）
async fn find_user_by_phone(pool: &PgPool, phone: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT u.* FROM users u
        INNER JOIN user_phones up ON u.id = up.user_id
        WHERE up.phone = $1
        "#,
        phone
    )
    .fetch_optional(pool)
    .await
}

// 電話番号の追加（簡単！）
async fn add_phone(
    pool: &PgPool,
    user_id: Uuid,
    phone: &str,
    phone_type: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO user_phones (user_id, phone, phone_type)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        user_id,
        phone,
        phone_type
    )
    .fetch_one(pool)
    .await
}

// ユーザーの全電話番号を取得
async fn get_user_phones(pool: &PgPool, user_id: Uuid) -> Result<Vec<UserPhone>, sqlx::Error> {
    sqlx::query_as!(
        UserPhone,
        "SELECT * FROM user_phones WHERE user_id = $1 ORDER BY is_primary DESC",
        user_id
    )
    .fetch_all(pool)
    .await
}
```

---

### 9章 メタデータトリブル（メタデータ大増殖）

#### 9.1 目的：スケーラビリティを高める

データ量の増加に対応し、クエリのパフォーマンスを維持したい。

#### 9.2 アンチパターン：テーブルや列をコピーする

**問題のあるスキーマ：**

```sql
-- アンチパターン: 年ごとにテーブルを分割
CREATE TABLE orders_2022 (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    total DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE orders_2023 (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    total DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE orders_2024 (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    total DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
```

##### 9.2.1 テーブルをまたいだクエリが複雑

```rust
// アンチパターン: 全年度の注文を取得
async fn get_user_orders(pool: &PgPool, user_id: Uuid) -> Result<Vec<Order>, sqlx::Error> {
    sqlx::query_as!(
        Order,
        r#"
        SELECT * FROM orders_2022 WHERE user_id = $1
        UNION ALL
        SELECT * FROM orders_2023 WHERE user_id = $1
        UNION ALL
        SELECT * FROM orders_2024 WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}
// 問題: 新しい年になるたびにクエリを修正する必要がある
```

##### 9.2.2 参照整合性の管理が困難

```sql
-- 外部キー制約を各テーブルに設定する必要がある
ALTER TABLE orders_2022 ADD FOREIGN KEY (user_id) REFERENCES users(id);
ALTER TABLE orders_2023 ADD FOREIGN KEY (user_id) REFERENCES users(id);
ALTER TABLE orders_2024 ADD FOREIGN KEY (user_id) REFERENCES users(id);
-- 新しいテーブルごとに追加が必要
```

##### 9.2.3 スキーマ変更の同期が必要

```sql
-- 列を追加する場合、全テーブルに適用が必要
ALTER TABLE orders_2022 ADD COLUMN status VARCHAR(20);
ALTER TABLE orders_2023 ADD COLUMN status VARCHAR(20);
ALTER TABLE orders_2024 ADD COLUMN status VARCHAR(20);
-- 漏れがあると不整合が発生
```

##### 9.2.4 メタデータトリブル列の発生

```sql
-- アンチパターン: 月ごとに列を追加
CREATE TABLE revenue (
    year INT PRIMARY KEY,
    jan_revenue DECIMAL(12,2),
    feb_revenue DECIMAL(12,2),
    mar_revenue DECIMAL(12,2),
    apr_revenue DECIMAL(12,2),
    may_revenue DECIMAL(12,2),
    jun_revenue DECIMAL(12,2),
    jul_revenue DECIMAL(12,2),
    aug_revenue DECIMAL(12,2),
    sep_revenue DECIMAL(12,2),
    oct_revenue DECIMAL(12,2),
    nov_revenue DECIMAL(12,2),
    dec_revenue DECIMAL(12,2)
);
```

#### 9.3 アンチパターンの見つけ方

- テーブル名に年や日付が含まれる（`orders_2024`）
- 列名に月や期間が含まれる（`jan_revenue`）
- 新しい期間ごとにDDLを実行する運用がある
- UNIONを多用するクエリがある

#### 9.4 アンチパターンを用いてもよい場合

1. **アーカイブ目的**: 古いデータを別テーブルに移動して参照頻度を下げる
2. **規制要件**: 法的にデータを分離して保管する必要がある
3. **異なるアクセスパターン**: 履歴データと現行データで完全に異なる使い方をする

#### 9.5 解決策：パーティショニングと正規化を行う

##### 9.5.1 PostgreSQLの宣言的パーティショニング

```sql
-- 正しいスキーマ: パーティショニング
CREATE TABLE orders (
    id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    total DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- パーティションの作成
CREATE TABLE orders_2023 PARTITION OF orders
    FOR VALUES FROM ('2023-01-01') TO ('2024-01-01');

CREATE TABLE orders_2024 PARTITION OF orders
    FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');

CREATE TABLE orders_2025 PARTITION OF orders
    FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');
```

```rust
// パーティショニングはアプリケーションから透過的
async fn get_user_orders(pool: &PgPool, user_id: Uuid) -> Result<Vec<Order>, sqlx::Error> {
    sqlx::query_as!(
        Order,
        r#"
        SELECT * FROM orders
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}
// PostgreSQLが自動的に適切なパーティションを選択
```

##### 9.5.2 メタデータトリブル列の修正

```sql
-- 正しいスキーマ: 行として正規化
CREATE TABLE monthly_revenue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    year INT NOT NULL,
    month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
    revenue DECIMAL(12,2) NOT NULL,
    UNIQUE (year, month)
);
```

```rust
// 特定の年の月別収益を取得
async fn get_yearly_revenue(pool: &PgPool, year: i32) -> Result<Vec<MonthlyRevenue>, sqlx::Error> {
    sqlx::query_as!(
        MonthlyRevenue,
        r#"
        SELECT * FROM monthly_revenue
        WHERE year = $1
        ORDER BY month
        "#,
        year
    )
    .fetch_all(pool)
    .await
}

// 年間合計を計算
async fn get_annual_total(pool: &PgPool, year: i32) -> Result<Decimal, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(revenue), 0) as "total!"
        FROM monthly_revenue
        WHERE year = $1
        "#,
        year
    )
    .fetch_one(pool)
    .await
}
```

##### 9.5.3 パーティション管理の自動化

```rust
// パーティションの自動作成
async fn ensure_partition_exists(pool: &PgPool, year: i32) -> Result<(), sqlx::Error> {
    let partition_name = format!("orders_{}", year);
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-01-01", year + 1);

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {} PARTITION OF orders
        FOR VALUES FROM ('{}') TO ('{}')
        "#,
        partition_name, start_date, end_date
    ))
    .execute(pool)
    .await?;

    Ok(())
}
```

---

## 第III部 データベース物理設計のアンチパターン

### 10章 ラウンディングエラー（丸め誤差）

#### 8.1 目的：整数の代わりに小数値を使用する

価格、パーセンテージ、座標など、小数点を含む数値をデータベースに格納したい。

#### 8.2 アンチパターン：FLOATデータ型を使用する

**問題のあるスキーマ：**

```sql
-- アンチパターン: 金額にFLOATを使用
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price FLOAT NOT NULL  -- 問題！
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL,
    product_id UUID NOT NULL,
    quantity INT NOT NULL,
    unit_price FLOAT NOT NULL,  -- 問題！
    discount_rate FLOAT DEFAULT 0  -- 問題！
);
```

##### 8.2.1 丸め誤差が発生する

```rust
// FLOATの丸め誤差のデモ
fn float_rounding_error() {
    let price: f64 = 19.99;
    let quantity: f64 = 3.0;

    let total = price * quantity;
    println!("Total: {}", total);  // 59.97 ではなく 59.970000000000006 などになる

    // 比較で問題が発生
    if total == 59.97 {
        println!("Equal");  // これは実行されない可能性がある
    }
}
```

```rust
// データベースでの丸め誤差
async fn calculate_order_total(pool: &PgPool, order_id: Uuid) -> Result<f64, sqlx::Error> {
    let total: Option<f64> = sqlx::query_scalar!(
        r#"
        SELECT SUM(unit_price * quantity * (1 - discount_rate)) as "total"
        FROM order_items
        WHERE order_id = $1
        "#,
        order_id
    )
    .fetch_one(pool)
    .await?;

    Ok(total.unwrap_or(0.0))
    // 結果: 100.00 のはずが 99.99999999999999 になることがある
}
```

##### 8.2.2 集計の誤差が累積する

```rust
// 多数の値を合計すると誤差が累積
async fn get_monthly_revenue(pool: &PgPool) -> Result<f64, sqlx::Error> {
    let revenue: Option<f64> = sqlx::query_scalar!(
        r#"
        SELECT SUM(oi.unit_price * oi.quantity) as "revenue"
        FROM order_items oi
        JOIN orders o ON oi.order_id = o.id
        WHERE o.created_at >= date_trunc('month', CURRENT_DATE)
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(revenue.unwrap_or(0.0))
    // 数千件のレコードを合計すると、誤差が数セント〜数ドルになることも
}
```

##### 8.2.3 比較が不安定

```rust
// アンチパターン: FLOATの等価比較
async fn find_products_at_price(pool: &PgPool, price: f64) -> Result<Vec<Product>, sqlx::Error> {
    sqlx::query_as!(
        Product,
        "SELECT * FROM products WHERE price = $1",  // 危険！
        price
    )
    .fetch_all(pool)
    .await
}
// 問題: 19.99 を検索しても、19.990000000000002 として保存されたレコードがヒットしない
```

#### 8.3 アンチパターンの見つけ方

- 金額やパーセンテージを格納するカラムに`FLOAT`、`REAL`、`DOUBLE PRECISION`を使用
- Rustコードで`f32`や`f64`を金額に使用
- 合計値が期待値と微妙にずれる
- 等価比較が期待通りに動作しない

#### 8.4 アンチパターンを用いてもよい場合

1. **科学計算**: 有効桁数が重要で、厳密な値より近似値が適切な場合
2. **座標・物理量**: 緯度経度、センサーデータなど
3. **パフォーマンス優先**: 大量の計算で精度より速度が重要な場合

金額や財務データには**絶対に**使用してはならない。

#### 8.5 解決策：適切なデータ型を使用する

##### 8.5.1 DECIMAL/NUMERIC型を使用する

```sql
-- 正しいスキーマ: DECIMALを使用
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10, 2) NOT NULL  -- 10桁、小数点以下2桁
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL,
    product_id UUID NOT NULL,
    quantity INT NOT NULL,
    unit_price DECIMAL(10, 2) NOT NULL,
    discount_rate DECIMAL(5, 4) DEFAULT 0  -- 0.0000〜0.9999
);
```

```rust
use rust_decimal::Decimal;
use sqlx::types::BigDecimal;

#[derive(Debug, sqlx::FromRow)]
struct Product {
    id: Uuid,
    name: String,
    price: BigDecimal,  // sqlxのBigDecimal型
}

#[derive(Debug, sqlx::FromRow)]
struct OrderItem {
    id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    unit_price: BigDecimal,
    discount_rate: BigDecimal,
}
```

##### 8.5.2 rust_decimalクレートを使用

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// 正確な計算
fn calculate_total() {
    let price = dec!(19.99);
    let quantity = dec!(3);

    let total = price * quantity;
    assert_eq!(total, dec!(59.97));  // 正確に一致

    // 割引計算
    let discount_rate = dec!(0.1);  // 10%
    let discounted = total * (Decimal::ONE - discount_rate);
    assert_eq!(discounted, dec!(53.973));
}

// 丸めの制御
fn round_to_cents(amount: Decimal) -> Decimal {
    amount.round_dp(2)  // 小数点以下2桁に丸め
}
```

##### 8.5.3 sqlx + rust_decimalの統合

```toml
# Cargo.toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "rust_decimal"] }
rust_decimal = { version = "1", features = ["db-postgres"] }
```

```rust
use rust_decimal::Decimal;

#[derive(Debug, sqlx::FromRow)]
struct Product {
    id: Uuid,
    name: String,
    price: Decimal,  // rust_decimal::Decimal
}

async fn create_product(pool: &PgPool, name: &str, price: Decimal) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO products (name, price)
        VALUES ($1, $2)
        RETURNING id
        "#,
        name,
        price
    )
    .fetch_one(pool)
    .await
}

async fn calculate_order_total(pool: &PgPool, order_id: Uuid) -> Result<Decimal, sqlx::Error> {
    let total: Option<Decimal> = sqlx::query_scalar!(
        r#"
        SELECT SUM(unit_price * quantity * (1 - discount_rate))::DECIMAL(12,2) as "total"
        FROM order_items
        WHERE order_id = $1
        "#,
        order_id
    )
    .fetch_one(pool)
    .await?;

    Ok(total.unwrap_or(Decimal::ZERO))
}
```

##### 8.5.4 整数型で最小単位を格納

金額を「セント」で格納するアプローチ：

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price_cents BIGINT NOT NULL  -- 1999 = $19.99
);
```

```rust
#[derive(Debug, Clone, Copy)]
struct Money(i64);  // セント単位

impl Money {
    fn from_dollars(dollars: f64) -> Self {
        Money((dollars * 100.0).round() as i64)
    }

    fn to_dollars(&self) -> f64 {
        self.0 as f64 / 100.0
    }

    fn cents(&self) -> i64 {
        self.0
    }
}

impl std::ops::Add for Money {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Money(self.0 + other.0)
    }
}

impl std::ops::Mul<i32> for Money {
    type Output = Self;
    fn mul(self, quantity: i32) -> Self {
        Money(self.0 * quantity as i64)
    }
}

// 使用例
async fn get_product_price(pool: &PgPool, product_id: Uuid) -> Result<Money, sqlx::Error> {
    let cents: i64 = sqlx::query_scalar!(
        "SELECT price_cents FROM products WHERE id = $1",
        product_id
    )
    .fetch_one(pool)
    .await?;

    Ok(Money(cents))
}
```

#### 8.6 データ型の比較

| データ型 | 精度 | 範囲 | 用途 |
|---------|------|------|------|
| `FLOAT` | 約15桁（相対） | ±1.8×10^308 | 科学計算 |
| `DECIMAL(p,s)` | p桁（絶対） | 10^p | 金額、財務 |
| `BIGINT`（セント） | 正確 | ±9.2×10^18 | 金額（シンプル） |

**推奨**: 金額には`DECIMAL(10,2)`または整数型（セント）を使用する。

---

### 11章 サーティワンフレーバー（31のフレーバー）

#### 9.1 目的：列を特定の値に限定する

ステータス、カテゴリ、タイプなど、列に格納できる値を特定の選択肢に制限したい。

#### 9.2 アンチパターン：限定する値を列定義で指定する（ENUM型）

**問題のあるスキーマ：**

```sql
-- アンチパターン: PostgreSQLのENUM型を使用
CREATE TYPE post_status AS ENUM ('draft', 'published', 'archived');

CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    status post_status NOT NULL DEFAULT 'draft'
);
```

##### 9.2.1 新しい値の追加が面倒

```sql
-- 新しいステータスを追加するにはALTER TYPEが必要
ALTER TYPE post_status ADD VALUE 'pending_review';

-- 問題:
-- 1. 値の追加は可能だが、削除や名前変更はできない
-- 2. トランザクション内では値を追加できない
-- 3. マイグレーションが複雑になる
```

```rust
// Rustコード側でも変更が必要
#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "post_status", rename_all = "snake_case")]
enum PostStatus {
    Draft,
    Published,
    Archived,
    PendingReview,  // 追加が必要
}
```

##### 9.2.2 既存の値を削除・変更できない

```sql
-- ENUMの値を削除するには、型を作り直す必要がある
-- これは非常に破壊的な操作

-- 1. 新しい型を作成
CREATE TYPE post_status_new AS ENUM ('draft', 'published', 'archived');

-- 2. 列の型を変更
ALTER TABLE posts
    ALTER COLUMN status TYPE post_status_new
    USING status::text::post_status_new;

-- 3. 古い型を削除
DROP TYPE post_status;

-- 4. 型の名前を変更
ALTER TYPE post_status_new RENAME TO post_status;

-- 問題: ダウンタイムが必要、古い値を持つレコードがあるとエラー
```

##### 9.2.3 値の一覧取得が複雑

```rust
// ENUMの値一覧を取得するクエリが複雑
async fn get_post_statuses(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT enumlabel::text
        FROM pg_enum
        JOIN pg_type ON pg_enum.enumtypid = pg_type.oid
        WHERE pg_type.typname = 'post_status'
        ORDER BY enumsortorder
        "#
    )
    .fetch_all(pool)
    .await
}
// pg_catalog への依存、PostgreSQL固有
```

##### 9.2.4 CHECK制約も同様の問題

```sql
-- CHECK制約を使うアプローチ
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    CONSTRAINT chk_status CHECK (status IN ('draft', 'published', 'archived'))
);

-- 新しい値を追加するにはCONSTRAINTの変更が必要
ALTER TABLE posts DROP CONSTRAINT chk_status;
ALTER TABLE posts ADD CONSTRAINT chk_status
    CHECK (status IN ('draft', 'published', 'archived', 'pending_review'));
```

#### 9.3 アンチパターンの見つけ方

- `CREATE TYPE ... AS ENUM`を使用している
- `CHECK (column IN (...))`で値を制限している
- 新しい値を追加するたびにマイグレーションが必要
- 値の一覧をハードコードしている箇所が複数ある

#### 9.4 アンチパターンを用いてもよい場合

1. **値が絶対に変わらない**: 曜日、月など
2. **小規模なプロジェクト**: マイグレーションの複雑さが許容できる
3. **型安全性が最優先**: Rustとの統合でコンパイル時チェックを活かしたい

#### 9.5 解決策：限定する値をデータで指定する（参照テーブル）

##### 9.5.1 参照テーブルを作成

```sql
-- ステータスのマスタテーブル
CREATE TABLE post_statuses (
    status_code VARCHAR(20) PRIMARY KEY,
    display_name VARCHAR(50) NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INT NOT NULL DEFAULT 0
);

-- 初期データ
INSERT INTO post_statuses (status_code, display_name, sort_order) VALUES
    ('draft', 'Draft', 1),
    ('pending_review', 'Pending Review', 2),
    ('published', 'Published', 3),
    ('archived', 'Archived', 4);

-- 投稿テーブル
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    status_code VARCHAR(20) NOT NULL DEFAULT 'draft'
        REFERENCES post_statuses(status_code)
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct PostStatus {
    status_code: String,
    display_name: String,
    description: Option<String>,
    is_active: bool,
    sort_order: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct Post {
    id: Uuid,
    title: String,
    status_code: String,
}

// ステータス一覧を取得（簡単！）
async fn get_active_statuses(pool: &PgPool) -> Result<Vec<PostStatus>, sqlx::Error> {
    sqlx::query_as!(
        PostStatus,
        r#"
        SELECT * FROM post_statuses
        WHERE is_active = true
        ORDER BY sort_order
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 9.5.2 値の追加・削除・変更が容易

```rust
// 新しいステータスの追加（マイグレーション不要）
async fn add_status(
    pool: &PgPool,
    code: &str,
    display_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO post_statuses (status_code, display_name, sort_order)
        VALUES ($1, $2, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM post_statuses))
        "#,
        code,
        display_name
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ステータスの無効化（削除の代わり）
async fn deactivate_status(pool: &PgPool, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE post_statuses SET is_active = false WHERE status_code = $1",
        code
    )
    .execute(pool)
    .await?;
    Ok(())
}

// 表示名の変更
async fn rename_status(
    pool: &PgPool,
    code: &str,
    new_display_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE post_statuses SET display_name = $1 WHERE status_code = $2",
        new_display_name,
        code
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

##### 9.5.3 Rust側でも型安全を実現

```rust
use std::str::FromStr;

// Rustの列挙型を定義（データベースとは独立）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostStatus {
    Draft,
    PendingReview,
    Published,
    Archived,
}

impl PostStatus {
    fn as_str(&self) -> &'static str {
        match self {
            PostStatus::Draft => "draft",
            PostStatus::PendingReview => "pending_review",
            PostStatus::Published => "published",
            PostStatus::Archived => "archived",
        }
    }
}

impl FromStr for PostStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(PostStatus::Draft),
            "pending_review" => Ok(PostStatus::PendingReview),
            "published" => Ok(PostStatus::Published),
            "archived" => Ok(PostStatus::Archived),
            _ => Err(anyhow::anyhow!("Unknown status: {}", s)),
        }
    }
}

// 使用例
async fn update_post_status(
    pool: &PgPool,
    post_id: Uuid,
    status: PostStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE posts SET status_code = $1 WHERE id = $2",
        status.as_str(),
        post_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

// 取得時に変換
async fn get_post(pool: &PgPool, post_id: Uuid) -> Result<(Post, PostStatus), anyhow::Error> {
    let post: Post = sqlx::query_as!(Post, "SELECT * FROM posts WHERE id = $1", post_id)
        .fetch_one(pool)
        .await?;

    let status = PostStatus::from_str(&post.status_code)?;
    Ok((post, status))
}
```

##### 9.5.4 メタデータの活用

参照テーブルには追加のメタデータを持たせられる：

```sql
CREATE TABLE post_statuses (
    status_code VARCHAR(20) PRIMARY KEY,
    display_name VARCHAR(50) NOT NULL,
    description TEXT,
    color_code VARCHAR(7),  -- UIの表示色
    icon_name VARCHAR(50),  -- アイコン名
    is_active BOOLEAN NOT NULL DEFAULT true,
    allows_editing BOOLEAN NOT NULL DEFAULT true,  -- この状態で編集可能か
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct PostStatusMeta {
    status_code: String,
    display_name: String,
    color_code: Option<String>,
    icon_name: Option<String>,
    allows_editing: bool,
}

// フロントエンドに渡すメタデータ
async fn get_status_metadata(pool: &PgPool) -> Result<Vec<PostStatusMeta>, sqlx::Error> {
    sqlx::query_as!(
        PostStatusMeta,
        r#"
        SELECT status_code, display_name, color_code, icon_name, allows_editing
        FROM post_statuses
        WHERE is_active = true
        ORDER BY sort_order
        "#
    )
    .fetch_all(pool)
    .await
}
```

#### 9.6 アプローチの比較

| アプローチ | 追加 | 削除/変更 | 型安全 | メタデータ |
|-----------|------|----------|--------|-----------|
| ENUM | 可能（制限あり） | 困難 | ◎ | × |
| CHECK制約 | 要マイグレーション | 要マイグレーション | × | × |
| 参照テーブル | 容易 | 容易 | △（Rust側で対応） | ◎ |

**推奨**: 参照テーブルを使用し、Rust側で必要に応じて列挙型を定義する。

---

### 12章 ファントムファイル（幻のファイル）

#### 12.1 目的：画像やファイルを格納する

ユーザーのアバター画像、投稿に添付されたファイル、PDFドキュメントなど、大容量のメディアファイルをアプリケーションで扱いたい。

#### 12.2 アンチパターン：物理ファイルの使用を必須と思い込む

「ファイルはファイルシステムに保存すべき」という固定観念から、常にファイルパスだけをデータベースに保存するアプローチ。

**問題のあるスキーマ：**

```sql
-- アンチパターン: ファイルパスのみを保存
CREATE TABLE user_avatars (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    file_path VARCHAR(500) NOT NULL,  -- '/uploads/avatars/user123/avatar.png'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE post_attachments (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id),
    file_path VARCHAR(500) NOT NULL,
    file_name VARCHAR(255) NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

##### 12.2.1 ファイル削除時の問題

```rust
// データベースのレコードを削除してもファイルが残る
async fn delete_post_with_attachments(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<(), AppError> {
    // アタッチメント情報を取得
    let attachments: Vec<Attachment> = sqlx::query_as!(
        Attachment,
        "SELECT * FROM post_attachments WHERE post_id = $1",
        post_id
    )
    .fetch_all(pool)
    .await?;

    // データベースから削除（CASCADE で attachments も削除される）
    sqlx::query!("DELETE FROM posts WHERE id = $1", post_id)
        .execute(pool)
        .await?;

    // 問題: ファイルシステムからの削除を忘れるとゴミファイルが残る
    for attachment in attachments {
        // これが失敗してもDBのトランザクションは既にコミット済み
        if let Err(e) = tokio::fs::remove_file(&attachment.file_path).await {
            tracing::error!("Failed to delete file: {}", e);
            // ファイルが残ってしまう（ファントムファイル）
        }
    }

    Ok(())
}
```

##### 12.2.2 トランザクション分離の問題

```rust
// データベースとファイルシステムのトランザクションが分離している
async fn upload_attachment(
    pool: &PgPool,
    post_id: Uuid,
    file_data: Bytes,
    file_name: &str,
) -> Result<Uuid, AppError> {
    let attachment_id = Uuid::new_v4();
    let file_path = format!("/uploads/posts/{}/{}", post_id, file_name);

    // 1. ファイルを保存
    tokio::fs::create_dir_all(format!("/uploads/posts/{}", post_id)).await?;
    tokio::fs::write(&file_path, &file_data).await?;

    // 2. データベースにレコードを挿入
    let result = sqlx::query!(
        r#"
        INSERT INTO post_attachments (id, post_id, file_path, file_name, mime_type, file_size)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        attachment_id,
        post_id,
        file_path,
        file_name,
        "application/octet-stream",
        file_data.len() as i64
    )
    .execute(pool)
    .await;

    // 問題: DBへの挿入が失敗した場合、ファイルが残る
    if let Err(e) = result {
        // クリーンアップを試みるが、これも失敗する可能性がある
        let _ = tokio::fs::remove_file(&file_path).await;
        return Err(e.into());
    }

    Ok(attachment_id)
}
```

##### 12.2.3 ロールバック時の問題

```rust
// トランザクションをロールバックしてもファイルは残る
async fn create_post_with_attachments(
    pool: &PgPool,
    title: &str,
    files: Vec<(String, Bytes)>,
) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;
    let post_id = Uuid::new_v4();

    // 投稿を作成
    sqlx::query!(
        "INSERT INTO posts (id, user_id, title, content, status) VALUES ($1, $2, $3, '', 'draft')",
        post_id,
        Uuid::nil(), // ダミー
        title
    )
    .execute(&mut *tx)
    .await?;

    let mut saved_files = Vec::new();

    for (file_name, file_data) in files {
        let file_path = format!("/uploads/posts/{}/{}", post_id, file_name);

        // ファイルを保存（トランザクション外）
        tokio::fs::create_dir_all(format!("/uploads/posts/{}", post_id)).await?;
        tokio::fs::write(&file_path, &file_data).await?;
        saved_files.push(file_path.clone());

        // DBにレコード挿入
        if let Err(e) = sqlx::query!(
            "INSERT INTO post_attachments (id, post_id, file_path, file_name, mime_type, file_size)
             VALUES ($1, $2, $3, $4, 'application/octet-stream', $5)",
            Uuid::new_v4(),
            post_id,
            file_path,
            file_name,
            file_data.len() as i64
        )
        .execute(&mut *tx)
        .await
        {
            // トランザクションをロールバック
            tx.rollback().await?;

            // 問題: 手動でファイルを削除する必要がある
            for path in saved_files {
                let _ = tokio::fs::remove_file(&path).await;
            }

            return Err(e.into());
        }
    }

    tx.commit().await?;
    Ok(post_id)
}
```

##### 12.2.4 バックアップと復元の問題

```rust
// データベースのバックアップにファイルが含まれない
// pg_dump ではファイルシステムのデータはバックアップされない

// 復元時にファイルが存在しないとエラー
async fn get_attachment_url(
    pool: &PgPool,
    attachment_id: Uuid,
) -> Result<String, AppError> {
    let attachment = sqlx::query_as!(
        Attachment,
        "SELECT * FROM post_attachments WHERE id = $1",
        attachment_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // 問題: DBを別環境に復元した場合、ファイルが存在しない
    if !tokio::fs::try_exists(&attachment.file_path).await? {
        return Err(AppError::FileNotFound);
    }

    Ok(attachment.file_path)
}
```

##### 12.2.5 アクセス権限の問題

```rust
// SQLのアクセス制御とファイルシステムのアクセス制御が分離
async fn get_user_avatar(
    pool: &PgPool,
    user_id: Uuid,
    requesting_user_id: Uuid,
) -> Result<Bytes, AppError> {
    // DBレベルでは権限チェックができる
    let avatar = sqlx::query_as!(
        Avatar,
        "SELECT * FROM user_avatars WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // 問題: ファイルパスがわかれば直接アクセスできてしまう可能性
    // /uploads/avatars/user123/avatar.png が推測可能
    let data = tokio::fs::read(&avatar.file_path).await?;
    Ok(Bytes::from(data))
}
```

#### 12.3 アンチパターンの見つけ方

- `file_path`、`image_path`などのカラムがあり、外部ファイルシステムを参照している
- ファイル削除処理が複雑で、エラーハンドリングが不完全
- バックアップ・復元手順にファイルの同期が含まれている
- 「ファイルが見つからない」系のエラーが本番で発生する

#### 12.4 アンチパターンを用いてもよい場合

1. **非常に大きなファイル**: 数GB以上のファイルはDBに格納すると問題が出る
2. **ストリーミング配信**: 動画など、部分的なアクセスが必要な場合
3. **CDN連携**: 静的ファイルをCDNから配信する場合
4. **既存インフラ**: S3などのオブジェクトストレージが既にある場合

##### 12.4.1 ファイルシステムを使う場合の注意点

```rust
// オブジェクトストレージ（S3等）を使う場合の改善例
use aws_sdk_s3::Client as S3Client;

struct AttachmentService {
    pool: PgPool,
    s3: S3Client,
    bucket: String,
}

impl AttachmentService {
    // S3のキーをDBに保存（パスではなくキー）
    async fn upload(&self, post_id: Uuid, file_name: &str, data: Bytes) -> Result<Uuid, AppError> {
        let attachment_id = Uuid::new_v4();
        let s3_key = format!("posts/{}/{}/{}", post_id, attachment_id, file_name);

        // S3にアップロード
        self.s3
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .body(data.into())
            .send()
            .await?;

        // DBにはS3キーを保存（パスではない）
        sqlx::query!(
            "INSERT INTO post_attachments (id, post_id, s3_key, file_name, mime_type)
             VALUES ($1, $2, $3, $4, 'application/octet-stream')",
            attachment_id,
            post_id,
            s3_key,
            file_name
        )
        .execute(&self.pool)
        .await?;

        Ok(attachment_id)
    }

    // 署名付きURLを生成してアクセス制御
    async fn get_download_url(&self, attachment_id: Uuid) -> Result<String, AppError> {
        let attachment = sqlx::query_as!(
            Attachment,
            "SELECT * FROM post_attachments WHERE id = $1",
            attachment_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound)?;

        // 署名付きURLを生成（有効期限付き）
        let presigned = self.s3
            .get_object()
            .bucket(&self.bucket)
            .key(&attachment.s3_key)
            .presigned(
                aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    std::time::Duration::from_secs(3600)
                )?
            )
            .await?;

        Ok(presigned.uri().to_string())
    }
}
```

#### 12.5 解決策：必要に応じてBLOB型を採用する

##### 12.5.1 小〜中規模のファイルはBLOB型で格納

```sql
-- 解決策: BLOBを使用してファイルをDBに格納
CREATE TABLE user_avatars (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    data BYTEA NOT NULL,  -- ファイルの実データ
    mime_type VARCHAR(100) NOT NULL,
    file_size INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE post_attachments (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    data BYTEA NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    file_size INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- インデックス（データ列を除外）
CREATE INDEX idx_post_attachments_post_id ON post_attachments(post_id);
```

##### 12.5.2 トランザクション整合性が保証される

```rust
// 全ての操作がトランザクション内で完結
async fn create_post_with_attachments(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    content: &str,
    attachments: Vec<AttachmentInput>,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let post_id = Uuid::new_v4();

    // 投稿を作成
    sqlx::query!(
        "INSERT INTO posts (id, user_id, title, content, status) VALUES ($1, $2, $3, $4, 'draft')",
        post_id,
        user_id,
        title,
        content
    )
    .execute(&mut *tx)
    .await?;

    // アタッチメントを追加（全てトランザクション内）
    for attachment in attachments {
        sqlx::query!(
            r#"
            INSERT INTO post_attachments (id, post_id, file_name, data, mime_type, file_size)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::new_v4(),
            post_id,
            attachment.file_name,
            attachment.data.as_ref(),
            attachment.mime_type,
            attachment.data.len() as i32
        )
        .execute(&mut *tx)
        .await?;
    }

    // コミット：全て成功するか全て失敗するか
    tx.commit().await?;

    Ok(post_id)
}

// ロールバック時もファイルは自動的に削除される
async fn failed_upload_example(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // ファイルをDBに挿入
    sqlx::query!(
        "INSERT INTO post_attachments (id, post_id, file_name, data, mime_type, file_size)
         VALUES ($1, $2, 'test.pdf', $3, 'application/pdf', $4)",
        Uuid::new_v4(),
        Uuid::new_v4(),
        &[0u8; 1000][..],
        1000
    )
    .execute(&mut *tx)
    .await?;

    // エラーが発生
    let _: i32 = sqlx::query_scalar!("SELECT 1/0")
        .fetch_one(&mut *tx)
        .await?;

    // ここには到達しない
    tx.commit().await?;
    Ok(())
}
// トランザクションがロールバックされると、ファイルデータも自動的に削除される
```

##### 12.5.3 削除が自動的に行われる

```rust
// CASCADE削除でファイルも自動的に削除
async fn delete_post(pool: &PgPool, post_id: Uuid) -> Result<(), sqlx::Error> {
    // これだけでOK！アタッチメントも自動的に削除される
    sqlx::query!("DELETE FROM posts WHERE id = $1", post_id)
        .execute(pool)
        .await?;

    Ok(())
}

// ユーザー削除時もアバターが自動的に削除される
async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await?;

    Ok(())
}
```

##### 12.5.4 バックアップと復元が簡単

```rust
// pg_dump で全てのデータがバックアップされる
// ファイルの同期を気にする必要がない

// 復元後も全てのファイルがそのまま使える
async fn get_attachment(pool: &PgPool, attachment_id: Uuid) -> Result<AttachmentData, sqlx::Error> {
    sqlx::query_as!(
        AttachmentData,
        r#"
        SELECT file_name, data, mime_type
        FROM post_attachments
        WHERE id = $1
        "#,
        attachment_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}
```

##### 12.5.5 アクセス制御が統一される

```rust
// SQLの権限でファイルアクセスも制御
async fn get_user_avatar_if_allowed(
    pool: &PgPool,
    avatar_user_id: Uuid,
    requesting_user_id: Uuid,
) -> Result<Option<AvatarData>, sqlx::Error> {
    // 権限チェックとデータ取得を1クエリで
    sqlx::query_as!(
        AvatarData,
        r#"
        SELECT ua.data, ua.mime_type
        FROM user_avatars ua
        JOIN users u ON ua.user_id = u.id
        WHERE ua.user_id = $1
        AND (
            u.profile_visibility = 'public'
            OR u.id = $2
            OR EXISTS (
                SELECT 1 FROM friendships
                WHERE (user_id = $1 AND friend_id = $2)
                   OR (user_id = $2 AND friend_id = $1)
            )
        )
        "#,
        avatar_user_id,
        requesting_user_id
    )
    .fetch_optional(pool)
    .await
}
```

##### 12.5.6 画像の最適化とキャッシュ

```rust
use image::{ImageFormat, DynamicImage};

// アップロード時に画像を最適化してから保存
async fn upload_avatar(
    pool: &PgPool,
    user_id: Uuid,
    image_data: Bytes,
) -> Result<(), AppError> {
    // 画像をデコード
    let img = image::load_from_memory(&image_data)?;

    // リサイズ（最大256x256）
    let resized = img.resize(256, 256, image::imageops::FilterType::Lanczos3);

    // WebPに変換して圧縮
    let mut optimized = Vec::new();
    resized.write_to(&mut std::io::Cursor::new(&mut optimized), ImageFormat::WebP)?;

    // DBに保存
    sqlx::query!(
        r#"
        INSERT INTO user_avatars (id, user_id, data, mime_type, file_size)
        VALUES ($1, $2, $3, 'image/webp', $4)
        ON CONFLICT (user_id) DO UPDATE SET
            data = EXCLUDED.data,
            mime_type = EXCLUDED.mime_type,
            file_size = EXCLUDED.file_size,
            updated_at = NOW()
        "#,
        Uuid::new_v4(),
        user_id,
        &optimized,
        optimized.len() as i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

// HTTPレスポンスにキャッシュヘッダーを付与
async fn serve_avatar(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let avatar = sqlx::query_as!(
        AvatarData,
        "SELECT data, mime_type, updated_at FROM user_avatars WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    use axum::http::header;

    Ok((
        [
            (header::CONTENT_TYPE, avatar.mime_type),
            (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
            (
                header::ETAG,
                format!("\"{}\"", avatar.updated_at.timestamp()),
            ),
        ],
        avatar.data,
    ))
}
```

#### 12.6 アプローチの比較

| アプローチ | トランザクション | バックアップ | スケーラビリティ | 推奨サイズ |
|-----------|----------------|-------------|-----------------|-----------|
| ローカルファイル | × | △ | × | 任意 |
| BLOB/BYTEA | ◎ | ◎ | △ | 〜10MB |
| オブジェクトストレージ | △ | ◎ | ◎ | 任意 |

**推奨**:
- **10MB以下のファイル**: BLOB型を使用してトランザクション整合性を確保
- **10MB以上または大量のファイル**: S3等のオブジェクトストレージを使用し、署名付きURLでアクセス制御

---

### 13章 インデックスショットガン（闇雲インデックス）

#### 10.1 目的：パフォーマンスを最適化する

クエリの応答時間を短縮し、データベースのパフォーマンスを向上させたい。

#### 10.2 アンチパターン：闇雲にインデックスを使用する

インデックスの仕組みを理解せずに、「とりあえずインデックスを追加すれば速くなる」と考えてしまうケース。

##### 10.2.1 インデックスをまったく定義しない

**問題のあるスキーマ：**

```sql
-- アンチパターン: 外部キーにインデックスがない
CREATE TABLE comments (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id),
    user_id UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    -- post_id, user_id にインデックスがない！
);
```

```rust
// このクエリはフルテーブルスキャンになる
async fn get_post_comments(pool: &PgPool, post_id: Uuid) -> Result<Vec<Comment>, sqlx::Error> {
    sqlx::query_as!(
        Comment,
        "SELECT * FROM comments WHERE post_id = $1 ORDER BY created_at",
        post_id
    )
    .fetch_all(pool)
    .await
}
// 問題: コメントが増えると急激に遅くなる
```

##### 10.2.2 インデックスを多く定義しすぎる

```sql
-- アンチパターン: 全ての列にインデックス
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,
    status VARCHAR(20) NOT NULL,
    view_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- 闇雲にインデックスを追加
CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_title ON posts(title);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_view_count ON posts(view_count);
CREATE INDEX idx_posts_created_at ON posts(created_at);
CREATE INDEX idx_posts_updated_at ON posts(updated_at);
CREATE INDEX idx_posts_user_status ON posts(user_id, status);
CREATE INDEX idx_posts_status_created ON posts(status, created_at);
-- ... さらに多くのインデックス
```

**問題点：**
- 挿入・更新・削除のたびに全てのインデックスを更新する必要がある
- ディスク容量を大量に消費する
- インデックスのメンテナンスコストが増大する

##### 10.2.3 使われないインデックス

```rust
// 複合インデックス idx_posts_user_status(user_id, status) があるが...
async fn get_posts_by_status(pool: &PgPool, status: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE status = $1",  // user_id がないので使われない
        status
    )
    .fetch_all(pool)
    .await
}
// 複合インデックスは左端の列から順に使用される
// status だけで検索する場合、このインデックスは使われない
```

##### 10.2.4 関数を適用するとインデックスが使われない

```rust
// アンチパターン: 列に関数を適用
async fn search_posts_by_title(pool: &PgPool, query: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE LOWER(title) LIKE $1",  // LOWER()でインデックス無効
        format!("%{}%", query.to_lowercase())
    )
    .fetch_all(pool)
    .await
}
```

#### 10.3 アンチパターンの見つけ方

- `EXPLAIN ANALYZE`を実行したことがない
- 外部キー列にインデックスがない
- すべての列にインデックスがある
- クエリが遅いが原因がわからない
- インデックスの使用状況を監視していない

#### 10.4 アンチパターンを用いてもよい場合

- **小さなテーブル**: 数千行以下ではインデックスの効果が薄い
- **頻繁な書き込み**: 読み取りより書き込みが多い場合、インデックスはオーバーヘッドになる

#### 10.5 解決策：MENTOR原則に基づく効果的なインデックス管理

##### 10.5.1 Measure（測定）

```rust
// クエリの実行計画を確認
async fn analyze_query(pool: &PgPool) -> Result<(), sqlx::Error> {
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN ANALYZE SELECT * FROM posts WHERE user_id = $1 AND status = 'published'"
    )
    .bind(Uuid::new_v4())
    .fetch_all(pool)
    .await?;

    for (line,) in explain {
        println!("{}", line);
    }
    Ok(())
}
```

```sql
-- 使われていないインデックスを特定
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC;
```

##### 10.5.2 Explain（解析）

```sql
-- EXPLAIN の結果を読む
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT * FROM posts
WHERE user_id = '...' AND status = 'published'
ORDER BY created_at DESC
LIMIT 10;

-- 結果の例:
-- Limit (cost=0.42..10.50 rows=10)
--   -> Index Scan using idx_posts_user_status on posts
--        Index Cond: (user_id = '...' AND status = 'published')
--        Filter: ...
--        Rows Removed by Filter: 5
--        Buffers: shared hit=15
```

**重要な指標：**
- `Seq Scan`: フルテーブルスキャン（大きなテーブルでは問題）
- `Index Scan`: インデックスを使用
- `Bitmap Index Scan`: 複数のインデックスを組み合わせ
- `Rows Removed by Filter`: フィルタで除外された行数

##### 10.5.3 Nominate（指名）

どのカラムにインデックスを作成すべきか決定する：

```sql
-- 1. 外部キーには必ずインデックスを作成
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);

-- 2. WHERE句で頻繁に使用される列
CREATE INDEX idx_posts_status ON posts(status);

-- 3. JOINで使用される列
-- (外部キーと同様)

-- 4. ORDER BYで使用される列（部分的に）
CREATE INDEX idx_posts_created_at ON posts(created_at DESC);
```

##### 10.5.4 複合インデックスの設計

```sql
-- 複合インデックスの列順序は重要
-- よく使うクエリパターンに基づいて設計する

-- クエリ: WHERE user_id = ? AND status = ? ORDER BY created_at DESC
CREATE INDEX idx_posts_user_status_created
ON posts(user_id, status, created_at DESC);

-- このインデックスは以下のクエリで使える:
-- 1. WHERE user_id = ?
-- 2. WHERE user_id = ? AND status = ?
-- 3. WHERE user_id = ? AND status = ? ORDER BY created_at DESC

-- 以下では使えない:
-- 1. WHERE status = ? (左端のuser_idがない)
-- 2. WHERE status = ? AND created_at > ? (左端のuser_idがない)
```

```rust
// 複合インデックスを活用するクエリ
async fn get_user_published_posts(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE user_id = $1 AND status = 'published'
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await
}
```

##### 10.5.5 部分インデックス

特定の条件を満たす行のみをインデックス化：

```sql
-- 公開済みの投稿のみインデックス化
CREATE INDEX idx_posts_published
ON posts(created_at DESC)
WHERE status = 'published';

-- アクティブユーザーのみ
CREATE INDEX idx_users_active
ON users(email)
WHERE is_active = true;
```

```rust
// 部分インデックスを活用
async fn get_recent_published_posts(pool: &PgPool, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE status = 'published'
        ORDER BY created_at DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}
```

##### 10.5.6 式インデックス（Expression Index）

```sql
-- 関数の結果をインデックス化
CREATE INDEX idx_posts_title_lower ON posts(LOWER(title));

-- JSONB の特定のキーをインデックス化
CREATE INDEX idx_products_category
ON products((attributes->>'category'));
```

```rust
// 式インデックスを活用
async fn search_posts_by_title(pool: &PgPool, query: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE LOWER(title) LIKE $1",
        format!("%{}%", query.to_lowercase())
    )
    .fetch_all(pool)
    .await
}
```

##### 10.5.7 インデックスの監視

```rust
// インデックスの使用状況を監視するクエリ
async fn get_index_usage(pool: &PgPool) -> Result<Vec<IndexUsage>, sqlx::Error> {
    sqlx::query_as!(
        IndexUsage,
        r#"
        SELECT
            schemaname as "schema_name!",
            tablename as "table_name!",
            indexname as "index_name!",
            idx_scan as "scan_count!",
            pg_size_pretty(pg_relation_size(indexrelid)) as "size!"
        FROM pg_stat_user_indexes
        ORDER BY idx_scan DESC
        "#
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
struct IndexUsage {
    schema_name: String,
    table_name: String,
    index_name: String,
    scan_count: i64,
    size: String,
}
```

#### 10.6 インデックス設計のガイドライン

| 条件 | インデックス |
|------|------------|
| 外部キー | 必須 |
| WHERE句で頻繁に使用 | 推奨 |
| 高カーディナリティ（値のバリエーションが多い） | 効果的 |
| 低カーディナリティ（値のバリエーションが少ない） | 部分インデックスを検討 |
| ORDER BY + LIMIT | 推奨 |
| 関数を適用する列 | 式インデックスを検討 |

**推奨**: `EXPLAIN ANALYZE`で実際のクエリパフォーマンスを測定し、必要なインデックスのみを追加する。

---

## 第IV部 クエリのアンチパターン

### 14章 フィア・オブ・ジ・アンノウン（恐怖のunknown）

#### 11.1 目的：欠けている値を区別する

「値がない」「値が不明」「値が適用されない」など、欠損値の意味を適切に表現したい。

#### 11.2 アンチパターン：NULLを一般値として使う、または避ける

NULLの特殊な振る舞いを理解せずに使用したり、逆にNULLを恐れて避けてしまうケース。

##### 11.2.1 NULLとの比較で意図しない結果

```rust
// アンチパターン: NULL との等価比較
async fn find_users_without_bio(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE bio = NULL"  // 常に0件！
    )
    .fetch_all(pool)
    .await
}
// 問題: NULL = NULL は NULL (TRUE でも FALSE でもない)
```

```rust
// アンチパターン: NULL との不等価比較
async fn find_users_with_bio(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE bio != 'default'"  // NULLの行が除外される
    )
    .fetch_all(pool)
    .await
}
// 問題: NULL != 'default' も NULL になるため、NULLの行は結果に含まれない
```

##### 11.2.2 集約関数でのNULL

```rust
// アンチパターン: NULLを含むカウント
async fn get_user_stats(pool: &PgPool) -> Result<UserStats, sqlx::Error> {
    sqlx::query_as!(
        UserStats,
        r#"
        SELECT
            COUNT(*) as "total_users!",
            COUNT(bio) as "users_with_bio!",  -- NULLは数えられない
            COUNT(email) as "users_with_email!"  -- NOT NULLなので同じ
        FROM users
        "#
    )
    .fetch_one(pool)
    .await
}
```

##### 11.2.3 NULLを避けるための空文字列

```sql
-- アンチパターン: NULLの代わりに空文字列
CREATE TABLE users (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    bio VARCHAR(500) NOT NULL DEFAULT ''  -- NULLを避けている
);
```

```rust
// 問題: 「未入力」と「空」を区別できない
async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await?;

    // bio が '' の場合、未入力なのか意図的に空なのか不明
    if user.bio.is_empty() {
        // ???
    }
    Ok(user)
}
```

##### 11.2.4 センチネル値の使用

```sql
-- アンチパターン: 特別な値でNULLを表現
CREATE TABLE products (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    discontinued_at TIMESTAMPTZ NOT NULL DEFAULT '9999-12-31'  -- NULLの代わり
);
```

```rust
// 問題: マジックナンバーが散らばる
async fn get_active_products(pool: &PgPool) -> Result<Vec<Product>, sqlx::Error> {
    sqlx::query_as!(
        Product,
        "SELECT * FROM products WHERE discontinued_at = '9999-12-31'"  // マジックナンバー
    )
    .fetch_all(pool)
    .await
}
```

#### 11.3 アンチパターンの見つけ方

- `WHERE column = NULL` または `WHERE column != NULL` を使用している
- 空文字列 `''` や `-1`、`'N/A'` などでNULLを代替している
- `COALESCE`や`IFNULL`を多用している
- NULLを含む可能性のある列の集計結果がおかしい

#### 11.4 アンチパターンを用いてもよい場合

- **パフォーマンス最適化**: 一部のケースでNULL以外の値を使うとインデックスが効率的になる
- **レガシーシステム**: NULLを扱えないシステムとの連携

#### 11.5 解決策：NULLを正しく理解し使用する

##### 11.5.1 IS NULL / IS NOT NULL を使用

```rust
// 正しいNULLチェック
async fn find_users_without_bio(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE bio IS NULL"  // 正しい
    )
    .fetch_all(pool)
    .await
}

async fn find_users_with_bio(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE bio IS NOT NULL"  // 正しい
    )
    .fetch_all(pool)
    .await
}
```

##### 11.5.2 COALESCE で安全にデフォルト値を使用

```rust
async fn search_users(pool: &PgPool, query: &str) -> Result<Vec<UserSearchResult>, sqlx::Error> {
    sqlx::query_as!(
        UserSearchResult,
        r#"
        SELECT
            id,
            name,
            COALESCE(bio, 'No bio') as "bio!"  -- NULLの場合のデフォルト値
        FROM users
        WHERE name ILIKE $1 OR COALESCE(bio, '') ILIKE $1
        "#,
        format!("%{}%", query)
    )
    .fetch_all(pool)
    .await
}
```

##### 11.5.3 RustのOption型との対応

```rust
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: Uuid,
    name: String,
    bio: Option<String>,  // NULLableな列はOption<T>
    email: String,        // NOT NULL列は直接T
}

// Optionを活用したビジネスロジック
fn display_user_bio(user: &User) -> &str {
    user.bio.as_deref().unwrap_or("No bio provided")
}

// パターンマッチング
fn process_user(user: User) {
    match user.bio {
        Some(bio) if !bio.is_empty() => println!("Bio: {}", bio),
        Some(_) => println!("Bio is empty"),
        None => println!("Bio not provided"),
    }
}
```

##### 11.5.4 クエリパラメータでのNULL処理

```rust
// Optionをパラメータとして使用
async fn update_user_bio(
    pool: &PgPool,
    user_id: Uuid,
    bio: Option<&str>,  // Noneを渡すとNULLになる
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET bio = $1 WHERE id = $2",
        bio,  // Option<&str>はそのまま渡せる
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

// 条件付きフィルタリング
async fn find_users(
    pool: &PgPool,
    status_filter: Option<&str>,
) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT * FROM users
        WHERE ($1::text IS NULL OR status = $1)
        "#,
        status_filter
    )
    .fetch_all(pool)
    .await
}
```

##### 11.5.5 NOT NULL制約の適切な使用

```sql
-- 必須フィールドにはNOT NULL制約
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,  -- 必須
    name VARCHAR(100) NOT NULL,   -- 必須
    bio VARCHAR(500),             -- 任意（NULLを許可）
    phone VARCHAR(20),            -- 任意（NULLを許可）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### 11.6 三値論理のまとめ

| 式 | 結果 |
|---|---|
| `NULL = NULL` | NULL |
| `NULL != NULL` | NULL |
| `NULL AND TRUE` | NULL |
| `NULL AND FALSE` | FALSE |
| `NULL OR TRUE` | TRUE |
| `NULL OR FALSE` | NULL |
| `NOT NULL` | NULL |

**推奨**: NULLは「値が不明」を意味する特別な値として正しく扱い、`IS NULL`/`IS NOT NULL`で比較する。

---

### 15章 アンビギュアスグループ（曖昧なグループ）

#### 15.1 目的：グループ内で最大値を持つ行を取得する

各カテゴリで最新の投稿、各ユーザーの最高スコア、各月の売上トップなど、グループ化した中から特定の条件を満たす行を取得したい。

#### 15.2 アンチパターン：非グループ化列を参照する

**問題のあるクエリ：**

```sql
-- アンチパターン: GROUP BY に含まれない列を SELECT する
SELECT user_id, title, MAX(created_at) as latest_date
FROM posts
GROUP BY user_id;
-- エラー: title は GROUP BY に含まれていない
```

```rust
// アンチパターン: 曖昧なグループ化
async fn get_latest_posts_per_user_wrong(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    // このクエリは PostgreSQL ではエラーになる
    // MySQL の一部バージョンでは動作するが、どの title が返されるか不定
    sqlx::query_as!(
        Post,
        r#"
        SELECT id, user_id, title, content, status, created_at, updated_at
        FROM posts
        GROUP BY user_id
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 15.2.1 単一値の原則（Single-Value Rule）

```rust
// GROUP BY を使用する場合、SELECT できるのは以下のみ：
// 1. GROUP BY に含まれる列
// 2. 集約関数の結果（COUNT, SUM, MAX, MIN, AVG 等）

// 正しい例：集約関数のみを使用
async fn get_user_post_stats(pool: &PgPool) -> Result<Vec<UserStats>, sqlx::Error> {
    sqlx::query_as!(
        UserStats,
        r#"
        SELECT
            user_id,
            COUNT(*) as "post_count!",
            MAX(created_at) as "latest_post_at"
        FROM posts
        GROUP BY user_id
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 15.2.2 SQLがクエリの意図を汲んでくれるとは限らない

```rust
// 開発者の意図: 最新の投稿の title を取得したい
// 実際の動作: どの title が返されるかは不定（MySQL の ONLY_FULL_GROUP_BY 無効時）

// このような「なんとなく動く」コードは危険
async fn get_user_latest_post_wrong(pool: &PgPool) -> Result<Vec<LatestPost>, sqlx::Error> {
    // MySQL でも ONLY_FULL_GROUP_BY が有効なら以下はエラー
    sqlx::query_as!(
        LatestPost,
        r#"
        SELECT user_id, title, MAX(created_at) as latest_date
        FROM posts
        GROUP BY user_id
        "#
    )
    .fetch_all(pool)
    .await
    // 問題: title は MAX(created_at) の行のものとは限らない
}
```

#### 15.3 アンチパターンの見つけ方

- `GROUP BY` を使用しているが、`SELECT` に集約関数でない列がある
- 「各〇〇で最新/最大の△△を取得」というロジックがある
- MySQL から PostgreSQL に移行した際にエラーが発生
- 結果が期待と異なる（ランダムな値が返される）

#### 15.4 アンチパターンを用いてもよい場合

1. **関数従属性が明らかな場合**: GROUP BY の列で他の列が一意に決まる場合
2. **MySQL の特殊な動作を理解している場合**: ONLY_FULL_GROUP_BY を無効にして意図的に使用

```sql
-- 関数従属性の例: user_id で email が一意に決まる
SELECT u.id, u.email, COUNT(p.id) as post_count
FROM users u
LEFT JOIN posts p ON u.id = p.user_id
GROUP BY u.id;  -- u.email は u.id に関数従属するので OK（PostgreSQL 9.1+）
```

#### 15.5 解決策：曖昧でない列を使用する

##### 15.5.1 ウィンドウ関数を使用する（推奨）

```rust
// 解決策1: ウィンドウ関数 ROW_NUMBER()
async fn get_latest_post_per_user(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT id, user_id, title, content, status, created_at, updated_at
        FROM (
            SELECT
                *,
                ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
            FROM posts
        ) ranked
        WHERE rn = 1
        "#
    )
    .fetch_all(pool)
    .await
}

// N番目まで取得する場合
async fn get_top_n_posts_per_user(pool: &PgPool, n: i32) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT id, user_id, title, content, status, created_at, updated_at
        FROM (
            SELECT
                *,
                ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY created_at DESC) as rn
            FROM posts
        ) ranked
        WHERE rn <= $1
        "#,
        n
    )
    .fetch_all(pool)
    .await
}
```

##### 15.5.2 DISTINCT ON を使用する（PostgreSQL固有）

```rust
// 解決策2: PostgreSQL の DISTINCT ON（シンプルで高速）
async fn get_latest_post_per_user_distinct_on(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT DISTINCT ON (user_id)
            id, user_id, title, content, status, created_at, updated_at
        FROM posts
        ORDER BY user_id, created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
}

// 注意: DISTINCT ON は ORDER BY の最初の列と一致させる必要がある
```

##### 15.5.3 相関サブクエリを使用する

```rust
// 解決策3: 相関サブクエリ
async fn get_latest_post_per_user_subquery(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT p.id, p.user_id, p.title, p.content, p.status, p.created_at, p.updated_at
        FROM posts p
        WHERE p.created_at = (
            SELECT MAX(p2.created_at)
            FROM posts p2
            WHERE p2.user_id = p.user_id
        )
        "#
    )
    .fetch_all(pool)
    .await
    // 注意: 同じ user_id で同じ created_at の投稿が複数あると複数行返る
}
```

##### 15.5.4 JOIN を使用する

```rust
// 解決策4: 導出テーブルとの JOIN
async fn get_latest_post_per_user_join(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT p.id, p.user_id, p.title, p.content, p.status, p.created_at, p.updated_at
        FROM posts p
        INNER JOIN (
            SELECT user_id, MAX(created_at) as max_created_at
            FROM posts
            GROUP BY user_id
        ) latest ON p.user_id = latest.user_id
                AND p.created_at = latest.max_created_at
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 15.5.5 LATERAL JOIN を使用する（PostgreSQL 9.3+）

```rust
// 解決策5: LATERAL JOIN（各ユーザーに対して個別にクエリを実行）
async fn get_latest_posts_with_user_info(pool: &PgPool) -> Result<Vec<UserWithLatestPost>, sqlx::Error> {
    sqlx::query_as!(
        UserWithLatestPost,
        r#"
        SELECT
            u.id as user_id,
            u.name as user_name,
            latest_post.id as "post_id?",
            latest_post.title as "post_title?",
            latest_post.created_at as "post_created_at?"
        FROM users u
        LEFT JOIN LATERAL (
            SELECT id, title, created_at
            FROM posts
            WHERE user_id = u.id
            ORDER BY created_at DESC
            LIMIT 1
        ) latest_post ON true
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 15.5.6 他の列に対しても集約関数を使用する

```rust
// 解決策6: 全ての非グループ化列に集約関数を使用
async fn get_user_post_summary(pool: &PgPool) -> Result<Vec<UserPostSummary>, sqlx::Error> {
    sqlx::query_as!(
        UserPostSummary,
        r#"
        SELECT
            user_id,
            COUNT(*) as "post_count!",
            MAX(created_at) as "latest_post_at",
            MIN(created_at) as "first_post_at",
            string_agg(title, ', ' ORDER BY created_at DESC) as "all_titles"
        FROM posts
        GROUP BY user_id
        "#
    )
    .fetch_all(pool)
    .await
}
```

#### 15.6 アプローチの比較

| アプローチ | パフォーマンス | 可読性 | 同着対応 | 移植性 |
|-----------|--------------|--------|---------|--------|
| ウィンドウ関数 | ◎ | ○ | ◎ | ◎ |
| DISTINCT ON | ◎ | ◎ | △ | × (PostgreSQL) |
| 相関サブクエリ | △ | ○ | × | ◎ |
| JOIN | ○ | △ | × | ◎ |
| LATERAL | ◎ | ○ | ◎ | △ |

**推奨**: PostgreSQLでは`DISTINCT ON`が最もシンプル。移植性が必要な場合はウィンドウ関数を使用。

---

### 16章 ランダムセレクション

#### 16.1 目的：ランダムに1行をフェッチする

「おすすめ記事」「ランダムなクイズ問題」「抽選」など、テーブルからランダムに行を選択したい。

#### 16.2 アンチパターン：データをランダムにソートする

**問題のあるクエリ：**

```rust
// アンチパターン: ORDER BY RANDOM() を使用
async fn get_random_post(pool: &PgPool) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE status = 'published'
        ORDER BY RANDOM()
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await
}
// 問題: 全ての行にランダム値を割り当ててソートするため、O(n log n) の計算量
```

##### 16.2.1 パフォーマンスの問題

```rust
// 100万行のテーブルでの問題
// ORDER BY RANDOM() は以下の処理を行う：
// 1. 全ての行を読み込む
// 2. 各行にランダム値を割り当てる
// 3. ランダム値でソートする（O(n log n)）
// 4. 最初の1行を返す

// ベンチマーク例：
// 1,000行: ~10ms
// 100,000行: ~500ms
// 1,000,000行: ~5秒
```

#### 16.3 アンチパターンの見つけ方

- `ORDER BY RANDOM()` または `ORDER BY RAND()` がクエリにある
- ランダム取得クエリが遅い
- テーブルサイズに比例してクエリ時間が増加

#### 16.4 アンチパターンを用いてもよい場合

1. **テーブルサイズが小さい場合**: 数千行以下なら許容範囲
2. **頻度が低い場合**: 1日1回程度のバッチ処理
3. **シンプルさを優先する場合**: プロトタイプや管理ツール

#### 16.5 解決策：特定の順番に依存しない

##### 16.5.1 オフセットを使用する

```rust
// 解決策1: ランダムなオフセットを使用（シンプルで効率的）
async fn get_random_post_offset(pool: &PgPool) -> Result<Option<Post>, sqlx::Error> {
    // 1. 総件数を取得
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM posts WHERE status = 'published'"
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    if count == 0 {
        return Ok(None);
    }

    // 2. ランダムなオフセットを生成
    use rand::Rng;
    let offset = rand::thread_rng().gen_range(0..count);

    // 3. オフセットを使用して取得
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE status = 'published'
        ORDER BY id
        OFFSET $1
        LIMIT 1
        "#,
        offset
    )
    .fetch_optional(pool)
    .await
}
```

##### 16.5.2 IDの範囲からランダムに選択

```rust
// 解決策2: ID範囲からランダムに選択（欠番がある場合は再試行）
async fn get_random_post_by_id_range(pool: &PgPool) -> Result<Option<Post>, sqlx::Error> {
    // 1. IDの最小値と最大値を取得
    let range = sqlx::query!(
        r#"
        SELECT MIN(id) as "min_id", MAX(id) as "max_id"
        FROM posts
        WHERE status = 'published'
        "#
    )
    .fetch_one(pool)
    .await?;

    let (min_id, max_id) = match (range.min_id, range.max_id) {
        (Some(min), Some(max)) => (min, max),
        _ => return Ok(None),
    };

    use rand::Rng;

    // 2. 最大5回まで試行
    for _ in 0..5 {
        let random_id = rand::thread_rng().gen_range(min_id..=max_id);

        let post = sqlx::query_as!(
            Post,
            r#"
            SELECT * FROM posts
            WHERE id >= $1 AND status = 'published'
            ORDER BY id
            LIMIT 1
            "#,
            random_id
        )
        .fetch_optional(pool)
        .await?;

        if post.is_some() {
            return Ok(post);
        }
    }

    // フォールバック
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE status = 'published' LIMIT 1"
    )
    .fetch_optional(pool)
    .await
}
```

##### 16.5.3 事前にランダム値を保存する

```rust
// 解決策3: ランダム値カラムを追加して事前計算
// スキーマ:
// ALTER TABLE posts ADD COLUMN random_sort DOUBLE PRECISION DEFAULT RANDOM();
// CREATE INDEX idx_posts_random_sort ON posts(random_sort) WHERE status = 'published';

async fn get_random_post_precomputed(pool: &PgPool) -> Result<Option<Post>, sqlx::Error> {
    use rand::Rng;
    let threshold: f64 = rand::thread_rng().gen();

    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE status = 'published' AND random_sort >= $1
        ORDER BY random_sort
        LIMIT 1
        "#,
        threshold
    )
    .fetch_optional(pool)
    .await
}

// 定期的にランダム値を更新（バッチジョブ）
async fn refresh_random_sort(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE posts SET random_sort = RANDOM()")
        .execute(pool)
        .await?;
    Ok(())
}
```

##### 16.5.4 TABLESAMPLE を使用する（PostgreSQL 9.5+）

```rust
// 解決策4: TABLESAMPLE（大きなテーブルに最適）
async fn get_random_posts_tablesample(pool: &PgPool, sample_size: i32) -> Result<Vec<Post>, sqlx::Error> {
    // SYSTEM は高速だが、不均一になる可能性がある
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts TABLESAMPLE SYSTEM(1)
        WHERE status = 'published'
        LIMIT $1
        "#,
        sample_size
    )
    .fetch_all(pool)
    .await
}

// BERNOULLI はより均一だが、少し遅い
async fn get_random_posts_bernoulli(pool: &PgPool) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts TABLESAMPLE BERNOULLI(0.1)
        WHERE status = 'published'
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 16.5.5 キャッシュを活用する

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct RandomPostCache {
    pool: PgPool,
    cache: Arc<RwLock<Vec<Uuid>>>,
}

impl RandomPostCache {
    // キャッシュを定期的に更新（例: 1時間ごと）
    async fn refresh_cache(&self) -> Result<(), sqlx::Error> {
        let ids: Vec<Uuid> = sqlx::query_scalar!(
            "SELECT id FROM posts WHERE status = 'published'"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cache = self.cache.write().await;
        *cache = ids;
        Ok(())
    }

    // キャッシュからランダムに選択
    async fn get_random(&self) -> Result<Option<Post>, sqlx::Error> {
        let cache = self.cache.read().await;

        if cache.is_empty() {
            return Ok(None);
        }

        use rand::seq::SliceRandom;
        let random_id = cache.choose(&mut rand::thread_rng());

        match random_id {
            Some(id) => {
                sqlx::query_as!(
                    Post,
                    "SELECT * FROM posts WHERE id = $1",
                    id
                )
                .fetch_optional(&self.pool)
                .await
            }
            None => Ok(None),
        }
    }
}
```

#### 16.6 アプローチの比較

| アプローチ | 計算量 | 均一性 | 欠番対応 | 複雑さ |
|-----------|--------|--------|---------|--------|
| ORDER BY RANDOM() | O(n log n) | ◎ | ◎ | ◎ |
| オフセット | O(offset) | ◎ | ◎ | ○ |
| ID範囲 | O(1) | △ | △ | △ |
| 事前計算 | O(1) | ○ | ◎ | △ |
| TABLESAMPLE | O(1) | △ | ◎ | ○ |
| キャッシュ | O(1) | ◎ | ◎ | × |

**推奨**: 小規模なら`ORDER BY RANDOM()`、大規模ならオフセット方式またはキャッシュを使用。

---

### 17章 プアマンズ・サーチエンジン（貧者のサーチエンジン）

#### 17.1 目的：全文検索を行う

ブログ記事の本文、製品の説明文、コメントなどから、キーワードに一致するレコードを検索したい。

#### 17.2 アンチパターン：パターンマッチ述語を使用する

**問題のあるクエリ：**

```rust
// アンチパターン: LIKE でワイルドカード検索
async fn search_posts_like(pool: &PgPool, keyword: &str) -> Result<Vec<Post>, sqlx::Error> {
    let pattern = format!("%{}%", keyword);

    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE title LIKE $1 OR content LIKE $1
        ORDER BY created_at DESC
        "#,
        pattern
    )
    .fetch_all(pool)
    .await
}
// 問題:
// 1. インデックスが使われない（前方一致以外）
// 2. 大文字小文字を区別する
// 3. 単語の境界を考慮しない
// 4. 関連度のランキングができない
```

##### 17.2.1 パフォーマンスの問題

```rust
// 前方ワイルドカード（%keyword）ではインデックスが使われない
// 全件スキャンが発生

// さらに複数カラムの OR 検索は最悪
async fn search_posts_multiple_columns(
    pool: &PgPool,
    keyword: &str,
) -> Result<Vec<Post>, sqlx::Error> {
    let pattern = format!("%{}%", keyword);

    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE title LIKE $1
           OR content LIKE $1
           OR EXISTS (
               SELECT 1 FROM comments
               WHERE post_id = posts.id AND body LIKE $1
           )
        "#,
        pattern
    )
    .fetch_all(pool)
    .await
    // 非常に遅い
}
```

##### 17.2.2 検索品質の問題

```rust
// LIKE は単純な文字列マッチング
// "Rust" で検索すると "trust", "rusty" もヒットしてしまう

// 大文字小文字の問題
// "rust" で検索しても "Rust" はヒットしない
async fn search_case_insensitive(pool: &PgPool, keyword: &str) -> Result<Vec<Post>, sqlx::Error> {
    let pattern = format!("%{}%", keyword.to_lowercase());

    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE LOWER(title) LIKE $1",  // インデックス使用不可
        pattern
    )
    .fetch_all(pool)
    .await
}
```

#### 17.3 アンチパターンの見つけ方

- `LIKE '%keyword%'` パターンがある
- 検索が遅いという報告がある
- 検索結果の関連度が低い
- 「〇〇で検索してもヒットしない」という問い合わせ

#### 17.4 アンチパターンを用いてもよい場合

1. **前方一致検索のみ**: `LIKE 'keyword%'` はインデックスを使用できる
2. **小規模なテーブル**: 数千行以下
3. **管理ツールやデバッグ用**: 頻繁に使用されない

```rust
// 前方一致検索はインデックスを使用できる
// CREATE INDEX idx_posts_title ON posts(title varchar_pattern_ops);
async fn search_posts_prefix(pool: &PgPool, prefix: &str) -> Result<Vec<Post>, sqlx::Error> {
    let pattern = format!("{}%", prefix);

    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE title LIKE $1",
        pattern
    )
    .fetch_all(pool)
    .await
}
```

#### 17.5 解決策：適切なツールを使用する

##### 17.5.1 PostgreSQLの全文検索機能を使用する

```sql
-- スキーマ: 全文検索用カラムとインデックスを追加
ALTER TABLE posts ADD COLUMN search_vector tsvector;

-- トリガーで自動更新
CREATE OR REPLACE FUNCTION posts_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('japanese', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('japanese', COALESCE(NEW.content, '')), 'B');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER posts_search_vector_trigger
    BEFORE INSERT OR UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_search_vector_update();

-- GINインデックス
CREATE INDEX idx_posts_search_vector ON posts USING GIN(search_vector);

-- 既存データを更新
UPDATE posts SET search_vector =
    setweight(to_tsvector('japanese', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('japanese', COALESCE(content, '')), 'B');
```

```rust
// 全文検索クエリ
async fn search_posts_fulltext(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    sqlx::query_as!(
        SearchResult,
        r#"
        SELECT
            id,
            title,
            ts_headline('japanese', content, plainto_tsquery('japanese', $1),
                'StartSel=<mark>, StopSel=</mark>, MaxWords=50') as "snippet!",
            ts_rank(search_vector, plainto_tsquery('japanese', $1)) as "rank!"
        FROM posts
        WHERE search_vector @@ plainto_tsquery('japanese', $1)
        ORDER BY rank DESC
        LIMIT 20
        "#,
        query
    )
    .fetch_all(pool)
    .await
}

// 複雑な検索クエリ（AND, OR, NOT）
async fn search_posts_advanced(
    pool: &PgPool,
    query: &str,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    sqlx::query_as!(
        SearchResult,
        r#"
        SELECT
            id,
            title,
            ts_rank_cd(search_vector, to_tsquery('japanese', $1)) as "rank!"
        FROM posts
        WHERE search_vector @@ to_tsquery('japanese', $1)
        ORDER BY rank DESC
        "#,
        query  // 例: "rust & (web | api) & !python"
    )
    .fetch_all(pool)
    .await
}
```

##### 17.5.2 pg_trgm 拡張機能を使用する（あいまい検索）

```sql
-- pg_trgm 拡張機能を有効化
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- トライグラムインデックス
CREATE INDEX idx_posts_title_trgm ON posts USING GIN(title gin_trgm_ops);
CREATE INDEX idx_posts_content_trgm ON posts USING GIN(content gin_trgm_ops);
```

```rust
// 類似検索（タイプミスに強い）
async fn search_posts_similar(
    pool: &PgPool,
    query: &str,
    threshold: f32,
) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        r#"
        SELECT *
        FROM posts
        WHERE similarity(title, $1) > $2
           OR similarity(content, $1) > $2
        ORDER BY GREATEST(similarity(title, $1), similarity(content, $1)) DESC
        LIMIT 20
        "#,
        query,
        threshold  // 通常 0.3〜0.5
    )
    .fetch_all(pool)
    .await
}

// LIKE の高速化（pg_trgm インデックスを使用）
async fn search_posts_like_fast(
    pool: &PgPool,
    keyword: &str,
) -> Result<Vec<Post>, sqlx::Error> {
    let pattern = format!("%{}%", keyword);

    // pg_trgm インデックスにより高速化される
    sqlx::query_as!(
        Post,
        r#"
        SELECT * FROM posts
        WHERE title ILIKE $1 OR content ILIKE $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
        pattern
    )
    .fetch_all(pool)
    .await
}
```

##### 17.5.3 外部検索エンジンを使用する

```rust
// Meilisearch との連携例
use meilisearch_sdk::client::Client;

struct SearchService {
    db_pool: PgPool,
    meili_client: Client,
}

impl SearchService {
    // インデックスを更新
    async fn index_post(&self, post: &Post) -> Result<(), AppError> {
        let document = serde_json::json!({
            "id": post.id,
            "title": post.title,
            "content": post.content,
            "user_id": post.user_id,
            "created_at": post.created_at.timestamp(),
        });

        self.meili_client
            .index("posts")
            .add_documents(&[document], Some("id"))
            .await?;

        Ok(())
    }

    // 検索
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Post>, AppError> {
        let results = self.meili_client
            .index("posts")
            .search()
            .with_query(query)
            .with_limit(limit)
            .execute::<serde_json::Value>()
            .await?;

        // 検索結果のIDでDBから取得
        let ids: Vec<Uuid> = results
            .hits
            .iter()
            .filter_map(|hit| {
                hit.result.get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .collect();

        let posts = sqlx::query_as!(
            Post,
            "SELECT * FROM posts WHERE id = ANY($1)",
            &ids
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(posts)
    }
}
```

#### 17.6 アプローチの比較

| アプローチ | 設定難度 | 日本語対応 | ランキング | スケーラビリティ |
|-----------|---------|-----------|-----------|-----------------|
| LIKE | ◎ | △ | × | × |
| tsvector | △ | ○ | ◎ | ○ |
| pg_trgm | ○ | ◎ | △ | ○ |
| Meilisearch | △ | ◎ | ◎ | ◎ |
| Elasticsearch | × | ◎ | ◎ | ◎ |

**推奨**:
- 簡単な検索: pg_trgm
- 本格的な全文検索: PostgreSQL tsvector または外部検索エンジン

---

### 18章 スパゲッティクエリ

#### 12.1 目的：SQLクエリの数を減らす

複数の情報を1回のクエリで取得し、データベースへのラウンドトリップを減らしたい。

#### 12.2 アンチパターン：複雑な問題を1つのクエリで解決しようとする

**問題のあるクエリ：**

```rust
// アンチパターン: 1つの巨大なクエリですべてを取得
async fn get_dashboard_data(pool: &PgPool, user_id: Uuid) -> Result<DashboardData, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT
            -- ユーザー情報
            u.id, u.name, u.email,
            -- 投稿統計
            (SELECT COUNT(*) FROM posts WHERE user_id = u.id) as post_count,
            (SELECT COUNT(*) FROM posts WHERE user_id = u.id AND status = 'published') as published_count,
            (SELECT COUNT(*) FROM posts WHERE user_id = u.id AND status = 'draft') as draft_count,
            -- コメント統計
            (SELECT COUNT(*) FROM comments WHERE user_id = u.id) as comment_count,
            -- 最新の投稿
            (SELECT title FROM posts WHERE user_id = u.id ORDER BY created_at DESC LIMIT 1) as latest_post_title,
            -- フォロワー数
            (SELECT COUNT(*) FROM follows WHERE following_id = u.id) as follower_count,
            -- フォロー数
            (SELECT COUNT(*) FROM follows WHERE follower_id = u.id) as following_count,
            -- 今月のビュー数
            (SELECT COALESCE(SUM(view_count), 0) FROM posts
             WHERE user_id = u.id
             AND created_at >= date_trunc('month', CURRENT_DATE)) as monthly_views
        FROM users u
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    // ... 結果の処理
}
// 問題:
// 1. 読みにくく、保守が困難
// 2. サブクエリが多く、パフォーマンスが悪い可能性
// 3. 1つの変更が全体に影響
```

##### 12.2.1 デカルト積の発生

```rust
// アンチパターン: 関係のないデータを同時にJOIN
async fn get_user_with_everything(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT u.*, p.*, c.*
        FROM users u
        LEFT JOIN posts p ON p.user_id = u.id
        LEFT JOIN comments c ON c.user_id = u.id
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    // 問題: posts 10件 × comments 100件 = 1000行が返される（デカルト積）
}
```

#### 12.3 アンチパターンの見つけ方

- 1つのクエリが100行を超える
- 多数のサブクエリやCASE式がある
- 同じテーブルを複数回JOINしている
- クエリの実行時間が長い
- クエリの結果の行数が予想より多い

#### 12.4 アンチパターンを用いてもよい場合

- レポート生成など、1回限りの複雑な集計
- バッチ処理で効率が重要な場合

#### 12.5 解決策：分割統治を行う

##### 12.5.1 クエリを論理的に分割

```rust
// 解決策: 目的ごとにクエリを分割
struct DashboardData {
    user: User,
    post_stats: PostStats,
    activity: UserActivity,
}

async fn get_dashboard_data(pool: &PgPool, user_id: Uuid) -> Result<DashboardData, anyhow::Error> {
    // ユーザー情報
    let user = get_user(pool, user_id).await?;

    // 投稿統計（並列実行可能）
    let post_stats = get_post_stats(pool, user_id).await?;

    // アクティビティ
    let activity = get_user_activity(pool, user_id).await?;

    Ok(DashboardData { user, post_stats, activity })
}

async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
}

async fn get_post_stats(pool: &PgPool, user_id: Uuid) -> Result<PostStats, sqlx::Error> {
    sqlx::query_as!(
        PostStats,
        r#"
        SELECT
            COUNT(*) as "total!",
            COUNT(*) FILTER (WHERE status = 'published') as "published!",
            COUNT(*) FILTER (WHERE status = 'draft') as "draft!"
        FROM posts
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
}
```

##### 12.5.2 並列クエリの実行

```rust
use tokio::try_join;

async fn get_dashboard_data(pool: &PgPool, user_id: Uuid) -> Result<DashboardData, anyhow::Error> {
    // 複数のクエリを並列実行
    let (user, post_stats, comment_count, follower_count) = try_join!(
        get_user(pool, user_id),
        get_post_stats(pool, user_id),
        get_comment_count(pool, user_id),
        get_follower_count(pool, user_id),
    )?;

    Ok(DashboardData {
        user,
        post_stats,
        comment_count,
        follower_count,
    })
}
```

##### 12.5.3 CTEを使った構造化

```rust
// Common Table Expression で段階的に構築
async fn get_user_summary(pool: &PgPool, user_id: Uuid) -> Result<UserSummary, sqlx::Error> {
    sqlx::query_as!(
        UserSummary,
        r#"
        WITH user_posts AS (
            SELECT user_id, COUNT(*) as count
            FROM posts
            WHERE user_id = $1
            GROUP BY user_id
        ),
        user_comments AS (
            SELECT user_id, COUNT(*) as count
            FROM comments
            WHERE user_id = $1
            GROUP BY user_id
        )
        SELECT
            u.id,
            u.name,
            COALESCE(p.count, 0) as "post_count!",
            COALESCE(c.count, 0) as "comment_count!"
        FROM users u
        LEFT JOIN user_posts p ON u.id = p.user_id
        LEFT JOIN user_comments c ON u.id = c.user_id
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
}
```

##### 12.5.4 ビューの活用

```sql
-- 複雑な集計をビューとして定義
CREATE VIEW user_stats AS
SELECT
    u.id as user_id,
    COUNT(DISTINCT p.id) as post_count,
    COUNT(DISTINCT c.id) as comment_count,
    COALESCE(SUM(p.view_count), 0) as total_views
FROM users u
LEFT JOIN posts p ON p.user_id = u.id
LEFT JOIN comments c ON c.user_id = u.id
GROUP BY u.id;
```

```rust
// ビューからシンプルにクエリ
async fn get_user_stats(pool: &PgPool, user_id: Uuid) -> Result<UserStats, sqlx::Error> {
    sqlx::query_as!(
        UserStats,
        "SELECT * FROM user_stats WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
}
```

#### 12.6 クエリ分割のガイドライン

| 状況 | アプローチ |
|------|-----------|
| 独立したデータの取得 | 別クエリ + 並列実行 |
| 関連するデータの集計 | CTE または ビュー |
| 階層的なデータ | 再帰CTE |
| キャッシュ可能な集計 | マテリアライズドビュー |

**推奨**: クエリは単一の責任を持つようにし、複雑な処理は分割して並列実行する。

---

### 19章 N+1クエリ問題

#### 13.1 目的：関連データを効率的に取得する

投稿一覧を取得し、それぞれの投稿の著者情報やコメント数も表示したい。

#### 13.2 アンチパターン：ループ内でクエリを実行する

**問題のあるコード：**

```rust
// アンチパターン: N+1クエリ
async fn get_posts_with_authors(pool: &PgPool) -> Result<Vec<PostWithAuthor>, anyhow::Error> {
    // 1回目のクエリ: 全投稿を取得
    let posts: Vec<Post> = sqlx::query_as!(Post, "SELECT * FROM posts LIMIT 100")
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();

    // N回のクエリ: 各投稿ごとに著者を取得
    for post in posts {
        let author: User = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            post.user_id
        )
        .fetch_one(pool)
        .await?;

        result.push(PostWithAuthor { post, author });
    }

    Ok(result)
}
// 問題: 100件の投稿で101回のクエリが実行される
```

##### 13.2.1 ネストしたN+1

```rust
// アンチパターン: 二重のN+1
async fn get_posts_with_comments_and_authors(
    pool: &PgPool,
) -> Result<Vec<PostDetail>, anyhow::Error> {
    let posts: Vec<Post> = sqlx::query_as!(Post, "SELECT * FROM posts LIMIT 10")
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();

    for post in posts {
        // 投稿ごとに著者を取得（N回）
        let author = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", post.user_id)
            .fetch_one(pool)
            .await?;

        // 投稿ごとにコメントを取得（N回）
        let comments: Vec<Comment> = sqlx::query_as!(
            Comment,
            "SELECT * FROM comments WHERE post_id = $1",
            post.id
        )
        .fetch_all(pool)
        .await?;

        // 各コメントの著者を取得（N×M回！）
        let mut comment_details = Vec::new();
        for comment in comments {
            let commenter = sqlx::query_as!(
                User,
                "SELECT * FROM users WHERE id = $1",
                comment.user_id
            )
            .fetch_one(pool)
            .await?;
            comment_details.push(CommentDetail { comment, author: commenter });
        }

        result.push(PostDetail { post, author, comments: comment_details });
    }

    Ok(result)
}
// 問題: 10投稿×10コメント = 1 + 10 + 10 + 100 = 121クエリ
```

#### 13.3 アンチパターンの見つけ方

- `for`ループ内で`sqlx::query`を実行している
- 一覧画面の表示が遅い
- データベースのクエリログに同じパターンのクエリが大量に記録されている
- `EXPLAIN ANALYZE`で個々のクエリは速いのに全体が遅い

#### 13.4 アンチパターンを用いてもよい場合

- 取得件数が非常に少ない（5件以下）
- 関連データがキャッシュされている
- 段階的なデータ取得が必要（ページング時に次のページのみ取得）

#### 13.5 解決策：一括取得とJOIN

##### 13.5.1 JOINを使用

```rust
// 解決策: JOINで一括取得
#[derive(Debug, sqlx::FromRow)]
struct PostWithAuthorRow {
    // Post fields
    post_id: Uuid,
    title: String,
    content: String,
    // User fields
    author_id: Uuid,
    author_name: String,
    author_email: String,
}

async fn get_posts_with_authors(pool: &PgPool) -> Result<Vec<PostWithAuthor>, sqlx::Error> {
    let rows = sqlx::query_as!(
        PostWithAuthorRow,
        r#"
        SELECT
            p.id as post_id,
            p.title,
            p.content,
            u.id as author_id,
            u.name as author_name,
            u.email as author_email
        FROM posts p
        INNER JOIN users u ON p.user_id = u.id
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await?;

    // 行をドメインオブジェクトに変換
    let result = rows.into_iter().map(|row| PostWithAuthor {
        post: Post {
            id: row.post_id,
            title: row.title,
            content: row.content,
        },
        author: User {
            id: row.author_id,
            name: row.author_name,
            email: row.author_email,
        },
    }).collect();

    Ok(result)
}
// 1回のクエリで完了
```

##### 13.5.2 IN句を使った一括取得

```rust
// 解決策: IN句で関連データを一括取得
async fn get_posts_with_authors(pool: &PgPool) -> Result<Vec<PostWithAuthor>, anyhow::Error> {
    // 1. 投稿を取得
    let posts: Vec<Post> = sqlx::query_as!(Post, "SELECT * FROM posts LIMIT 100")
        .fetch_all(pool)
        .await?;

    // 2. 必要なユーザーIDを収集
    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();

    // 3. ユーザーを一括取得
    let users: Vec<User> = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = ANY($1)",
        &user_ids
    )
    .fetch_all(pool)
    .await?;

    // 4. HashMapでルックアップを高速化
    let user_map: HashMap<Uuid, User> = users.into_iter().map(|u| (u.id, u)).collect();

    // 5. 結合
    let result = posts
        .into_iter()
        .filter_map(|post| {
            user_map.get(&post.user_id).map(|author| PostWithAuthor {
                post,
                author: author.clone(),
            })
        })
        .collect();

    Ok(result)
}
// 2回のクエリで完了
```

##### 13.5.3 DataLoaderパターン

```rust
use std::collections::HashMap;
use async_trait::async_trait;

// DataLoader: バッチ処理でN+1を解決
struct UserLoader {
    pool: PgPool,
}

impl UserLoader {
    async fn load_many(&self, ids: &[Uuid]) -> Result<HashMap<Uuid, User>, sqlx::Error> {
        let users: Vec<User> = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = ANY($1)",
            ids
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users.into_iter().map(|u| (u.id, u)).collect())
    }
}

// 使用例
async fn get_posts_with_authors(
    pool: &PgPool,
    loader: &UserLoader,
) -> Result<Vec<PostWithAuthor>, anyhow::Error> {
    let posts: Vec<Post> = sqlx::query_as!(Post, "SELECT * FROM posts LIMIT 100")
        .fetch_all(pool)
        .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let users = loader.load_many(&user_ids).await?;

    let result = posts
        .into_iter()
        .filter_map(|post| {
            users.get(&post.user_id).cloned().map(|author| PostWithAuthor { post, author })
        })
        .collect();

    Ok(result)
}
```

##### 13.5.4 サブクエリを使った集計

```rust
// コメント数を効率的に取得
async fn get_posts_with_comment_count(pool: &PgPool) -> Result<Vec<PostWithStats>, sqlx::Error> {
    sqlx::query_as!(
        PostWithStats,
        r#"
        SELECT
            p.id,
            p.title,
            p.content,
            (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as "comment_count!"
        FROM posts p
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await
}

// または、LEFT JOIN + GROUP BY
async fn get_posts_with_comment_count_v2(pool: &PgPool) -> Result<Vec<PostWithStats>, sqlx::Error> {
    sqlx::query_as!(
        PostWithStats,
        r#"
        SELECT
            p.id,
            p.title,
            p.content,
            COUNT(c.id) as "comment_count!"
        FROM posts p
        LEFT JOIN comments c ON c.post_id = p.id
        GROUP BY p.id
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await
}
```

##### 13.5.5 PostgreSQLの配列集約

```rust
// コメントを配列として取得
async fn get_posts_with_comments(pool: &PgPool) -> Result<Vec<PostWithComments>, sqlx::Error> {
    sqlx::query_as!(
        PostWithCommentsRow,
        r#"
        SELECT
            p.id,
            p.title,
            COALESCE(
                ARRAY_AGG(
                    jsonb_build_object(
                        'id', c.id,
                        'body', c.body,
                        'user_id', c.user_id
                    )
                ) FILTER (WHERE c.id IS NOT NULL),
                '{}'
            ) as "comments!: Json<Vec<CommentJson>>"
        FROM posts p
        LEFT JOIN comments c ON c.post_id = p.id
        GROUP BY p.id
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await
}
```

#### 13.6 解決策の比較

| 方法 | クエリ数 | 複雑さ | 適用場面 |
|------|---------|--------|----------|
| ループ内クエリ | N+1 | 低 | × 避けるべき |
| JOIN | 1 | 中 | 1対1、1対多 |
| IN句 | 2 | 中 | 多対多、条件付き |
| DataLoader | 2 | 高 | 再利用性が必要 |
| 配列集約 | 1 | 高 | PostgreSQL固有 |

**推奨**: 1対1/1対多はJOIN、多対多はIN句を使用する。

---

### 20章 インプリシットカラム（暗黙の列）

#### 19.1 目的：タイプ数を減らす

クエリを書く際に、全ての列名を書くのが面倒なので`SELECT *`を使いたい。

#### 19.2 アンチパターン：ショートカットの罠に陥る

**問題のあるコード：**

```rust
// アンチパターン: SELECT * を使用
async fn get_user(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
}

// アンチパターン: INSERT で列名を省略
async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, NOW(), NOW())",  // 列名がない！
        id,
        email,
        name
    )
    .execute(pool)
    .await?;

    Ok(id)
}
```

##### 19.2.1 リファクタリングにおける問題

```rust
// スキーマ変更前:
// users (id, email, name, created_at, updated_at)

// スキーマ変更後（profile_image カラムを追加）:
// users (id, email, name, profile_image, created_at, updated_at)

// 問題1: SELECT * は新しいカラムも取得する
async fn get_user(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,  // この構造体にも profile_image が必要になる
        "SELECT * FROM users WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
}

// 問題2: INSERT の列名省略はエラーになる
async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, NOW(), NOW())",  // カラム数不一致！
        Uuid::new_v4(),
        email,
        name
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

##### 19.2.2 パフォーマンスの問題

```rust
// 大きなカラム（TEXT、BYTEA）が含まれている場合
// users (id, email, name, bio TEXT, avatar BYTEA, ...)

// アンチパターン: 必要ない大きなカラムも取得
async fn list_user_names(pool: &PgPool) -> Result<Vec<UserName>, sqlx::Error> {
    sqlx::query_as!(
        UserName,
        "SELECT * FROM users"  // bio や avatar も転送される！
    )
    .fetch_all(pool)
    .await
}
// 問題: ネットワーク帯域とメモリを浪費
```

##### 19.2.3 JOINでの曖昧さ

```rust
// 複数テーブルに同じ名前のカラムがある場合
async fn get_post_with_author_ambiguous(pool: &PgPool, post_id: Uuid) -> Result<PostWithAuthor, sqlx::Error> {
    sqlx::query_as!(
        PostWithAuthor,
        r#"
        SELECT *
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.id = $1
        "#,
        post_id
    )
    .fetch_one(pool)
    .await
    // 問題: posts.id と users.id、posts.created_at と users.created_at が競合
}
```

#### 19.3 アンチパターンの見つけ方

- `SELECT *` がクエリに含まれている
- `INSERT INTO table VALUES (...)` で列名が指定されていない
- スキーマ変更後にアプリケーションがエラーになる
- JOINクエリで「曖昧な列名」エラーが出る
- 不要なデータを大量に転送している

#### 19.4 アンチパターンを用いてもよい場合

1. **アドホッククエリ**: 開発中のデバッグや調査
2. **テーブルの全カラムが必要な場合**: 単一テーブルからの取得で全列が必要
3. **動的スキーマ**: EAVパターンなどでカラムが動的

```rust
// 許容される例: デバッグ用のクエリ
async fn debug_dump_users(pool: &PgPool) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    sqlx::query_scalar!("SELECT to_jsonb(users.*) FROM users LIMIT 10")
        .fetch_all(pool)
        .await
}
```

#### 19.5 解決策：列名を明示的に指定する

##### 19.5.1 SELECT で列名を明示する

```rust
// 解決策: 必要な列のみを明示的に指定
async fn get_user(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, email, name, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(pool)
    .await
}

// 必要な列だけを取得（パフォーマンス向上）
async fn list_user_names(pool: &PgPool) -> Result<Vec<UserName>, sqlx::Error> {
    sqlx::query_as!(
        UserName,
        "SELECT id, name FROM users ORDER BY name"
    )
    .fetch_all(pool)
    .await
}
```

##### 19.5.2 INSERT で列名を明示する

```rust
// 解決策: INSERT でも列名を明示
async fn create_user(
    pool: &PgPool,
    email: &str,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO users (id, email, name, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
        id,
        email,
        name
    )
    .execute(pool)
    .await?;

    Ok(id)
}

// RETURNING を使用して挿入結果を取得
async fn create_user_returning(
    pool: &PgPool,
    email: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, email, name, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        RETURNING id, email, name, created_at, updated_at
        "#,
        Uuid::new_v4(),
        email,
        name
    )
    .fetch_one(pool)
    .await
}
```

##### 19.5.3 JOINでエイリアスを使用する

```rust
// 解決策: JOINでは必ずエイリアスを使用
async fn get_post_with_author(pool: &PgPool, post_id: Uuid) -> Result<PostWithAuthor, sqlx::Error> {
    sqlx::query_as!(
        PostWithAuthor,
        r#"
        SELECT
            p.id as post_id,
            p.title,
            p.content,
            p.status,
            p.created_at as post_created_at,
            u.id as author_id,
            u.name as author_name,
            u.email as author_email
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.id = $1
        "#,
        post_id
    )
    .fetch_one(pool)
    .await
}

// 対応する構造体
#[derive(Debug)]
struct PostWithAuthor {
    post_id: Uuid,
    title: String,
    content: String,
    status: String,
    post_created_at: chrono::DateTime<chrono::Utc>,
    author_id: Uuid,
    author_name: String,
    author_email: String,
}
```

##### 19.5.4 sqlxのマクロで型安全性を確保

```rust
// sqlx::query_as! マクロはコンパイル時に列名と型をチェック
// スキーマ変更時にコンパイルエラーで検出できる

// 存在しない列を指定するとコンパイルエラー
async fn get_user_invalid(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, email, nonexistent_column FROM users WHERE id = $1",  // コンパイルエラー！
        id
    )
    .fetch_one(pool)
    .await
}

// 型が一致しない場合もコンパイルエラー
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: Uuid,
    email: String,
    name: String,
    created_at: i32,  // 型が違う！コンパイルエラー
}
```

##### 19.5.5 ビューを活用する

```rust
// よく使うクエリパターンはビューとして定義
// CREATE VIEW user_summaries AS
//     SELECT id, email, name, created_at
//     FROM users;

async fn list_users(pool: &PgPool) -> Result<Vec<UserSummary>, sqlx::Error> {
    sqlx::query_as!(
        UserSummary,
        "SELECT * FROM user_summaries ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
}

// 複雑なJOINもビュー化
// CREATE VIEW posts_with_authors AS
//     SELECT
//         p.id, p.title, p.content, p.status, p.created_at,
//         u.id as author_id, u.name as author_name
//     FROM posts p
//     JOIN users u ON p.user_id = u.id;
```

#### 19.6 ベストプラクティス

| 場面 | 推奨 | 避けるべき |
|------|------|-----------|
| SELECT | 必要な列を明示 | SELECT * |
| INSERT | 列名を明示 | VALUES のみ |
| JOIN | エイリアスを使用 | 曖昧な列名 |
| 大きなテーブル | 必要な列のみ | 全列取得 |

**推奨**: 常に列名を明示的に指定する。sqlxのマクロを活用してコンパイル時チェックを有効にする。

---

## 第V部 アプリケーション開発のアンチパターン

### 21章 SQLインジェクション

#### 21.1 目的：動的SQLを記述する

ユーザー入力に基づいて検索条件やソート順を動的に変更したい。

#### 14.2 アンチパターン：未検証の入力をSQLに組み込む

**危険なコード：**

```rust
// アンチパターン: 文字列連結でSQLを組み立て
async fn search_users_unsafe(pool: &PgPool, search_term: &str) -> Result<Vec<User>, sqlx::Error> {
    let query = format!(
        "SELECT * FROM users WHERE name LIKE '%{}%'",
        search_term  // 危険！
    );

    sqlx::query_as::<_, User>(&query)
        .fetch_all(pool)
        .await
}
// 攻撃例: search_term = "'; DROP TABLE users; --"
```

```rust
// アンチパターン: 動的なカラム名
async fn get_users_sorted_unsafe(pool: &PgPool, sort_column: &str) -> Result<Vec<User>, sqlx::Error> {
    let query = format!(
        "SELECT * FROM users ORDER BY {}",
        sort_column  // 危険！
    );

    sqlx::query_as::<_, User>(&query)
        .fetch_all(pool)
        .await
}
// 攻撃例: sort_column = "1; DELETE FROM users; --"
```

#### 14.3 アンチパターンの見つけ方

- `format!()`や文字列連結でSQLクエリを組み立てている
- ユーザー入力がそのままSQLに含まれている
- `query_as::<_, T>(&dynamic_string)`のパターンが見られる

#### 14.4 解決策：パラメータ化クエリと入力検証

##### 14.4.1 プレースホルダを使用

```rust
// 正しいアプローチ: プレースホルダ
async fn search_users_safe(pool: &PgPool, search_term: &str) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE name ILIKE $1",
        format!("%{}%", search_term)  // パラメータとして渡す
    )
    .fetch_all(pool)
    .await
}
```

##### 14.4.2 動的カラム名はホワイトリストで検証

```rust
// 正しいアプローチ: ホワイトリスト検証
#[derive(Debug, Clone, Copy)]
enum UserSortColumn {
    Name,
    Email,
    CreatedAt,
}

impl UserSortColumn {
    fn as_sql(&self) -> &'static str {
        match self {
            UserSortColumn::Name => "name",
            UserSortColumn::Email => "email",
            UserSortColumn::CreatedAt => "created_at",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "name" => Some(Self::Name),
            "email" => Some(Self::Email),
            "created_at" => Some(Self::CreatedAt),
            _ => None,
        }
    }
}

async fn get_users_sorted_safe(
    pool: &PgPool,
    sort_column: UserSortColumn,
    ascending: bool,
) -> Result<Vec<User>, sqlx::Error> {
    let order = if ascending { "ASC" } else { "DESC" };

    // 動的なSQL構築だが、値はホワイトリスト検証済み
    let query = format!(
        "SELECT * FROM users ORDER BY {} {}",
        sort_column.as_sql(),
        order
    );

    sqlx::query_as::<_, User>(&query)
        .fetch_all(pool)
        .await
}
```

##### 14.4.3 動的なWHERE句の構築

```rust
// 安全な動的WHERE句の構築
struct UserFilter {
    name: Option<String>,
    email: Option<String>,
    is_active: Option<bool>,
}

async fn find_users(pool: &PgPool, filter: UserFilter) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT * FROM users
        WHERE ($1::text IS NULL OR name ILIKE $1)
          AND ($2::text IS NULL OR email ILIKE $2)
          AND ($3::bool IS NULL OR is_active = $3)
        "#,
        filter.name.map(|n| format!("%{}%", n)),
        filter.email.map(|e| format!("%{}%", e)),
        filter.is_active
    )
    .fetch_all(pool)
    .await
}
```

---

### 22章 接続プール管理の失敗

#### 15.1 目的：データベース接続を効率的に管理する

Webサービスでは多数の同時リクエストを処理するため、データベース接続を効率的に管理したい。

#### 15.2 アンチパターン：接続プールを適切に設定しない

##### 15.2.1 リクエストごとに新しい接続を作成

```rust
// アンチパターン: 毎回接続を作成
async fn get_user_bad(user_id: Uuid) -> Result<User, anyhow::Error> {
    // リクエストごとに新しいプールを作成（非常に遅い）
    let pool = PgPoolOptions::new()
        .connect("postgres://...")
        .await?;

    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(&pool)
        .await?;

    Ok(user)
}
```

##### 15.2.2 接続数の設定が不適切

```rust
// アンチパターン: 接続数が少なすぎる/多すぎる
let pool = PgPoolOptions::new()
    .max_connections(1)  // 少なすぎる：同時リクエストがブロックされる
    .connect("postgres://...")
    .await?;

// または
let pool = PgPoolOptions::new()
    .max_connections(1000)  // 多すぎる：DBサーバーに負荷
    .connect("postgres://...")
    .await?;
```

##### 15.2.3 接続リークの発生

```rust
// アンチパターン: 接続をリーク
async fn process_data_bad(pool: &PgPool) -> Result<(), anyhow::Error> {
    let mut tx = pool.begin().await?;

    // 処理...
    if some_condition {
        return Err(anyhow::anyhow!("Error occurred"));
        // トランザクションがロールバックされない！
    }

    tx.commit().await?;
    Ok(())
}
```

#### 15.3 解決策：適切な接続プール設定

##### 15.3.1 アプリケーション起動時にプールを作成

```rust
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        // 最大接続数（CPUコア数の2-4倍が目安）
        .max_connections(32)
        // 最小接続数（常時維持）
        .min_connections(5)
        // 接続取得のタイムアウト
        .acquire_timeout(Duration::from_secs(5))
        // アイドル接続のタイムアウト
        .idle_timeout(Duration::from_secs(600))
        // 接続の最大寿命
        .max_lifetime(Duration::from_secs(1800))
        // 接続テスト
        .test_before_acquire(true)
        .connect(database_url)
        .await
}

// Axumでの使用例
#[tokio::main]
async fn main() {
    let pool = create_pool(&std::env::var("DATABASE_URL").unwrap())
        .await
        .expect("Failed to create pool");

    let app = Router::new()
        .route("/users/:id", get(get_user))
        .with_state(pool);

    // ...
}
```

##### 15.3.2 トランザクションの適切な管理

```rust
// 正しいアプローチ: スコープガードでトランザクション管理
async fn process_data_safe(pool: &PgPool) -> Result<(), anyhow::Error> {
    let mut tx = pool.begin().await?;

    // 処理...
    do_something(&mut tx).await?;  // ?でエラー時は自動ロールバック

    tx.commit().await?;
    Ok(())
}

// より安全なパターン: 関数でラップ
async fn with_transaction<F, T, E>(pool: &PgPool, f: F) -> Result<T, E>
where
    F: for<'c> FnOnce(&'c mut Transaction<'_, Postgres>) -> BoxFuture<'c, Result<T, E>>,
    E: From<sqlx::Error>,
{
    let mut tx = pool.begin().await?;
    let result = f(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}
```

---

### 23章 トランザクション境界の誤り

#### 16.1 目的：データの一貫性を保証する

複数の操作をアトミックに実行し、途中で失敗した場合は全ての変更を取り消したい。

#### 16.2 アンチパターン：トランザクションの範囲が不適切

##### 16.2.1 トランザクションなしで複数の操作

```rust
// アンチパターン: トランザクションなし
async fn transfer_points_bad(
    pool: &PgPool,
    from_user: Uuid,
    to_user: Uuid,
    points: i32,
) -> Result<(), sqlx::Error> {
    // 1. 送信元からポイントを減らす
    sqlx::query!(
        "UPDATE users SET points = points - $1 WHERE id = $2",
        points,
        from_user
    )
    .execute(pool)
    .await?;

    // ここでエラーが発生すると...
    // ポイントが消失する！

    // 2. 送信先にポイントを追加
    sqlx::query!(
        "UPDATE users SET points = points + $1 WHERE id = $2",
        points,
        to_user
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

##### 16.2.2 トランザクションが長すぎる

```rust
// アンチパターン: 長いトランザクション
async fn process_order_bad(pool: &PgPool, order_id: Uuid) -> Result<(), anyhow::Error> {
    let mut tx = pool.begin().await?;

    // 注文を処理
    let order = get_order(&mut tx, order_id).await?;

    // 外部APIを呼び出し（非常に遅い可能性）
    let payment_result = external_payment_api::process(&order).await?;

    // まだトランザクション内...
    update_order_status(&mut tx, order_id, "paid").await?;

    tx.commit().await?;
    Ok(())
}
// 問題: 外部API呼び出し中にDB接続がロックされる
```

#### 16.3 解決策：適切なトランザクション境界

##### 16.3.1 必要な操作をトランザクションでラップ

```rust
// 正しいアプローチ: 関連する操作をトランザクションで囲む
async fn transfer_points_safe(
    pool: &PgPool,
    from_user: Uuid,
    to_user: Uuid,
    points: i32,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 楽観的ロックまたは悲観的ロック
    let from_points: i32 = sqlx::query_scalar!(
        "SELECT points FROM users WHERE id = $1 FOR UPDATE",
        from_user
    )
    .fetch_one(&mut *tx)
    .await?;

    if from_points < points {
        return Err(sqlx::Error::Protocol("Insufficient points".into()));
    }

    sqlx::query!(
        "UPDATE users SET points = points - $1 WHERE id = $2",
        points,
        from_user
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE users SET points = points + $1 WHERE id = $2",
        points,
        to_user
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

##### 16.3.2 外部呼び出しはトランザクション外で

```rust
// 正しいアプローチ: 外部APIはトランザクション外
async fn process_order_safe(pool: &PgPool, order_id: Uuid) -> Result<(), anyhow::Error> {
    // 1. 注文を取得（トランザクション外）
    let order = get_order(pool, order_id).await?;

    // 2. 外部APIを呼び出し（トランザクション外）
    let payment_result = external_payment_api::process(&order).await?;

    // 3. ステータス更新のみトランザクション
    let mut tx = pool.begin().await?;

    update_order_status(&mut tx, order_id, "processing").await?;
    save_payment_result(&mut tx, order_id, &payment_result).await?;

    tx.commit().await?;
    Ok(())
}
```

---

### 24章 エラーハンドリングの軽視

#### 17.1 目的：データベースエラーを適切に処理する

データベース操作で発生するエラーを適切にハンドリングし、ユーザーに有用なフィードバックを提供したい。

#### 17.2 アンチパターン：エラーを適切に処理しない

##### 17.2.1 エラーを無視または握りつぶす

```rust
// アンチパターン: unwrap()の乱用
async fn get_user_bad(pool: &PgPool, user_id: Uuid) -> User {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
        .unwrap()  // パニック！
}

// アンチパターン: エラーを無視
async fn delete_user_bad(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;  // エラーを無視
}
```

##### 17.2.2 一般的なエラーメッセージ

```rust
// アンチパターン: 情報のないエラー
async fn create_user_bad(pool: &PgPool, email: &str) -> Result<(), String> {
    sqlx::query!("INSERT INTO users (email) VALUES ($1)", email)
        .execute(pool)
        .await
        .map_err(|_| "Database error".to_string())?;
    Ok(())
}
// ユニーク制約違反なのか、接続エラーなのかわからない
```

#### 17.3 解決策：構造化されたエラーハンドリング

##### 17.3.1 カスタムエラー型を定義

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    #[error("Email already exists: {0}")]
    EmailAlreadyExists(String),

    #[error("Database connection error")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => {
                // 特定のエラーに変換
                AppError::ConnectionError(err)
            }
            sqlx::Error::Database(db_err) => {
                // PostgreSQLのエラーコードをチェック
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => {
                            // unique_violation
                            AppError::EmailAlreadyExists("email".to_string())
                        }
                        _ => AppError::ConnectionError(err),
                    }
                } else {
                    AppError::ConnectionError(err)
                }
            }
            _ => AppError::ConnectionError(err),
        }
    }
}
```

##### 17.3.2 HTTPレスポンスへの変換

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::UserNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::EmailAlreadyExists(_) => (StatusCode::CONFLICT, self.to_string()),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::ConnectionError(_) => {
                // 内部エラーの詳細は隠す
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        let body = Json(serde_json::json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
```

##### 17.3.3 fetch_optionalの活用

```rust
// 正しいアプローチ: 存在しない場合を適切に処理
async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::UserNotFound(user_id))
}

// 正しいアプローチ: 制約違反をチェック
async fn create_user(pool: &PgPool, email: &str, name: &str) -> Result<User, AppError> {
    sqlx::query_as!(
        User,
        "INSERT INTO users (email, name) VALUES ($1, $2) RETURNING *",
        email,
        name
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::EmailAlreadyExists(email.to_string())
        }
        _ => AppError::from(e),
    })
}
```

---

### 25章 マイグレーション管理の失敗

#### 18.1 目的：スキーマの変更を安全に管理する

開発環境と本番環境のスキーマを一致させ、変更履歴を追跡したい。

#### 18.2 アンチパターン：マイグレーションを適切に管理しない

##### 18.2.1 手動でスキーマを変更

```sql
-- アンチパターン: 本番DBに直接ALTER TABLE
ALTER TABLE users ADD COLUMN phone VARCHAR(20);
-- 変更履歴がない、チームメンバーは知らない
```

##### 18.2.2 破壊的なマイグレーション

```sql
-- アンチパターン: カラム削除のマイグレーション
ALTER TABLE users DROP COLUMN legacy_field;
-- ロールバックできない
```

##### 18.2.3 ダウンタイムを伴う変更

```sql
-- アンチパターン: 大きなテーブルにNOT NULL制約を追加
ALTER TABLE orders ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'pending';
-- 数百万行のテーブルではロックが長時間続く
```

#### 18.3 解決策：安全なマイグレーション戦略

##### 18.3.1 sqlxのマイグレーション機能を使用

```bash
# マイグレーションファイルの作成
sqlx migrate add create_users_table

# マイグレーションの実行
sqlx migrate run

# マイグレーションの状態確認
sqlx migrate info
```

```sql
-- migrations/20231215_create_users_table.sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

##### 18.3.2 ゼロダウンタイムマイグレーション

```sql
-- 1. NULLを許可する新しいカラムを追加
ALTER TABLE orders ADD COLUMN status VARCHAR(20);

-- 2. バックフィル（バッチ処理）
UPDATE orders SET status = 'pending' WHERE status IS NULL AND id < 1000;
-- 小さなバッチで実行

-- 3. デフォルト値を設定
ALTER TABLE orders ALTER COLUMN status SET DEFAULT 'pending';

-- 4. NOT NULL制約を追加
ALTER TABLE orders ALTER COLUMN status SET NOT NULL;
```

##### 18.3.3 アプリケーションコードとの整合性

```rust
// マイグレーション中の互換性を保つ
#[derive(Debug, sqlx::FromRow)]
struct Order {
    id: Uuid,
    // 新しいカラム（移行期間中はOption）
    status: Option<String>,
}

impl Order {
    fn status(&self) -> &str {
        self.status.as_deref().unwrap_or("pending")
    }
}
```

---

### 26章 セキュリティの考慮不足

#### 19.1 目的：データを安全に管理する

機密データを適切に保護し、認可されたアクセスのみを許可したい。

#### 19.2 アンチパターン：セキュリティを後回しにする

##### 19.2.1 パスワードの平文保存

```rust
// アンチパターン: パスワードを平文で保存
async fn create_user_unsafe(pool: &PgPool, email: &str, password: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO users (email, password) VALUES ($1, $2)",
        email,
        password  // 危険！
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

##### 19.2.2 機密データの露出

```rust
// アンチパターン: 全フィールドを返す
async fn get_user_unsafe(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",  // password_hashも含まれる
        user_id
    )
    .fetch_one(pool)
    .await
}
```

#### 19.3 解決策：セキュリティのベストプラクティス

##### 19.3.1 パスワードのハッシュ化

```rust
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

async fn create_user_safe(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<Uuid, anyhow::Error> {
    let password_hash = hash_password(password)?;

    let id = sqlx::query_scalar!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        email,
        password_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}
```

##### 19.3.2 必要なフィールドのみを取得

```rust
// 公開用のユーザー情報
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct PublicUser {
    id: Uuid,
    name: String,
    // password_hashは含まない
}

async fn get_public_user(pool: &PgPool, user_id: Uuid) -> Result<PublicUser, sqlx::Error> {
    sqlx::query_as!(
        PublicUser,
        "SELECT id, name FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
}
```

##### 19.3.3 行レベルセキュリティ

```sql
-- PostgreSQLの行レベルセキュリティ
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;

CREATE POLICY posts_owner_policy ON posts
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::uuid);
```

```rust
async fn query_with_rls(pool: &PgPool, user_id: Uuid) -> Result<Vec<Post>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // セッション変数を設定
    sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_id))
        .execute(&mut *tx)
        .await?;

    let posts = sqlx::query_as!(Post, "SELECT * FROM posts")
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(posts)
}
```

---

### 27章 シュードキー・ニートフリーク（疑似キー潔癖症）

#### 22.1 目的：欠番を詰める

連番のIDに欠番があると気持ち悪いので、削除されたIDを再利用したい、または連番を詰め直したい。

#### 22.2 アンチパターン：隙間を埋める

**問題のあるアプローチ：**

```rust
// アンチパターン1: 最小の欠番を探して再利用
async fn find_next_available_id(pool: &PgPool) -> Result<i64, sqlx::Error> {
    // 欠番を探すクエリ
    let next_id: Option<i64> = sqlx::query_scalar!(
        r#"
        SELECT MIN(t1.id + 1) as "next_id"
        FROM posts t1
        LEFT JOIN posts t2 ON t1.id + 1 = t2.id
        WHERE t2.id IS NULL
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(next_id.unwrap_or(1))
}

async fn create_post_with_gap_filling(
    pool: &PgPool,
    title: &str,
    content: &str,
) -> Result<i64, sqlx::Error> {
    let id = find_next_available_id(pool).await?;

    sqlx::query!(
        "INSERT INTO posts (id, title, content) VALUES ($1, $2, $3)",
        id,
        title,
        content
    )
    .execute(pool)
    .await?;

    Ok(id)
}
```

##### 22.2.1 競合状態の問題

```rust
// アンチパターン: 同時実行で同じIDが割り当てられる
// スレッドA: find_next_available_id() → 42
// スレッドB: find_next_available_id() → 42  // 同じ！
// スレッドA: INSERT (42, ...) → 成功
// スレッドB: INSERT (42, ...) → 主キー重複エラー！
```

##### 22.2.2 既存のIDを振り直す

```rust
// アンチパターン2: 全てのIDを振り直して隙間をなくす
async fn renumber_all_posts(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 全レコードを取得してID順に振り直し
    sqlx::query!(
        r#"
        WITH numbered AS (
            SELECT id, ROW_NUMBER() OVER (ORDER BY id) as new_id
            FROM posts
        )
        UPDATE posts p
        SET id = n.new_id
        FROM numbered n
        WHERE p.id = n.id
        "#
    )
    .execute(&mut *tx)
    .await?;

    // シーケンスもリセット
    sqlx::query!("SELECT setval('posts_id_seq', (SELECT MAX(id) FROM posts))")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
// 問題:
// 1. 外部キー制約でエラーになる可能性
// 2. 外部システムがIDで参照していると不整合
// 3. URLがIDを含んでいると全てのリンクが壊れる
// 4. ログやキャッシュのIDが無効になる
```

##### 22.2.3 データ不整合の発生

```rust
// アンチパターン: 削除されたIDを再利用すると過去のデータと混同される

// シナリオ:
// 1. ユーザーが投稿#42を作成
// 2. 投稿#42が削除される
// 3. 外部サービスは投稿#42への参照を保持
// 4. 新しい投稿が#42として作成される
// 5. 外部サービスは新しい投稿を古い投稿と誤認

// ログに残っているID
// [2024-01-01] User 123 viewed post 42  // 古い投稿
// [2024-06-01] User 456 liked post 42   // 新しい投稿（別の内容）
```

#### 22.3 アンチパターンの見つけ方

- 「IDに欠番があるのを直したい」という要望
- 欠番を探すクエリがコードにある
- IDの最大値をチェックして制限する処理
- `setval()` や `ALTER SEQUENCE RESTART` が頻繁に使われる

#### 22.4 アンチパターンを用いてもよい場合

1. **完全な開発環境リセット時**: 開発DBを初期化する際
2. **データ移行時**: 新システムへの一括移行
3. **テストデータ**: 自動テストで毎回クリーンな状態から開始

```rust
// 許容される例: テストフィクスチャのリセット
#[cfg(test)]
async fn reset_test_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("TRUNCATE posts RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}
```

#### 22.5 解決策：疑似キーの欠番は埋めない

##### 22.5.1 UUIDを使用する（推奨）

```sql
-- 解決策1: UUIDを主キーにする
CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(200) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
// UUIDなら欠番の概念がない
async fn create_post(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    content: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();  // 衝突の心配なし

    sqlx::query!(
        r#"
        INSERT INTO posts (id, user_id, title, content)
        VALUES ($1, $2, $3, $4)
        "#,
        id,
        user_id,
        title,
        content
    )
    .execute(pool)
    .await?;

    Ok(id)
}

// UUIDの利点:
// - 分散システムでも衝突しない
// - IDからの情報漏洩がない（作成順序がわからない）
// - マージや移行が容易
```

##### 22.5.2 連番を維持したい場合はそのまま放置

```rust
// 連番でも欠番を気にしない
// PostgreSQLのSERIALは自動的に次の番号を割り当てる

async fn create_post_with_serial(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    content: &str,
) -> Result<i64, sqlx::Error> {
    let post_id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO posts (user_id, title, content)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        user_id,
        title,
        content
    )
    .fetch_one(pool)
    .await?;

    Ok(post_id)
    // id: 1, 2, 3, 5, 6, 10, 11, ...  // 欠番があっても問題なし
}
```

##### 22.5.3 表示用の連番が必要な場合

```rust
// 表示用の番号はクエリで動的に生成
async fn list_posts_with_row_number(pool: &PgPool) -> Result<Vec<PostWithNumber>, sqlx::Error> {
    sqlx::query_as!(
        PostWithNumber,
        r#"
        SELECT
            id,
            ROW_NUMBER() OVER (ORDER BY created_at) as "row_num!",
            title,
            content,
            created_at
        FROM posts
        ORDER BY created_at
        "#
    )
    .fetch_all(pool)
    .await
}

// ページネーション用
async fn list_posts_paginated(
    pool: &PgPool,
    page: i64,
    per_page: i64,
) -> Result<Vec<PostWithNumber>, sqlx::Error> {
    sqlx::query_as!(
        PostWithNumber,
        r#"
        SELECT
            id,
            ROW_NUMBER() OVER (ORDER BY created_at) as "row_num!",
            title,
            content,
            created_at
        FROM posts
        ORDER BY created_at
        OFFSET $1
        LIMIT $2
        "#,
        (page - 1) * per_page,
        per_page
    )
    .fetch_all(pool)
    .await
}
```

##### 22.5.4 番号を表示したい場合は別カラムを使用

```sql
-- 表示用の番号を別カラムで管理
CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number SERIAL NOT NULL UNIQUE,  -- 表示用の連番
    user_id UUID NOT NULL REFERENCES users(id),
    amount DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- invoice_number: 1, 2, 3, ...（削除されても振り直さない）
-- id: UUID（内部での参照に使用）
```

```rust
struct Invoice {
    id: Uuid,              // 内部キー
    invoice_number: i32,    // 表示用番号
    user_id: Uuid,
    amount: rust_decimal::Decimal,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn get_invoice_by_number(
    pool: &PgPool,
    invoice_number: i32,
) -> Result<Option<Invoice>, sqlx::Error> {
    sqlx::query_as!(
        Invoice,
        "SELECT * FROM invoices WHERE invoice_number = $1",
        invoice_number
    )
    .fetch_optional(pool)
    .await
}
```

##### 22.5.5 BIGINT の範囲を信頼する

```rust
// BIGINT (i64) の最大値: 9,223,372,036,854,775,807
// 毎秒1000件のINSERTでも 292,471,208年 持つ

// IDの枯渇を心配する必要はない
// 1日100万件でも約25,000年

async fn check_id_usage(pool: &PgPool) -> Result<(), sqlx::Error> {
    let (current, max_possible): (i64, i64) = sqlx::query_as(
        "SELECT MAX(id), 9223372036854775807 FROM posts"
    )
    .fetch_one(pool)
    .await?;

    let usage_percent = (current as f64 / max_possible as f64) * 100.0;
    println!("ID usage: {:.10}%", usage_percent);
    // 現実的に枯渇することはない

    Ok(())
}
```

#### 22.6 アプローチの比較

| アプローチ | 欠番 | 競合 | 分散 | 予測可能性 |
|-----------|------|------|------|-----------|
| 欠番を埋める | × | × | × | ◎ |
| SERIALそのまま | ◎ | ◎ | × | ○ |
| UUID | ◎ | ◎ | ◎ | × |
| 表示用番号分離 | ◎ | ◎ | ◎ | ◎ |

**推奨**:
- **新規プロジェクト**: UUIDを主キーに使用
- **既存プロジェクト**: 連番の欠番は放置、必要なら表示用番号を別に持つ

---

## 付録A：チェックリスト

### データベース設計チェックリスト

- [ ] 外部キー制約を設定しているか
- [ ] 適切なインデックスを作成しているか
- [ ] NOT NULL制約を必要な列に設定しているか
- [ ] カンマ区切り値を使用していないか
- [ ] ENUM型の代わりに参照テーブルを使用しているか
- [ ] 金額にDECIMAL型を使用しているか

### クエリチェックリスト

- [ ] N+1クエリが発生していないか
- [ ] プレースホルダを使用しているか
- [ ] WHERE句でインデックスが効くか
- [ ] NULLの比較に IS NULL を使用しているか
- [ ] トランザクションの範囲は適切か

### アプリケーションチェックリスト

- [ ] 接続プールを適切に設定しているか
- [ ] エラーを適切にハンドリングしているか
- [ ] パスワードをハッシュ化しているか
- [ ] マイグレーションを使用しているか
- [ ] 機密データを露出していないか

---

## 付録B：参考資料

### 書籍

- Bill Karwin『SQLアンチパターン』
- Craig Kerstiens『High Performance PostgreSQL』

### ドキュメント

- [sqlx公式ドキュメント](https://docs.rs/sqlx)
- [PostgreSQL公式ドキュメント](https://www.postgresql.org/docs/)

### クレート

- `sqlx`: 非同期SQLツールキット
- `rust_decimal`: 正確な10進数計算
- `argon2`: パスワードハッシュ
- `thiserror`: エラー型定義

---

*本ドキュメントは継続的に更新されます。*
