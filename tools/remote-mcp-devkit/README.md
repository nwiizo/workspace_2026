# remote-mcp-devkit

Remote MCP server + OAuth 2.1 の **conformance + 認証フロー検証ループ** を、ローカル開発環境だけで短く・再現可能に・cleanup 込みで回すための小さな CLI。

設計仕様は [`docs/development-spec.md`](docs/development-spec.md)、進行中の作業と判断は [`TODO.md`](TODO.md) を参照。

## ジョブ理論での位置づけ

「Remote MCP 実装そのもの」ではなく、**「Remote MCP + OAuth を Claude Desktop / Claude Web / ChatGPT Web のような実機クライアントに貼る前後の検証ループを短くし、公開・認証・routing・cleanup の失敗を再現可能にする」** ために雇われる隙間家具。実装の完成度ではなく「貼る前に何が壊れているかを 30 秒で説明できる artifact を残す」ことを最優先する。

## 何ができるか (現時点で動くもの)

| サブコマンド | 役割 |
| --- | --- |
| `up`          | localhost に自己署名 HTTPS proxy + mock OAuth 2.1 AS を立ち上げ、`--upstream` MCP server を後ろに置く |
| `smoke`       | PRM / 401 challenge / AS metadata / authorize redirect の conformance を検査、`report.md` / `network.json` / `network.har` / `curl-equivalent.sh` を出力 |
| `client-dance` | fake AI client として PRM → 401 → DCR → authorize → token → authorized MCP call を完走 |
| `oauth-code`  | 実 AS のログイン画面経由で `code` → token を取る (ブラウザ click だけ人間 / agent に任せ、tool は callback listener と token exchange を担当) |
| `down`        | session 状態を冪等に畳む |
| `doctor`      | port / cert / upstream / 古い session の点検 |
| `init-config` | サンプル yaml を stdout に出す |
| `list`        | session state を一覧表示 |

完了済み機能の詳細は [TODO.md `## 完了した改善`](TODO.md) を参照。未着手の P1 (tunnel run / k8s adapter / provider doctor) も同じ TODO で管理。

## 設計の境界

| 採用 | 採用しない |
| --- | --- |
| HTTP 仕様の外形検証 (PRM / 401 / AS metadata / authorize redirect) | OAuth 2.1 AS 本体実装 (token rotation / 永続 store) |
| PKCE 付き token exchange までを 1 コマンドで | UI 完全自動操作 (Claude Desktop / ChatGPT Web の selector 追従) |
| HAR / curl による証跡 | 本番 IdP の恒久設定変更 |
| 短時間 HTTPS tunnel の補助 (将来) | 常時公開 hosting / 固定 domain / 監視 |
| `kubernetes-traefik` adapter (将来) | 汎用 Kubernetes 管理 UI |
| `provider doctor` (Zitadel 等、将来) | secret vault / credential manager |
| 単一 Rust バイナリ | Playwright / Chromium / Node / kubectl / ngrok を必須依存にすること |

依存最小化は強い制約。外部 CLI 依存を持ち込むときは P1 の adapter / provider 境界に閉じ込め、core の動作には影響させない。

## クイックスタート

```sh
# 1) 自分の MCP server を別 process で立てる (例として同梱の simple-upstream を使う)
cargo run --example simple-upstream -- --port 18080

# 2) devkit を起動 (https://localhost:8443 を公開、後ろに MCP を置く)
remote-mcp-devkit up --upstream http://127.0.0.1:18080

# 3) 別ターミナルで spec 適合性を測る
remote-mcp-devkit smoke \
  --base-url https://localhost:8443 \
  --client-profile claude \
  --resource auto \
  --out artifacts/smoke

# 4) full OAuth dance を回して 200 まで届くか確認
remote-mcp-devkit client-dance \
  --base-url https://localhost:8443 \
  --out artifacts/dance
```

成果物:

- `artifacts/smoke/report.md` — 検査結果の人間向けまとめ
- `artifacts/smoke/network.json` — 全 request/response の JSON
- `artifacts/smoke/network.har` — DevTools "Import HAR" で開ける形式
- `artifacts/smoke/curl-equivalent.sh` — 同じ request を curl で再現するスクリプト

## エージェント向け規律

`remote-mcp-devkit` はコーディングエージェント (Claude Code / Cursor 等) がサブプロセスで呼び出す用途を主に想定する。人間が手で叩いても困らないが、設計上の primary は machine-readable な動作。

- **stdout** — 各サブコマンドが出すのは `serde_json::to_string_pretty` で書ける単一の値 (1 コマンド = 1 JSON object)。`up` は session 開始時に 1 行の JSON event を出す。
- **stderr** — 進捗バナー、`open this URL` 案内、SIGINT 通知などの人間向けメッセージはすべてこちら。
- **exit code** — `0` 成功、`1` report が `passed()==false`、anyhow error は非ゼロで stderr に出る。
- **interactive 要素** — 1 つだけ: `oauth-code` がブラウザのクリックを待つ。`--open-browser` を渡さない限り tool 側ではブラウザを開かないので、agent は提示された URL を別 client に渡せばよい。

```sh
# session 立ち上げ、stdout は JSON 一行
remote-mcp-devkit up --upstream http://127.0.0.1:18080 --no-smoke 2>/dev/null | jq '.session_id, .base_url'

# spec smoke、stdout の JSON を全部食って通った/落ちたを判定
remote-mcp-devkit smoke --base-url https://localhost:8443 --client-profile claude --out artifacts/smoke 2>/dev/null \
  | jq '[.checks[] | {name, passed, messages}]'
```

## 2 通りの組合せ

### A. mock AS で MCP server の 401 challenge を検証する

devkit の mock AS と HTTPS proxy をフロントに置き、upstream を自分の MCP server に向ける。OAuth 周りは devkit が用意するので、MCP server は「Bearer 受け取ったときの挙動」だけ実装すればよい。

```sh
remote-mcp-devkit up --upstream http://127.0.0.1:YOUR_MCP_PORT
remote-mcp-devkit client-dance --base-url https://localhost:8443
```

### B. 自前 OAuth AS の conformance を検証する

自分の AS (例: 自前の `mcp-oauth-as`) を localhost で起動し、その base URL に対して直接 `smoke` を打つ。AS 単体は PRM を出さないので PRM check は意図的に FAIL になる (PRM は MCP resource side の責務)。AS metadata / authorize の項だけ pass すれば OK。

```sh
remote-mcp-devkit smoke \
  --base-url https://localhost:YOUR_AS_PORT \
  --mcp-path /mcp \
  --client-profile claude \
  --resource auto \
  --out artifacts/as-conformance
```

### C. 実 AS を devkit の前段に差し込む (pass-through)

mock AS を使わず、本物の AS を devkit の後ろに置く。PRM と 401 challenge は devkit が引き続き提供し、`/oauth/*` と `/.well-known/oauth-authorization-server` だけ実 AS に proxy する。`x-forwarded-host` / `x-forwarded-proto` を付けるので、AS は `https://localhost:8443` を自分の issuer として認識する。

```sh
remote-mcp-devkit up \
  --upstream http://127.0.0.1:18080 \
  --upstream-oauth http://127.0.0.1:7000

remote-mcp-devkit smoke --base-url https://localhost:8443 --out artifacts/real-as
```

## `oauth-code`: 実 AS のブラウザログインを通して token まで取る

`smoke` では PRM / 401 / AS metadata / authorize redirect までしか証明できない。Zitadel 等の実 AS でユーザがログイン画面を抜けた後の token exchange を証跡化したいときは `oauth-code` を使う。

```sh
remote-mcp-devkit oauth-code \
  --base-url https://localhost:8443 \
  --client-profile claude \
  --resource auto \
  --redirect-uri 'http://127.0.0.1:18454/callback' \
  --open-browser \
  --out artifacts/oauth-code
```

実行内容:

1. PRM → AS metadata discovery → PKCE 付き authorize URL を生成
2. `redirect_uri` のホスト:ポートでローカル axum listener を起動
3. authorize URL を stderr に出す (+ `--open-browser` 時は OS の `open`/`xdg-open` も呼ぶ)
4. ユーザ (または agent が指示する別 client) がブラウザでログイン
5. listener が `code` + `state` を捕捉、`/oauth/token` で交換
6. `access_token` / `refresh_token` / `id_token` は **redact** し、TokenSummary に `has_*` + `*_len` だけ残す
7. JWT は header / payload のみ decode (署名検証はしない、JWKS は取りに行かない)

意図的にやらないこと: login form selector 設定、screenshot、page text 取得、retry、IdP 固有処理。Playwright / Chromium / Node 依存は持ち込まない。

## 設定ファイル (`remote-mcp-devkit.yaml`)

```yaml
version: 1
server:
  host: localhost
  port: 8443
  scheme: https
upstreams:
  mcp:
    url: http://127.0.0.1:18080
  # oauth:
  #   url: http://127.0.0.1:7000  # pass-through mode
profile:
  mcp_path: /mcp
  forwarded_proto: https
  oauth:
    client: local-fake-client
    scopes:
      - mcp:read
```

`remote-mcp-devkit init-config` でサンプル yaml を stdout に出力できる。

## TLS

`up` 起動時に `rcgen` で `localhost` 用の自己署名証明書を `.remote-mcp-devkit/sessions/<host>.cert.pem` に生成・キャッシュする。クライアント側で trust するか、smoke / client-dance / oauth-code は `danger_accept_invalid_certs(true)` で進む。

公開鍵を trust store に入れたい場合:

```sh
# macOS
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain \
  .remote-mcp-devkit/sessions/localhost.cert.pem
```

## ファイル

| パス | 役割 |
| --- | --- |
| `src/main.rs` / `src/cli.rs`             | CLI entry、subcommand dispatch、stdout=JSON / stderr=human の出力規律 |
| `src/proxy.rs`                            | axum HTTPS reverse proxy、401 challenge、`X-Forwarded-Proto/Host` 付与、`--upstream-oauth` pass-through |
| `src/mock_as.rs`                          | mock OAuth 2.1 AS (PRM / AS metadata / authorize / token / DCR / revoke / `/_devkit/state` 等) |
| `src/smoke.rs`                            | conformance smoke、`SmokeOptions` (client profile / resource / CIMD assert)、HAR + curl artifact 生成 |
| `src/client_dance.rs`                     | fake AI client の OAuth dance |
| `src/oauth_code.rs`                       | 実 AS の auth-code flow を localhost callback listener で捕捉、token を redact、JWT header/payload を decode |
| `src/doctor.rs`                           | port / cert / upstream / 古い session の点検 |
| `src/cleanup.rs`                          | `down` 冪等実装 |
| `src/tls.rs`                              | `rcgen` 自己署名証明書 (state dir cache) |
| `src/pkce.rs`                             | RFC 7636 S256 (reference vector 込み) |
| `src/state.rs`                            | session state JSON |
| `src/config.rs`                           | yaml schema |
| `examples/simple-upstream.rs`             | 最小 MCP-shape upstream (検証用 stub) |
| `examples/remote-mcp-devkit.yaml`         | サンプル config |
| `tests/integration.rs`                    | smoke / client-dance / oauth-code / introspection / pass-through を実 HTTPS で end-to-end 検証 |

## ビルド・テスト

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

新規 crate 0 / 外部 CLI 依存 0 (Node / Chromium / kubectl / ngrok 不要)。
