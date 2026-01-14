# Ory Kratosで認証を委譲する ― 自前実装からの卒業

## 前回からの続き

前回の記事では、Playwright MCPを使ったE2Eテストで5つのバグを発見した。CORS設定の欠如、JWTトークンの切り詰め、Hydraトークンとの不一致、ミドルウェアの適用漏れ、X-Tenant-Slugヘッダーの欠如。RBACの検証とOWASP Top 10との比較まで行い、マルチテナント認証システムが一通り動くようになった。

> **前提知識**: この記事はOry Hydraシリーズの続編です。OAuth2認可コードフローの基礎知識と、Login/Consent Providerの役割を理解している前提で進めます。前回記事は[こちら](https://syu-m-5151.hatenablog.com/entry/2026/01/11/064311)。

動く。ちゃんと動く。でも、レビューコメントが気になった。

「パスワードリセット機能は？」
「MFA対応の予定は？」
「メール確認フローは？」

全部、自分で実装しなければならない。

Argon2idでパスワードをハッシュ化するコードは書いた。ログイン認証は動く。でも、パスワードを忘れたユーザーへのリセットメール送信、そのトークン管理、有効期限の検証。TOTPによる二要素認証。メールアドレス確認のフロー。

これ全部、自分で実装するのか？

RFCを読んでいたあの3日間を思い出した。仕様は理解できる。実装もできる。でも、プロダクション品質で検証し続けることは、私たちの仕事ではない。

同じ結論に至った。今度は認証機能についてだ。

## Ory Kratosという選択肢

[https://www.ory.sh/kratos/:embed:cite]

[https://github.com/ory/kratos:embed:cite]

Ory Kratosは「ヘッドレスID管理システム」だ。

Hydraが「認証をしない認可サーバー」だったことを思い出してほしい。Hydraはプロトコル層（OAuth2/OIDC）に特化し、認証は私たちに任せた。

Kratosはその「任された認証」を担当する。

```
┌─────────────────────────────────────────────────────────────┐
│                     Ory Stack                               │
├────────────────────────┬────────────────────────────────────┤
│      Ory Kratos        │           Ory Hydra                │
│   (Identity Provider)  │      (Authorization Server)        │
├────────────────────────┼────────────────────────────────────┤
│ - ユーザー登録         │ - OAuth2/OIDC                      │
│ - ログイン認証         │ - トークン発行                     │
│ - MFA (TOTP, WebAuthn) │ - クライアント管理                 │
│ - パスワードリセット   │ - Consent管理                      │
│ - プロフィール管理     │ - セッション管理                   │
│ - メール確認           │                                    │
└────────────────────────┴────────────────────────────────────┘
```

つまり、これまでに私がRustで書いた`AuthService`——パスワード検証、ユーザー登録、セッション管理——これらをKratosに任せられる。

## アーキテクチャの変化

これまでの構成を振り返る。

```
【01-03の構成】
┌─────────────┐     ┌─────────────────────┐     ┌─────────────┐
│   Browser   │────▶│ Rust Login Provider │────▶│  Ory Hydra  │
│             │     │ (自前実装)           │     │             │
└─────────────┘     └─────────────────────┘     └─────────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │ PostgreSQL (users)  │
                    └─────────────────────┘
```

私が書いたRust Login Providerは認証を担当していた。ユーザーテーブルも自前で管理していた。

Kratosを導入すると、こうなる。

```
【Kratos導入後の構成】
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Browser   │────▶│  Kratos UI  │────▶│  Ory Kratos │────▶│  Ory Hydra  │
│             │     │  (Node.js)  │     │             │     │             │
└─────────────┘     └─────────────┘     └──────┬──────┘     └─────────────┘
                                               │
                                               ▼
                                      ┌─────────────────────┐
                                      │ PostgreSQL          │
                                      │ (identities)        │
                                      └─────────────────────┘
```

私が書くコードは、ほぼゼロになる。パスワード検証、ユーザー登録、セッション管理——これまでに私がRustで実装した機能は、全てKratosが提供する。私が書くのはKratosの設定ファイルと、必要に応じたUIのカスタマイズだけだ。

「それって、学習した意味がないのでは？」

いや、逆だ。

認証システムを自前で実装した経験は、**Kratosの設定を理解する上で役立った**。例えば、Kratosの設定に`hashers.argon2.memory: 128MB`という項目がある。自前実装の経験がなければ、その意味を理解できなかっただろう。メモリコストを上げればセキュリティは向上する。しかし同時接続数の増加でOOMのリスクも上がる——この判断ができるのは、OWASPのドキュメントを読み、自分でパラメータを選んだ経験があるからだ。

「ドキュメントを読めば同じでは？」——そう思うかもしれない。確かに、ドキュメントを読めば設定はできる。しかし、障害時に「この設定が原因かもしれない」と仮説を立てられるのは、自分で同じ問題に苦しんだ経験があるからだ。ログを見て「これはセッション固定化攻撃への対策が発動した」と判断できるか。エラーメッセージから「Identity Schemaの定義が間違っている」と気づけるか。これは学習効率の問題ではなく、**デバッグ能力の問題**だ。

これまでの実装で、認証システムの複雑さを体験した。Argon2idのパラメータ設定、ユーザー列挙攻撃への対策、セッション管理の罠。58個のテストを書いて「できないこと」を確認した。

だからこそ、Kratosのありがたみが分かる。そして、Kratosで問題が起きたときに対処できる。全員が自前実装を経験すべきとは言わない。しかし、チームに1人は「中身を理解している人」がいた方がいい。

## Docker Composeで動かす

[https://www.ory.com/docs/kratos/quickstart:embed:cite]

実際に動かしてみよう。

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: postgres
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro  # 後述の初期化スクリプト
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d postgres"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - ory

  kratos-migrate:
    image: oryd/kratos:v1.3.1
    environment:
      DSN: postgres://postgres:secret@postgres:5432/kratos?sslmode=disable
    volumes:
      - ./kratos:/etc/config/kratos:ro
    command: migrate sql -e --yes --config /etc/config/kratos/kratos.yml
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - ory

  kratos:
    image: oryd/kratos:v1.3.1
    environment:
      DSN: postgres://postgres:secret@postgres:5432/kratos?sslmode=disable
      LOG_LEVEL: debug
      SERVE_PUBLIC_BASE_URL: http://localhost:4433
      SERVE_ADMIN_BASE_URL: http://localhost:4434
    volumes:
      - ./kratos:/etc/config/kratos:ro
    command: serve all --dev --config /etc/config/kratos/kratos.yml
    ports:
      - "4433:4433"  # Public API
      - "4434:4434"  # Admin API
    depends_on:
      kratos-migrate:
        condition: service_completed_successfully
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:4433/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - ory

  hydra-migrate:
    image: oryd/hydra:v2.2
    environment:
      DSN: postgres://postgres:secret@postgres:5432/hydra?sslmode=disable
    command: migrate sql -e --yes
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - ory

  hydra:
    image: oryd/hydra:v2.2
    environment:
      DSN: postgres://postgres:secret@postgres:5432/hydra?sslmode=disable
      SECRETS_SYSTEM: super-secret-system-secret-at-least-32-chars
      URLS_SELF_ISSUER: http://localhost:4444
      URLS_CONSENT: http://localhost:4455/consent
      URLS_LOGIN: http://localhost:4455/login
      URLS_LOGOUT: http://localhost:4455/logout
      LOG_LEVEL: debug
    command: serve all --dev
    ports:
      - "4444:4444"  # Public API
      - "4445:4445"  # Admin API
    depends_on:
      hydra-migrate:
        condition: service_completed_successfully
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:4444/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - ory

  kratos-ui:
    image: oryd/kratos-selfservice-ui-node:v1.3.1
    environment:
      PORT: 4455
      KRATOS_PUBLIC_URL: http://kratos:4433
      KRATOS_BROWSER_URL: http://localhost:4433
      HYDRA_ADMIN_URL: http://hydra:4445
      COOKIE_SECRET: super-secret-cookie-secret-32chars
      CSRF_COOKIE_NAME: ory_csrf_ui
      CSRF_COOKIE_SECRET: super-secret-csrf-secret-32-chars
    ports:
      - "4455:4455"
    depends_on:
      kratos:
        condition: service_healthy
      hydra:
        condition: service_healthy
    networks:
      - ory

volumes:
  postgres_data:

networks:
  ory:
```

> **注意**: 上記の設定は開発環境用です。本番環境では`SECRETS_SYSTEM`や`COOKIE_SECRET`に32文字以上の暗号学的に安全な値を設定してください。

サービスが6つある。PostgreSQL、KratosとHydraそれぞれのmigrate/serveサービス、そしてKratos UI。以前の自前実装（auth-provider）はKratosに置き換わった。

ポイントは`kratos-ui`だ。これはOry公式が提供するセルフサービスUI。ログイン画面、登録画面、パスワードリセット画面などが含まれている。

「自分でUI書かなくていいの？」

開発環境ではこれで十分だ。本番環境では、このUIを参考に自前のUIを実装できる。Kratosの「ヘッドレス」設計により、UIは完全に切り離されている。

[https://github.com/ory/kratos-selfservice-ui-node:embed:cite]

## Kratos設定ファイルの解説

Kratosの設定ファイル`kratos.yml`を見てみよう。

```yaml
version: v1.3.1

dsn: memory

serve:
  public:
    base_url: http://localhost:4433/
    cors:
      enabled: true
      allowed_origins:
        - http://localhost:4455
  admin:
    base_url: http://localhost:4434/

selfservice:
  default_browser_return_url: http://localhost:4455/
  allowed_return_urls:
    - http://localhost:4455
    - http://localhost:4444

  methods:
    password:
      enabled: true
    totp:
      enabled: true
      config:
        issuer: OryKratosVerification
    lookup_secret:
      enabled: true
    link:
      enabled: true
    code:
      enabled: true

  flows:
    error:
      ui_url: http://localhost:4455/error

    settings:
      ui_url: http://localhost:4455/settings
      privileged_session_max_age: 15m

    recovery:
      enabled: true
      ui_url: http://localhost:4455/recovery
      use: code

    verification:
      enabled: true
      ui_url: http://localhost:4455/verification
      use: code
      after:
        default_browser_return_url: http://localhost:4455/

    logout:
      after:
        default_browser_return_url: http://localhost:4455/login

    login:
      ui_url: http://localhost:4455/login
      lifespan: 10m

    registration:
      lifespan: 10m
      ui_url: http://localhost:4455/registration
      after:
        password:
          hooks:
            - hook: session

log:
  level: debug
  format: text
  leak_sensitive_values: true

secrets:
  cookie:
    - super-secret-cookie-secret-32chars
  cipher:
    - super-secret-cipher-key-32-chars

ciphers:
  algorithm: xchacha20-poly1305

hashers:
  algorithm: argon2
  argon2:
    parallelism: 1
    memory: 128MB
    iterations: 2
    salt_length: 16
    key_length: 16

identity:
  default_schema_id: default
  schemas:
    - id: default
      url: file:///etc/config/kratos/identity.schema.json

courier:
  smtp:
    connection_uri: smtps://test:test@mailslurper:1025/?skip_ssl_verify=true

oauth2_provider:
  url: http://hydra:4445
```

### セルフサービスフロー

```yaml
selfservice:
  methods:
    password:
      enabled: true
    totp:
      enabled: true
```

以前、私がRustで実装したパスワード認証。Kratosでは`password: enabled: true`の一行で有効になる。

TOTPも同様だ。以前は「MFA対応の予定は？」という質問に答えられなかった。Kratosなら設定1つで有効化できる。

### パスワードハッシュ

```yaml
hashers:
  algorithm: argon2
  argon2:
    parallelism: 1
    memory: 128MB
    iterations: 2
    salt_length: 16
    key_length: 16
```

以前、私は`Argon2::default()`を使った。Kratosも同じArgon2を使っている。設定値を明示的に指定することで、チーム内で「なぜこのパラメータか」を共有できる。

[https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html:embed:cite]

### Hydra連携

```yaml
oauth2_provider:
  url: http://hydra:4445
```

これが最も重要な設定だ。KratosがHydraのAdmin APIに接続し、`login_challenge`を処理する。

以前は私がRustで`HydraClient`を実装し、`accept_login`を呼び出していた。Kratosはこれを自動で行う。

[https://www.ory.com/docs/kratos/self-hosted/hydra-integration:embed:cite]

## Identity Schemaの設計

Kratosはユーザー情報を「Identity」として管理する。その構造はJSON Schemaで定義する。

```json
{
  "$id": "https://schemas.ory.sh/presets/kratos/identity.email.schema.json",
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Person",
  "type": "object",
  "properties": {
    "traits": {
      "type": "object",
      "properties": {
        "email": {
          "type": "string",
          "format": "email",
          "title": "E-Mail",
          "ory.sh/kratos": {
            "credentials": {
              "password": {
                "identifier": true
              },
              "totp": {
                "account_name": true
              }
            },
            "recovery": {
              "via": "email"
            },
            "verification": {
              "via": "email"
            }
          }
        },
        "name": {
          "type": "object",
          "properties": {
            "first": {
              "title": "First Name",
              "type": "string"
            },
            "last": {
              "title": "Last Name",
              "type": "string"
            }
          }
        }
      },
      "required": ["email"],
      "additionalProperties": false
    }
  }
}
```

`ory.sh/kratos`という拡張プロパティが特徴的だ。

- `credentials.password.identifier: true` — このフィールドがログインIDになる
- `recovery.via: email` — パスワードリセットはこのメールアドレスに送信される
- `verification.via: email` — メール確認もこのアドレスに送信される

以前、私はユーザーテーブルを自前で設計した。Kratosではスキーマを宣言的に定義するだけでいい。

[https://www.ory.com/docs/kratos/manage-identities/identity-schema:embed:cite]

## 実際にハマったこと

でも、最初の`docker compose up`は失敗した。

### データベースが存在しない

```
FATAL: database "kratos" does not exist (SQLSTATE 3D000)
```

KratosとHydraはそれぞれ`kratos`と`hydra`という名前のデータベースを期待する。でも、PostgreSQLコンテナは`postgres`データベースしか作らない。

解決策は初期化スクリプトだ。

```sql
-- init.sql
CREATE DATABASE kratos;
CREATE DATABASE hydra;
```

```yaml
# docker-compose.yml
postgres:
  volumes:
    - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro
```

PostgreSQLは`/docker-entrypoint-initdb.d/`にあるSQLファイルを起動時に実行する。これで両方のデータベースが作成される。

最初は「なぜ自動で作ってくれないんだ」と思った。おそらく、本番環境では既存のデータベースサーバーに接続することが多いからだろう。いずれにせよ、初期化スクリプトで解決できる。

### ポート競合

```
Bind for 0.0.0.0:4444 failed: port is already allocated
```

以前の記事で作ったory-hydra-rust環境がまだ動いていた。同じポート4444を使おうとして衝突。

```sh
# 他の環境を停止
cd ../ory-hydra-rust && docker compose down
```

複数のOry環境を並行して動かす時は、ポートを変える必要がある。開発環境では素直に片方を停止した方がいい。

## Kratosが教えてくれた盲点

E2Eテストで`TestPassword123!`というパスワードを使おうとした。

```json
{
  "id": 4000034,
  "text": "The password has been found in data breaches and must no longer be used.",
  "context": {
    "breaches": 3330
  }
}
```

Kratosはデフォルトで[Have I Been Pwned](https://haveibeenpwned.com/)のAPIを使い、パスワードが過去のデータ漏洩に含まれていないかチェックする。`TestPassword123!`は3,330件の漏洩で見つかっていた。

[https://haveibeenpwned.com/Passwords:embed:cite]

なぜ私は思いつかなかったのか。

振り返ると、私の58個のテストは「攻撃者がシステムに対して行う操作」をテストしていた。

- 間違ったパスワードでログインできないこと
- 存在しないユーザーで情報が漏れないこと
- 同時登録で競合状態が起きないこと

これは全て「システムへの攻撃」に対するテストだ。攻撃者がシステムの外側から突破を試みるシナリオ。

HIBPチェックは視点が異なる。「ユーザーが持ち込むリスク」に対処している。

- ユーザーが「password123」を使おうとする
- ユーザーが他のサービスで使い回しているパスワードを登録する
- ユーザーが過去に漏洩したパスワードを選ぶ

これはシステムへの攻撃ではない。ユーザー自身がリスクを持ち込むシナリオだ。私はこのカテゴリを完全に見落としていた。

なぜ見落としたのか。おそらく「ユーザーは正しく行動する」という暗黙の前提があった。パスワード強度のバリデーション（8文字以上、英数字混合など）を入れれば十分だと思っていた。でも、`TestPassword123!`は典型的な強度バリデーションを通過する。英大文字、英小文字、数字、記号、8文字以上。全ての条件を満たしている。にもかかわらず、3,330件の漏洩で見つかっている。

強度バリデーションは「推測しやすいか」をチェックする。HIBPチェックは「既に漏洩しているか」をチェックする。両者は補完関係にある。

Kratosを使うことで、私が想定していなかった脅威カテゴリまでカバーできる。これが「専門家が作ったツールを使う」ことの価値だ。**自分の盲点を、他者の知見で補える**。

E2Eテストではタイムスタンプを含むランダムなパスワードを生成して回避した。

```sh
TEST_PASSWORD="Kratos$(date +%s)E2E!Xk9#mN"
```

本番環境では、この機能を有効にしたまま運用すべきだ。ユーザーに「このパスワードは漏洩しています」と伝えることで、アカウント乗っ取りのリスクを下げられる。

## 環境の起動と動作確認

初期化スクリプトを追加した状態で起動する。

```sh
docker compose up -d
docker compose logs -f
```

ヘルスチェック用エンドポイントにアクセスしてみる。

```sh
# Kratosのヘルスチェック
curl http://localhost:4433/health/ready
# {"status":"ok"}

# Hydraのヘルスチェック
curl http://localhost:4444/health/ready
# {"status":"ok"}
```

両方とも`ok`が返ってきた。

## セルフサービスフローの確認

ブラウザで`http://localhost:4455/registration`にアクセスする。

登録画面が表示される。メールアドレスとパスワードを入力して登録。

次に`http://localhost:4455/login`にアクセス。

ログイン画面が表示される。先ほど登録した認証情報でログイン。

ログイン成功。

これだけだ。拍子抜けするほど簡単だった。

以前、私は以下を実装した。

- `AuthService::register()` — ユーザー登録
- `AuthService::authenticate()` — パスワード検証
- `login_page()` — ログインフォームのHTML
- `login_submit()` — フォーム送信処理
- 58個のテスト

Kratosでは、設定ファイルを書くだけでこれらが全て動く。

## OAuth2フローの確認

OAuth2クライアントを作成する。

```sh
docker compose exec hydra hydra create oauth2-client \
  --endpoint http://localhost:4445 \
  --grant-type authorization_code \
  --response-type code \
  --scope openid,profile,email \
  --redirect-uri http://localhost:8080/callback \
  --name "Test Client"
```

クライアントIDとシークレットが出力される。

ブラウザで認可エンドポイントにアクセスする。

```
http://localhost:4444/oauth2/auth?client_id=<CLIENT_ID>&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=test-state
```

1. HydraがKratos UIにリダイレクト
2. Kratos UIがログイン画面を表示
3. ログイン成功後、Kratosが`login_challenge`をHydraに送信
4. HydraがConsent画面にリダイレクト
5. Consent承認後、認可コードがコールバックURLに返される

以前、私がRustで実装した`login_submit()`の処理を、Kratosが自動で行っている。

```rust
// 前回の実装（不要になった）
pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    let user = state.auth.authenticate(&form.email, &form.password).await?;
    let completed = state.hydra
        .accept_login(&form.login_challenge, &user.id.to_string(), false)
        .await?;
    Ok(Redirect::to(&completed.redirect_to))
}
```

このコードは、もう書く必要がない。

## E2Eテストで確認したこと

実際にAPIを叩いて、フロー全体が動くことを確認した。

### Registration Flow

```sh
# 1. フローを初期化
curl -s -X GET "http://localhost:4433/self-service/registration/api"
# Flow ID: 77ff9653-ccd2-4f91-aeea-8fbb4d67fce7

# 2. 登録を実行
curl -s -X POST "http://localhost:4433/self-service/registration?flow=$FLOW_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "password": "Kratos1767517527E2E!Xk9#mN",
    "traits": {
      "email": "e2etest@example.com",
      "name": { "first": "E2E", "last": "Test" }
    }
  }'
```

```
Registration successful!
Identity ID: 169e0834-4b45-441f-95f8-5adc45d8a3e9
Email: e2etest-1767517527@example.com
Session Token: ory_st_WugR5gisST7SO...
```

Kratosのセルフサービスフローは2段階構成だ。まずフローを初期化してFlow IDを取得し、そのIDを使ってデータを送信する。これにより、CSRFトークンやフローの有効期限が管理される。

### Login Flow

```sh
# 1. フローを初期化
curl -s -X GET "http://localhost:4433/self-service/login/api"

# 2. ログインを実行
curl -s -X POST "http://localhost:4433/self-service/login?flow=$FLOW_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "identifier": "e2etest@example.com",
    "password": "Kratos1767517527E2E!Xk9#mN"
  }'
```

```
Login successful!
Session ID: 8b97d548-8436-48ee-b4fd-8e1c643dac04
Session Token: ory_st_ty15oU5JLIABh...
```

### Session Verification

```sh
curl -s -X GET "http://localhost:4433/sessions/whoami" \
  -H "Authorization: Bearer $SESSION_TOKEN"
```

```
Session valid!
Identity: e2etest-1767517527@example.com
Active: true
```

セッショントークンを使って`/sessions/whoami`を呼ぶと、現在のセッション情報が返ってくる。これは以前私がRustで実装した`JwtService::verify()`に相当する機能だ。

### OAuth2 Authorization Flow

```sh
# OAuth2クライアントを作成
curl -s -X POST "http://localhost:4445/admin/clients" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "e2e-test-client",
    "client_secret": "e2e-test-secret",
    "grant_types": ["authorization_code"],
    "response_types": ["code"],
    "scope": "openid profile email",
    "redirect_uris": ["http://localhost:8080/callback"]
  }'
```

認可エンドポイントにアクセスすると、HydraがKratos UIにリダイレクトする。

```
http://localhost:4444/oauth2/auth?client_id=e2e-test-client&...
  ↓
http://localhost:4455/login?login_challenge=Xv84rhGlXQQrVNL7SlICdNobNbYvcK7z8il...
```

`login_challenge`パラメータが付与されている。Kratos UIはこのチャレンジを使ってHydraと連携し、認証完了後に適切なリダイレクトを行う。

### E2Eテスト結果サマリー

| テスト項目 | 結果 |
|-----------|------|
| Registration Flow | 成功 |
| Login Flow | 成功 |
| Session Verification | 成功 |
| OAuth2 Client Setup | 成功 |
| OAuth2 Authorization Flow | 成功（login_challenge生成確認） |

全てのフローが期待通りに動作した。以前の自前実装と比較して、コード量はゼロになり、機能は増えた。

## 自前実装との比較

| 観点 | 自前実装（02） | Kratos |
|------|---------------|--------|
| パスワード認証 | Argon2id実装 | 組み込み |
| MFA | 未実装 | TOTP, WebAuthn対応 |
| パスワードリセット | 未実装 | フロー組み込み |
| メール確認 | 未実装 | フロー組み込み |
| ソーシャルログイン | 未実装 | OIDC対応 |
| **漏洩パスワードチェック** | **未実装** | **HIBP連携** |
| ログイン画面 | HTML手書き | 公式UI or 自前 |
| セキュリティテスト | 58個書いた | Oryが検証済み |
| 学習コスト | Rust知識 | Kratos設定 |
| カスタマイズ性 | 完全自由 | スキーマ/フック |

特筆すべきは**漏洩パスワードチェック**だ。Have I Been Pwnedとの連携により、過去のデータ漏洩で流出したパスワードを拒否できる。これは以前書いた58個のテストでも考慮していなかった観点だ。Kratosを使うことで、私が思いつかなかったセキュリティ対策まで自動的に適用される。

自前実装は無駄だったのか？いや、違う。

以前の実装で学んだこと——Argon2idのパラメータ、ユーザー列挙攻撃への対策、タイミング攻撃の考慮——これらはKratosの設定を理解する上で役立った。

「なぜこの設定があるのか」が分かるのは、自分で実装した経験があるからだ。

## いつKratosを使うべきか

Kratosを選ぶかどうかは、3つの軸で判断する。

**技術的要件**: カスタマイズの複雑さはどの程度か。Kratosはフック機構やIdentity Schemaで柔軟性を提供するが、「3回目のログインでは必ずCAPTCHAを表示」のような独自フローは難しい。標準的な認証フローなら、Kratosで十分だ。

**組織的要件**: チームにセキュリティ専門家がいるか。いないなら、Kratosに任せた方がいい。脆弱性対応、ベストプラクティスの追従——これらを自前でやるには専門性が必要だ。SOC2やISO27001の監査でも「専門企業の製品を使っています」と答えられる。

**ビジネス要件**: 認証がコア価値か否か。パスワードマネージャーや認証SaaSなら、自前実装に意味がある。ECサイトや社内ツールなら、認証に時間をかけるより本業に集中すべきだ。

私がこれまで関わってきたプロジェクトの8割は、最初からKratosで良かった。残り2割は、レガシーシステムとの統合が複雑すぎるか、認証自体がプロダクトの価値だった。今回のケースでは、学習目的で自前実装から始めたが、本番プロジェクトなら最初からKratosを選ぶ。認証に独自性は不要で、チームにセキュリティ専門家もいない——判断は明確だ。

## 次回予告

Kratosを導入したことで、認証（Authentication）は解決した。ユーザーはログインできる。セッションも管理される。

でも、ログインしたユーザーが「何をできるか」は、まだ決まっていない。

認証と認可は別物だ。認証は「誰であるか」を確認する。認可は「何ができるか」を判断する。次回は、Ory Ketoを使ってZanzibarモデルによる認可システムを構築する。

## おわりに

正直に言うと、Kratosの設定を書いている時、何度か「自分で実装した方が分かりやすいのでは」と思った。YAMLの設定項目が多い。ドキュメントを何度も読み返した。

でも、動いた時の感覚が違う。

これまでに私が書いた数百行のRustコード。それがYAML数十行で置き換わった。しかも、MFAやパスワードリセットなど、私が「次回以降に実装する」と書いていた機能が、既に含まれている。

「自前で作ることの非合理性」

第1回で書いた言葉を思い出した。認可サーバーだけでなく、認証システムも同じだった。

仕様は理解できる。実装もできる。でも、プロダクション品質で検証し続けることは、私たちの仕事ではない。Kratosに移行しても、設定の検証やアップグレード対応、障害時の判断は残る。責任が消えるのではなく、「**実装の責任**」から「**選定と運用の責任**」に形を変える。その上で、認証の基本的な部分——パスワード認証、MFA、セッション管理——は、毎回ゼロから考える問題ではなくなった。

そして、もう1つ学んだことがある。Have I Been Pwnedの件だ。私は58個のテストを書いて「完璧だ」と思っていた。でも、「ユーザーが持ち込むリスク」という視点が完全に抜けていた。専門家が作ったツールを使う価値は、自分の盲点を補えることにある。

レビューコメントに返信しよう。「パスワードリセット機能は？」——Kratosで対応します。

この記事が参考になれば、**読者になったり**、**nwiizo**の**X**や**Github**をフォローしてくれると嬉しいです。

## 参考資料

### Ory Kratos

- [Ory Kratos GitHub](https://github.com/ory/kratos)
- [Ory Kratos Documentation](https://www.ory.com/docs/kratos/)
- [Kratos Quickstart](https://www.ory.com/docs/kratos/quickstart)
- [Identity Schema](https://www.ory.com/docs/kratos/manage-identities/identity-schema)
- [Hydra Integration](https://www.ory.com/docs/kratos/self-hosted/hydra-integration)

### Ory Hydra

- [Ory Hydra Documentation](https://www.ory.com/docs/hydra/)
- [Login and Consent Flow](https://www.ory.com/docs/hydra/concepts/login)

### セキュリティガイドライン

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

### 検証環境

- [ory-kratos-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-kratos-verification)
