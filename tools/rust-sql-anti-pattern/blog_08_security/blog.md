# 平文パスワードがSlackに流れた日：認証設計の教訓

## 発端

「ログにパスワードが出てるんですけど」

新人エンジニアからのSlackメッセージだった。確認すると、デバッグログにユーザーの入力がそのまま出力されていた。パスワードも。

ログはCloudWatchに送られ、アラート通知でSlackにも転送されていた。つまり、ユーザーのパスワードがSlackのチャンネルに流れていた。

すぐにログを削除し、該当ユーザーにパスワード変更を依頼した。幸い、社内の限定チャンネルだったため外部流出は免れた。ただ、これをきっかけにセキュリティ設計を全面的に見直すことになった。

## 教訓1：パスワードは平文で保存しない

DBに平文パスワードを保存しているシステムは、今でも存在する。

```sql
-- ❌ アンチパターン
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    password VARCHAR(255) NOT NULL  -- 平文！
);
```

DBが漏洩したら全ユーザーのパスワードが露出する。他サービスで同じパスワードを使い回していたら、被害は連鎖する。

### Argon2でハッシュ化

```rust
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Hashing error: {}", e))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

Argon2はパスワードハッシュ化のための専用アルゴリズムだ。SHA-256やMD5と違い、意図的に遅く設計されている。ブルートフォース攻撃を困難にするためだ。

```rust
// 登録時
let password_hash = hash_password("SecureP@ssw0rd123")?;
sqlx::query(
    "INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3)"
)
.bind(email)
.bind(&password_hash)
.bind(name)
.execute(&pool).await?;

// 認証時
let user: UserCredentials = sqlx::query_as(
    "SELECT id, password_hash FROM users WHERE email = $1"
)
.bind(email)
.fetch_one(&pool).await?;

if verify_password(input_password, &user.password_hash)? {
    // 認証成功
}
```

同じパスワードでもハッシュは毎回異なる（ソルトが異なるため）。レインボーテーブル攻撃を防げる。

## 教訓2：機密データをログに出力しない

冒頭の事故は、デバッグログが原因だった。

```rust
// ❌ アンチパターン
log::debug!("Login attempt: email={}, password={}", email, password);

// ✅ 正しい方法
log::debug!("Login attempt: email={}", email);  // パスワードは出力しない
```

構造体を丸ごとログに出すのも危険だ。

```rust
#[derive(Debug)]
struct LoginRequest {
    email: String,
    password: String,  // Debugで出力される
}

// ❌ 危険
log::debug!("{:?}", request);

// ✅ 安全な構造体設計
#[derive(Debug)]
struct LoginRequest {
    email: String,
    #[allow(dead_code)]
    password: SecretString,  // Debugで[REDACTED]と表示される
}
```

`secrecy`クレートの`SecretString`を使うと、Debug出力時に値が隠される。

## 教訓3：APIレスポンスに機密データを含めない

ユーザー情報を返すAPIで、全カラムを返していないか。

```rust
// ❌ アンチパターン: 全カラムを返す
let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = $1")
    .bind(user_id)
    .fetch_one(&pool).await?;
// password_hashが含まれている！

// ✅ 正しい方法: 必要なカラムだけ
#[derive(Debug, sqlx::FromRow)]
struct PublicUser {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    // password_hashは含まない
}

let user: PublicUser = sqlx::query_as(
    "SELECT id, name, created_at FROM users WHERE id = $1"
)
.bind(user_id)
.fetch_one(&pool).await?;
```

内部用と公開用で別の構造体を使い分ける。

## 教訓4：Row Level Securityでデータを分離

マルチテナントアプリケーションでは、テナント間のデータ分離が必須だ。アプリケーションコードでWHERE句を付け忘れると、他テナントのデータが見えてしまう。

PostgreSQLのRow Level Security（RLS）を使うと、データベースレベルで強制できる。

```sql
-- RLSを有効化
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;

-- ポリシーを作成
CREATE POLICY posts_owner_policy ON posts
    FOR ALL
    USING (user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::uuid);
```

```rust
// リクエストごとにユーザーIDを設定
let mut tx = pool.begin().await?;
sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_id))
    .execute(&mut *tx).await?;

// WHERE句なしでも自分のデータしか見えない
let posts: Vec<Post> = sqlx::query_as(
    "SELECT id, title, content FROM posts"  // WHERE user_id = ... は不要
)
.fetch_all(&mut *tx).await?;
```

`current_setting`が空の場合は何も見えない。設定し忘れても安全だ。

## 教訓5：SQLインジェクションを防ぐ

sqlxはプリペアドステートメントを使うため、基本的にインジェクションは防げる。

```rust
// ✅ 安全: プレースホルダー使用
let users: Vec<User> = sqlx::query_as(
    "SELECT id, name FROM users WHERE email = $1"
)
.bind(email)  // 自動でエスケープ
.fetch_all(&pool).await?;
```

危険なのは、カラム名やテーブル名を動的に組み立てる場合だ。

```rust
// ❌ 危険: ユーザー入力をそのまま埋め込み
let query = format!(
    "SELECT * FROM users ORDER BY {}",
    user_input  // 攻撃者が "id; DROP TABLE users" を入力
);

// ✅ 安全: ホワイトリストで制限
enum SortColumn { Name, CreatedAt, Email }

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
    _ => return Err(anyhow!("Invalid sort column")),
};

let query = format!(
    "SELECT * FROM users ORDER BY {}",
    sort_column.as_str()  // 許可された値のみ
);
```

enumでホワイトリスト化すれば、許可されていない値は指定できない。

## 教訓6：パスワードポリシーを強制する

弱いパスワードを許可すると、ブルートフォース攻撃のリスクが高まる。

```rust
struct PasswordPolicy {
    min_length: usize,
    require_uppercase: bool,
    require_lowercase: bool,
    require_digit: bool,
}

fn validate_password(password: &str, policy: &PasswordPolicy) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if password.len() < policy.min_length {
        errors.push(format!("{}文字以上必要です", policy.min_length));
    }
    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        errors.push("大文字を含めてください".to_string());
    }
    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        errors.push("小文字を含めてください".to_string());
    }
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("数字を含めてください".to_string());
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

## チェックリスト

認証・セキュリティ設計のチェックリストをまとめる。

```
□ パスワードはArgon2でハッシュ化している
□ 平文パスワードはログに出力しない
□ APIレスポンスにpassword_hashを含めない
□ SQLはプレースホルダーを使っている
□ 動的なカラム名はホワイトリストで制限している
□ マルチテナントではRLSを使っている
□ パスワードポリシーを強制している
```

## 結論

冒頭の「平文パスワードがSlackに流れた」事故は、デバッグログの設定ミスが原因だった。ただ、根本的な問題はセキュリティを後回しにしていたことだ。

セキュリティは機能追加ではなく、設計の前提だ。後から追加するのは難しい。最初から組み込む。

1. **パスワードはハッシュ化**: Argon2を使う
2. **機密データはログに出さない**: `secrecy`クレートを使う
3. **APIは必要なデータだけ返す**: 公開用の構造体を分ける
4. **RLSでデータを分離**: アプリケーションコードのミスを防ぐ
5. **SQLインジェクションを防ぐ**: プレースホルダー、ホワイトリスト

これらは最低限だ。本格的なセキュリティ対策はもっと広範囲にわたる。ただ、この最低限を守るだけでも、多くの事故は防げる。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_08_security
cargo run
```

## 参考資料

- [OWASP - Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [PostgreSQL - Row Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [argon2 - docs.rs](https://docs.rs/argon2/latest/argon2/)
