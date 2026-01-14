# マルチテナント設計FAQ：テナントID列とRLS、どっちがいい？

## Q1: マルチテナント設計って何？

1つのアプリケーションで複数の顧客（テナント）のデータを扱う設計だ。SaaSでは必須のパターン。

```
Company A のデータ → 同じアプリ、同じDB ← Company B のデータ
                     ↓
             テナント間でデータを分離
```

問題は「どうやってデータを分離するか」だ。Company AがCompany Bのデータを見れてしまったら大問題。

## Q2: どんなアプローチがある？

3つのアプローチがある。

### 1. テナントID列

最もシンプル。全テーブルに`tenant_id`カラムを追加する。

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (tenant_id, email)  -- テナント内でユニーク
);

CREATE TABLE projects (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id)
);

-- インデックス（必須）
CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_projects_tenant ON projects(tenant_id);
```

### 2. Row Level Security（RLS）

PostgreSQLの機能で、データベースレベルでアクセス制御する。

```sql
-- RLSを有効化
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;

-- テナント分離ポリシー
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID);

CREATE POLICY tenant_isolation_projects ON projects
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID);
```

### 3. スキーマ分離

テナントごとに別スキーマを使う。最も強い分離だが、管理が複雑。

```sql
CREATE SCHEMA tenant_alpha;
CREATE SCHEMA tenant_beta;

CREATE TABLE tenant_alpha.users (...);
CREATE TABLE tenant_beta.users (...);
```

## Q3: テナントID列のメリット・デメリットは？

### メリット

- シンプルで理解しやすい
- 特別な設定不要
- 全テナントを横断するクエリが簡単

### デメリット

- 全クエリに`WHERE tenant_id = $1`が必要
- 書き忘れるとデータ漏洩

```rust
// ❌ 書き忘れ：全テナントのデータが見える
let users: Vec<User> = sqlx::query_as("SELECT * FROM users")
    .fetch_all(&pool).await?;

// ✅ 正しい：テナントでフィルタ
let users: Vec<User> = sqlx::query_as(
    "SELECT * FROM users WHERE tenant_id = $1"
)
.bind(tenant_id)
.fetch_all(&pool).await?;
```

### 実装例

```rust
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

// 全クエリにテナントIDを含める
pub async fn get_users(pool: &PgPool, ctx: &TenantContext) -> Result<Vec<User>> {
    sqlx::query_as("SELECT * FROM users WHERE tenant_id = $1")
        .bind(ctx.tenant_id)
        .fetch_all(pool).await
}

pub async fn get_projects(pool: &PgPool, ctx: &TenantContext) -> Result<Vec<Project>> {
    sqlx::query_as("SELECT * FROM projects WHERE tenant_id = $1")
        .bind(ctx.tenant_id)
        .fetch_all(pool).await
}
```

## Q4: RLSのメリット・デメリットは？

### メリット

- データベースレベルで強制
- 書き忘れがない
- SQLを変更せずにポリシーを変更可能

### デメリット

- 設定を誤ると全データが見えなくなる
- デバッグが難しい
- パフォーマンスオーバーヘッド（わずか）

### 実装例

```rust
// トランザクション開始時にテナントIDを設定
async fn with_tenant<F, T>(
    pool: &PgPool,
    tenant_id: Uuid,
    f: F,
) -> Result<T>
where
    F: FnOnce(&mut Transaction<'_, Postgres>) -> Pin<Box<dyn Future<Output = Result<T>> + Send + '_>>,
{
    let mut tx = pool.begin().await?;

    // セッション変数でテナントIDを設定
    sqlx::query(&format!("SET LOCAL app.current_tenant_id = '{}'", tenant_id))
        .execute(&mut *tx).await?;

    let result = f(&mut tx).await?;

    tx.commit().await?;
    Ok(result)
}

// 使用例
with_tenant(&pool, tenant_id, |tx| Box::pin(async move {
    // RLSにより自動的にテナントのデータのみが取得される
    let users: Vec<User> = sqlx::query_as("SELECT * FROM users")
        .fetch_all(&mut **tx).await?;

    Ok(users)
})).await?;
```

## Q5: どっちを選ぶべき？

### テナントID列を選ぶ場合

- チームがRLSに慣れていない
- シンプルさを優先
- テナント横断のレポートが必要

### RLSを選ぶ場合

- セキュリティ要件が厳しい
- 既存コードに手を入れたくない
- データ漏洩のリスクを最小化したい

### 推奨

**両方使う**のがベスト。

```sql
-- テナントID列でフィルタ（明示的）
SELECT * FROM users WHERE tenant_id = $1;

-- かつRLSで二重チェック（セーフティネット）
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_check ON users
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID);
```

WHERE句の書き忘れがあっても、RLSがセーフティネットになる。

## Q6: クロステナントアクセスの問題は？

外部キー制約だけでは不十分。

```rust
// テナントAとしてログイン中に
// テナントBのユーザーをプロジェクトのオーナーに設定できてしまう

let user_b = get_user_from_tenant_b();  // 何らかの方法で取得

sqlx::query("INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3)")
    .bind(tenant_a)  // テナントA
    .bind("Sneaky Project")
    .bind(user_b.id)  // テナントBのユーザー！
    .execute(&pool).await?;

// FK制約は「users.idが存在するか」だけをチェック
// テナントが一致するかはチェックしない
```

### 解決策

1. **アプリケーションでチェック**

```rust
async fn create_project(
    pool: &PgPool,
    ctx: &TenantContext,
    name: &str,
    owner_id: Uuid,
) -> Result<Project> {
    // オーナーが同じテナントか確認
    let owner: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE id = $1 AND tenant_id = $2"
    )
    .bind(owner_id)
    .bind(ctx.tenant_id)
    .fetch_optional(pool).await?;

    let owner = owner.ok_or(anyhow!("Owner not found in this tenant"))?;

    // 安全にプロジェクトを作成
    sqlx::query_as(
        "INSERT INTO projects (tenant_id, name, owner_id) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(ctx.tenant_id)
    .bind(name)
    .bind(owner.id)
    .fetch_one(pool).await
}
```

2. **RLSでINSERTもチェック**

```sql
CREATE POLICY tenant_insert ON projects
    FOR INSERT
    WITH CHECK (
        tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID
        AND owner_id IN (
            SELECT id FROM users
            WHERE tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID
        )
    );
```

## Q7: インデックス戦略は？

テナントID列を含む複合インデックスを作る。

```sql
-- 基本：テナントでフィルタ
CREATE INDEX idx_users_tenant ON users(tenant_id);

-- よく使うクエリに合わせて
CREATE INDEX idx_users_tenant_email ON users(tenant_id, email);
CREATE INDEX idx_projects_tenant_owner ON projects(tenant_id, owner_id);
CREATE INDEX idx_projects_tenant_created ON projects(tenant_id, created_at DESC);
```

テナントIDが最初に来るのが重要。`WHERE tenant_id = $1`が全クエリに含まれるため。

## Q8: デバッグ時のRLS無効化は？

テスト時にRLSを無効化したい場合。

```sql
-- スーパーユーザーはRLSをバイパス
ALTER TABLE users FORCE ROW LEVEL SECURITY;  -- スーパーユーザーにも適用

-- または特定のロールをバイパス
CREATE ROLE admin_role BYPASSRLS;
```

開発時のデバッグ。

```rust
// デバッグ用：全テナントのデータを見る
sqlx::query("SET LOCAL app.current_tenant_id = ''")
    .execute(&mut *tx).await?;

// または
sqlx::query("SET LOCAL row_security = off")  // スーパーユーザーのみ
    .execute(&mut *tx).await?;
```

## Q9: マイグレーション時の注意点は？

既存テーブルにRLSを追加する際の注意。

```sql
-- 1. まずポリシーなしで有効化（何も見えなくなる）
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- 2. すぐにポリシーを追加
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', TRUE), '')::UUID);

-- これを同一トランザクションで実行すること！
```

本番で段階的に導入する場合。

```sql
-- ステップ1：ポリシーを作成（まだ有効化しない）
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = ...);

-- ステップ2：アプリケーションを更新
-- SET LOCAL app.current_tenant_id を全リクエストで設定

-- ステップ3：RLSを有効化
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
```

## まとめ

| 観点 | テナントID列 | RLS |
|------|------------|-----|
| 実装の簡単さ | ◎ | △ |
| セキュリティ | △（書き忘れリスク） | ◎（強制） |
| デバッグ | ◎ | △ |
| パフォーマンス | ◎ | ○ |
| 推奨 | 基本として使う | セーフティネットとして追加 |

両方を組み合わせるのがベストプラクティスだ。

1. **テナントID列**: 全テーブルに追加、全クエリで使用
2. **RLS**: セーフティネットとして設定
3. **インデックス**: tenant_idを含む複合インデックス
4. **アプリケーション**: TenantContextを全操作で渡す

これでデータ漏洩のリスクを最小化できる。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_24_multi_tenant
cargo run
```

## 参考資料

- [PostgreSQL - Row Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [PostgreSQL - CREATE POLICY](https://www.postgresql.org/docs/current/sql-createpolicy.html)
- [Multi-tenant Data Architecture - AWS](https://docs.aws.amazon.com/wellarchitected/latest/saas-lens/multi-tenant-data-architecture.html)
