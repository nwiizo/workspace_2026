# 本番マイグレーション前のチェックリスト：ダウンタイムゼロへの8ステップ

## はじめに

「ALTER TABLEでテーブルがロックされて、サービスが5分止まりました」

本番環境でのスキーマ変更は緊張する。小さなミスが大きな障害につながる。カラムを追加するだけのつもりが、テーブル全体がロックされてサービスが停止した経験は、一度や二度ではない。

本記事では、PostgreSQL + sqlxでのゼロダウンタイムマイグレーションのチェックリストを8ステップにまとめた。各ステップには「やること」と「やってはいけないこと」を明記している。

## チェックリスト概要

| # | ステップ | 危険度 |
|---|---------|--------|
| 1 | NULLable列を追加する | 低 |
| 2 | バッチでバックフィルする | 中 |
| 3 | デフォルト値を設定する | 低 |
| 4 | NOT NULL制約を追加する | 高 |
| 5 | インデックスを並行作成する | 中 |
| 6 | カラム名を安全に変更する | 高 |
| 7 | テーブル構造を変更する | 高 |
| 8 | マイグレーション状態を記録する | 低 |

## □ ステップ1：NULLable列を追加する

新しいカラムを追加するとき、最初はNULLを許可する。

```sql
-- ✅ OK: NULLable列の追加（即座に完了、ロックなし）
ALTER TABLE users ADD COLUMN status VARCHAR(20);

-- ❌ NG: NOT NULLで追加（テーブルロック、全行書き換え）
ALTER TABLE users ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'active';
```

NULLable列の追加はメタデータの変更だけで済むため、即座に完了する。NOT NULLで追加すると、PostgreSQLは全行にデフォルト値を書き込む必要があり、テーブル全体がロックされる。

```rust
// Rust側で確認
sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS status VARCHAR(20)")
    .execute(&pool).await?;
```

`IF NOT EXISTS`を付けると、既に存在する場合はスキップされる。冪等性が保たれる。

## □ ステップ2：バッチでバックフィルする

既存データに値を設定する。一括UPDATEは避け、バッチで処理する。

```sql
-- ❌ NG: 一括UPDATE（大量の行ロック）
UPDATE users SET status = 'active' WHERE status IS NULL;

-- ✅ OK: バッチ処理
UPDATE users SET status = 'active'
WHERE id IN (
    SELECT id FROM users WHERE status IS NULL LIMIT 1000
);
```

```rust
async fn backfill_status(pool: &PgPool) -> Result<u64> {
    let batch_size: i64 = 1000;
    let mut total = 0u64;

    loop {
        let result = sqlx::query(
            r#"
            UPDATE users SET status = 'active'
            WHERE id IN (
                SELECT id FROM users WHERE status IS NULL LIMIT $1
            )
            "#
        )
        .bind(batch_size)
        .execute(pool).await?;

        let affected = result.rows_affected();
        total += affected;

        if affected == 0 {
            break;
        }

        // 他のクエリに影響を与えないよう待機
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(total)
}
```

バッチサイズは1000〜10000程度が目安。大きすぎるとロック時間が長くなり、小さすぎると効率が悪い。

## □ ステップ3：デフォルト値を設定する

バックフィルが完了したら、新規行のためにデフォルト値を設定する。

```sql
ALTER TABLE users ALTER COLUMN status SET DEFAULT 'pending';
```

```rust
sqlx::query("ALTER TABLE users ALTER COLUMN status SET DEFAULT 'pending'")
    .execute(&pool).await?;
```

これはメタデータの変更だけなので、即座に完了する。

## □ ステップ4：NOT NULL制約を追加する

全行にデータが入っていることを確認してから、NOT NULL制約を追加する。

```sql
-- 先に確認
SELECT COUNT(*) FROM users WHERE status IS NULL;
-- 0件であることを確認

-- NOT NULL制約を追加
ALTER TABLE users ALTER COLUMN status SET NOT NULL;
```

```rust
// NULLが残っていないか確認
let null_count: (i64,) = sqlx::query_as(
    "SELECT COUNT(*) FROM users WHERE status IS NULL"
)
.fetch_one(&pool).await?;

if null_count.0 > 0 {
    return Err(anyhow!("{}件のNULL値が残っています", null_count.0));
}

// NOT NULL制約を追加
sqlx::query("ALTER TABLE users ALTER COLUMN status SET NOT NULL")
    .execute(&pool).await?;
```

PostgreSQL 11以降では、NOT NULL制約の追加時にテーブル全体をスキャンする。行数が多いと時間がかかる。

## □ ステップ5：インデックスを並行作成する

通常のCREATE INDEXはテーブルをロックする。CONCURRENTLYオプションを使う。

```sql
-- ❌ NG: テーブルロック
CREATE INDEX idx_users_status ON users(status);

-- ✅ OK: 並行作成（ロックなし）
CREATE INDEX CONCURRENTLY idx_users_status ON users(status);
```

```rust
// CONCURRENTLYはトランザクション内で使えない
sqlx::query("CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_users_status ON users(status)")
    .execute(&pool).await?;
```

`CONCURRENTLY`はトランザクション内では使えない。また、失敗した場合は不完全なインデックスが残る。

```sql
-- 失敗したインデックスを確認
SELECT * FROM pg_indexes WHERE indexname = 'idx_users_status';

-- 不完全なインデックスを削除
DROP INDEX CONCURRENTLY IF EXISTS idx_users_status;
```

## □ ステップ6：カラム名を安全に変更する

カラム名の直接変更は、アプリケーションとの整合性を壊す。段階的に移行する。

```sql
-- ❌ NG: 直接リネーム（アプリが壊れる）
ALTER TABLE users RENAME COLUMN name TO display_name;
```

### 安全な手順

```sql
-- Step 1: 新しいカラムを追加
ALTER TABLE users ADD COLUMN display_name VARCHAR(100);

-- Step 2: データをコピー
UPDATE users SET display_name = name WHERE display_name IS NULL;

-- Step 3: トリガーで同期（両方のカラムを更新）
CREATE OR REPLACE FUNCTION sync_name_columns()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.name IS DISTINCT FROM OLD.name THEN
        NEW.display_name = NEW.name;
    ELSIF NEW.display_name IS DISTINCT FROM OLD.display_name THEN
        NEW.name = NEW.display_name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sync_name_trigger
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION sync_name_columns();

-- Step 4: アプリケーションを新しいカラム名に移行

-- Step 5: トリガーと古いカラムを削除
DROP TRIGGER sync_name_trigger ON users;
DROP FUNCTION sync_name_columns();
ALTER TABLE users DROP COLUMN name;
```

この手順なら、アプリケーションを段階的に移行できる。旧カラムと新カラムが共存する期間を設ける。

## □ ステップ7：テーブル構造を変更する

非正規化されたデータを別テーブルに分離する場合も、段階的に行う。

```rust
// Step 1: 新しいテーブルを作成
sqlx::query(
    r#"
    CREATE TABLE user_addresses (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES users(id),
        street VARCHAR(255),
        city VARCHAR(100),
        country VARCHAR(100)
    )
    "#
)
.execute(&pool).await?;

// Step 2: データを移行（バッチ処理）
let users: Vec<(Uuid, String, String, String)> = sqlx::query_as(
    "SELECT id, street, city, country FROM users WHERE street IS NOT NULL LIMIT 1000"
)
.fetch_all(&pool).await?;

for (user_id, street, city, country) in users {
    sqlx::query(
        "INSERT INTO user_addresses (user_id, street, city, country) VALUES ($1, $2, $3, $4)"
    )
    .bind(user_id)
    .bind(street)
    .bind(city)
    .bind(country)
    .execute(&pool).await?;
}

// Step 3: アプリケーションを新しいテーブルに移行

// Step 4: 古いカラムを削除
// ALTER TABLE users DROP COLUMN street, DROP COLUMN city, DROP COLUMN country;
```

## □ ステップ8：マイグレーション状態を記録する

どのマイグレーションが適用済みか追跡する。

```sql
CREATE TABLE _migrations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
async fn apply_migration(pool: &PgPool, name: &str, sql: &str) -> Result<bool> {
    // 既に適用済みかチェック
    let applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)"
    )
    .bind(name)
    .fetch_one(pool).await?;

    if applied {
        println!("Migration already applied: {}", name);
        return Ok(false);
    }

    // マイグレーションを実行
    sqlx::query(sql).execute(pool).await?;

    // 適用を記録
    sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
        .bind(name)
        .execute(pool).await?;

    println!("Applied migration: {}", name);
    Ok(true)
}
```

sqlx-cliを使う場合は、`sqlx migrate`コマンドが自動で管理してくれる。

```sh
# マイグレーションファイルを作成
sqlx migrate add create_users

# マイグレーションを実行
sqlx migrate run
```

## まとめチェックリスト

本番マイグレーション前に確認する。

```
□ 1. 新しいカラムはNULLableで追加している
□ 2. バックフィルはバッチ処理で行っている
□ 3. デフォルト値を設定している
□ 4. NOT NULL制約はバックフィル完了後に追加している
□ 5. インデックスはCONCURRENTLYで作成している
□ 6. カラム名変更は段階的に行っている
□ 7. ステージング環境でテスト済み
□ 8. ロールバック手順を用意している
```

## 危険な操作一覧

これらの操作はテーブルロックを引き起こす。本番環境では避ける。

| 操作 | 危険度 | 代替手段 |
|------|--------|---------|
| `ADD COLUMN ... NOT NULL` | 高 | NULLableで追加 → バックフィル → NOT NULL |
| `CREATE INDEX` | 高 | `CREATE INDEX CONCURRENTLY` |
| `ALTER TYPE` | 高 | 新カラム追加 → コピー → 削除 |
| `RENAME COLUMN` | 中 | 新カラム追加 → トリガー同期 → 削除 |
| `DROP COLUMN` | 中 | 先にアプリから参照を削除 |

## 結論

ゼロダウンタイムマイグレーションの鍵は「段階的に行う」ことだ。

1. **小さなステップに分割**: 一度に大きな変更をしない
2. **ロックを最小化**: CONCURRENTLYを使う、バッチ処理する
3. **冪等性を保つ**: IF NOT EXISTS、IF EXISTSを使う
4. **ロールバック可能に**: 各ステップで戻れるようにする

スキーマ変更は慎重に。テストは本番と同等のデータ量で行う。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_09_migration
cargo run
```

## 参考資料

- [PostgreSQL - ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html)
- [PostgreSQL - CREATE INDEX CONCURRENTLY](https://www.postgresql.org/docs/current/sql-createindex.html#SQL-CREATEINDEX-CONCURRENTLY)
- [sqlx - Migrations](https://github.com/launchbadge/sqlx#migrations)
