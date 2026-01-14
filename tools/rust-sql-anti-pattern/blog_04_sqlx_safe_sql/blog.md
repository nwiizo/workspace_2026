# sqlx安全チェックリスト：本番投入前の10項目

## はじめに

コードレビューで「このクエリ、大丈夫？」と聞かれることがある。大丈夫かどうかを判断するには基準が必要だ。

sqlxはRustのSQLクライアントライブラリとして広く使われている。コンパイル時にSQLをチェックしてくれる機能があるが、それだけで安全というわけではない。NULLの扱い、インジェクション対策、エラーハンドリング、型変換……確認すべき点は多い。

本記事ではsqlxを使った本番投入前のチェックリストを10項目にまとめた。チェックリストとして使えるように、各項目には「OK例」と「NG例」を添えている。

## チェックリスト概要

| # | 項目 | 重要度 |
|---|------|--------|
| 1 | NULLの比較に`IS NULL`を使っているか | 必須 |
| 2 | SELECT *を避けて必要なカラムを明示しているか | 推奨 |
| 3 | ユーザー入力をバインドパラメータで渡しているか | 必須 |
| 4 | 動的なカラム名・テーブル名をホワイトリストで制限しているか | 必須 |
| 5 | エラーを適切に分類してハンドリングしているか | 必須 |
| 6 | トランザクションでロールバックを忘れていないか | 必須 |
| 7 | 金額計算にDecimalを使っているか | 必須 |
| 8 | タイムスタンプにタイムゾーン情報を含めているか | 推奨 |
| 9 | コネクションプールを適切に設定しているか | 推奨 |
| 10 | Option型でNULLを表現しているか | 必須 |

## □ 1. NULLの比較に`IS NULL`を使っているか

SQLでは`NULL = NULL`は`TRUE`ではなく`UNKNOWN`を返す。これはSQLの三値論理によるもので、NULLは「不明な値」を意味するからだ。

```rust
// ❌ NG: 常に0件を返す
let count: (i64,) = sqlx::query_as(
    "SELECT COUNT(*) FROM users WHERE email = NULL"
)
.fetch_one(&pool).await?;

// ✅ OK: NULLの行を正しく取得
let count: (i64,) = sqlx::query_as(
    "SELECT COUNT(*) FROM users WHERE email IS NULL"
)
.fetch_one(&pool).await?;
```

COALESCEを使うとNULLにデフォルト値を設定できる。

```rust
// NULLなら'未設定'を返す
let users: Vec<(String, String)> = sqlx::query_as(
    "SELECT name, COALESCE(email, '未設定') FROM users"
)
.fetch_all(&pool).await?;
```

### 関連チェック

- `= NULL`がコード内にないか検索
- `IS DISTINCT FROM`でNULL同士を比較していないか確認（NULL同士で`FALSE`を返す）
- ORDER BYでNULLの順序を制御する場合は`NULLS FIRST`/`NULLS LAST`を使用

## □ 2. SELECT *を避けて必要なカラムを明示しているか

`SELECT *`は手軽だが、本番環境では避けるべきだ。

```rust
// ❌ NG: スキーマ変更で壊れる可能性
let users = sqlx::query("SELECT * FROM users")
    .fetch_all(&pool).await?;

// ✅ OK: 必要なカラムを明示
#[derive(sqlx::FromRow)]
struct UserSummary {
    id: i32,
    name: String,
}

let users: Vec<UserSummary> = sqlx::query_as(
    "SELECT id, name FROM users"
)
.fetch_all(&pool).await?;
```

### 問題点

- **帯域の無駄**: 大きなTEXT列やBLOB列を不要に転送
- **脆弱性**: `password_hash`のような機密カラムを意図せず取得
- **保守性**: カラム追加で構造体とのマッピングが壊れる
- **型推論**: sqlx!マクロのコンパイル時チェックが効かない

### 例外

開発中のデバッグや、動的なスキーマを扱う管理ツールでは`SELECT *`が便利な場合もある。本番のアプリケーションコードでは避ける。

## □ 3. ユーザー入力をバインドパラメータで渡しているか

SQLインジェクションは2024年現在も OWASP Top 10 に入る脆弱性だ。

```rust
// ❌ NG: SQLインジェクション脆弱性
let query = format!(
    "SELECT * FROM users WHERE name = '{}'",
    user_input  // 攻撃者が "'; DROP TABLE users; --" を入力
);

// ✅ OK: バインドパラメータで安全
let users: Vec<User> = sqlx::query_as(
    "SELECT id, name FROM users WHERE name = $1"
)
.bind(&user_input)
.fetch_all(&pool).await?;
```

sqlxはプリペアドステートメントを使用するため、バインドパラメータを使えばインジェクションは防げる。`format!`でSQL文字列を組み立てることは原則としてしない。

### 動的な条件の組み立て

オプショナルな検索条件は、SQLの条件式で処理する。

```rust
struct SearchParams {
    name: Option<String>,
    has_email: Option<bool>,
}

let users: Vec<User> = sqlx::query_as(
    r#"
    SELECT id, name FROM users
    WHERE ($1::text IS NULL OR name = $1)
      AND ($2::boolean IS NULL OR (email IS NOT NULL) = $2)
    "#
)
.bind(&params.name)
.bind(&params.has_email)
.fetch_all(&pool).await?;
```

`$1::text IS NULL OR name = $1`というパターンは「パラメータがNULLなら条件を無視、値があれば条件を適用」という意味になる。

## □ 4. 動的なカラム名・テーブル名をホワイトリストで制限しているか

バインドパラメータはカラム名やテーブル名には使えない。動的に指定する場合はホワイトリストで制限する。

```rust
// ❌ NG: カラム名を直接埋め込み
let sort_column = user_input;  // 攻撃者が "id; DROP TABLE users" を入力
let query = format!("SELECT * FROM users ORDER BY {}", sort_column);

// ✅ OK: enumでホワイトリスト化
enum SortColumn {
    Name,
    CreatedAt,
    Email,
}

impl SortColumn {
    fn as_str(&self) -> &'static str {
        match self {
            SortColumn::Name => "name",
            SortColumn::CreatedAt => "created_at",
            SortColumn::Email => "email",
        }
    }
}

let sort_column = match user_input.as_str() {
    "name" => SortColumn::Name,
    "created_at" => SortColumn::CreatedAt,
    "email" => SortColumn::Email,
    _ => return Err(AppError::InvalidSortColumn),
};

let query = format!(
    "SELECT id, name FROM users ORDER BY {}",
    sort_column.as_str()
);
```

enumを使うことで、許可されたカラム名以外は指定できなくなる。

### QueryBuilderを使う方法

sqlxの`QueryBuilder`を使うと、動的なIN句やバルクインサートを安全に構築できる。

```rust
use sqlx::QueryBuilder;

// 動的なIN句
let ids = vec![1, 2, 3];
let mut builder = QueryBuilder::new("SELECT * FROM users WHERE id IN (");
let mut separated = builder.separated(", ");
for id in &ids {
    separated.push_bind(*id);
}
builder.push(")");

let users = builder.build_query_as::<User>()
    .fetch_all(&pool).await?;
```

## □ 5. エラーを適切に分類してハンドリングしているか

sqlxのエラーは種類によって対処が異なる。

```rust
// ❌ NG: 全てのエラーを同じように扱う
let result = sqlx::query("INSERT INTO users ...").execute(&pool).await;
if result.is_err() {
    return Err(anyhow!("database error"));
}

// ✅ OK: エラーの種類に応じた処理
match result {
    Ok(_) => Ok(()),
    Err(sqlx::Error::Database(db_err)) => {
        if db_err.is_unique_violation() {
            Err(AppError::DuplicateEmail)
        } else if db_err.is_foreign_key_violation() {
            Err(AppError::InvalidReference)
        } else {
            Err(AppError::Database(db_err.to_string()))
        }
    }
    Err(sqlx::Error::RowNotFound) => Err(AppError::NotFound),
    Err(e) => Err(AppError::Database(e.to_string())),
}
```

### エラー分類表

| エラー種別 | 対処方法 | リトライ |
|-----------|---------|---------|
| `RowNotFound` | 404を返す | 不要 |
| `UniqueViolation` | 409 Conflictを返す | 不要 |
| `ForeignKeyViolation` | 400 Bad Requestを返す | 不要 |
| `PoolTimedOut` | 503を返すかリトライ | 可能 |
| `Io` | 503を返すかリトライ | 可能 |

### カスタムエラー型

`thiserror`を使ってアプリケーション固有のエラー型を定義する。

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("User not found")]
    NotFound,
    #[error("Email already registered")]
    DuplicateEmail,
    #[error("Database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::DuplicateEmail
            }
            e => AppError::Database(e.to_string()),
        }
    }
}
```

## □ 6. トランザクションでロールバックを忘れていないか

複数の操作を一括で行う場合、途中で失敗したらロールバックが必要だ。

```rust
// ❌ NG: ロールバックを忘れている
let mut tx = pool.begin().await?;
sqlx::query("INSERT INTO orders ...").execute(&mut *tx).await?;
sqlx::query("UPDATE inventory ...").execute(&mut *tx).await?;  // ここで失敗
tx.commit().await?;  // 到達しない、txはdropされるがコミットもロールバックもされない

// ✅ OK: 明示的なロールバック
let mut tx = pool.begin().await?;

let result: Result<(), sqlx::Error> = async {
    sqlx::query("INSERT INTO orders ...").execute(&mut *tx).await?;
    sqlx::query("UPDATE inventory ...").execute(&mut *tx).await?;
    Ok(())
}.await;

match result {
    Ok(_) => {
        tx.commit().await?;
    }
    Err(e) => {
        tx.rollback().await?;
        return Err(e.into());
    }
}
```

sqlxのTransactionはDropされると自動でロールバックされる仕様だが、明示的に呼び出す方が意図が明確になる。

### トランザクション分離レベル

デフォルトはREAD COMMITTEDだが、用途によっては変更が必要。

```rust
use sqlx::postgres::PgPool;

let mut tx = pool.begin().await?;
sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
    .execute(&mut *tx).await?;
// 以降の操作...
```

## □ 7. 金額計算にDecimalを使っているか

浮動小数点数は金額計算に適さない。

```rust
// ❌ NG: FLOATは誤差が生じる
let price: f64 = 19.99;
let tax = price * 0.10;  // 1.9989999999999999...

// ✅ OK: Decimalで正確に計算
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

let price = dec!(19.99);
let tax = price * dec!(0.10);  // 正確に1.999
let rounded = tax.round_dp(2);  // 2.00

// PostgreSQLのDECIMAL型と対応
let product: (Decimal,) = sqlx::query_as(
    "SELECT price FROM products WHERE id = $1"
)
.bind(product_id)
.fetch_one(&pool).await?;
```

### 型マッピング

```
PostgreSQL: DECIMAL(10, 2), NUMERIC
Rust: rust_decimal::Decimal
```

`rust_decimal`クレートを使い、sqlxのfeatureフラグで有効にする。

```toml
[dependencies]
sqlx = { version = "0.8", features = ["postgres", "rust_decimal"] }
rust_decimal = "1"
```

## □ 8. タイムスタンプにタイムゾーン情報を含めているか

タイムゾーンなしのタイムスタンプは、アプリケーションとDBでの解釈がずれる原因になる。

```sql
-- ❌ NG: タイムゾーンなし
created_at TIMESTAMP DEFAULT NOW()

-- ✅ OK: タイムゾーンあり
created_at TIMESTAMPTZ DEFAULT NOW()
```

```rust
use chrono::{DateTime, Utc};

// TIMESTAMPTZ → DateTime<Utc>
let created_at: (DateTime<Utc>,) = sqlx::query_as(
    "SELECT created_at FROM users WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;

// フォーマット
println!("{}", created_at.0.format("%Y-%m-%d %H:%M:%S %Z"));
```

### 型マッピング

```
PostgreSQL: TIMESTAMPTZ
Rust: chrono::DateTime<Utc>
```

chronoクレートを使い、sqlxのfeatureフラグで有効にする。

```toml
[dependencies]
sqlx = { version = "0.8", features = ["postgres", "chrono"] }
chrono = "0.4"
```

## □ 9. コネクションプールを適切に設定しているか

デフォルト設定のままだとコネクション枯渇が起きることがある。

```rust
// ❌ NG: デフォルト設定
let pool = PgPoolOptions::new()
    .connect(DATABASE_URL).await?;

// ✅ OK: 本番環境向け設定
use std::time::Duration;

let pool = PgPoolOptions::new()
    .max_connections(20)                    // 最大接続数
    .min_connections(5)                      // 最小接続数
    .acquire_timeout(Duration::from_secs(3))  // 取得タイムアウト
    .idle_timeout(Duration::from_secs(600))   // アイドルタイムアウト
    .max_lifetime(Duration::from_secs(1800))  // 最大生存時間
    .test_before_acquire(true)               // 取得前に接続確認
    .connect(DATABASE_URL).await?;
```

### 設定指針

| 設定項目 | 開発環境 | 本番環境 |
|---------|---------|---------|
| max_connections | 5 | (max_connections - 余裕) / インスタンス数 |
| min_connections | 0 | 2-5 |
| acquire_timeout | 30秒 | 3-5秒 |
| test_before_acquire | false | true |

PostgreSQLのmax_connectionsが100で、アプリケーションインスタンスが4つなら、各インスタンスは最大20-25接続程度に抑える。

## □ 10. Option型でNULLを表現しているか

Nullable列はRustの`Option<T>`にマッピングする。

```rust
// ❌ NG: NULLableな列をTで受ける
struct User {
    email: String,  // email列がNULLならパニック
}

// ✅ OK: Option<T>で受ける
struct User {
    email: Option<String>,
}

let user: User = sqlx::query_as(
    "SELECT name, email FROM users WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;

// NULLチェック
match user.email {
    Some(email) => println!("Email: {}", email),
    None => println!("Email not set"),
}
```

### 便利なメソッド

```rust
// デフォルト値を使用
let email = user.email.unwrap_or_else(|| "unknown@example.com".to_string());

// 変換を適用
let domain = user.email.map(|e| e.split('@').last().unwrap_or(""));

// 条件に応じた処理
if let Some(email) = user.email {
    send_notification(&email);
}
```

## まとめチェックリスト

本番投入前に以下を確認する。

```
□ 1. WHERE句で = NULL を使っていない
□ 2. SELECT * ではなく必要なカラムを明示している
□ 3. ユーザー入力は全て .bind() でバインドしている
□ 4. 動的なカラム名はenumでホワイトリスト化している
□ 5. sqlx::Errorをパターンマッチで分類している
□ 6. トランザクションでrollback()を呼んでいる
□ 7. 金額計算にrust_decimal::Decimalを使っている
□ 8. タイムスタンプはTIMESTAMPTZ + DateTime<Utc>
□ 9. コネクションプールのタイムアウトを設定している
□ 10. Nullable列はOption<T>で表現している
```

この10項目を守れば、sqlxを使った本番アプリケーションで遭遇する大半の問題は回避できる。特に1, 3, 4, 5は必須だ。セキュリティに関わる項目は妥協しない。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_04_sqlx_safe_sql
cargo run
```

## 参考資料

- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
- [rust_decimal - docs.rs](https://docs.rs/rust_decimal/latest/rust_decimal/)
- [chrono - docs.rs](https://docs.rs/chrono/latest/chrono/)
- [OWASP - SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
