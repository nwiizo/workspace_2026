# remote-mcp-devkit 開発仕様

## 目的

`remote-mcp-devkit` は、Remote MCP server と OAuth 2.1 / Protected Resource Metadata / CIMD の接続性を、ローカル開発環境で再現よく検証するための汎用CLIである。

特定のプロダクトやIdPに閉じず、次のような開発中の問題を短いループで確認できるようにする。

- MCP clientに貼るURLが正しいmetadataを返しているか
- 未認証MCP requestが仕様どおりの401 challengeを返すか
- OAuth authorization server metadataがAI clientに必要な能力を表明しているか
- authorization redirectとtoken endpointが期待したbackendへ到達しているか
- ngrokなどの一時HTTPS公開を使ったあと、公開endpointやport-forwardを閉じ忘れていないか
- IdPのissuer、host、redirect URI、public URLの不整合を原因付きで切り分けられるか

## 背景課題

Remote MCPの実機検証では、ローカル開発環境だけでは閉じない制約がある。

- Claude DesktopやWebクライアントはRemote Custom ConnectorとしてHTTPS URLを要求する
- 自己署名CAをAI client側へ信頼させる手段がないことが多い
- ngrok free accountでは公開endpointが1つだけになる
- MCP resource、OAuth shim、IdPを1つのpublic host配下でpath routingする必要がある
- IdPがhostやissuerを強く検証する場合、public hostへの置換だけでは動かない
- 手動でport-forward、ngrok、Kubernetes patchを行うとcleanup漏れが起きやすい

このツールは、これらを「検証session」としてまとめて起動、診断、証跡保存、停止する。

## 非目標

- 本番向けOAuth2/OIDC Authorization Serverを実装しない
- refresh token rotation、JWT署名鍵、永続token storeを持たない
- 本番IdPの恒久設定を自動変更しない
- Claude DesktopやChatGPT WebのUI完全自動操作を必須機能にしない
- production secretをstate fileやreportへ保存しない
- tunnelを常時運用する仕組みにしない

## 設計方針

### 汎用化境界

coreはMCP / OAuth検証に集中し、プロダクト固有の処理はadapterとして分離する。

| 層 | 責務 | 例 |
|---|---|---|
| core | session、routing model、smoke checks、report、cleanup | product非依存 |
| tunnel provider | public HTTPS endpointの起動停止 | ngrok、cloudflared、tailscale funnel |
| environment adapter | local backendへtrafficを流す | kubernetes-traefik、docker-compose、localhost |
| idp adapter | issuer/host/redirect URIの診断と任意patch | zitadel、keycloak、auth0 |
| client profile | AI clientごとの期待値 | claude、chatgpt、generic |

### 2つの検証モード

1. `local-conformance`
   - localhost HTTPSとmock OAuth ASで仕様適合性を検証する
   - 実機AI clientは使わない
   - 完全にローカルで閉じる

2. `remote-client`
   - ngrokなどで一時HTTPS endpointを作り、Claude Desktopなどの実機から接続する
   - 1 public endpointだけでMCP resource、OAuth shim、IdPをpath routingする
   - 起動時にsession stateを作り、停止時に必ずcleanupする

## 用語

### Session

1回の検証実行単位。`up` で作成し、`down` で破棄する。

保存する情報:

- session id
- created_at
- public base URL
- tunnel provider
- 起動したprocess id
- 適用したrouting resource
- patch前のresource snapshot
- artifact directory
- cleanup status

### Route Profile

1つのpublic host配下で、pathをどのupstreamへ流すかの定義。

```yaml
routes:
  - upstream: mcp
    paths:
      - /mcp
      - /.well-known/oauth-protected-resource
  - upstream: oauth-shim
    paths:
      - /.well-known/oauth-authorization-server
      - /oauth/authorize
      - /oauth/token
      - /oauth/revoke
  - upstream: idp
    path_prefixes:
      - /oauth/v2/
      - /ui/
      - /assets/
```

### Client Profile

AI clientごとのOAuth期待値。

初期profile:

- `claude`
  - CIMD URL: `https://claude.ai/oauth/claude-code-client-metadata`
  - token endpoint auth method: `none`
  - localhost callbackを許容
- `chatgpt`
  - CIMD URL: `https://chatgpt.com/oauth/client.json`
  - token endpoint auth method: `private_key_jwt`
  - `jwks_uri` が必要
- `generic`
  - static client idまたは任意CIMD URLを指定

## CLI仕様

### `remote-mcp-devkit up`

一時検証sessionを開始する。

```sh
remote-mcp-devkit up \
  --config remote-mcp-devkit.yaml \
  --upstream http://127.0.0.1:18080
```

処理:

1. configを読み込む
2. 古いsessionと衝突するprocessを検出する
3. (将来) tunnel providerの認証状態を確認する
4. localhost HTTPS proxy + mock OAuth 2.1 AS を起動する
5. (`tunnel run` 経由のとき) public base URLを取得する
6. (`kubernetes-traefik` adapter 経由のとき) routingを適用する
7. 必要に応じて`X-Forwarded-Proto=https`を補正する
8. (`provider doctor` 経由のとき) IdP host/issuer/redirect URI不整合を診断する
9. smoke checkを実行する
10. MCP URL、停止コマンド、artifact pathを表示する

成功出力 (stdout に 1 行 JSON、stderr に banner):

```text
{"event":"session_started","session_id":"20260521-010203","base_url":"https://localhost:8443", ...}

┌─ remote-mcp-devkit session started
│  session     : 20260521-010203
│  base_url    : https://localhost:8443
│  mcp_url     : https://localhost:8443/mcp
└─ Stop with: remote-mcp-devkit down --session 20260521-010203
```

### `remote-mcp-devkit down`

検証sessionを停止し、変更を戻す。

```sh
remote-mcp-devkit down --session 20260521-010203
```

要件:

- 冪等である
- tunnel processが既に終了していてもcleanupを続行する
- routing resourceをsnapshotから戻す
- public URLが閉じたことを確認する
- 失敗したcleanup項目を次の手順付きでreportする
- `--force`なしで別sessionのresourceを削除しない

### `remote-mcp-devkit smoke`

既存URLに対してHTTP観点の仕様検証だけを行う。

```sh
remote-mcp-devkit smoke \
  --base-url https://example.ngrok-free.dev \
  --mcp-path /mcp \
  --client-profile claude \
  --out artifacts/smoke
```

検証項目:

- `GET /.well-known/oauth-protected-resource`
  - 200
  - `resource == <base-url>/mcp`
  - `authorization_servers` に `<base-url>` が含まれる
- `POST /mcp` without Authorization
  - 401
  - `WWW-Authenticate: Bearer`
  - `error="invalid_token"`
  - `resource_metadata="<base-url>/.well-known/oauth-protected-resource"`
- `GET /.well-known/oauth-authorization-server`
  - 200
  - `authorization_endpoint`
  - `token_endpoint`
  - `client_id_metadata_document_supported`
  - `code_challenge_methods_supported` に `S256`
- `GET /oauth/authorize?...`
  - 302/303/307
  - redirect先が想定IdPに向く
  - `state` と `code_challenge` が保持される
  - `client_id` translation構成では変換後IDが含まれる

成果物:

- `report.md`
- `network.json`
- `curl-equivalent.sh`
- `headers/*.txt`
- `bodies/*.json`

### ブラウザ観点の証跡 (core 非提供)

devkit は Playwright / Chromium / Node を core 依存に取り込まない。実機ブラウザ経由の証跡が必要な場合は次の二段で行う。

1. `remote-mcp-devkit oauth-code --callback-mode manual` で PKCE 付き authorize URL を発行し、callback URL を stdin から受け取る経路を開く。
2. 外部の Playwright CLI (P2 の Playwright CLI recipe artifact 参照) でブラウザを開き、`requestfailed` などの event で callback URL を捕捉して devkit の stdin に渡す。

理由:

- core に Node / Chromium / selector 保守が混じると、依存最小化と「貼る前の HTTP / OAuth 証跡を出す」主目的を侵食するため。
- UI selector は AI client UI の変化に追従する必要があり、devkit の射程と更新サイクルが噛み合わないため。

### `remote-mcp-devkit doctor`

ローカル環境の既知問題を検出する。

検査項目:

- tunnel CLIが存在する
- tunnel providerが認証済み
- free accountで複数endpointを開こうとしていない
- local portが空いている
- Kubernetes contextが期待値と一致する
- routing対象serviceが存在する
- IngressRouteやHTTPRouteのCRDが存在する
- IdP issuerとpublic URLが矛盾していない
- redirect URIがclient profileと一致する
- 古いsessionやport-forwardが残っていない

### `remote-mcp-devkit client-dance`

実機AI clientを使わず、fake clientとしてOAuth danceを再生する。

用途:

- MCP serverのPRMと401 challengeを検証する
- OAuth shimのauthorize/token proxyを検証する
- AI client UIなしでcode exchangeまで確認する

注意:

- production ASの完全な代替ではない
- local-conformanceではmock ASを使ってよい
- remote-clientでは実際のAS/token endpointへ到達するsmokeに留める

## Config仕様

```yaml
version: 1

workspace:
  state_dir: .remote-mcp-devkit/sessions
  artifact_dir: .remote-mcp-devkit/artifacts

tunnel:
  provider: ngrok
  local_port: 18080
  command: ngrok
  ttl: 30m

adapter:
  type: kubernetes-traefik
  context: kind-example
  namespace: app
  traefik_namespace: traefik
  traefik_service: traefik
  entrypoint: web

upstreams:
  mcp:
    namespace: app
    service: mcp-server
    port: http
    scheme: http
  oauth-shim:
    namespace: app
    service: oauth-shim
    port: http
    scheme: http
  idp:
    namespace: idp
    service: idp
    port: http
    scheme: http

profiles:
  claude-ngrok:
    mode: remote-client
    client_profile: claude
    mcp_path: /mcp
    forwarded_proto: https
    routes:
      - upstream: mcp
        paths:
          - /mcp
          - /.well-known/oauth-protected-resource
      - upstream: oauth-shim
        paths:
          - /.well-known/oauth-authorization-server
          - /oauth/authorize
          - /oauth/token
          - /oauth/revoke
      - upstream: idp
        path_prefixes:
          - /oauth/v2/
          - /ui/
          - /assets/
    oauth:
      scopes:
        - mcp:read
      resource: auto
```

## Adapter仕様

### `kubernetes-traefik`

MVP対象。

責務:

- Traefik serviceへのport-forward
- public host用IngressRouteの作成
- 変更前resourceのsnapshot保存
- `stop`時のresource復元

禁止:

- production namespaceへの自動patch
- secret値のreport出力

### `localhost-reverse-proxy`

ローカルで閉じる検証向け。

責務:

- `https://localhost:<port>` でMCP/OAuth path routerを起動
- 自己署名証明書を生成する
- fake clientとcurl検証では証明書検証を明示的に扱う

制約:

- Claude Desktop Remote Custom Connectorの実機検証には使えない可能性が高い
- AI clientにprivate CAを信頼させる機能は提供しない

### `ngrok-single-endpoint`

ngrok無料アカウント向け。

責務:

- 1 public endpointだけを使う
- path routingでMCP/OAuth/IdPを分配する
- session終了時にngrokを必ず停止する

制約:

- IdPがpublic hostをinstance domainとして受け入れない場合は、doctorで明示する
- IdP DB domainの自動書き換えはMVP対象外

## IdP診断仕様

IdP adapterは自動修正より先に原因分類を行う。

分類例:

- `instance-domain-mismatch`
- `issuer-mismatch`
- `redirect-uri-mismatch`
- `client-id-mapping-missing`
- `token-endpoint-invalid-client`
- `authorization-code-not-found`
- `audience-mismatch`
- `resource-host-alias-missing`

reportには以下を出す。

- 観測したURL
- HTTP status
- redacted response body
- 推定原因
- 次に試す具体的な設定名またはコマンド

## Security要件

- state fileにaccess token、refresh token、client secretを書かない
- reportではsecretらしい値をredactする
- public URLを起動時と停止時に明示する
- `--ttl` 経過時に自動cleanupする
- SIGINT/SIGTERMでcleanupを試行する
- cleanup失敗時は危険な公開状態が残っているかを強調表示する
- `remote-client` modeではsessionごとにartifact directoryを分離する

## dev cluster adapter 例

プロジェクト固有の知識は core に入れず、config または adapter に閉じ込める。

```yaml
adapter:
  type: kubernetes-traefik
  context: kind-dev-cluster

upstreams:
  mcp:
    namespace: app
    service: mcp-server
    port: svc
    scheme: http
  oauth-shim:
    namespace: app
    service: oauth-shim
    port: svc
    scheme: http
  idp:
    namespace: idp
    service: idp
    port: http
    scheme: http

idp:
  type: zitadel
  internal_issuer: http://login.example.localhost
  known_domain_error: Instance not found
```

プロジェクト固有 doctor 項目の例 (config から差し込む):

- OAuth shim の client id mapping file が pod 内で読める
- auth gateway の audience file が pod 内で読める
- public host 検証時の env (`MCP_RESOURCE_HOST_ALIASES` 等) が設定されている
- IdP dev instance が tunnel host を instance domain として受け入れるか
- shim の `/oauth/*` route と IdP 本体の `/oauth/v2/*` route が衝突していない

## 実装フェーズ

### Phase 1: local-conformance

- Rust実装の既存`remote-mcp-devkit`を基礎にする
- `up`、`down`、`smoke`、`client-dance`、`doctor`
- localhost HTTPS + mock AS
- artifact保存

完了条件:

- 任意upstream MCP serverに対してPRM、401、AS metadata、authorize、token、authorized MCP requestを検証できる

### Phase 2: remote-client smoke

- ngrok provider
- kubernetes-traefik adapter
- one-public-host routing
- cleanup付きsession
- (Playwright screenshot は core では提供しない。 `oauth-code --callback-mode manual` と外部 Playwright CLI recipe (P2) を組合せる)

完了条件:

- 1つのngrok endpointでMCP PRM、401、AS metadata、authorize redirectまで検証できる
- `down`でngrok、port-forward、一時routingが消える

### Phase 3: IdP diagnostics

- Zitadel adapter
- issuer/host/redirect URIの分類
- client id mapping fileの存在確認
- token invalid_client/code not foundの原因分類

完了条件:

- 実機接続が失敗した場合でも、次に直すべき設定がreportに残る

### Phase 4: real-client assist

- Claude Desktop向け手順生成 (`handoff`)
- ChatGPT向け手順生成 (`handoff`)
- Playwright CLI recipe artifact (`oauth-code --callback-mode manual` と組合せて使う、外部 CLI で実行)

完了条件:

- 実機AI clientで必要なURL、期待される画面、失敗時の確認点がsession artifactにまとまる

## 受け入れ条件

- `local-conformance` はインターネット公開なしで完走する
- `remote-client` は1つのpublic endpointだけで検証できる
- `up` 後にClaude Desktopへ貼るMCP URLが表示される
- `smoke` がPRM、401 challenge、AS metadata、authorize redirectを検証する
- (Playwright 等のブラウザ自動操作は core では提供せず、`oauth-code --callback-mode manual` と外部 recipe で代替する)
- `down` が冪等で、公開endpointとport-forwardを閉じる
- IdP host/issuer問題をdoctorが原因分類する
- product固有処理がcoreに混入しない

## 未解決事項

- AI clientがlocalhost HTTPSやprivate CAを許容しない場合の完全ローカル実機検証は、client側制約に依存する
- IdPのinstance domainを一時変更する機能は安全境界を別途設計する
- Claude Desktop UI自動操作はOS依存が強いため、初期版では手順生成に留める
- token endpointまで実ASで完走するには、issuerとbrowser-facing hostを一致させる必要がある
