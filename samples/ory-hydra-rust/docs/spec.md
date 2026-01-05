# Rust 認証基盤構築 仕様書

## 1. 概要

本仕様書は、Rustを用いて認証基盤（Identity Provider: IdP）を構築する際の技術選定、アーキテクチャ設計、および実装に必要な要素をまとめたものです。

Ory Hydra を採用し、「認証処理は既存システムに委譲、認可プロトコルは Hydra に任せる」というアーキテクチャを採用しています。

### 1.1 実装状況

本プロジェクトでは、以下の機能を実装済みです：

| 機能 | 状態 | 説明 |
|------|------|------|
| Login Provider | 実装済み | ユーザー認証画面とHydra連携 |
| Consent Provider | 実装済み | スコープ同意画面とHydra連携 |
| Logout Provider | 実装済み | ログアウト処理とセッション破棄 |
| パスワード認証 | 実装済み | Argon2idによるハッシュ化 |
| JWT発行 | 実装済み | アクセストークン・リフレッシュトークン |
| Docker環境 | 実装済み | Docker Compose による開発環境 |
| ユニットテスト | 実装済み | 正常系・異常系35テスト |

---

## 2. アーキテクチャ設計方針

### 2.1 設計の選択肢

| アプローチ                     | 概要                                | メリット                         | デメリット                         |
| ------------------------------ | ----------------------------------- | -------------------------------- | ---------------------------------- |
| **フルスクラッチ実装**         | OAuth2/OIDCを全て自前実装           | 完全なカスタマイズ性             | 実装コスト大、セキュリティリスク   |
| **OSS認可サーバー + 自前認証** | Ory Hydra等を認可に使用、認証は自前 | プロトコル準拠が担保、認証は自由 | 運用する外部コンポーネントが増える |
| **ライブラリベース実装**       | Rust crateを活用して実装            | Rustの型安全性を活用可能         | 組み合わせの検証が必要             |

### 2.2 Ory Hydra を利用したアーキテクチャ（推奨）

既存システムに OAuth2.0/OIDC 対応を追加する場合、**Ory Hydra** の採用が有効です。

#### Ory Hydra の特徴

Ory Hydra はそれ自体が IdP としての機能を持たない点が特徴的です。

```
┌─────────────────────────────────────────────────────────────────┐
│                         Ory Hydra の役割分担                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   Ory Hydra が担当するもの:                                        │
│   ├── OAuth2.0 / OIDC プロトコル処理                               │
│   ├── クライアント管理                                             │
│   ├── トークン発行・検証                                           │
│   ├── 各種チャレンジの生成                                         │
│   └── state / PKCE の検証                                        │
│                                                                   │
│   自前で実装するもの:                                              │
│   ├── 認証処理（ログイン）                                         │
│   ├── 認可への同意（Consent）                                      │
│   ├── ユーザー・アカウント管理                                      │
│   └── Scope 体系の設計                                            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

#### Hydra 採用のメリット

1. **既存認証処理の活用**: ID管理の主体を変更せず、最小限の変更で標準仕様対応が可能
2. **プロトコル準拠**: OpenID Connect Certification 取得済み
3. **将来の拡張性**: 標準仕様なので、1st party / 3rd party 問わず差し替えや移行が容易

#### 実装が必要なエンドポイント

```
Hydra との連携に必要なエンドポイント:

1. Login Provider
   GET  /login          - 認証画面表示
   POST /login          - 認証処理実行
   → hydra に login_verifier を返却

2. Consent Provider
   GET  /consent        - 同意画面表示
   POST /consent        - 同意処理実行
   → hydra に consent_verifier を返却

3. Logout Provider（実装済み）
   GET  /logout         - ログアウト処理
   → hydra に logout_verifier を返却

4. 認証 API（実装済み）
   POST /api/auth/register - ユーザー登録
   POST /api/auth/login    - JWT発行
   POST /api/auth/refresh  - トークンリフレッシュ
```

#### シーケンス（概要）

```
┌────────┐     ┌────────────┐     ┌──────────────┐     ┌────────────────┐
│ Client │     │ Ory Hydra  │     │ Login/Consent│     │ 既存認証基盤   │
│        │     │ (認可サーバー)│     │ Provider     │     │ (Rust実装)     │
└────┬───┘     └─────┬──────┘     └──────┬───────┘     └───────┬────────┘
     │               │                   │                     │
     │ GET /oauth2/auth                  │                     │
     │──────────────>│                   │                     │
     │               │                   │                     │
     │               │ Redirect /login   │                     │
     │               │ ?login_challenge  │                     │
     │               │──────────────────>│                     │
     │               │                   │                     │
     │               │                   │ 認証処理委譲         │
     │               │                   │────────────────────>│
     │               │                   │                     │
     │               │                   │ 認証結果             │
     │               │                   │<────────────────────│
     │               │                   │                     │
     │               │ Accept Login      │                     │
     │               │ (login_verifier)  │                     │
     │               │<──────────────────│                     │
     │               │                   │                     │
     │               │ Redirect /consent │                     │
     │               │ ?consent_challenge│                     │
     │               │──────────────────>│                     │
     │               │                   │                     │
     │               │ Accept Consent    │                     │
     │               │ (consent_verifier)│                     │
     │               │<──────────────────│                     │
     │               │                   │                     │
     │ Redirect with │                   │                     │
     │ Authorization │                   │                     │
     │ Code          │                   │                     │
     │<──────────────│                   │                     │
```

### 2.3 マルチテナント環境での考慮事項

マルチテナント環境における認可サーバー開発では、以下の点を考慮する必要があります。

| 考慮事項                   | 説明                                                               |
| -------------------------- | ------------------------------------------------------------------ |
| **会社・従業員の概念**     | 個人向けと異なり、会社単位でのテナント管理が必要                   |
| **連携担当の権限チェック** | 外部連携を行う担当者が管理者権限を有しているか確認                 |
| **想定外連携の防止**       | 一般権限の従業員が攻撃者のサービスに誤ってデータ連携することを防止 |
| **Scope 設計**             | 適切な粒度で Scope を設計                                          |

### 2.4 推奨アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Application                       │
│                    (Web / Mobile / API)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    認証基盤 (Rust)                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │   API Gateway   │  │  Auth Service   │  │ User Service │ │
│  │    (Axum)       │  │   (認証処理)     │  │ (ID管理)     │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
│           │                   │                    │         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Session / Token Management                  ││
│  │         (JWT, Redis, tower-sessions)                     ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Data Store Layer                          │
│    ┌──────────┐    ┌──────────┐    ┌──────────────────┐     │
│    │ PostgreSQL│    │  Redis   │    │ External IdP     │     │
│    │(User DB)  │    │(Session) │    │(Google, etc.)    │     │
│    └──────────┘    └──────────┘    └──────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. 技術スタック選定

### 3.1 Webフレームワーク

| Crate     | 特徴                                                   | 推奨度 |
| --------- | ------------------------------------------------------ | ------ |
| **axum**  | Tower ベース、型安全、モジュラー設計、エコシステム充実 | ★★★★★  |
| actix-web | 高性能、成熟したエコシステム                           | ★★★★☆  |
| warp      | 関数型スタイル、コンポーザブル                         | ★★★☆☆  |

**推奨: axum**

```toml
[dependencies]
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

> **Note**: 本プロジェクトでは Rust Edition 2024 を使用しています。ビルドには nightly コンパイラが必要です。

### 3.2 OAuth2 / OpenID Connect

| Crate             | 用途                | 特徴                                      |
| ----------------- | ------------------- | ----------------------------------------- |
| **oauth2**        | OAuth2 クライアント | 強い型付け、RFC 6749準拠、PKCEサポート    |
| **openidconnect** | OIDC クライアント   | oauth2の上に構築、OpenID Connect Core準拠 |
| oxide-auth        | OAuth2 サーバー     | 認可サーバー構築用                        |

```toml
[dependencies]
oauth2 = "4"
openidconnect = "3"
```

**使用例（OAuth2クライアント）:**

```rust
use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, RedirectUrl, Scope,
    TokenResponse, TokenUrl
};
use oauth2::basic::BasicClient;
use oauth2::reqwest;

let client = BasicClient::new(ClientId::new("client_id".to_string()))
    .set_client_secret(ClientSecret::new("secret".to_string()))
    .set_auth_uri(AuthUrl::new("https://auth.example.com/authorize".to_string())?)
    .set_token_uri(TokenUrl::new("https://auth.example.com/token".to_string())?)
    .set_redirect_uri(RedirectUrl::new("https://app.example.com/callback".to_string())?);

// PKCE チャレンジ生成
let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

// 認可URL生成
let (auth_url, csrf_token) = client
    .authorize_url(CsrfToken::new_random)
    .add_scope(Scope::new("openid".to_string()))
    .add_scope(Scope::new("profile".to_string()))
    .set_pkce_challenge(pkce_challenge)
    .url();
```

### 3.3 Ory Hydra との連携（Rust 実装）

Ory Hydra を認可サーバーとして利用する場合、Login/Consent Provider を Rust (Axum) で実装します。

**Hydra Admin API クライアント:**

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub challenge: String,
    pub skip: bool,
    pub subject: String,
    pub client: OAuthClient,
    pub request_url: String,
    pub requested_scope: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AcceptLoginRequest {
    pub subject: String,
    pub remember: bool,
    pub remember_for: i64,
    pub acr: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RedirectResponse {
    pub redirect_to: String,
}

pub struct HydraClient {
    client: Client,
    admin_url: String,
}

impl HydraClient {
    pub fn new(admin_url: String) -> Self {
        Self {
            client: Client::new(),
            admin_url,
        }
    }

    /// Login Challenge の情報を取得
    pub async fn get_login_request(&self, challenge: &str) -> Result<LoginRequest, Error> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login?login_challenge={}",
            self.admin_url, challenge
        );
        let resp = self.client.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Login を承認
    pub async fn accept_login(
        &self,
        challenge: &str,
        body: AcceptLoginRequest
    ) -> Result<RedirectResponse, Error> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login/accept?login_challenge={}",
            self.admin_url, challenge
        );
        let resp = self.client.put(&url).json(&body).send().await?;
        Ok(resp.json().await?)
    }

    /// Consent を承認
    pub async fn accept_consent(
        &self,
        challenge: &str,
        grant_scope: Vec<String>,
    ) -> Result<RedirectResponse, Error> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent/accept?consent_challenge={}",
            self.admin_url, challenge
        );
        let body = serde_json::json!({
            "grant_scope": grant_scope,
            "remember": true,
            "remember_for": 3600
        });
        let resp = self.client.put(&url).json(&body).send().await?;
        Ok(resp.json().await?)
    }
}
```

**Axum ハンドラー実装:**

```rust
use axum::{
    extract::{Query, State},
    response::Redirect,
    Form,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginQuery {
    login_challenge: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

/// Login Provider: 認証処理
pub async fn handle_login(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    // 1. Hydra から login request 情報を取得
    let login_request = state.hydra
        .get_login_request(&query.login_challenge)
        .await?;

    // 2. 既存の認証ロジックでユーザー認証
    let user = state.auth_service
        .authenticate(&form.email, &form.password)
        .await?;

    // 3. Hydra に認証成功を通知
    let redirect = state.hydra.accept_login(
        &query.login_challenge,
        AcceptLoginRequest {
            subject: user.id.to_string(),
            remember: true,
            remember_for: 3600,
            acr: None,
        },
    ).await?;

    // 4. Hydra が指定するURLにリダイレクト
    Ok(Redirect::to(&redirect.redirect_to))
}

#[derive(Deserialize)]
pub struct ConsentQuery {
    consent_challenge: String,
}

/// Consent Provider: 同意処理
pub async fn handle_consent(
    State(state): State<AppState>,
    Query(query): Query<ConsentQuery>,
) -> Result<Redirect, AppError> {
    // 1. Hydra から consent request 情報を取得
    let consent_request = state.hydra
        .get_consent_request(&query.consent_challenge)
        .await?;

    // 2. 連携担当者の権限チェック
    let user_id = &consent_request.subject;
    let has_admin_permission = state.permission_service
        .check_admin_permission(user_id)
        .await?;

    if !has_admin_permission {
        return Err(AppError::Forbidden("Admin permission required"));
    }

    // 3. Hydra に同意を通知
    let redirect = state.hydra.accept_consent(
        &query.consent_challenge,
        consent_request.requested_scope,
    ).await?;

    Ok(Redirect::to(&redirect.redirect_to))
}
```

### 3.4 JWT (JSON Web Token)

| Crate            | 特徴                                    | 推奨度 |
| ---------------- | --------------------------------------- | ------ |
| **jsonwebtoken** | デファクトスタンダード、RS256/HS256対応 | ★★★★★  |
| axum-jwt-auth    | Axum用ミドルウェア、JWKS自動更新対応    | ★★★★☆  |

```toml
[dependencies]
jsonwebtoken = "9"
```

**JWT生成・検証:**

```rust
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,        // Subject (user ID)
    exp: usize,         // Expiration time
    iat: usize,         // Issued at
    iss: String,        // Issuer
    aud: Vec<String>,   // Audience
}

// JWT生成
fn create_token(user_id: &str, secret: &[u8]) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
        iss: "auth.example.com".to_string(),
        aud: vec!["api.example.com".to_string()],
    };

    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_secret(secret)
    )
}

// JWT検証
fn verify_token(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["auth.example.com"]);
    validation.set_audience(&["api.example.com"]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation
    )?;

    Ok(token_data.claims)
}
```

### 3.5 WebAuthn / パスキー

| Crate           | 提供元    | 特徴                                    |
| --------------- | --------- | --------------------------------------- |
| **webauthn-rs** | Kanidm    | Relying Party実装、セキュリティ監査済み |
| passkey         | 1Password | WebAuthn Level 3 + CTAP2準拠            |

```toml
[dependencies]
webauthn-rs = { version = "0.5", features = ["danger-allow-state-serialisation"] }
```

**WebAuthn登録フロー:**

```rust
use webauthn_rs::prelude::*;

// Webauthnインスタンス作成
let rp_id = "auth.example.com";
let rp_origin = Url::parse("https://auth.example.com")?;
let webauthn = WebauthnBuilder::new(rp_id, &rp_origin)?
    .rp_name("Example Auth Service")
    .build()?;

// 登録開始（チャレンジ生成）
let (ccr, reg_state) = webauthn.start_passkey_registration(
    Uuid::new_v4(),                    // user_id
    "user@example.com",                // username
    "User Name",                       // display_name
    None,                              // exclude_credentials
)?;
// reg_state をセッションに保存
// ccr をクライアントに返却

// 登録完了（認証器レスポンス検証）
let passkey = webauthn.finish_passkey_registration(
    &client_response,   // クライアントからの応答
    &reg_state,         // 保存していた状態
)?;
// passkey をDBに保存
```

### 3.6 パスワードハッシュ

| Crate      | アルゴリズム | 推奨度            |
| ---------- | ------------ | ----------------- |
| **argon2** | Argon2id     | ★★★★★ (OWASP推奨) |
| bcrypt     | Bcrypt       | ★★★★☆             |
| scrypt     | Scrypt       | ★★★☆☆             |

**OWASP推奨設定:**

- Argon2id: 19 MiB メモリ, 2 イテレーション, 1 並列度
- Bcrypt: work factor 10以上, 72バイト制限

```toml
[dependencies]
argon2 = "0.5"
password-hash = "0.5"
```

**パスワードハッシュ実装:**

```rust
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

// パスワードハッシュ生成
fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // OWASP推奨設定がデフォルト

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

// パスワード検証
fn verify_password(password: &str, hash: &str) -> Result<bool, password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

### 3.7 セッション管理

| Crate              | 特徴                       | バックエンド対応                      |
| ------------------ | -------------------------- | ------------------------------------- |
| **tower-sessions** | Tower/Axum向け、型安全     | Redis, PostgreSQL, SQLite, Memory     |
| axum_session       | Axum専用、多機能           | Redis, PostgreSQL, MongoDB, SurrealDB |
| ruts               | 軽量、レイヤードストア対応 | Redis, PostgreSQL                     |

```toml
[dependencies]
tower-sessions = "0.13"
tower-sessions-redis-store = "0.14"
```

**セッション管理実装:**

```rust
use axum::{Router, routing::get, response::IntoResponse};
use tower_sessions::{Session, SessionManagerLayer, Expiry};
use tower_sessions_redis_store::{fred::prelude::*, RedisStore};
use time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserSession {
    user_id: String,
    email: String,
    authenticated_at: i64,
}

const USER_SESSION_KEY: &str = "user";

async fn protected_handler(session: Session) -> impl IntoResponse {
    match session.get::<UserSession>(USER_SESSION_KEY).await.unwrap() {
        Some(user) => format!("Hello, {}!", user.email),
        None => "Unauthorized".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Redis接続
    let pool = Pool::new(Config::default(), None, None, None, 6)?;
    pool.connect();
    pool.wait_for_connect().await?;

    let session_store = RedisStore::new(pool);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(session_layer);

    // サーバー起動...
    Ok(())
}
```

---

## 4. データベース設計

### 4.1 ユーザーテーブル

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    password_hash VARCHAR(255),  -- パスワードレス時はNULL
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'active'  -- active, suspended, deleted
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_status ON users(status);
```

### 4.2 認証情報テーブル（WebAuthn/パスキー）

```sql
CREATE TABLE user_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA UNIQUE NOT NULL,
    public_key BYTEA NOT NULL,
    counter BIGINT NOT NULL DEFAULT 0,
    aaguid UUID,
    credential_type VARCHAR(50) NOT NULL,  -- 'passkey', 'security_key'
    device_name VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_user_credentials_user_id ON user_credentials(user_id);
CREATE INDEX idx_user_credentials_credential_id ON user_credentials(credential_id);
```

### 4.3 外部IdP連携テーブル

```sql
CREATE TABLE user_external_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,  -- 'google', 'github', 'apple'
    provider_user_id VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    UNIQUE(provider, provider_user_id)
);

CREATE INDEX idx_external_accounts_user_id ON user_external_accounts(user_id);
CREATE INDEX idx_external_accounts_provider ON user_external_accounts(provider, provider_user_id);
```

### 4.4 リフレッシュトークンテーブル

```sql
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) UNIQUE NOT NULL,
    device_info JSONB,
    ip_address INET,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    revoked_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
```

---

## 5. 認証フロー実装

### 5.1 パスワード認証フロー

```
┌────────┐                    ┌────────────┐                    ┌──────────┐
│ Client │                    │ Auth Server│                    │ Database │
└────┬───┘                    └─────┬──────┘                    └────┬─────┘
     │                              │                                │
     │ POST /auth/login             │                                │
     │ {email, password}            │                                │
     │─────────────────────────────>│                                │
     │                              │                                │
     │                              │ SELECT user WHERE email        │
     │                              │───────────────────────────────>│
     │                              │                                │
     │                              │ user record                    │
     │                              │<───────────────────────────────│
     │                              │                                │
     │                              │ Argon2 verify password         │
     │                              │───────┐                        │
     │                              │       │                        │
     │                              │<──────┘                        │
     │                              │                                │
     │                              │ Generate JWT (access + refresh)│
     │                              │───────┐                        │
     │                              │       │                        │
     │                              │<──────┘                        │
     │                              │                                │
     │                              │ Store refresh_token            │
     │                              │───────────────────────────────>│
     │                              │                                │
     │ 200 OK                       │                                │
     │ {access_token, refresh_token}│                                │
     │<─────────────────────────────│                                │
     │                              │                                │
```

### 5.2 OAuth2/OIDC フロー（Google認証例）

```
┌────────┐          ┌────────────┐          ┌────────────┐          ┌──────────┐
│ Client │          │ Auth Server│          │ Google IdP │          │ Database │
└────┬───┘          └─────┬──────┘          └─────┬──────┘          └────┬─────┘
     │                    │                       │                      │
     │ GET /auth/google   │                       │                      │
     │───────────────────>│                       │                      │
     │                    │                       │                      │
     │                    │ Generate PKCE + state │                      │
     │                    │──────┐                │                      │
     │                    │      │                │                      │
     │                    │<─────┘                │                      │
     │                    │                       │                      │
     │ 302 Redirect       │                       │                      │
     │ → Google Auth URL  │                       │                      │
     │<───────────────────│                       │                      │
     │                    │                       │                      │
     │ User authenticates │                       │                      │
     │─────────────────────────────────────────>  │                      │
     │                    │                       │                      │
     │ 302 Redirect       │                       │                      │
     │ → /auth/callback   │                       │                      │
     │ ?code=xxx&state=yyy│                       │                      │
     │───────────────────>│                       │                      │
     │                    │                       │                      │
     │                    │ Verify state          │                      │
     │                    │ Exchange code→tokens  │                      │
     │                    │──────────────────────>│                      │
     │                    │                       │                      │
     │                    │ id_token, access_token│                      │
     │                    │<──────────────────────│                      │
     │                    │                       │                      │
     │                    │ Verify id_token       │                      │
     │                    │ Upsert user           │                      │
     │                    │──────────────────────────────────────────────>
     │                    │                       │                      │
     │ 200 OK (JWT tokens)│                       │                      │
     │<───────────────────│                       │                      │
```

### 5.3 パスキー認証フロー

```
┌────────┐          ┌────────────┐          ┌─────────────┐          ┌──────────┐
│ Client │          │ Auth Server│          │ Authenticator│          │ Database │
└────┬───┘          └─────┬──────┘          └──────┬──────┘          └────┬─────┘
     │                    │                        │                      │
     │ POST /auth/passkey/start                    │                      │
     │ {username}         │                        │                      │
     │───────────────────>│                        │                      │
     │                    │                        │                      │
     │                    │ Get user credentials   │                      │
     │                    │───────────────────────────────────────────────>
     │                    │                        │                      │
     │                    │ Generate challenge     │                      │
     │                    │──────┐                 │                      │
     │                    │      │                 │                      │
     │                    │<─────┘                 │                      │
     │                    │                        │                      │
     │ {publicKeyCredentialRequestOptions}         │                      │
     │<───────────────────│                        │                      │
     │                    │                        │                      │
     │ navigator.credentials.get()                 │                      │
     │────────────────────────────────────────────>│                      │
     │                    │                        │                      │
     │ User verification (PIN/Biometric)           │                      │
     │                    │                        │──────┐               │
     │                    │                        │      │               │
     │                    │                        │<─────┘               │
     │                    │                        │                      │
     │ {authenticatorResponse}                     │                      │
     │<────────────────────────────────────────────│                      │
     │                    │                        │                      │
     │ POST /auth/passkey/finish                   │                      │
     │ {authenticatorResponse}                     │                      │
     │───────────────────>│                        │                      │
     │                    │                        │                      │
     │                    │ Verify signature       │                      │
     │                    │ Update counter         │                      │
     │                    │───────────────────────────────────────────────>
     │                    │                        │                      │
     │ 200 OK (JWT tokens)│                        │                      │
     │<───────────────────│                        │                      │
```

---

## 6. API エンドポイント設計

### 6.1 認証 API

| Method | Endpoint                            | 説明                 |
| ------ | ----------------------------------- | -------------------- |
| POST   | `/auth/register`                    | ユーザー登録         |
| POST   | `/auth/login`                       | パスワードログイン   |
| POST   | `/auth/logout`                      | ログアウト           |
| POST   | `/auth/refresh`                     | トークンリフレッシュ |
| GET    | `/auth/oauth/{provider}`            | OAuth開始            |
| GET    | `/auth/oauth/{provider}/callback`   | OAuthコールバック    |
| POST   | `/auth/passkey/register/start`      | パスキー登録開始     |
| POST   | `/auth/passkey/register/finish`     | パスキー登録完了     |
| POST   | `/auth/passkey/authenticate/start`  | パスキー認証開始     |
| POST   | `/auth/passkey/authenticate/finish` | パスキー認証完了     |

### 6.2 レスポンス形式

**成功時:**

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

**エラー時:**

```json
{
  "error": "invalid_credentials",
  "error_description": "The provided credentials are invalid",
  "error_code": "AUTH_001"
}
```

---

## 7. セキュリティ要件

### 7.1 必須セキュリティ対策

| 対策                        | 実装方法                             |
| --------------------------- | ------------------------------------ |
| **HTTPS強制**               | TLS 1.3、HSTS ヘッダー               |
| **CSRF保護**                | SameSite Cookie、CSRFトークン        |
| **レート制限**              | tower-governor、Redis による分散制限 |
| **入力検証**                | validator crate、型安全性活用        |
| **SQLインジェクション防止** | SQLx のパラメータバインディング      |
| **XSS防止**                 | HttpOnly Cookie、CSP ヘッダー        |
| **ブルートフォース対策**    | アカウントロックアウト、遅延応答     |

### 7.2 トークンセキュリティ

```rust
// トークン設定例
const ACCESS_TOKEN_EXPIRY: Duration = Duration::minutes(15);
const REFRESH_TOKEN_EXPIRY: Duration = Duration::days(30);
const REFRESH_TOKEN_ROTATION: bool = true;  // リフレッシュ時に新トークン発行
```

### 7.3 Cookie設定

```rust
use tower_sessions::cookie::{SameSite, Cookie};

let cookie_config = Cookie::build("session")
    .secure(true)           // HTTPS only
    .http_only(true)        // JavaScript からアクセス不可
    .same_site(SameSite::Lax)  // CSRF対策
    .path("/")
    .max_age(Duration::hours(24));
```

---

## 8. 依存関係一覧（Cargo.toml）

### 8.1 本プロジェクトで使用している依存関係

```toml
[package]
name = "ory-hydra-rust"
version = "0.1.0"
edition = "2024"

[dependencies]
# Web Framework
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["trace", "cors"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP Client (Hydra Admin API)
reqwest = { version = "0.12", features = ["json"] }

# Authentication
jsonwebtoken = "9"
argon2 = "0.5"
password-hash = "0.5"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
thiserror = "2"
url = "2"
```

### 8.2 将来の拡張で使用する依存関係

```toml
# OAuth2 / OIDC (外部IdP連携時)
oauth2 = "4"
openidconnect = "3"

# WebAuthn (パスキー認証時)
webauthn-rs = { version = "0.5", features = ["danger-allow-state-serialisation"] }

# Session Management (Redis利用時)
tower-sessions = "0.13"
tower-sessions-redis-store = "0.14"

# Database (PostgreSQL利用時)
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }

# Validation
validator = { version = "0.18", features = ["derive"] }
```

---

## 9. 実装チェックリスト

### 9.1 Phase 1: 基盤構築

- [x] プロジェクト構造セットアップ
- [ ] データベーススキーマ作成・マイグレーション（インメモリ実装済み）
- [x] 基本的なエラーハンドリング
- [x] ロギング・トレーシング設定

### 9.2 Phase 2: 基本認証

- [x] ユーザー登録（メール/パスワード）
- [x] パスワードハッシュ（Argon2id）
- [x] ログイン/ログアウト
- [x] JWTトークン発行・検証
- [x] リフレッシュトークン

### 9.3 Phase 3: Ory Hydra 連携

- [x] Login Provider 実装
- [x] Consent Provider 実装
- [x] Logout Provider 実装
- [x] Hydra Admin API クライアント
- [x] Docker Compose 環境構築

### 9.4 Phase 4: テスト

- [x] ユニットテスト（正常系）
- [x] ユニットテスト（異常系・エッジケース）
- [x] 並行処理テスト
- [ ] 結合テスト

### 9.5 Phase 5: セッション管理（未実装）

- [ ] Redis セッションストア
- [ ] セッション有効期限管理
- [ ] 同時セッション制御

### 9.6 Phase 6: OAuth2/OIDC（未実装）

- [ ] Google OAuth2 連携
- [ ] （オプション）GitHub, Apple 連携
- [ ] アカウントリンク機能

### 9.7 Phase 7: WebAuthn/パスキー（未実装）

- [ ] パスキー登録フロー
- [ ] パスキー認証フロー
- [ ] 複数デバイス管理

### 9.8 Phase 8: セキュリティ強化（未実装）

- [ ] レート制限
- [ ] CSRF保護
- [ ] ブルートフォース対策
- [ ] 監査ログ

---

## 10. クイックスタート

### 10.1 環境起動

```sh
# Docker Compose で環境を起動
docker compose up -d --build

# ログ確認
docker compose logs -f auth-provider

# ヘルスチェック
curl http://localhost:3000/health
```

### 10.2 OAuth2 クライアント登録

```sh
docker compose exec hydra hydra create oauth2-client \
  --endpoint http://localhost:4445 \
  --grant-type authorization_code \
  --response-type code \
  --scope openid,offline_access,profile,email \
  --redirect-uri http://localhost:8080/callback \
  --name "Test Client"
```

### 10.3 認可フローテスト

```sh
# テストユーザー作成
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123"}'

# ブラウザで認可エンドポイントにアクセス
open "http://localhost:4444/oauth2/auth?\
client_id=<CLIENT_ID>&\
response_type=code&\
scope=openid+profile+email&\
redirect_uri=http://localhost:8080/callback&\
state=random_state"
```

### 10.4 テスト実行

```sh
cargo test
```

### 10.5 トラブルシューティング

#### Edition 2024 がビルドできない

`rust:1.83-alpine` ではEdition 2024がサポートされていません。`rustlang/rust:nightly-alpine` を使用してください。

#### OpenSSL が見つからない

Dockerビルド時に `Could not find directory of OpenSSL installation` エラーが出る場合：

```dockerfile
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static
```

#### ポート 5432 が使用中

ローカルのPostgreSQLと競合する場合、docker-compose.yml でPostgreSQLのポートマッピングをコメントアウトしてください。

```yaml
postgres:
  # ports:
  #   - "5432:5432"
```

---

## 11. 参考資料

### Ory Hydra

- [Ory Hydra 公式ドキュメント](https://www.ory.com/docs/oauth2-oidc)
- [Ory Hydra GitHub](https://github.com/ory/hydra)

### 仕様書・RFC

- [RFC 6749 - OAuth 2.0](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 7519 - JSON Web Token (JWT)](https://datatracker.ietf.org/doc/html/rfc7519)
- [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html)
- [WebAuthn Level 3](https://www.w3.org/TR/webauthn-3/)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

### Rust Crate Documentation

- [oauth2-rs](https://docs.rs/oauth2)
- [openidconnect](https://docs.rs/openidconnect)
- [jsonwebtoken](https://docs.rs/jsonwebtoken)
- [webauthn-rs](https://docs.rs/webauthn-rs)
- [argon2](https://docs.rs/argon2)
- [tower-sessions](https://docs.rs/tower-sessions)
- [axum](https://docs.rs/axum)

---

_本仕様書は2026年1月時点の情報に基づいています。各crateのバージョンは最新版を確認してください。_
