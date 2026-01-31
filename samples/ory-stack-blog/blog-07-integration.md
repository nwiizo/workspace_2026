# Ory Stack統合実践：4つのコンポーネントで作る完全な認証認可基盤

このシリーズも7回目になりました。Hydra、Kratos、Keto、Oathkeeperと個別に見てきましたが、「で、結局どう組み合わせるの？」という疑問が残っているかもしれません。今回はその答えを出します。

4つのコンポーネントを統合し、「ログインからAPIアクセスまで」を一気通貫で実現するシステムを構築します。

## 前回からの続き

前回はOathkeeperによるZero Trust API Gatewayパターンを解説しました。Oathkeeper単体でもKetoと連携した認可は実現できましたが、認証はヘッダーベースの簡易的なものでした。

> **前提知識**: このシリーズのblog-01〜06を読んでいることを前提とします。各コンポーネントの基本概念は既知として進めます。

今回は、Kratosによる本格的な認証フローと、Hydraによるトークン発行を組み合わせます。

## なぜ統合が必要なのか

個別のコンポーネントだけでは、以下の課題が残ります。

**Kratosだけの場合**：
- セッション管理はできるが、APIアクセス用のトークン発行ができない
- サードパーティアプリケーションへの認可委譲ができない

**Hydraだけの場合**：
- OAuth2/OIDCのトークン発行はできるが、ユーザー管理UIがない
- ログイン・登録画面を自前で作る必要がある

**Ketoだけの場合**：
- 認可判定はできるが、「誰が」リクエストしているか分からない
- 認証レイヤーが別途必要

**Oathkeeperだけの場合**：
- プロキシとして機能するが、認証情報の発行元がない
- 外部の認証プロバイダーが必要

これらを組み合わせることで、**完全なセルフホスト型の認証認可基盤**が構築できます。

## 統合アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                        クライアント                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Oathkeeper (API Gateway)                    │
│  - リクエストの認証検証                                           │
│  - Ketoへの認可問い合わせ                                         │
│  - バックエンドへのプロキシ                                        │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   Hydra     │      │    Keto     │      │  Backend    │
│  (トークン)  │◀────▶│   (認可)    │      │   (API)     │
└─────────────┘      └─────────────┘      └─────────────┘
         │
         ▼
┌─────────────┐
│   Kratos    │
│  (ユーザー)  │
└─────────────┘
```

### 各コンポーネントの役割

| コンポーネント | 役割 | 連携先 |
|--------------|------|--------|
| **Kratos** | ユーザー登録・ログイン・セッション管理 | Hydra（Login/Consent Provider） |
| **Hydra** | OAuth2/OIDCトークン発行 | Kratos（認証委譲）、Oathkeeper（トークン検証） |
| **Keto** | 細粒度アクセス制御 | Oathkeeper（認可判定） |
| **Oathkeeper** | API Gateway、リクエスト検証 | Hydra（トークン検証）、Keto（認可判定） |

## 実装するユースケース

今回は「**ドキュメント管理SaaS**」を想定します。

### 機能要件

1. **ユーザー認証**: メールアドレス + パスワードでログイン
2. **OAuth2対応**: 外部アプリケーションからのAPIアクセス
3. **細粒度認可**: ドキュメントごとにviewer/editor権限を制御
4. **API Gateway**: 全リクエストをOathkeeperで検証

### 認証フロー

```
1. ユーザーがログイン画面にアクセス
2. Kratosが認証フローを開始
3. 認証成功後、Hydraがアクセストークンを発行
4. クライアントがトークンを使ってAPIにアクセス
5. OathkeeperがHydraでトークンを検証
6. OathkeeperがKetoで認可を確認
7. 認可OKならバックエンドにプロキシ
```

## Docker Compose による統合環境

完全な統合環境をDocker Composeで構築します。

### ディレクトリ構成

```
ory-stack-integration/
├── docker-compose.yml
├── hydra/
│   └── hydra.yml
├── kratos/
│   ├── kratos.yml
│   └── identity.schema.json
├── keto/
│   └── keto.yml
├── oathkeeper/
│   ├── oathkeeper.yml
│   └── rules.yml
├── ui/
│   └── ... (Kratos Self-Service UI)
└── backend/
    └── ... (サンプルAPI)
```

### docker-compose.yml

```yaml
version: "3.8"

services:
  # PostgreSQL - 共有データベース
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: ory
      POSTGRES_PASSWORD: secret
      POSTGRES_MULTIPLE_DATABASES: hydra,kratos,keto
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init-db.sh:/docker-entrypoint-initdb.d/init-db.sh
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ory"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Ory Hydra - OAuth2/OIDC Provider
  hydra:
    image: oryd/hydra:v2.2.0
    command: serve all --dev
    environment:
      DSN: postgres://ory:secret@postgres:5432/hydra?sslmode=disable
      URLS_SELF_ISSUER: http://localhost:4444
      URLS_LOGIN: http://localhost:4433/self-service/login/browser
      URLS_CONSENT: http://localhost:3000/consent
      URLS_LOGOUT: http://localhost:4433/self-service/logout/browser
      SECRETS_SYSTEM: youReallyNeedToChangeThis
      OIDC_SUBJECT_IDENTIFIERS_SUPPORTED_TYPES: public,pairwise
      OIDC_SUBJECT_IDENTIFIERS_PAIRWISE_SALT: youReallyNeedToChangeThis
    ports:
      - "4444:4444"  # Public API
      - "4445:4445"  # Admin API
    depends_on:
      hydra-migrate:
        condition: service_completed_successfully
    restart: unless-stopped

  hydra-migrate:
    image: oryd/hydra:v2.2.0
    command: migrate sql -e --yes
    environment:
      DSN: postgres://ory:secret@postgres:5432/hydra?sslmode=disable
    depends_on:
      postgres:
        condition: service_healthy

  # Ory Kratos - Identity Management
  kratos:
    image: oryd/kratos:v1.1.0
    command: serve --dev --watch-courier
    environment:
      DSN: postgres://ory:secret@postgres:5432/kratos?sslmode=disable
      SERVE_PUBLIC_BASE_URL: http://localhost:4433
      SERVE_ADMIN_BASE_URL: http://localhost:4434
      SELFSERVICE_DEFAULT_BROWSER_RETURN_URL: http://localhost:3000/
      SELFSERVICE_ALLOWED_RETURN_URLS: http://localhost:3000
    volumes:
      - ./kratos:/etc/config/kratos
    ports:
      - "4433:4433"  # Public API
      - "4434:4434"  # Admin API
    depends_on:
      kratos-migrate:
        condition: service_completed_successfully
    restart: unless-stopped

  kratos-migrate:
    image: oryd/kratos:v1.1.0
    command: migrate sql -e --yes
    environment:
      DSN: postgres://ory:secret@postgres:5432/kratos?sslmode=disable
    depends_on:
      postgres:
        condition: service_healthy

  # Ory Keto - Authorization
  keto:
    image: oryd/keto:v0.12.0-alpha.0
    command: serve
    environment:
      DSN: postgres://ory:secret@postgres:5432/keto?sslmode=disable
    volumes:
      - ./keto:/etc/config/keto
    ports:
      - "4466:4466"  # Read API
      - "4467:4467"  # Write API
    depends_on:
      keto-migrate:
        condition: service_completed_successfully
    restart: unless-stopped

  keto-migrate:
    image: oryd/keto:v0.12.0-alpha.0
    command: migrate up -y
    environment:
      DSN: postgres://ory:secret@postgres:5432/keto?sslmode=disable
    volumes:
      - ./keto:/etc/config/keto
    depends_on:
      postgres:
        condition: service_healthy

  # Ory Oathkeeper - API Gateway
  oathkeeper:
    image: oryd/oathkeeper:v0.40.7
    command: serve --config /etc/config/oathkeeper/oathkeeper.yml
    volumes:
      - ./oathkeeper:/etc/config/oathkeeper
    ports:
      - "4455:4455"  # Proxy
      - "4456:4456"  # API
    depends_on:
      - hydra
      - keto
    restart: unless-stopped

  # Self-Service UI (Kratos用)
  ui:
    image: oryd/kratos-selfservice-ui-node:v1.1.0
    environment:
      KRATOS_PUBLIC_URL: http://kratos:4433
      KRATOS_BROWSER_URL: http://localhost:4433
      HYDRA_ADMIN_URL: http://hydra:4445
      PORT: 3000
    ports:
      - "3000:3000"
    depends_on:
      - kratos
      - hydra
    restart: unless-stopped

  # Backend API
  backend:
    image: nginx:alpine
    volumes:
      - ./backend/nginx.conf:/etc/nginx/nginx.conf:ro
    restart: unless-stopped

volumes:
  postgres_data:
```

### Oathkeeper ルール設定

Oathkeeperの設定が統合の要です。

```yaml
# oathkeeper/rules.yml
- id: "public:health"
  match:
    url: "<http|https>://<[^/]+>/health"
    methods: ["GET"]
  authenticators:
    - handler: anonymous
  authorizer:
    handler: allow
  mutators:
    - handler: noop
  upstream:
    url: "http://backend:80"
    strip_path: ""

# Kratos Self-Service エンドポイント（認証不要）
- id: "kratos:public"
  match:
    url: "<http|https>://<[^/]+>/<(self-service|.well-known|schemas)>/<.*>"
    methods: ["GET", "POST", "PUT", "DELETE"]
  authenticators:
    - handler: anonymous
  authorizer:
    handler: allow
  mutators:
    - handler: noop
  upstream:
    url: "http://kratos:4433"
    strip_path: ""

# OAuth2 トークンエンドポイント
- id: "hydra:token"
  match:
    url: "<http|https>://<[^/]+>/oauth2/<.*>"
    methods: ["GET", "POST"]
  authenticators:
    - handler: anonymous
  authorizer:
    handler: allow
  mutators:
    - handler: noop
  upstream:
    url: "http://hydra:4444"
    strip_path: ""

# 保護されたAPI - OAuth2トークン必須 + Keto認可
- id: "api:documents:view"
  match:
    url: "<http|https>://<[^/]+>/api/documents/<[a-zA-Z0-9_-]+>"
    methods: ["GET"]
  authenticators:
    - handler: oauth2_introspection
      config:
        introspection_url: http://hydra:4445/admin/oauth2/introspect
  authorizer:
    handler: remote_json
    config:
      remote: http://keto:4466/relation-tuples/check
      payload: |
        {
          "namespace": "Document",
          "object": "{{ index .MatchContext.RegexpCaptureGroups 2 }}",
          "relation": "viewer",
          "subject_id": "{{ print .Subject }}"
        }
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: '{{ print .Subject }}'
  upstream:
    url: "http://backend:80"
    strip_path: ""

- id: "api:documents:edit"
  match:
    url: "<http|https>://<[^/]+>/api/documents/<[a-zA-Z0-9_-]+>"
    methods: ["PUT", "DELETE"]
  authenticators:
    - handler: oauth2_introspection
      config:
        introspection_url: http://hydra:4445/admin/oauth2/introspect
  authorizer:
    handler: remote_json
    config:
      remote: http://keto:4466/relation-tuples/check
      payload: |
        {
          "namespace": "Document",
          "object": "{{ index .MatchContext.RegexpCaptureGroups 2 }}",
          "relation": "editor",
          "subject_id": "{{ print .Subject }}"
        }
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: '{{ print .Subject }}'
  upstream:
    url: "http://backend:80"
    strip_path: ""
```

### Oathkeeper メイン設定

```yaml
# oathkeeper/oathkeeper.yml
serve:
  proxy:
    port: 4455
    cors:
      enabled: true
      allowed_origins: ["*"]
  api:
    port: 4456

log:
  level: debug
  format: json

access_rules:
  repositories:
    - file:///etc/config/oathkeeper/rules.yml

authenticators:
  anonymous:
    enabled: true
    config:
      subject: anonymous

  oauth2_introspection:
    enabled: true
    config:
      introspection_url: http://hydra:4445/admin/oauth2/introspect
      scope_strategy: exact
      pre_authorization:
        enabled: false

  noop:
    enabled: true

authorizers:
  allow:
    enabled: true

  deny:
    enabled: true

  remote_json:
    enabled: true
    config:
      remote: http://keto:4466/relation-tuples/check
      payload: "{}"

mutators:
  noop:
    enabled: true

  header:
    enabled: true
    config:
      headers: {}

errors:
  fallback:
    - json

  handlers:
    json:
      enabled: true
      config:
        verbose: true
```

## 統合のポイント

### 1. Kratos → Hydra 連携（Login/Consent Provider）

HydraはLogin UIとConsent UIを持ちません。これらをKratosのSelf-Service UIが担当します。

```
Hydra OAuth2 Authorization Request
        │
        ▼
    Login Challenge
        │
        ▼
    Kratos Login Flow
        │
        ▼
    Consent Challenge
        │
        ▼
    Consent UI (承認画面)
        │
        ▼
    Token発行
```

Kratos UIがHydraのAdmin APIを呼び出してチャレンジを処理します。

### 2. Hydra → Oathkeeper 連携（トークン検証）

Oathkeeperは`oauth2_introspection`認証ハンドラでHydraにトークンを問い合わせます。

```yaml
authenticators:
  - handler: oauth2_introspection
    config:
      introspection_url: http://hydra:4445/admin/oauth2/introspect
```

Introspection APIはトークンの有効性を確認し、subjectやscopeを返します。

### 3. Oathkeeper → Keto 連携（認可判定）

トークンが有効なら、次にKetoで細粒度の認可チェックを行います。

```yaml
authorizer:
  handler: remote_json
  config:
    remote: http://keto:4466/relation-tuples/check
    payload: |
      {
        "namespace": "Document",
        "object": "{{ index .MatchContext.RegexpCaptureGroups 2 }}",
        "relation": "viewer",
        "subject_id": "{{ print .Subject }}"
      }
```

`.Subject`はOAuth2 Introspectionから取得したユーザーIDです。

### 4. 認証コンテキストの伝播

認証情報をバックエンドに伝えるため、Mutatorでヘッダーを追加します。

```yaml
mutators:
  - handler: header
    config:
      headers:
        X-User-Id: '{{ print .Subject }}'
```

バックエンドはこのヘッダーを信頼し、ユーザーを識別します。

## 実際のフロー

### ユーザー登録からAPIアクセスまで

**Step 1: ユーザー登録**

```sh
# Kratos Self-Service Registration
curl -X GET http://localhost:4433/self-service/registration/browser
```

Kratos UIが登録フォームを表示し、ユーザーを作成します。

**Step 2: OAuth2クライアント作成**

```sh
# Hydra Admin API
curl -X POST http://localhost:4445/admin/clients \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "my-app",
    "client_secret": "my-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile",
    "redirect_uris": ["http://localhost:8080/callback"]
  }'
```

**Step 3: 認可コードの取得**

```
ブラウザで以下にアクセス:
http://localhost:4444/oauth2/auth?
  client_id=my-app&
  response_type=code&
  scope=openid+profile&
  redirect_uri=http://localhost:8080/callback&
  state=random-state
```

1. Hydraがログインチャレンジを発行
2. Kratos UIにリダイレクト
3. ユーザーがログイン
4. Consent画面で承認
5. 認可コードがリダイレクトURIに返される

**Step 4: トークン交換**

```sh
curl -X POST http://localhost:4444/oauth2/token \
  -u "my-app:my-secret" \
  -d "grant_type=authorization_code" \
  -d "code=AUTHORIZATION_CODE" \
  -d "redirect_uri=http://localhost:8080/callback"
```

アクセストークンとリフレッシュトークンが返されます。

**Step 5: 権限の付与**

```sh
# Keto Write API
curl -X PUT http://localhost:4467/admin/relation-tuples \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "viewer",
    "subject_id": "USER_ID"
  }'
```

**Step 6: APIアクセス**

```sh
# Oathkeeper経由でAPIにアクセス
curl -H "Authorization: Bearer ACCESS_TOKEN" \
  http://localhost:4455/api/documents/doc1
```

Oathkeeperが以下を実行：
1. Hydraでトークンを検証
2. Ketoで`viewer`権限を確認
3. 権限OKならバックエンドにプロキシ

## 統合時の注意点

### 1. サービス間の依存関係

起動順序が重要です。`depends_on`と`healthcheck`を適切に設定してください。

```yaml
oathkeeper:
  depends_on:
    hydra:
      condition: service_healthy
    keto:
      condition: service_healthy
```

### 2. 内部通信 vs 外部通信

- **内部通信**: Docker内部DNS（`http://hydra:4444`）
- **外部通信**: ホストからのアクセス（`http://localhost:4444`）

設定ファイルでは両方を意識する必要があります。

### 3. シークレット管理

本番環境では以下を必ず変更してください：

- `SECRETS_SYSTEM`: Hydraのシステムシークレット
- `OIDC_SUBJECT_IDENTIFIERS_PAIRWISE_SALT`: ペアワイズsubjectのソルト
- データベースパスワード
- OAuth2クライアントシークレット

### 4. TLS終端

本番環境ではOathkeeperの前段にTLS終端を配置します：

```
Client → Load Balancer (TLS) → Oathkeeper → Backend
```

### 5. セッションとトークンの使い分け

| 用途 | 推奨 |
|------|------|
| ブラウザSPA | Kratosセッション + CSRF保護 |
| モバイルアプリ | OAuth2アクセストークン |
| サーバー間通信 | OAuth2 Client Credentials |
| サードパーティ連携 | OAuth2 Authorization Code |

## トラブルシューティング

### Oathkeeperが401を返す

1. トークンが有効か確認：
```sh
curl -X POST http://localhost:4445/admin/oauth2/introspect \
  -d "token=ACCESS_TOKEN"
```

2. Introspection URLが正しいか確認
3. Hydraのログを確認

### Oathkeeperが403を返す

1. Ketoの権限を確認：
```sh
curl -X POST http://localhost:4466/relation-tuples/check \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "viewer",
    "subject_id": "USER_ID"
  }'
```

2. ルールのpayloadテンプレートを確認
3. `.Subject`が正しく取得できているか確認

### ログイン後にConsentに遷移しない

1. HydraのURLs設定を確認
2. Kratos UIがHydra Admin APIにアクセスできるか確認
3. ネットワーク設定を確認

## まとめ

4つのOryコンポーネントを統合することで、以下を実現できます。

- **Kratos**: ユーザー管理とセッション
- **Hydra**: OAuth2/OIDCトークン発行
- **Keto**: Zanzibarモデルによる細粒度認可
- **Oathkeeper**: Zero Trust API Gateway

統合の要点は：

1. **Hydra-Kratos連携**: Login/Consent Providerパターン
2. **Oathkeeper-Hydra連携**: OAuth2 Introspectionによるトークン検証
3. **Oathkeeper-Keto連携**: Remote JSON Authorizerによる認可判定
4. **コンテキスト伝播**: ヘッダーMutatorによるユーザー情報の転送

自前でこれらを実装することを想像してみてください。OAuth2サーバー、ユーザー管理、認可エンジン、API Gateway。それぞれが複雑で、組み合わせるとさらに複雑になります。Ory Stackはこの複雑さを、設定ファイルとDocker Composeで管理可能な形に落とし込んでいます。

次回は、この統合環境を本番運用するためのベストプラクティスを解説します。

---

**シリーズ記事**
- [blog-01: Ory Stack入門](/blog-01-introduction)
- [blog-02: 実装編](/blog-02-implementation)
- [blog-03: Hydra詳解](/blog-03-hydra)
- [blog-04: Kratos詳解](/blog-04-kratos)
- [blog-05: Keto詳解](/blog-05-keto)
- [blog-06: Oathkeeper詳解](/blog-06-oathkeeper)
- **blog-07: 統合実践** ← 今回
- blog-08: 本番運用のベストプラクティス（予定）
