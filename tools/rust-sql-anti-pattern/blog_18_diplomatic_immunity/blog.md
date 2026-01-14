# SQLに免責特権はない：スキーマ変更前の7つのセーフティチェック

## 発端

「マイグレーションを本番適用したら、外部キー制約でエラーが出て戻せません」

Slackにこのメッセージが流れてきた。開発環境では動いたマイグレーションが、本番で失敗していた。孤立データが残っていて、外部キー制約を追加できなかったのだ。

SQLはアプリケーションコードと違って「動くかどうか」が環境に依存する。開発環境にはないデータが本番にはある。同じDDLでも結果が異なる。

本記事では、本番マイグレーション前に確認すべき7つのチェック項目を示す。

## チェック1：マイグレーション追跡システムがあるか

適用済みのマイグレーションを追跡する仕組みがなければ、どこまで適用したかわからない。

```sql
CREATE TABLE IF NOT EXISTS _migrations (
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
        return Ok(false);  // スキップ
    }

    // トランザクション内で実行
    let mut tx = pool.begin().await?;

    sqlx::query(sql).execute(&mut *tx).await?;

    sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
        .bind(name)
        .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(true)
}
```

sqlxのマイグレーション機能を使うなら、`_sqlx_migrations`テーブルが自動で作られる。

```bash
# マイグレーションファイルの作成
sqlx migrate add create_users

# 適用
sqlx migrate run
```

### 確認項目

- [ ] マイグレーション追跡テーブルが存在するか
- [ ] 全環境（開発・ステージング・本番）で同じ仕組みを使っているか
- [ ] マイグレーションファイルはバージョン管理されているか

## チェック2：ロールバック手順が明確か

マイグレーションが失敗したとき、どう戻すか決まっているか。

```sql
-- 20241201_create_users.sql

-- UP
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- DOWN
DROP TABLE users;
```

sqlxでは`sqlx migrate revert`で直前のマイグレーションを取り消せる。ただし、データが入った後のDROP TABLEは取り返しがつかない。

### 破壊的変更の前には

1. **バックアップを取る**
2. **復旧手順を文書化する**
3. **可能ならステージング環境で本番データを使ってテストする**

### 確認項目

- [ ] ロールバック用SQLが準備されているか
- [ ] データ損失を伴う変更の場合、バックアップ手順が明確か
- [ ] 失敗時の連絡先・エスカレーションパスが決まっているか

## チェック3：制約追加前にデータを検証したか

外部キー制約やNOT NULL制約を追加する前に、既存データが条件を満たすか確認する。

```rust
// 外部キー制約を追加する前に孤立データをチェック
async fn check_orphaned_posts(pool: &PgPool) -> Result<Vec<Uuid>> {
    let orphans: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT p.id
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE u.id IS NULL
        "#
    )
    .fetch_all(pool).await?;

    Ok(orphans.into_iter().map(|(id,)| id).collect())
}

// NOT NULL制約を追加する前にNULLデータをチェック
async fn check_null_emails(pool: &PgPool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email IS NULL"
    )
    .fetch_one(pool).await?;

    Ok(count)
}
```

`information_schema`を使ってカラムの情報を確認することもできる。

```rust
#[derive(Debug, sqlx::FromRow)]
struct ColumnInfo {
    column_name: String,
    data_type: String,
    is_nullable: String,
}

async fn get_table_columns(pool: &PgPool, table: &str) -> Result<Vec<ColumnInfo>> {
    sqlx::query_as(
        r#"
        SELECT
            column_name::TEXT,
            data_type::TEXT,
            is_nullable::TEXT
        FROM information_schema.columns
        WHERE table_name = $1
        ORDER BY ordinal_position
        "#
    )
    .bind(table)
    .fetch_all(pool).await
}
```

### 確認項目

- [ ] 追加する制約に違反するデータが存在しないか確認したか
- [ ] 違反データがある場合、クリーンアップ手順が明確か
- [ ] ステージング環境で本番相当のデータでテストしたか

## チェック4：制約が正しく動作するかテストしたか

制約を追加したら、正常系と異常系の両方をテストする。

```rust
async fn test_unique_constraint(pool: &PgPool) -> Result<()> {
    // サンプルデータを作成
    sqlx::query("INSERT INTO users (email, name) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("Test User")
        .execute(pool).await?;

    // 重複メールでエラーになることを確認
    let result = sqlx::query("INSERT INTO users (email, name) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("Another User")
        .execute(pool).await;

    match result {
        Ok(_) => panic!("UNIQUE constraint not working!"),
        Err(e) if e.to_string().contains("duplicate") => {
            println!("PASS: UNIQUE constraint works");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    Ok(())
}

async fn test_foreign_key_constraint(pool: &PgPool) -> Result<()> {
    let fake_user_id = Uuid::new_v4();

    let result = sqlx::query(
        "INSERT INTO posts (user_id, title, content) VALUES ($1, $2, $3)"
    )
    .bind(fake_user_id)
    .bind("Test Post")
    .bind("Content")
    .execute(pool).await;

    match result {
        Ok(_) => panic!("FK constraint not working!"),
        Err(e) if e.to_string().contains("foreign key") => {
            println!("PASS: FK constraint works");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    Ok(())
}

async fn test_check_constraint(pool: &PgPool) -> Result<()> {
    let user_id = get_test_user_id(pool).await?;

    let result = sqlx::query(
        "INSERT INTO posts (user_id, title, content, status) VALUES ($1, $2, $3, $4)"
    )
    .bind(user_id)
    .bind("Test")
    .bind("Content")
    .bind("invalid_status")
    .execute(pool).await;

    match result {
        Ok(_) => panic!("CHECK constraint not working!"),
        Err(e) if e.to_string().contains("check") => {
            println!("PASS: CHECK constraint works");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    Ok(())
}
```

### 確認項目

- [ ] UNIQUE制約が重複を防ぐことをテストしたか
- [ ] FK制約が不正な参照を防ぐことをテストしたか
- [ ] CHECK制約が不正な値を防ぐことをテストしたか
- [ ] NOT NULL制約がNULLを防ぐことをテストしたか

## チェック5：スキーマ変更が段階的か

破壊的な変更を一度に行うと、ロールバックが困難になる。段階的に行う。

### カラム名の変更例

```
Phase 1: 新しいカラムを追加
ALTER TABLE users ADD COLUMN display_name VARCHAR(100);

Phase 2: データをコピー
UPDATE users SET display_name = name WHERE display_name IS NULL;

Phase 3: アプリケーションを新カラム対応にデプロイ

Phase 4: 古いカラムを削除
ALTER TABLE users DROP COLUMN name;
```

```rust
async fn phase1_add_column(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name VARCHAR(100)"
    )
    .execute(pool).await?;

    println!("Phase 1 complete: Added display_name column");
    Ok(())
}

async fn phase2_copy_data(pool: &PgPool) -> Result<()> {
    let rows = sqlx::query(
        "UPDATE users SET display_name = name WHERE display_name IS NULL"
    )
    .execute(pool).await?
    .rows_affected();

    println!("Phase 2 complete: Copied {} rows", rows);
    Ok(())
}

async fn phase4_drop_column(pool: &PgPool) -> Result<()> {
    // Phase 3が完了し、アプリケーションが新カラムを使っていることを確認してから実行
    sqlx::query("ALTER TABLE users DROP COLUMN name")
        .execute(pool).await?;

    println!("Phase 4 complete: Dropped name column");
    Ok(())
}
```

### 確認項目

- [ ] 破壊的変更を段階的に分解しているか
- [ ] 各フェーズで動作確認ができるか
- [ ] フェーズ間でロールバック可能か

## チェック6：インデックスと制約を確認したか

`pg_indexes`と`pg_constraint`で現在の状態を確認する。

```rust
async fn verify_indexes(pool: &PgPool, table: &str) -> Result<()> {
    let indexes: Vec<(String,)> = sqlx::query_as(
        "SELECT indexname::TEXT FROM pg_indexes WHERE tablename = $1"
    )
    .bind(table)
    .fetch_all(pool).await?;

    println!("Indexes on {}:", table);
    for (name,) in &indexes {
        println!("  - {}", name);
    }

    Ok(())
}

async fn verify_constraints(pool: &PgPool, table: &str) -> Result<()> {
    let constraints: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT conname::TEXT, contype::TEXT
        FROM pg_constraint c
        JOIN pg_class t ON c.conrelid = t.oid
        WHERE t.relname = $1
        "#
    )
    .bind(table)
    .fetch_all(pool).await?;

    println!("Constraints on {}:", table);
    for (name, contype) in &constraints {
        let type_name = match contype.as_str() {
            "p" => "PRIMARY KEY",
            "u" => "UNIQUE",
            "f" => "FOREIGN KEY",
            "c" => "CHECK",
            _ => contype,
        };
        println!("  - {} ({})", name, type_name);
    }

    Ok(())
}
```

### FK用インデックスの確認

PostgreSQLは外部キー列に自動でインデックスを作らない。手動で確認・追加が必要。

```rust
async fn check_fk_indexes(pool: &PgPool) -> Result<()> {
    // FK列にインデックスがないケースを検出
    let missing: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT
            conrelid::regclass::TEXT AS table_name,
            a.attname::TEXT AS column_name
        FROM pg_constraint c
        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
        WHERE c.contype = 'f'
        AND NOT EXISTS (
            SELECT 1 FROM pg_index i
            WHERE i.indrelid = c.conrelid
            AND a.attnum = ANY(i.indkey)
        )
        "#
    )
    .fetch_all(pool).await?;

    if missing.is_empty() {
        println!("All FK columns have indexes");
    } else {
        println!("FK columns missing indexes:");
        for (table, column) in &missing {
            println!("  - {}.{}", table, column);
        }
    }

    Ok(())
}
```

### 確認項目

- [ ] 必要なインデックスが存在するか
- [ ] FK列にインデックスがあるか
- [ ] 不要なインデックスがないか

## チェック7：アンチパターンを避けているか

### やってはいけないこと

| アンチパターン | 問題点 | 解決策 |
|--------------|--------|--------|
| SQLをバージョン管理しない | 履歴がわからない | Gitで管理 |
| コードレビューしない | ミスが見つからない | PRレビュー必須 |
| テストしない | 本番で初めて問題発覚 | ステージングでテスト |
| ロールバック計画なし | 失敗時に戻せない | DOWN SQLを準備 |
| 本番で直接変更 | 履歴が残らない | 必ずマイグレーション経由 |

```rust
async fn demo_anti_patterns_check(pool: &PgPool) -> Result<()> {
    println!("Anti-pattern checklist:");

    // マイグレーション数を確認
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM _migrations"
    )
    .fetch_one(pool).await?;

    println!("  [{}] {} migrations tracked",
        if count > 0 { "✓" } else { "✗" },
        count);

    // テーブルに制約があるか確認
    let constraint_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM pg_constraint c
        JOIN pg_class t ON c.conrelid = t.oid
        WHERE t.relname NOT LIKE 'pg_%' AND t.relname NOT LIKE '_%'
        "#
    )
    .fetch_one(pool).await?;

    println!("  [{}] {} constraints defined",
        if constraint_count > 0 { "✓" } else { "✗" },
        constraint_count);

    Ok(())
}
```

### 確認項目

- [ ] マイグレーションファイルがバージョン管理されているか
- [ ] スキーマ変更にコードレビューがあるか
- [ ] ステージング環境でテストしてから本番適用しているか
- [ ] ロールバック手順が文書化されているか
- [ ] 本番で手動DDLを実行していないか

## 冒頭の問題を振り返る

外部キー制約の追加で失敗した問題は、チェック3（データ検証）を怠ったために起きた。

```rust
// 本来やるべきだったこと
let orphans = check_orphaned_posts(&pool).await?;
if !orphans.is_empty() {
    println!("Cannot add FK: {} orphaned posts found", orphans.len());
    for id in &orphans {
        println!("  - {}", id);
    }
    return Err(anyhow!("Clean up orphaned data first"));
}

// 孤立データがないことを確認してから制約追加
sqlx::query(
    "ALTER TABLE posts ADD CONSTRAINT fk_posts_user
     FOREIGN KEY (user_id) REFERENCES users(id)"
)
.execute(&pool).await?;
```

今はマイグレーション前に自動でデータ検証を実行している。CIパイプラインで本番データのコピーを使ってテストすることで、この種の問題を事前に検出できるようになった。

## まとめ：7つのチェックリスト

本番マイグレーション前に確認すること。

1. **マイグレーション追跡**: 適用履歴を記録しているか
2. **ロールバック手順**: 失敗時の復旧方法が明確か
3. **データ検証**: 制約に違反するデータがないか
4. **制約テスト**: 正常系・異常系を両方テストしたか
5. **段階的変更**: 破壊的変更を分割しているか
6. **インデックス確認**: 必要なインデックスが存在するか
7. **アンチパターン回避**: バージョン管理・レビュー・テストがあるか

SQLに「動くから大丈夫」という免責特権はない。アプリケーションコードと同じレベルの慎重さが必要だ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_18_diplomatic_immunity
cargo run
```

## 参考資料

- [PostgreSQL - information_schema](https://www.postgresql.org/docs/current/information-schema.html)
- [PostgreSQL - System Catalogs](https://www.postgresql.org/docs/current/catalogs.html)
- [sqlx - Migrations](https://docs.rs/sqlx/latest/sqlx/migrate/index.html)
