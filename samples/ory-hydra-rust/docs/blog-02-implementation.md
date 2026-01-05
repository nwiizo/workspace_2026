# RustでOry Hydra用認証プロバイダーを実装する ― 「できない」を確認するテストの話

## 認証システムを書くときの独特の緊張感

認証システムを実装するのは、なんとも言えない緊張感がある。普通の機能開発とは違う。

ECサイトの商品一覧画面でバグがあれば「表示がおかしい」で済む。でも認証システムでバグがあると「本来見えてはいけないものが見える」になる。ログイン画面は単なる入り口ではなく、システム全体を守る城門だ。城門の鍵が壊れていたら、いくら城壁を高くしても意味がない。

前回の記事ではOry Hydraのアーキテクチャを解説した。今回は実際にRustでLogin/Consent Providerを実装する。そして認証システムならではのテストの考え方について話したい。

[https://github.com/ory/hydra:embed:cite]

[https://www.ory.sh/docs/hydra/:embed:cite]

## 始める前に：なぜAxumを選んだか

Webフレームワークの選定で少し悩んだ。Rustには複数の選択肢がある。

actix-webは成熟していて高速だ。でもActorモデルを前提としたAPIは、Hydraとの単純なHTTP連携には少し大げさに感じた。warpは関数型スタイルで美しいが、エラーハンドリングが独特で慣れが必要だ。

Axumを選んだのは、Towerエコシステムとの統合が自然だったから。ミドルウェアの組み合わせが直感的で、Extractorパターンによる型安全なリクエスト処理が認証システムと相性がいい。何より、Tokioチームが開発しているという安心感がある。

[https://github.com/tokio-rs/axum:embed:cite]

## プロジェクト構成：テストしやすさを意識した設計

```
src/
├── main.rs
├── config.rs
├── error.rs
├── state.rs
├── handlers/
│   ├── mod.rs
│   ├── login.rs
│   ├── consent.rs
│   └── logout.rs
├── services/
│   ├── mod.rs
│   ├── hydra.rs
│   └── auth.rs
└── models/
    ├── mod.rs
    └── hydra.rs
```

この構成には意図がある。ハンドラー層とサービス層を分離しているのは、単なるお作法ではない。

ハンドラーはHTTPリクエストの受け取りとレスポンスの返却だけを担う。実際の認証ロジックはサービス層に置く。こうすることで、サービス層は純粋なRustコードとしてテストできる。Axumのテスト用ヘルパーを使わなくても、ビジネスロジックの検証ができる。

認証システムでは特にテストが重要だ。だからテストしやすい構造を最初から意識した。

## エラー型の設計：最初に決めておくべきこと

認証システムでエラーハンドリングを後回しにすると痛い目を見る。経験上、これは最初に設計すべきだ。

[https://github.com/dtolnay/thiserror:embed:cite]

```rust
use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Hydra API error: {0}")]
    HydraError(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}
```

`thiserror`を使うとエラー型の定義が簡潔になる。でも本当に重要なのはその先、Axumのレスポンスへの変換だ。

```rust
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
    error_code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, error, description) = match &self {
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "AUTH_002",
                "invalid_credentials",
                "The provided credentials are invalid".to_string(),
            ),
            // 他のエラーも同様にマッピング
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            error_description: description,
            error_code: error_code.to_string(),
        });

        (status, body).into_response()
    }
}
```

OAuth2仕様に沿ったエラーレスポンス形式を採用している。`error`と`error_description`というフィールド名はRFC 6749で定義されている。独自のエラー形式を発明したくなる誘惑があるが、標準に従っておくとクライアント側の実装が楽になる。

[https://datatracker.ietf.org/doc/html/rfc6749:embed:cite]

## 認証サービス：見落としがちな細部

パスワード認証の実装に入る。OWASPのガイドラインに従い、Argon2idを採用した。

[https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html:embed:cite]

[https://docs.rs/argon2/latest/argon2/:embed:cite]

```rust
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

pub async fn register(&self, email: &str, password: &str) -> Result<User, AppError> {
    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest("Email and password are required".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    // ユーザー作成処理...
}
```

ここで`Argon2::default()`を使っているが、これはOWASPの推奨設定がデフォルトになっているから意図的だ。メモリ19MiB、イテレーション2回、並列度1。カスタマイズしたくなるかもしれないが、暗号の専門家でなければデフォルトを信頼した方がいい。

認証部分で見落としがちなのが次の点だ。

```rust
pub async fn authenticate(&self, email: &str, password: &str) -> Result<User, AppError> {
    if password.is_empty() {
        return Err(AppError::InvalidCredentials);
    }

    let users = self.users.read().await;
    let user = users.get(email).ok_or(AppError::InvalidCredentials)?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    Ok(user.clone())
}
```

ユーザーが存在しない場合も、パスワードが間違っている場合も、返すエラーは同じ`InvalidCredentials`だ。「ユーザーが見つかりません」というエラーを返したくなるが、それは攻撃者に情報を与えてしまう。

これはユーザー列挙攻撃（User Enumeration Attack）への対策だ。攻撃者はまず有効なメールアドレスを特定しようとする。エラーメッセージが違えば、登録済みかどうかが分かってしまう。

## Login Handler：Hydraとの連携

Login Providerのエンドポイントを実装する。

```rust
#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    login_challenge: String,
}

pub async fn login_page(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
) -> Result<Html<String>, AppError> {
    let challenge = &query.login_challenge;
    let login_request = state.hydra.get_login_request(challenge).await?;

    // skipフラグが立っている場合は既にセッションがある
    if login_request.skip {
        let completed = state.hydra
            .accept_login(challenge, &login_request.subject, false)
            .await?;
        // ログイン画面をスキップしてリダイレクト
    }

    // ログインフォームを表示
    let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head><title>Login</title></head>
        <body>
            <h1>Login</h1>
            <form method="post" action="/login">
                <input type="hidden" name="login_challenge" value="{}" />
                <label>Email: <input type="email" name="email" required /></label>
                <label>Password: <input type="password" name="password" required /></label>
                <button type="submit">Login</button>
            </form>
        </body>
        </html>
    "#, challenge);

    Ok(Html(html))
}
```

`login_challenge`という概念が重要だ。これはHydraが発行する一時的なトークンで、OAuth2フロー全体を紐づける役割を持つ。このchallengeを検証することで、正規のフローからのリクエストであることを確認できる。

フォーム送信時の処理も見てみよう。

```rust
pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    // 認証処理
    let user = state.auth.authenticate(&form.email, &form.password).await?;

    // Hydraに認証成功を通知
    let completed = state.hydra
        .accept_login(&form.login_challenge, &user.id.to_string(), false)
        .await?;

    // Hydraが指示するURLにリダイレクト
    Ok(Redirect::to(&completed.redirect_to))
}
```

認証成功後、Hydraに`accept_login`を送る。するとHydraは次のステップ（Consent画面）へのリダイレクトURLを返す。このURL生成をHydraに任せているのがポイントだ。OAuth2のstate検証やPKCEなど、複雑なセキュリティ処理はHydraが担当する。

## テスト設計：「できないこと」を確認する発想

ここからがこの記事で一番伝えたいことだ。

認証システムのテストで「正常系が動く」だけ確認しても意味がない。むしろ「本来できないことが、ちゃんとできない」ことを確認する方が重要だ。

[https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html:embed:cite]

普通の機能開発では「この操作をしたらこうなる」というテストを書く。でも認証システムでは「この操作をしてもこうならない」というテストの方が価値がある。

### 正常系：まずは基本から

```rust
#[tokio::test]
async fn test_register_and_authenticate() {
    let service = AuthService::new();
    let email = "test@example.com";
    let password = "secure_password123";

    let user = service.register(email, password).await
        .expect("Registration should succeed");
    assert_eq!(user.email, email);

    let authenticated = service.authenticate(email, password).await
        .expect("Authentication should succeed");
    assert_eq!(authenticated.id, user.id);
}
```

これは当然必要だ。でもこれだけでは不十分。

### 異常系：できないことの確認

```rust
#[tokio::test]
async fn test_cannot_authenticate_with_wrong_password() {
    let service = AuthService::new();
    service.register("user@example.com", "correct_password").await.unwrap();

    let result = service.authenticate("user@example.com", "wrong_password").await;
    assert!(result.is_err());
}
```

間違ったパスワードでログインできないこと。当たり前だと思うだろう。でもこのテストがなければ、誰かが誤って認証ロジックを壊しても気づけない。

```rust
#[tokio::test]
async fn test_cannot_authenticate_with_empty_password() {
    let service = AuthService::new();
    service.register("user@example.com", "valid_password").await.unwrap();

    let result = service.authenticate("user@example.com", "").await;
    assert!(result.is_err());
}
```

空文字でログインできないこと。これも当たり前だが、テストがないとバグが潜む可能性がある。特に「空文字チェックを追加しよう」と思って実装したなら、そのチェックが機能していることを確認すべきだ。

### セキュリティテスト：攻撃者の視点で考える

```rust
#[tokio::test]
async fn test_login_does_not_reveal_user_existence() {
    let service = AuthService::new();
    service.register("exists@example.com", "password").await.unwrap();

    let err1 = service.authenticate("exists@example.com", "wrong").await.unwrap_err();
    let err2 = service.authenticate("nobody@example.com", "password").await.unwrap_err();

    // エラーメッセージが同じであることを確認
    assert_eq!(err1.to_string(), err2.to_string());
}
```

これが前述のユーザー列挙攻撃への対策テストだ。攻撃者は「このメールアドレスは登録されているか？」を知りたがる。もしエラーメッセージが違えば、その情報を得られてしまう。

このテストは「エラーメッセージが同じ」という実装の意図を明示化している。将来誰かが「親切なエラーメッセージにしよう」と思って変更しても、このテストが警告を出す。

### 並行処理テスト：競合状態を検出する

```rust
#[tokio::test]
async fn test_concurrent_registration_same_email() {
    let service = AuthService::new();
    let email = "race@example.com";

    let service1 = service.clone();
    let service2 = service.clone();
    let email1 = email.to_string();
    let email2 = email.to_string();

    let handle1 = tokio::spawn(async move {
        service1.register(&email1, "password1").await
    });
    let handle2 = tokio::spawn(async move {
        service2.register(&email2, "password2").await
    });

    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();

    let success_count = [result1.is_ok(), result2.is_ok()]
        .iter().filter(|&&x| x).count();

    assert_eq!(success_count, 1, "Exactly one registration should succeed");
}
```

同時に同じメールアドレスで2つの登録リクエストが来たらどうなるか。両方成功してしまうと、同じメールアドレスに2つのアカウントができてしまう。

このテストは競合状態（Race Condition）が適切に処理されていることを確認している。`RwLock`を使った実装なら片方だけが成功するはずだ。

### エッジケース：特殊文字とUnicode

```rust
#[tokio::test]
async fn test_special_characters_in_password() {
    let service = AuthService::new();
    let password = r#"p@$$w0rd!@#$%^&*()_+-=[]{}|;':",.<>?/`~"#;

    service.register("special@example.com", password).await.unwrap();
    let result = service.authenticate("special@example.com", password).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unicode_password() {
    let service = AuthService::new();
    let password = "パスワード123🔐";

    service.register("unicode@example.com", password).await.unwrap();
    let result = service.authenticate("unicode@example.com", password).await;
    assert!(result.is_ok());
}
```

現実世界のユーザーは様々な文字を使う。特殊記号を含むパスワードや、日本語・絵文字を含むパスワードが正しく処理されることを確認しておく。

[https://doc.rust-lang.org/book/ch11-00-testing.html:embed:cite]

## E2Eテスト：実際に動かして確認する

ユニットテストを書き終えたら、次は実際にシステム全体を動かして確認したい。Docker Composeで環境を立ち上げ、OAuth2認可コードフローが最後まで通ることを確認する。

```bash
# 環境起動
docker compose up -d --build

# ヘルスチェック
curl http://localhost:3000/health
# => {"status":"healthy","version":"0.1.0"}

curl http://localhost:4444/health/ready
# => {"status":"ok"}
```

両サービスが起動したら、テストユーザーとOAuth2クライアントを作成する。

```bash
# ユーザー登録
curl -X POST http://localhost:3000/api/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email": "test@example.com", "password": "password123"}'
# => {"id":"90c99303-d082-4b3e-b658-a4e5333a2d4f","email":"test@example.com",...}

# OAuth2クライアント作成
curl -X POST http://localhost:4445/admin/clients \
  -H 'Content-Type: application/json' \
  -d '{
    "client_id": "test-client",
    "client_secret": "test-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile email",
    "redirect_uris": ["http://localhost:8080/callback"]
  }'
```

ここからがOAuth2フローだ。実際にはブラウザで操作するが、curlでも確認できる。

```bash
# 1. 認可エンドポイントにアクセス → Hydraがログインページにリダイレクト
curl -w "%{redirect_url}" -o /dev/null \
  'http://localhost:4444/oauth2/auth?client_id=test-client&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=test-state'
# => http://localhost:3000/login?login_challenge=xxxxx

# 2. ログインフォーム送信 → Consent画面にリダイレクト
# 3. Consent承認 → 認可コード取得
# => http://localhost:8080/callback?code=ory_ac_xxxxx&state=test-state
```

認可コードを取得したら、トークンエンドポイントでアクセストークンに交換する。

```bash
curl -X POST http://localhost:4444/oauth2/token \
  -u "test-client:test-secret" \
  -d "grant_type=authorization_code&code=ory_ac_xxxxx&redirect_uri=http://localhost:8080/callback"
```

レスポンスにはアクセストークンとIDトークンが含まれる。IDトークンをデコードすると、ユーザー情報が確認できる。

```json
{
  "aud": ["test-client"],
  "email": "test@example.com",
  "email_verified": false,
  "role": "customer",
  "sub": "90c99303-d082-4b3e-b658-a4e5333a2d4f"
}
```

`email`、`role`、`sub`（ユーザーID）が正しく含まれている。これでLogin Provider → Consent Provider → トークン発行まで、OAuth2認可コードフロー全体が動作することを確認できた。

ユニットテストでは個々のコンポーネントが正しく動くことを確認した。E2Eテストでは、それらが組み合わさって全体として機能することを確認する。両方あって初めて、システムに対する信頼が得られる。

## テストを書くことで得られるもの

58個のテストを書き終えた時、ある種の安心感があった。

正常系のテストは「作ったものが動く」ことを確認する。異常系のテストは「作ったものが壊れにくい」ことを確認する。そしてセキュリティテストは「作ったものが悪用されにくい」ことを確認する。

認証システムでは特に後者が重要だ。攻撃者は正常な使い方をしない。だから「正常に使えば動く」だけでは不十分なのだ。

## まとめ

RustでHydra用のLogin Providerを実装した。振り返ると、コードを書くこと自体より、「何をテストすべきか」を考える時間の方が長かったかもしれない。

技術的なポイントをまとめると：

- **エラーハンドリング**: 最初に設計する。`thiserror`とAxumの`IntoResponse`で型安全に
- **認証サービス**: Argon2idのデフォルト設定を信頼する。ユーザー列挙攻撃に注意
- **テスト設計**: 「できないこと」のテストが認証システムでは特に重要
- **セキュリティテスト**: 攻撃者の視点でテストケースを考える

次回は、このバックエンドに対応するフロントエンドの実装について書く予定だ。認証UIのUXとセキュリティのバランスは、また別の難しさがある。

## 参考資料

### Ory Hydra

- [Ory Hydra GitHub](https://github.com/ory/hydra)
- [Ory Hydra Documentation](https://www.ory.sh/docs/hydra/)
- [Login and Consent Flow](https://www.ory.sh/docs/hydra/concepts/login)

### Rust Crates

- [Axum - Web Framework](https://github.com/tokio-rs/axum)
- [thiserror - Error Handling](https://github.com/dtolnay/thiserror)
- [argon2 - Password Hashing](https://docs.rs/argon2/)
- [jsonwebtoken - JWT](https://docs.rs/jsonwebtoken/)

### 仕様・ガイドライン

- [RFC 6749 - OAuth 2.0](https://datatracker.ietf.org/doc/html/rfc6749)
- [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
