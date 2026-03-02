# Leptos 0.8 で Twitter クローンを作って学んだこと【中編】バックエンドの設計パターン

[https://leptos.dev/:embed:cite]

[https://github.com/leptos-rs/leptos:embed:cite]

> 本記事は3部構成です。
> - [前編](blog_01.md): Leptos の基本機能 — プロジェクト構成、コンポーネント、Signal、サーバー関数、ルーティング
> - **中編（本記事）**: バックエンドの設計パターン — DB 設計、認証、検索、通知
> - [後編](blog_03.md): Rust の言語哲学とフロントエンドの交差点 — 所有権、クロージャ、型システムの衝突と折り合い

## 目次

1. [DB 設計 — 非正規化とカウンターキャッシュ](#1-db-設計--非正規化とカウンターキャッシュ)
2. [カーソルベースページネーションと UUID v7](#2-カーソルベースページネーションと-uuid-v7)
3. [認証 — Argon2 とセッション管理](#3-認証--argon2-とセッション管理)
4. [条件付きコンパイルの実践パターン](#4-条件付きコンパイルの実践パターン)
5. [検索 — pg_trgm によるトライグラム検索](#5-検索--pg_trgm-によるトライグラム検索)
6. [通知システム — PostgreSQL ENUM とイベント駆動](#6-通知システム--postgresql-enum-とイベント駆動)

---

## 1. DB 設計 — 非正規化とカウンターキャッシュ

SNS のタイムラインでは、各投稿に「いいね数」「リプライ数」「リチャープ数」を表示する。素朴に実装すると投稿ごとに `COUNT(*)` を実行することになり、N+1 問題の温床になる。最初の実装ではまさにこれをやっていて、20件のタイムライン表示に60本のCOUNTクエリが走っていた。

Chirp ではこの問題を **カウンターキャッシュ** で解決した。`posts` テーブルに `like_count`, `rechirp_count`, `reply_count` カラムを持たせ、PostgreSQL のトリガーで自動更新する。

```sql
-- likes テーブルへの INSERT/DELETE をフックして posts.like_count を更新
CREATE OR REPLACE FUNCTION update_like_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE posts SET like_count = like_count + 1 WHERE id = NEW.post_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE posts SET like_count = like_count - 1 WHERE id = OLD.post_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_like_count
    AFTER INSERT OR DELETE ON likes
    FOR EACH ROW EXECUTE FUNCTION update_like_count();
```

同様のトリガーを `rechirps`, `follows`, `posts`（リプライ用）にも設定している。合計 6 つのトリガーが、アプリケーションコードを一切変更せずにカウントの整合性を保証する。

```sql
-- フォロー数は双方向の更新が必要
CREATE OR REPLACE FUNCTION update_follow_counts() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE users SET following_count = following_count + 1 WHERE id = NEW.follower_id;
        UPDATE users SET followers_count = followers_count + 1 WHERE id = NEW.following_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE users SET following_count = following_count - 1 WHERE id = OLD.follower_id;
        UPDATE users SET followers_count = followers_count - 1 WHERE id = OLD.following_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
```

### トリガーを選んだ理由

カウンター更新をアプリケーション側で行うこともできるが、トリガーにはいくつかの利点がある:

- **原子性**: INSERT と UPDATE が同一トランザクション内で実行される。アプリケーションがクラッシュしてもカウントがずれない
- **DRY**: `toggle_like` の Rust コードは「いいねの INSERT/DELETE」だけに集中でき、カウント更新のことを知らなくてよい
- **一貫性**: psql から直接データを操作しても、カウントが正しく更新される

トレードオフとして、トリガーは暗黙的なロジックなので、コードレビューで見落としやすい。マイグレーションファイルをきちんと管理し、トリガーの存在をドキュメント化することが重要だ。

### ソフトデリートと部分インデックス

投稿の削除は物理削除ではなく **ソフトデリート** を採用した:

```sql
-- 物理削除だと外部キー制約（リプライ、いいね等）が壊れる
-- ソフトデリートなら関連データを保持したまま「削除済み」にできる
UPDATE posts SET is_deleted = TRUE WHERE id = $1 AND user_id = $2
```

すべてのタイムラインクエリで `WHERE is_deleted = FALSE` フィルタが必要になるが、**部分インデックス** でパフォーマンスへの影響を最小化している:

```sql
-- 削除済みの投稿はインデックスに含まれない → インデックスサイズが小さくなる
CREATE INDEX idx_posts_active ON posts (created_at DESC) WHERE is_deleted = FALSE;
CREATE INDEX idx_posts_user ON posts (user_id, created_at DESC) WHERE is_deleted = FALSE;
CREATE INDEX idx_posts_reply_to ON posts (reply_to_id, created_at DESC) WHERE is_deleted = FALSE;
```

部分インデックスは通常のインデックスと比べてサイズが小さく、INSERT/UPDATE 時のオーバーヘッドも少ない。削除済み投稿が増えても、アクティブな投稿のクエリ性能は一定に保たれる。

---

## 2. カーソルベースページネーションと UUID v7

タイムラインのページネーションには **カーソルベース（keyset pagination）** を採用した。OFFSET ベースのページネーションと比較して、以下のメリットがある:

- **一定のパフォーマンス**: OFFSET は大きくなるほど遅くなるが、カーソルは常に INDEX SCAN で O(log N)
- **データ整合性**: ページ間でスクロール中に新しい投稿が追加されても、同じ投稿が2回表示されたりスキップされたりしない
- **ステートレス**: サーバーに「今何ページ目か」を保持する必要がない

```rust
#[server]
pub async fn get_home_timeline(
    cursor: Option<String>,   // 前回の最後の投稿 ID（初回は None）
    limit: Option<i64>,
) -> Result<Vec<PostWithMeta>, ServerFnError> {
    let limit = limit.unwrap_or(20).min(50);  // 上限を制限
    let cursor_id: Option<Uuid> = cursor
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ServerFnError::new("Invalid cursor"))?;
    // ...
```

SQL 側では `$2::UUID IS NULL` で初回アクセス（カーソルなし）と2ページ目以降を同一クエリで処理する:

```sql
SELECT p.id, p.content, ...
FROM posts p
JOIN users u ON p.user_id = u.id
WHERE p.is_deleted = FALSE
  AND p.reply_to_id IS NULL
  AND (p.user_id = $1 OR p.user_id IN (
      SELECT following_id FROM follows WHERE follower_id = $1
  ))
  AND ($2::UUID IS NULL OR p.id < $2)  -- カーソル条件
ORDER BY p.created_at DESC
LIMIT $3
```

### なぜ UUID v7 か

カーソルベースページネーションには **ソート可能な ID** が必要だ。UUID v4 はランダムなので `ORDER BY id` に意味がない。UUID v7 はタイムスタンプベースで生成されるため、`id < $cursor` が「このカーソルより前に作成された投稿」を正確に表現する。

```rust
let id = Uuid::now_v7();  // タイムスタンプ + ランダムで一意かつソート可能
```

UUID v7 を使うことで、`created_at` カラムとは別に **ID 自体でソート順序を表現** できる。これにより、同一タイムスタンプの投稿があっても ID の大小で確実に順序が決まる。

### EXISTS サブクエリによるユーザー状態の取得

タイムラインの各投稿に「自分がいいね済みか」「リチャープ済みか」を表示する必要がある。これを JOIN で取得すると行が増殖するリスクがあるため、`EXISTS` サブクエリを使う:

```sql
EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $1) as liked_by_me,
EXISTS(SELECT 1 FROM rechirps WHERE post_id = p.id AND user_id = $1) as rechirped_by_me
```

`EXISTS` は該当行が 1 つでも見つかれば即座に `TRUE` を返して走査を終了する。`COUNT(*)` と違い、全行をスキャンする必要がないため高速だ。未ログインユーザーの場合は `$1` が `NULL` になるため、`COALESCE` で `FALSE` にフォールバックさせる:

```sql
COALESCE(EXISTS(SELECT 1 FROM likes WHERE post_id = p.id AND user_id = $1), FALSE) as liked_by_me
```

---

## 3. 認証 — Argon2 とセッション管理

### パスワードハッシュ

パスワードの保存には **Argon2id** を使用している。Argon2 は 2015 年の Password Hashing Competition の勝者で、bcrypt や scrypt より新しく、GPU 耐性とメモリハードネスを兼ね備えている。

```rust
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;

// サインアップ時: ソルト生成 → ハッシュ化 → DB に保存
let salt = SaltString::generate(&mut OsRng);
let argon2 = Argon2::default();  // Argon2id, m=19456, t=2, p=1
let password_hash = argon2
    .hash_password(password.as_bytes(), &salt)
    .map_err(|e| ServerFnError::new(format!("Password hashing error: {e}")))?
    .to_string();
```

`Argon2::default()` は Argon2id バリアント（サイドチャネル攻撃耐性 + GPU 耐性のハイブリッド）をデフォルトパラメータで使用する。パラメータは PHC 文字列フォーマットでハッシュ値に埋め込まれるため、将来パラメータを変更しても既存のハッシュの検証に影響しない。

```rust
// ログイン時: 保存されたハッシュを解析 → 検証
use argon2::{Argon2, PasswordHash, PasswordVerifier};

let parsed_hash = PasswordHash::new(&user.password_hash)?;
Argon2::default()
    .verify_password(password.as_bytes(), &parsed_hash)
    .map_err(|_| ServerFnError::new("Invalid username or password"))?;
```

認証エラーのメッセージは「ユーザー名が間違っている」「パスワードが間違っている」を区別しない。これはユーザー列挙攻撃を防ぐためのセキュリティプラクティスだ。

### セッションの流れ

```
1. POST /api/login (ActionForm 経由)
   ↓
2. tower-sessions の SessionManagerLayer がリクエストを受信
   ↓
3. サーバー関数内で leptos_axum::extract::<Session>() でセッション取得
   ↓
4. session.insert("user_id", uuid) でユーザー ID を保存
   ↓
5. レスポンスに Set-Cookie ヘッダーが自動付与（HMAC 署名付き）
   ↓
6. 以降のリクエスト: Cookie → Session → session.get("user_id")
```

セッションストレージには PostgreSQL を使用している（`tower-sessions-sqlx-store`）。Redis のような外部ストアを追加する必要がなく、既存の DB インフラだけで完結する。

```rust
// main.rs: セッションレイヤーの設定
let session_store = PostgresStore::new(pool.clone());
let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(false)           // 開発環境では HTTPS 不要
    .with_same_site(SameSite::Lax);  // CSRF 保護の基本
```

`SameSite::Lax` はクロスサイトの POST リクエストでセッション Cookie を送信しないため、基本的な CSRF 保護として機能する。`ActionForm` が通常の HTML フォーム送信を使うため、この設定と相性が良い。

### サーバー関数からのリダイレクト

認証後のリダイレクトは `leptos_axum::redirect` で行う:

```rust
// ログイン成功後にホームへ遷移
leptos_axum::redirect("/");
Ok(())
```

この関数は SSR 時にはレスポンスヘッダーに `Location` を設定し、クライアントサイドでは `window.location` を変更する。`ActionForm` のプログレッシブエンハンスメントと組み合わさり、JS の有無に関わらず正しくリダイレクトが動作する。

---

## 4. 条件付きコンパイルの実践パターン

Leptos アプリは SSR（サーバー）と Hydrate（WASM）の 2 つのターゲットでコンパイルされる。サーバー専用のコード（DB アクセス、認証処理）がクライアントにバンドルされないよう、`#[cfg(feature = "ssr")]` を多用する。

### パターン 1: 条件付き derive

モデル型はサーバーとクライアントの両方で使うが、`sqlx::FromRow` はサーバーでのみ必要だ:

```rust
#[cfg(feature = "ssr")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]  // SSR ビルドでのみ FromRow を導出
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]                  // クライアントに送信しない
    #[cfg_attr(feature = "ssr", sqlx(default))] // SSR でのみ sqlx 属性を適用
    pub password_hash: String,
    // ...
}
```

`#[cfg_attr(condition, attr)]` は条件が真のときだけ属性を適用する。これにより、WASM ビルドでは `sqlx` クレートの依存を完全に排除できる。

### パターン 2: SSR 専用のヘルパー関数

DB プールの取得やセッションの抽出は SSR でのみ使うため、関数全体を `#[cfg(feature = "ssr")]` でゲートする:

```rust
#[cfg(feature = "ssr")]
pub(crate) async fn get_current_user_id() -> Option<uuid::Uuid> {
    let session = super::auth::extract_session().await.ok()?;
    session.get("user_id").await.ok()?
}
```

### パターン 3: SSR 専用の行変換型

SQLx の `query_as` はフラットな行構造を期待するが、API レスポンスではネストされた構造体が欲しい。この変換を SSR 専用の型と `From` 実装で行う:

```rust
// サーバー専用の「フラット」な行型
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
pub(crate) struct PostWithMetaRow {
    id: uuid::Uuid,
    content: String,
    // JOINで取得したフラットなカラム
    author_id: uuid::Uuid,
    author_username: String,
    author_display_name: String,
    author_avatar_url: Option<String>,
    liked_by_me: bool,
    rechirped_by_me: bool,
    // ...
}

// フラット → ネスト構造への変換（SSR でのみコンパイル）
#[cfg(feature = "ssr")]
impl From<PostWithMetaRow> for PostWithMeta {
    fn from(row: PostWithMetaRow) -> Self {
        Self {
            id: row.id,
            content: row.content,
            author: UserSummary {
                id: row.author_id,
                username: row.author_username,
                display_name: row.author_display_name,
                avatar_url: row.author_avatar_url,
            },
            liked_by_me: row.liked_by_me,
            // ...
        }
    }
}
```

なぜこの「フラット → ネスト」変換が必要なのか？ SQL の JOIN はフラットな列を返すが、クライアントに送る `PostWithMeta` は `author: UserSummary` というネスト構造を持つ。ORM なら自動でこの変換をしてくれるが、生 SQL + SQLx ではこのブリッジが必要になる。

### パターン 4: サーバー関数内の限定的 use

`#[server]` 関数の中では、サーバー専用の `use` 文を関数本体に書ける:

```rust
#[server]
pub async fn signup(username: String, password: String) -> Result<(), ServerFnError> {
    // この use 文はサーバーでのみ有効
    // #[server] マクロが自動的に cfg ゲートを付ける
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;
    use uuid::Uuid;
    // ...
}
```

`#[server]` マクロは関数本体をサーバー専用にコンパイルし、クライアント側には RPC スタブだけを生成する。そのため関数内の `use` 文はサーバー依存を参照しても問題ない。

### モデルバリアント — 同じエンティティの異なるビュー

`User` エンティティには 3 つのバリアントがある:

| 型 | 用途 | フィールド |
|---|---|---|
| `User` | DB 行そのもの（サーバー内部） | 全フィールド（password_hash 含む） |
| `UserSummary` | 投稿カードへの埋め込み | id, username, display_name, avatar_url |
| `UserProfile` | プロフィールページ | 統計 + is_following, is_followed_by |

`User` は `#[serde(skip_serializing)]` で `password_hash` をクライアントに送信しないが、そもそも `UserSummary` や `UserProfile` にはパスワード関連のフィールドが存在しない。**型レベルで情報漏洩を防ぐ** 設計だ。

---

## 5. 検索 — pg_trgm によるトライグラム検索

PostgreSQL の全文検索（`tsvector` + `tsquery`）は形態素解析ベースで、日本語には追加辞書が必要だ。Chirp ではよりシンプルな **トライグラム検索** を採用した。

```sql
-- pg_trgm 拡張を有効化
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- GIN インデックスでトライグラム検索を高速化
CREATE INDEX idx_posts_content_trgm ON posts
    USING gin (content gin_trgm_ops) WHERE is_deleted = FALSE;
CREATE INDEX idx_users_username_trgm ON users
    USING gin (username gin_trgm_ops);
CREATE INDEX idx_users_display_name_trgm ON users
    USING gin (display_name gin_trgm_ops);
```

トライグラム（3 文字の部分文字列）は言語非依存なので、日本語でもそのまま動作する。例えば「プログラミング」は「プロ」「ログ」「グラ」「ラミ」「ミン」「ング」に分解され、`ILIKE '%プログラ%'` のような部分一致検索が GIN インデックスを使って高速に実行される。

```rust
#[server]
pub async fn search_posts(query: String, limit: Option<i64>) -> Result<Vec<PostWithMeta>, ServerFnError> {
    let pattern = format!("%{}%", query.trim());

    let rows = sqlx::query_as::<_, PostWithMetaRow>(
        r#"
        SELECT p.id, p.content, ...
        FROM posts p
        JOIN users u ON p.user_id = u.id
        WHERE p.is_deleted = FALSE
          AND p.content ILIKE $2       -- GIN トライグラムインデックスが効く
        ORDER BY p.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(current_user_id)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
```

ユーザー検索では `username` と `display_name` を `OR` で検索し、`followers_count DESC` でソートする。フォロワーの多いユーザーが上位に表示される:

```sql
SELECT id, username, display_name, avatar_url
FROM users
WHERE username ILIKE $1 OR display_name ILIKE $1
ORDER BY followers_count DESC
LIMIT $2
```

### pg_trgm の限界

トライグラム検索は 3 文字未満のクエリではインデックスが効きにくい。また、形態素解析ベースの全文検索と違い、同義語や活用形の処理はできない。しかし、SNS の投稿検索には十分な精度で、セットアップの簡単さとのトレードオフとして妥当だ。

---

## 6. 通知システム — PostgreSQL ENUM とイベント駆動

### ENUM 型の活用

通知のイベント種別を PostgreSQL の ENUM 型で定義する:

```sql
CREATE TYPE notification_event AS ENUM ('like', 'rechirp', 'follow', 'reply', 'mention');

CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,   -- 受信者
    actor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,  -- アクション実行者
    event_type notification_event NOT NULL,
    post_id UUID REFERENCES posts(id) ON DELETE CASCADE,            -- NULL 可（follow には不要）
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 未読通知に特化した部分インデックス
CREATE INDEX idx_notifications_unread ON notifications (user_id, is_read)
    WHERE is_read = FALSE;
```

ENUM 型を使う利点は、不正な値が DB に入ることを型レベルで防げる点だ。`CHECK` 制約や文字列カラム + バリデーションよりも厳格で、ストレージ効率も良い（内部的には 4 バイト整数）。

### Rust 側の ENUM マッピング

PostgreSQL ENUM と Rust enum の変換は、`Display` と `FromStr` トレイトの実装で行う:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationEvent {
    Like, Rechirp, Follow, Reply, Mention,
}

impl std::fmt::Display for NotificationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Like => write!(f, "like"),
            Self::Rechirp => write!(f, "rechirp"),
            Self::Follow => write!(f, "follow"),
            Self::Reply => write!(f, "reply"),
            Self::Mention => write!(f, "mention"),
        }
    }
}

impl std::str::FromStr for NotificationEvent {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "like" => Ok(Self::Like),
            "rechirp" => Ok(Self::Rechirp),
            "follow" => Ok(Self::Follow),
            "reply" => Ok(Self::Reply),
            "mention" => Ok(Self::Mention),
            other => Err(format!("Unknown notification event: {other}")),
        }
    }
}
```

SQL クエリでは `n.event_type::text` でキャストして文字列として取得し、Rust 側で `parse()` する:

```rust
// NotifRow の event_type は String
let event_type: NotificationEvent = row.event_type.parse().ok()?;
```

`sqlx::Type` derive を使えば自動変換もできるが、明示的な `FromStr` 実装の方がエラーハンドリングの制御がしやすく、`#[cfg(feature = "ssr")]` ゲートも不要になる。

### 通知の生成パターン

通知は社会的アクション（いいね、リチャープ、フォロー）の実行時に生成する。重要なのは **自分自身への通知を除外する** ことだ:

```rust
#[cfg(feature = "ssr")]
async fn create_notification(
    pool: &PgPool, actor_id: Uuid, post_id: Uuid, event_type: &str,
) -> Result<(), ServerFnError> {
    // 投稿の作者を取得
    let post_author: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await?;

    if let Some((author_id,)) = post_author {
        if author_id != actor_id {  // 自分自身には通知しない
            let notif_id = Uuid::now_v7();
            sqlx::query(
                "INSERT INTO notifications (id, user_id, actor_id, event_type, post_id) \
                 VALUES ($1, $2, $3, $4::notification_event, $5)",
            )
            .bind(notif_id)
            .bind(author_id)   // 受信者 = 投稿の作者
            .bind(actor_id)    // 実行者 = いいねした人
            .bind(event_type)
            .bind(post_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
```

フォロー通知だけは `post_id` が不要なため、別のパスで直接 INSERT している:

```rust
// follow_user 内
sqlx::query(
    "INSERT INTO notifications (id, user_id, actor_id, event_type) \
     VALUES ($1, $2, $3, 'follow'::notification_event)",
)
.bind(notif_id)
.bind(target_id)   // フォローされた人
.bind(user_id)     // フォローした人
.execute(&pool)
.await?;
```

---


次回の[後編](blog_03.md)では、Rust の言語哲学とフロントエンドの交差点 — 所有権モデルとリアクティブ UI、`move` クロージャの意味、TypeScript との対比を掘り下げる。

## 参考資料

- [Leptos 公式サイト](https://leptos.dev/)
- [leptos-rs/leptos - GitHub](https://github.com/leptos-rs/leptos)
- [SQLx - GitHub](https://github.com/launchbadge/sqlx)
- [tower-sessions - GitHub](https://github.com/maxcountryman/tower-sessions)
- [PostgreSQL: pg_trgm](https://www.postgresql.org/docs/current/pgtrgm.html)
- [Argon2 - Password Hashing Competition](https://www.password-hashing.net/)
- [UUID v7 (RFC 9562)](https://www.rfc-editor.org/rfc/rfc9562)
