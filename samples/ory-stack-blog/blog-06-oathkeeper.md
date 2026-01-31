# Ory OathkeeperでAPI Gatewayを構築する ― Zero Trustモデルの実践

## 前回からの続き

前回の記事では、Ory Ketoを導入してZanzibarモデルによる認可システムを構築した。「論理演算ではなくグラフ探索」という発想の転換で、複雑な権限モデルをシンプルに表現できるようになった。

> **前提知識**: この記事はOry Stackシリーズの続編です。Ory Kratos（認証）、Ory Hydra（OAuth2/OIDC）、Ory Keto（認可）の基本的な役割を理解している前提で進めます。

でも、ふと気づいた。

```rust
// 毎回これを書くのか？
async fn edit_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    // Ketoで認可チェック
    if !check_permission(&state.keto, &user.id, &doc_id, "editor").await? {
        return Err(AppError::Forbidden);
    }
    // ...
}
```

すべてのエンドポイントで`check_permission`を呼び出す。書き忘れたら？呼び出し順序を間違えたら？

認可ロジックをアプリケーションコードに埋め込んでいる限り、ヒューマンエラーのリスクは消えない。

「アプリケーションの外で認可を強制できないか？」

答えはOry Oathkeeperにあった。

## Ory Oathkeeperとは

[https://github.com/ory/oathkeeper:embed:cite]

[https://www.ory.com/docs/oathkeeper:embed:cite]

Ory Oathkeeperは「Identity & Access Proxy（IAP）」だ。GoogleのBeyondCorpモデルを実装したZero Trustプロキシと言える。

従来のセキュリティモデルは「境界防御」だった。ファイアウォールの内側は信頼し、外側は信頼しない。でも、クラウドネイティブな環境では境界が曖昧になる。マイクロサービスが複数のネットワークに分散し、内部通信も攻撃対象になりうる。

BeyondCorp / Zero Trustモデルは違う。**すべてのリクエストを検証する**。内部からのリクエストも、外部からのリクエストも、同じルールで認証・認可する。

```
┌─────────────────────────────────────────────────────────────┐
│                      Oathkeeper                              │
│              (Identity & Access Proxy)                       │
├─────────────────────────────────────────────────────────────┤
│ 1. Rule Matching    - URLパターンにマッチするルールを検索   │
│ 2. Authentication   - トークン検証、セッション確認          │
│ 3. Authorization    - 権限チェック（Ketoに委譲）            │
│ 4. Mutation         - ヘッダー付与、トークン変換            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │    Backend      │
                    │  (認可済みの    │
                    │   リクエストのみ) │
                    └─────────────────┘
```

Oathkeeperを通過したリクエストは、すでに認証・認可済みだ。バックエンドは「誰がアクセスしているか」を信頼できる。

## アーキテクチャの全体像

これまで構築してきたOry Stackに、Oathkeeperを追加する。

```
【Oathkeeper導入前】
Browser → Backend → Keto (権限チェック)
                 → Kratos (セッション確認)

【Oathkeeper導入後】
Browser → Oathkeeper → Backend
              │
              ├── Kratos (認証)
              ├── Hydra (トークン検証)
              └── Keto (認可)
```

**変わったこと**: 認証・認可がバックエンドの「前」で行われる。バックエンドに到達するリクエストは、すでに検証済みだ。

**変わらないこと**: Kratos、Hydra、Ketoの役割は同じ。Oathkeeperはこれらを「オーケストレーション」するだけだ。

## Access Rulesの設計

Oathkeeperの核心は「Access Rules」だ。どのURLに対して、どのように認証・認可するかを宣言的に定義する。

```yaml
# oathkeeper/rules.yml
- id: "api:documents:protected"
  match:
    url: "http://backend:8080/api/documents/<**>"
    methods:
      - GET
      - POST
      - PUT
      - DELETE
  authenticators:
    - handler: jwt
      config:
        jwks_urls:
          - http://hydra:4444/.well-known/jwks.json
        token_from:
          header: Authorization
  authorizer:
    handler: remote_json
    config:
      remote: http://keto:4466/relation-tuples/check
      payload: |
        {
          "namespace": "Document",
          "object": "{{ .MatchContext.RegexpCaptureGroups.0 }}",
          "relation": "{{ if eq .MatchContext.Method "GET" }}viewer{{ else }}editor{{ end }}",
          "subject_id": "{{ .Subject }}"
        }
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: "{{ .Subject }}"
          X-User-Email: "{{ .Extra.email }}"
```

これが1つのルールだ。分解して説明する。

### Authenticator（認証）

```yaml
authenticators:
  - handler: jwt
    config:
      jwks_urls:
        - http://hydra:4444/.well-known/jwks.json
```

`jwt`ハンドラーは、AuthorizationヘッダーからJWTトークンを取り出し、HydraのJWKSエンドポイントで署名を検証する。検証に失敗すれば、リクエストは401で拒否される。

他にも複数のAuthenticatorがある。

| Handler | 用途 |
|---------|------|
| `jwt` | JWTトークン検証 |
| `oauth2_introspection` | OAuth2トークンのイントロスペクション |
| `cookie_session` | Kratosセッションクッキー検証 |
| `bearer_token` | APIキー検証 |
| `anonymous` | 認証なし（公開API用） |
| `noop` | 何もしない（テスト用） |

### Authorizer（認可）

```yaml
authorizer:
  handler: remote_json
  config:
    remote: http://keto:4466/relation-tuples/check
    payload: |
      {
        "namespace": "Document",
        "object": "{{ .MatchContext.RegexpCaptureGroups.0 }}",
        "relation": "{{ if eq .MatchContext.Method "GET" }}viewer{{ else }}editor{{ end }}",
        "subject_id": "{{ .Subject }}"
      }
```

`remote_json`ハンドラーは、外部サービス（Keto）に権限チェックを委譲する。テンプレート構文で動的な値を埋め込める。

- `{{ .Subject }}` — 認証されたユーザーID
- `{{ .MatchContext.Method }}` — HTTPメソッド（GET, POST, etc.）
- `{{ .MatchContext.RegexpCaptureGroups.0 }}` — URLパスからキャプチャした値

GETリクエストなら`viewer`権限、それ以外なら`editor`権限をチェックする。Ketoが`{"allowed": false}`を返せば、リクエストは403で拒否される。

### Mutator（変換）

```yaml
mutators:
  - handler: header
    config:
      headers:
        X-User-Id: "{{ .Subject }}"
        X-User-Email: "{{ .Extra.email }}"
```

認証・認可を通過したリクエストに、ヘッダーを付与する。バックエンドは`X-User-Id`ヘッダーを見るだけで、誰がアクセスしているか分かる。

**これが重要だ**。バックエンドはトークンを検証する必要がない。Oathkeeperが検証済みであることを信頼できる。

## Docker Composeで動かす

既存のOry Stack環境にOathkeeperを追加する。

```yaml
services:
  # ... (postgres, kratos, hydra, keto は既存)

  oathkeeper:
    image: oryd/oathkeeper:v0.40.7
    command: serve --config /etc/config/oathkeeper/oathkeeper.yml
    volumes:
      - ./oathkeeper:/etc/config/oathkeeper:ro
    ports:
      - "4455:4455"  # Proxy
      - "4456:4456"  # API
    environment:
      LOG_LEVEL: debug
    depends_on:
      hydra:
        condition: service_healthy
      keto:
        condition: service_healthy
    networks:
      - ory

  backend:
    build: ./backend
    environment:
      # Oathkeeperを信頼するため、トークン検証は不要
      TRUST_PROXY_HEADERS: "true"
    networks:
      - ory
    # ポートを公開しない — Oathkeeper経由でのみアクセス可能
```

**ポイント**: バックエンドのポートを外部に公開しない。すべてのトラフィックはOathkeeperを経由する。これがZero Trustモデルの実践だ。

## Oathkeeper設定ファイル

```yaml
# oathkeeper/oathkeeper.yml
serve:
  proxy:
    port: 4455
  api:
    port: 4456

access_rules:
  repositories:
    - file:///etc/config/oathkeeper/rules.yml

authenticators:
  jwt:
    enabled: true
    config:
      jwks_urls:
        - http://hydra:4444/.well-known/jwks.json
      scope_strategy: wildcard

  cookie_session:
    enabled: true
    config:
      check_session_url: http://kratos:4433/sessions/whoami
      preserve_path: true
      extra_from: "@this"
      subject_from: "identity.id"

  anonymous:
    enabled: true
    config:
      subject: anonymous

authorizers:
  remote_json:
    enabled: true
    config:
      remote: http://keto:4466/relation-tuples/check
      forward_response_headers_to_upstream: []

  allow:
    enabled: true

  deny:
    enabled: true

mutators:
  header:
    enabled: true

  noop:
    enabled: true

errors:
  fallback:
    - json

  handlers:
    json:
      enabled: true
      config:
        verbose: true
```

## Access Rulesの実例

実際のAPIに対するルールを定義する。

```yaml
# oathkeeper/rules.yml

# 公開API（認証不要）
- id: "api:health"
  match:
    url: "http://backend:8080/health"
    methods: [GET]
  authenticators:
    - handler: anonymous
  authorizer:
    handler: allow
  mutators:
    - handler: noop

# ログインユーザーのみ（認可チェックなし）
- id: "api:profile"
  match:
    url: "http://backend:8080/api/profile"
    methods: [GET]
  authenticators:
    - handler: cookie_session
  authorizer:
    handler: allow
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: "{{ .Subject }}"

# ドキュメントAPI（Ketoで認可）
- id: "api:documents:list"
  match:
    url: "http://backend:8080/api/documents"
    methods: [GET]
  authenticators:
    - handler: jwt
  authorizer:
    handler: allow  # 一覧取得は認証のみ
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: "{{ .Subject }}"

- id: "api:documents:detail"
  match:
    url: "http://backend:8080/api/documents/<[0-9a-f-]+>"
    methods: [GET, PUT, DELETE]
  authenticators:
    - handler: jwt
  authorizer:
    handler: remote_json
    config:
      remote: http://keto:4466/relation-tuples/check
      payload: |
        {
          "namespace": "Document",
          "object": "{{ index .MatchContext.RegexpCaptureGroups 0 }}",
          "relation": "{{ if eq .MatchContext.Method "GET" }}viewer{{ else }}editor{{ end }}",
          "subject_id": "{{ .Subject }}"
        }
  mutators:
    - handler: header
      config:
        headers:
          X-User-Id: "{{ .Subject }}"
```

## バックエンドの変化

Oathkeeper導入前と後で、バックエンドのコードがどう変わるか。

**導入前**:

```rust
async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Document>, AppError> {
    // 1. トークン検証
    let token = headers
        .get("Authorization")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims = state.jwt.verify(token).await?;
    let user_id = claims.sub;

    // 2. Ketoで認可チェック
    if !state.keto.check("Document", &doc_id, "viewer", &user_id).await? {
        return Err(AppError::Forbidden);
    }

    // 3. ビジネスロジック
    let doc = state.db.get_document(&doc_id).await?;
    Ok(Json(doc))
}
```

**導入後**:

```rust
async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Document>, AppError> {
    // Oathkeeperが検証済み — X-User-Idを信頼する
    let user_id = headers
        .get("X-User-Id")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;

    // ビジネスロジックに集中
    let doc = state.db.get_document(&doc_id).await?;
    Ok(Json(doc))
}
```

**認証コードが消えた。認可コードも消えた**。バックエンドはビジネスロジックに集中できる。

「でも、X-User-Idヘッダーを偽装されたら？」

いい質問だ。だからこそ、バックエンドへの直接アクセスを遮断する。すべてのトラフィックはOathkeeperを経由する。Oathkeeperが付与したヘッダーのみを信頼する。

## E2Eテストで確認したこと

実際にAPIを叩いて、Oathkeeperが正しく動作することを確認した。

```sh
# 1. 認証なしでアクセス → 401
curl -i http://localhost:4455/api/documents/doc1
# HTTP/1.1 401 Unauthorized

# 2. 無効なトークンでアクセス → 401
curl -i http://localhost:4455/api/documents/doc1 \
  -H "Authorization: Bearer invalid-token"
# HTTP/1.1 401 Unauthorized

# 3. 権限のないユーザーでアクセス → 403
curl -i http://localhost:4455/api/documents/doc1 \
  -H "Authorization: Bearer $BOB_TOKEN"
# HTTP/1.1 403 Forbidden
# (bobはdoc1のviewerではない)

# 4. 権限のあるユーザーでアクセス → 200
curl -i http://localhost:4455/api/documents/doc1 \
  -H "Authorization: Bearer $ALICE_TOKEN"
# HTTP/1.1 200 OK
# (aliceはdoc1のviewer)

# 5. 閲覧者が編集を試みる → 403
curl -i -X PUT http://localhost:4455/api/documents/doc1 \
  -H "Authorization: Bearer $CHARLIE_TOKEN" \
  -d '{"title": "hacked"}'
# HTTP/1.1 403 Forbidden
# (charlieはviewerだがeditorではない)
```

### テスト結果サマリー

| テスト項目 | 結果 |
|-----------|------|
| 認証なしアクセス | 401 ✅ |
| 無効トークン | 401 ✅ |
| 権限なしユーザー | 403 ✅ |
| 権限ありユーザー（閲覧） | 200 ✅ |
| 閲覧者の編集試行 | 403 ✅ |
| 編集者の編集 | 200 ✅ |

すべてのケースで期待通りの動作を確認した。

## Decision APIモード

Oathkeeperには2つの動作モードがある。

**Reverse Proxyモード（デフォルト）**: Oathkeeperがリバースプロキシとして動作し、バックエンドにリクエストを転送する。

**Decision APIモード**: 既存のAPI Gateway（Nginx, Envoy, Kong, AWS API Gateway）と連携する。

```
【Reverse Proxyモード】
Browser → Oathkeeper:4455 → Backend

【Decision APIモード】
Browser → Nginx → Backend
            ↓
       Oathkeeper:4456/decisions
```

Decision APIモードでは、NginxがOathkeeperの`/decisions`エンドポイントに問い合わせ、認可判断を取得する。既存のインフラを活かしつつ、Oathkeeperの認可機能を利用できる。

```nginx
# nginx.conf
location /api/ {
    auth_request /oathkeeper-decision;
    auth_request_set $user_id $upstream_http_x_user_id;
    proxy_set_header X-User-Id $user_id;
    proxy_pass http://backend:8080;
}

location = /oathkeeper-decision {
    internal;
    proxy_pass http://oathkeeper:4456/decisions$request_uri;
    proxy_pass_request_body off;
    proxy_set_header Content-Length "";
}
```

## 次回予告

Oathkeeperを導入したことで、認証・認可がアプリケーションの「外」で行われるようになった。バックエンドはビジネスロジックに集中できる。

でも、Access Rulesの`payload`テンプレートを見て気づいただろうか。

```yaml
"relation": "{{ if eq .MatchContext.Method "GET" }}viewer{{ else }}editor{{ end }}"
```

これはシンプルなケースだ。「GETなら閲覧者、それ以外なら編集者」。

でも、もっと複雑な権限モデルが必要になったらどうする？「親フォルダの権限を子ファイルに継承」「編集者は自動的に閲覧もできる」——Relation Tupleだけでは表現が難しい。

次回は、OPL（Ory Permission Language）を使って、複雑な権限モデルを宣言的に定義する方法を解説する。

## おわりに

正直に言うと、Oathkeeperの設定には手間取った。Access Rulesのテンプレート構文、複数のAuthenticatorの使い分け、エラーハンドリングの設定——覚えることは多い。

でも、動いた時の感覚が違う。

バックエンドのコードを見返してみた。認証コードが消えている。認可コードも消えている。残っているのは、ビジネスロジックだけだ。

「自前で作ることの非合理性」——シリーズを通じて何度も思い出す言葉だ。認可の実装をアプリケーションから追い出すことで、2つのことが起きた。

1つ目は、**関心の分離**。バックエンドはビジネスロジックに集中できる。認証・認可は「インフラの責務」になった。

2つ目は、**一貫性の保証**。すべてのリクエストが同じルールで検証される。エンドポイントごとに`check_permission`を書き忘れる心配がない。

「でも、設定ファイルを書き間違えたら？」

その通りだ。コードのバグが設定のバグに置き換わっただけ、という見方もできる。でも、**設定ファイルはレビューしやすい**。YAMLで宣言的に書かれたルールは、コードに埋め込まれた条件分岐より、はるかに見通しがいい。

前回まででKratos（認証）とKeto（認可）を導入した。今回Oathkeeper（プロキシ）を導入した。これでOry Stackの主要コンポーネントが揃った。

「このエンドポイントに認可を追加して」——Access Rulesを書くだけで対応できます。

この記事が参考になれば、**読者になったり**、**nwiizo**の**X**や**Github**をフォローしてくれると嬉しいです。

## 参考資料

### Ory Oathkeeper

- [Ory Oathkeeper GitHub](https://github.com/ory/oathkeeper)
- [Ory Oathkeeper Documentation](https://www.ory.com/docs/oathkeeper)
- [Access Rules Reference](https://www.ory.com/docs/oathkeeper/api-access-rules)

### BeyondCorp / Zero Trust

- [BeyondCorp: A New Approach to Enterprise Security](https://research.google/pubs/pub43231/)
- [Zero Trust Architecture (NIST SP 800-207)](https://csrc.nist.gov/publications/detail/sp/800-207/final)

### 関連プロジェクト

- [ory-oathkeeper-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-oathkeeper-verification)
- [ory-keto-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-keto-verification)
- [ory-kratos-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-kratos-verification)
