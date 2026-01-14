# WHERE deleted_at IS NULLを100箇所に書いた話：論理削除の沼

## 発端

「削除したはずのユーザーがAPIレスポンスに含まれています」

バグ報告を受けて調査すると、新しく追加したエンドポイントで`WHERE deleted_at IS NULL`を付け忘れていた。修正してデプロイ。翌週、また同じ報告。別のエンドポイントだった。

codebaseを検索すると、`deleted_at IS NULL`が100箇所以上に散らばっていた。新しいクエリを追加するたびに、この条件を忘れないよう気をつける必要がある。人間の注意力に依存するアーキテクチャは、必ず破綻する。

## なぜ論理削除を使うのか

物理削除（DELETE）ではなく論理削除（deleted_atをセット）を使う理由は明確だ。

1. **データ復旧**: 誤削除からの復旧が容易
2. **監査**: 「誰がいつ削除したか」の記録
3. **参照整合性**: 外部キー制約を壊さずに「削除」できる
4. **ソフトランディング**: 完全削除前の猶予期間を設ける

ただし、実装を誤ると上記のバグが発生する。

## 失敗1：全てのクエリにWHERE句を手動追加

最初の実装は素朴だった。

```rust
// ❌ 毎回WHERE句を付ける
let users: Vec<User> = sqlx::query_as(
    "SELECT * FROM users WHERE deleted_at IS NULL"
)
.fetch_all(&pool).await?;

let posts: Vec<Post> = sqlx::query_as(
    "SELECT * FROM posts WHERE deleted_at IS NULL"
)
.fetch_all(&pool).await?;

// 新しいエンドポイントを追加するたびに忘れるリスク
```

JOINも複雑になる。

```sql
-- 両方のテーブルで条件が必要
SELECT p.*, u.name as author_name
FROM posts p
JOIN users u ON p.user_id = u.id
WHERE p.deleted_at IS NULL
  AND u.deleted_at IS NULL  -- 忘れがち
```

## 解決策1：Newtype Patternで型を分離

有効なデータと削除済みデータを別の型として表現する。

```rust
/// 有効なユーザー（deleted_at IS NULL）
#[derive(Debug, sqlx::FromRow)]
pub struct ActiveUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// 削除済みユーザー
#[derive(Debug, sqlx::FromRow)]
pub struct DeletedUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub deleted_at: DateTime<Utc>,
}

impl ActiveUser {
    pub async fn all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, email, created_at FROM users WHERE deleted_at IS NULL"
        )
        .fetch_all(pool).await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>> {
        sqlx::query_as(
            "SELECT id, name, email, created_at FROM users WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(pool).await
    }
}
```

APIハンドラでは`ActiveUser`しか使わない。型システムが「削除済みを返さない」ことを保証する。

```rust
async fn list_users(pool: &PgPool) -> Result<Vec<ActiveUser>> {
    ActiveUser::all(pool).await  // 削除済みは絶対に含まれない
}
```

管理機能では`DeletedUser`や両方を含む型を使い分ける。

## 解決策2：ビューで安全なデフォルトを作る

データベースレベルで「有効なデータのみ」のビューを作成する。

```sql
CREATE VIEW active_users AS
SELECT id, name, email, created_at, updated_at
FROM users
WHERE deleted_at IS NULL;

CREATE VIEW active_posts AS
SELECT id, user_id, title, content, created_at, updated_at
FROM posts
WHERE deleted_at IS NULL;
```

```rust
// ビューから取得すれば、削除済みは含まれない
let users: Vec<User> = sqlx::query_as(
    "SELECT * FROM active_users"
)
.fetch_all(&pool).await?;

// JOINも簡潔
let posts: Vec<PostWithAuthor> = sqlx::query_as(
    r#"
    SELECT p.id, p.title, u.name as author_name
    FROM active_posts p
    JOIN active_users u ON p.user_id = u.id
    "#
)
.fetch_all(&pool).await?;
```

ビューを使えば、WHERE句を忘れる心配がない。削除済みが必要な管理機能だけ、元のテーブルを直接参照する。

## 解決策3：Row Level Security（RLS）

PostgreSQLのRLSを使うと、データベースレベルで強制できる。

```sql
-- RLSを有効化
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- デフォルトで有効なデータのみ表示
CREATE POLICY active_only ON users
FOR SELECT
USING (deleted_at IS NULL OR current_setting('app.include_deleted', TRUE) = 'true');
```

```rust
// 通常のクエリは有効なデータのみ
let users = sqlx::query_as::<_, User>("SELECT * FROM users")
    .fetch_all(&pool).await?;

// 削除済みを含めたい場合
sqlx::query("SET LOCAL app.include_deleted = 'true'")
    .execute(&pool).await?;
let all_users = sqlx::query_as::<_, User>("SELECT * FROM users")
    .fetch_all(&pool).await?;
```

RLSは強力だが、設定を誤ると全データが見えなくなるリスクがある。十分なテストが必要だ。

## 解決策4：リポジトリパターン

データアクセスをリポジトリに集約する。

```rust
#[async_trait]
pub trait UserRepository {
    async fn find(&self, id: Uuid) -> Result<Option<ActiveUser>>;
    async fn all(&self) -> Result<Vec<ActiveUser>>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

pub struct PgUserRepository {
    pool: PgPool,
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find(&self, id: Uuid) -> Result<Option<ActiveUser>> {
        sqlx::query_as(
            "SELECT id, name, email, created_at FROM users WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(&self.pool).await
    }

    async fn all(&self) -> Result<Vec<ActiveUser>> {
        sqlx::query_as(
            "SELECT id, name, email, created_at FROM users WHERE deleted_at IS NULL"
        )
        .fetch_all(&self.pool).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }
}
```

アプリケーションコードはリポジトリ経由でのみデータにアクセスする。WHERE句はリポジトリ内に隠蔽される。

## 解決策5：トレイトで共通化

論理削除可能なエンティティに共通のトレイトを定義する。

```rust
#[async_trait]
pub trait SoftDeletable: Sized {
    type Id: Send + Sync;

    async fn find_active(pool: &PgPool, id: Self::Id) -> Result<Option<Self>>;
    async fn all_active(pool: &PgPool) -> Result<Vec<Self>>;
    async fn soft_delete(pool: &PgPool, id: Self::Id) -> Result<bool>;
    async fn restore(pool: &PgPool, id: Self::Id) -> Result<bool>;
}
```

各エンティティでこのトレイトを実装する。統一されたインターフェースで論理削除を扱える。

## 解決策6：マクロで定型コードを削減

Rustのマクロで定型コードを生成する。

```rust
macro_rules! impl_soft_deletable {
    ($struct:ident, $table:literal) => {
        impl $struct {
            pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool> {
                let result = sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
                    $table
                ))
                .bind(id)
                .execute(pool).await?;
                Ok(result.rows_affected() > 0)
            }

            pub async fn restore(pool: &PgPool, id: Uuid) -> Result<bool> {
                let result = sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL",
                    $table
                ))
                .bind(id)
                .execute(pool).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    };
}

impl_soft_deletable!(User, "users");
impl_soft_deletable!(Post, "posts");
impl_soft_deletable!(Comment, "comments");
```

## パターン選択の指針

```
論理削除が必要？
├─ NO → 物理削除で十分
└─ YES → チーム規模は？
          ├─ 小規模（1-3人）
          │   └─ ビュー + Newtype Pattern
          ├─ 中規模（4-10人）
          │   └─ リポジトリパターン + トレイト
          └─ 大規模（10人以上）
              └─ RLS + ビュー + 厳密な型定義
```

小規模ならビューだけでも十分機能する。チームが大きくなると、強制力のあるRLSやリポジトリパターンが必要になる。

## 追加の考慮事項

### インデックス

削除済みを除外したインデックスを作成する。

```sql
-- 有効なデータのみをインデックス
CREATE INDEX idx_users_active ON users(email) WHERE deleted_at IS NULL;
```

### UNIQUE制約

削除済みを除外したUNIQUE制約。同じemailで再登録を許可する場合。

```sql
CREATE UNIQUE INDEX idx_users_email_active ON users(email) WHERE deleted_at IS NULL;
```

### 外部キー制約

論理削除されたユーザーへの参照を許可するか？ケースバイケースだが、多くの場合は許可する。

```sql
-- CASCADEで物理削除すると参照も消える
-- 論理削除なら参照は残る（意図した動作）
```

## 今はこうしている

冒頭の「100箇所にWHERE句」は、以下の対策で解消した。

1. **ビューを作成**: `active_users`, `active_posts`
2. **Newtype Pattern**: `ActiveUser`, `DeletedUser`
3. **リポジトリパターン**: 直接SQLを書かない

新しいエンドポイントを追加するときは、ビューまたはリポジトリ経由でアクセスする。元のテーブルを直接参照するのは管理機能だけ。WHERE句の付け忘れは構造的に起きなくなった。

論理削除は単純な機能に見えるが、codebase全体に影響する設計判断だ。最初から適切なパターンを選ぶことで、後々のバグを防げる。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_12_soft_delete_patterns
cargo run
```

## 参考資料

- [PostgreSQL - Row Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [PostgreSQL - Partial Indexes](https://www.postgresql.org/docs/current/indexes-partial.html)
- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
