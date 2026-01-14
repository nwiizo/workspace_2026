# なぜDB統合テストが必要なのか：モックでは見つからない7つのバグ

## Q1: モックでテストすればいいのでは？

「データベースのモックを作れば、統合テストなしでユニットテストだけで済むのでは？」

この質問をよく受ける。答えはNoだ。モックでは見つからないバグがある。

## A1: モックでは見つからないバグの例

### 1. UNIQUE制約違反

```rust
// モックテスト：成功してしまう
#[test]
fn test_create_user_mock() {
    let mock_db = MockDatabase::new();
    mock_db.expect_insert().returning(|_| Ok(user.clone()));

    // 同じメールで2回作成しても成功する
    create_user(&mock_db, "alice@example.com", "Alice").await?;  // OK
    create_user(&mock_db, "alice@example.com", "Alice2").await?; // OK（本来は失敗すべき）
}
```

```rust
// 統合テスト：正しく失敗する
#[tokio::test]
async fn test_duplicate_email_fails() {
    let pool = setup_test_db().await;

    create_user(&pool, "alice@example.com", "Alice").await.unwrap();

    // 同じメールで作成しようとするとエラー
    let result = create_user(&pool, "alice@example.com", "Alice2").await;
    assert!(result.is_err());

    let error = result.unwrap_err().to_string();
    assert!(error.contains("duplicate") || error.contains("unique"));
}
```

### 2. 外部キー制約違反

```rust
// 統合テスト：FK違反を検出
#[tokio::test]
async fn test_foreign_key_violation() {
    let pool = setup_test_db().await;

    // 存在しないユーザーIDで注文を作成
    let fake_user_id = Uuid::new_v4();
    let result = create_order(&pool, fake_user_id, Decimal::new(10000, 2)).await;

    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("violates foreign key"));
}
```

モックではFK違反を検出できない。実際のデータベースに対してテストして初めて、参照整合性の問題が見つかる。

### 3. トランザクションのロールバック

```rust
#[tokio::test]
async fn test_transaction_rollback() {
    let pool = setup_test_db().await;

    let mut tx = pool.begin().await?;

    let user = create_user_in_tx(&mut tx, "david@example.com", "David").await?;
    println!("Created user in transaction: {}", user.name);

    // ロールバック
    tx.rollback().await?;

    // ロールバック後にデータが存在しないことを確認
    let found = get_user_by_email(&pool, "david@example.com").await;
    assert!(found.is_none(), "User should not exist after rollback");
}
```

トランザクションが正しくロールバックされるかは、実際のデータベースでテストしないとわからない。

### 4. NULL処理

```rust
#[tokio::test]
async fn test_null_handling() {
    let pool = setup_test_db().await;

    // 電話番号なし
    let user_without = create_user_with_phone(
        &pool, "grace@example.com", "Grace", None
    ).await?;

    // 電話番号あり
    let user_with = create_user_with_phone(
        &pool, "henry@example.com", "Henry", Some("090-1234-5678")
    ).await?;

    assert!(user_without.phone.is_none());
    assert_eq!(user_with.phone, Some("090-1234-5678".to_string()));

    // データベースから再取得してNULL処理を確認
    let fetched = get_user_by_id(&pool, user_without.id).await.unwrap();
    assert!(fetched.phone.is_none(), "NULL should map to Option::None");
}
```

PostgreSQLのNULLがRustの`Option::None`に正しくマッピングされるかは、実際のクエリで確認する必要がある。

### 5. CASCADE DELETE

```rust
#[tokio::test]
async fn test_cascade_delete() {
    let pool = setup_test_db().await;

    let user = create_user(&pool, "ivan@example.com", "Ivan").await?;
    let order1 = create_order(&pool, user.id, Decimal::new(5000, 2)).await?;
    let order2 = create_order(&pool, user.id, Decimal::new(7500, 2)).await?;

    // 注文が2件あることを確認
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM orders WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&pool).await?;
    assert_eq!(count, 2);

    // ユーザーを削除
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user.id)
        .execute(&pool).await?;

    // CASCADE DELETEで注文も削除されたことを確認
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM orders WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&pool).await?;
    assert_eq!(count, 0);
}
```

### 6. セーブポイント（ネストトランザクション）

```rust
#[tokio::test]
async fn test_savepoint() {
    let pool = setup_test_db().await;

    let mut tx = pool.begin().await?;

    // 最初のユーザー
    let user1 = create_user_in_tx(&mut tx, "eve@example.com", "Eve").await?;

    // セーブポイント開始
    let mut savepoint = tx.begin().await?;
    let user2 = create_user_in_tx(&mut savepoint, "frank@example.com", "Frank").await?;

    // セーブポイントをロールバック
    savepoint.rollback().await?;

    // 外側のトランザクションをコミット
    tx.commit().await?;

    // user1は存在するがuser2は存在しない
    let found1 = get_user_by_email(&pool, "eve@example.com").await;
    let found2 = get_user_by_email(&pool, "frank@example.com").await;

    assert!(found1.is_some(), "user1 should exist");
    assert!(found2.is_none(), "user2 should not exist");
}
```

### 7. CHECK制約

```sql
CREATE TABLE orders (
    ...
    status TEXT NOT NULL CHECK (status IN ('pending', 'confirmed', 'shipped', 'delivered'))
);
```

```rust
#[tokio::test]
async fn test_check_constraint() {
    let pool = setup_test_db().await;

    let result = sqlx::query(
        "INSERT INTO orders (user_id, total, status) VALUES ($1, $2, $3)"
    )
    .bind(user_id)
    .bind(Decimal::new(10000, 2))
    .bind("invalid_status")  // CHECK制約違反
    .execute(&pool).await;

    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("check"));
}
```

## Q2: sqlx::testマクロの使い方は？

sqlxには`#[sqlx::test]`マクロがあり、テスト用のトランザクション内で自動的にテストを実行する。テスト終了時に自動でロールバックされるので、テスト間でデータが汚染されない。

### A2: 基本的な使い方

```rust
// tests/integration_tests.rs
use sqlx::PgPool;

#[sqlx::test]
async fn test_create_user(pool: PgPool) {
    let email = format!("test_{}@example.com", Uuid::new_v4());

    let user = create_user(&pool, &email, "Test User").await.unwrap();

    assert_eq!(user.email, email);
    assert_eq!(user.name, "Test User");
    assert!(user.id != Uuid::nil());
}

#[sqlx::test]
async fn test_duplicate_fails(pool: PgPool) {
    let email = format!("dup_{}@example.com", Uuid::new_v4());

    create_user(&pool, &email, "First").await.unwrap();

    let result = create_user(&pool, &email, "Second").await;
    assert!(result.is_err());
}
```

### 前提条件

```bash
# DATABASE_URLを設定
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/antipattern_test

# マイグレーションを適用
sqlx migrate run

# テスト実行
cargo test
```

## Q3: フィクスチャはどう使う？

### A3: SQLファイルでフィクスチャを定義

```rust
// tests/fixtures/users.sql
INSERT INTO users (id, email, name) VALUES
    ('11111111-1111-1111-1111-111111111111', 'alice@example.com', 'Alice'),
    ('22222222-2222-2222-2222-222222222222', 'bob@example.com', 'Bob');

// テストで使用
#[sqlx::test(fixtures("users"))]
async fn test_with_fixture(pool: PgPool) {
    let users: Vec<User> = sqlx::query_as("SELECT * FROM users")
        .fetch_all(&pool).await.unwrap();

    assert_eq!(users.len(), 2);
}
```

## Q4: CIでどう実行する？

### A4: Testcontainersを使う

```toml
# Cargo.toml
[dev-dependencies]
testcontainers = "0.15"
testcontainers-modules = { version = "0.3", features = ["postgres"] }
```

```rust
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_with_container() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());

    let port = container.get_host_port_ipv4(5432);
    let url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

    let pool = PgPool::connect(&url).await.unwrap();

    // マイグレーションを実行
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // テスト
    let user = create_user(&pool, "test@example.com", "Test").await.unwrap();
    assert_eq!(user.email, "test@example.com");
}
```

GitHub Actionsでの設定例。

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: test
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/test
        run: cargo test
```

## Q5: モックは完全に不要？

### A5: 使い分ける

モックが有効なケース。

- 外部API（決済、メール送信）のテスト
- ネットワークエラーのシミュレーション
- 高速なユニットテストが必要な場合

統合テストが必要なケース。

- 制約（UNIQUE, FK, CHECK）のテスト
- トランザクション動作のテスト
- NULLハンドリングのテスト
- CASCADE DELETE/UPDATEのテスト
- パフォーマンステスト

両方を組み合わせるのがベストプラクティスだ。

## Q6: テストデータのクリーンアップは？

### A6: sqlx::testなら自動

`#[sqlx::test]`マクロを使えば、各テストはトランザクション内で実行され、終了時に自動でロールバックされる。手動でのクリーンアップは不要。

手動でテストする場合。

```rust
async fn setup_test_db() -> PgPool {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await.unwrap();

    // テスト用のスキーマを作成
    sqlx::query("CREATE SCHEMA IF NOT EXISTS test")
        .execute(&pool).await.unwrap();

    // search_pathを設定
    sqlx::query("SET search_path TO test, public")
        .execute(&pool).await.unwrap();

    pool
}

async fn teardown_test_db(pool: &PgPool) {
    sqlx::query("DROP SCHEMA IF EXISTS test CASCADE")
        .execute(pool).await.unwrap();
}
```

## Q7: 遅くならない？

### A7: 工夫次第

統合テストは確かに遅い。高速化のテクニック。

1. **並列実行**: `cargo test -- --test-threads=N`
2. **トランザクションベースのテスト**: `#[sqlx::test]`でロールバック
3. **共有データベース**: テスト間で共有し、トランザクションで分離
4. **Testcontainers**: CIでオンデマンドにDBを起動

ユニットテストとの使い分けも重要。ビジネスロジックはモックでテストし、データベース操作は統合テストでカバーする。

## まとめ

モックでは見つからないバグがある。

1. UNIQUE制約違反
2. 外部キー制約違反
3. トランザクションのロールバック
4. NULL処理
5. CASCADE DELETE
6. セーブポイント
7. CHECK制約

これらはすべて、実際のデータベースに対してテストしないと発見できない。`#[sqlx::test]`マクロやTestcontainersを使えば、統合テストの実行も容易だ。

モックとDB統合テスト、両方を使い分けるのがベストプラクティスだ。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_22_integration_testing
cargo run
```

## 参考資料

- [sqlx - Testing](https://docs.rs/sqlx/latest/sqlx/attr.test.html)
- [testcontainers-rs - GitHub](https://github.com/testcontainers/testcontainers-rs)
- [PostgreSQL - Constraints](https://www.postgresql.org/docs/current/ddl-constraints.html)
