# TODO

## ジョブ理論での整理

`remote-mcp-devkit` は「Remote MCP 実装そのもの」ではなく、「Remote MCP + OAuth を実機クライアントに貼る前後の検証ループを短くし、公開・認証・routing・cleanup の失敗を再現可能にする」ために雇われるツール。

### Main Job

開発者が Remote MCP server / OAuth shim / IdP 設定を変更したとき、Claude Desktop / Claude Web / ChatGPT Web に URL を貼る前に、ローカル環境だけで仕様違反と配線ミスを短時間で検出し、実機検証が必要な場合だけ一時 HTTPS endpoint を開き、検証後に確実に閉じたい。

### Job Steps

1. ローカルで MCP endpoint を指定する。
2. PRM / 401 challenge / AS metadata / authorize redirect / token exchange のどこまで通るかを知る。
3. 失敗した場合、MCP 実装・OAuth shim・IdP・tunnel・Kubernetes routing のどこが原因か切り分ける。
4. 必要なときだけ ngrok / cloudflared 等で HTTPS endpoint を短時間公開する。
5. Claude Desktop 等に貼る URL、検証結果、curl / HAR 等の証跡を得る。
6. 検証後に port-forward / tunnel / temporary ingress / env patch を閉じ忘れず戻す。

### Functional / Emotional / Social Jobs

- **Functional**: MCP Authorization 仕様の外形を HTTP だけで検証する / CIMD・static client・DCR 風の client profile 差分を再現する / 実 AS のブラウザログインを通して authorization code と token を取得し JWT payload を読む / 一時的に開いたものを必ず閉じる。
- **Emotional**: 「Claude Desktop で試したらなぜか動かない」を画面の目視ではなく artifact で説明できる / 公開 URL を開けっぱなしにしていないと確信できる / OAuth 仕様を毎回思い出さなくてもツールが失敗箇所を絞り込んでくれる。
- **Social**: PR レビューや検収時に `report.md` / `network.har` / `curl-equivalent.sh` を添えて説明できる / 非担当者でも同じ検証を再実行できる / 「ローカルだけで通った」「ngrok 実機でも通った」「Claude Desktop UI で通った」を段階別に証跡化できる。

### 2026-05-21 再評価: どのジョブに雇われるか

`remote-mcp-devkit` が雇われる場面は、次の3つに絞る。ここから外れる機能は、便利そうでも core には入れない。

| Job | 開発者が本当に知りたいこと | 必要な出力 |
| --- | --- | --- |
| 貼る前に壊れている箇所を知る | URL、PRM、401 challenge、AS metadata、authorize redirect、token exchange のどこで落ちるか | PASS/FAIL report、HAR、curl、原因候補 |
| 実機検証を短時間だけ安全に開く | ngrok 等の public URL、single endpoint routing、temporary env patch が正しく閉じるか | session state、public URL、cleanup report |
| レビュー/検収に渡せる証跡を作る | 人間が見た画面ではなく、HTTP と設定の事実で説明できるか | redacted token summary、JWT payload、再実行コマンド |

逆に、次のジョブには雇わない。

| 雇わない Job | 理由 |
| --- | --- |
| IdP や OAuth AS の代替になる | セキュリティ責務が大きすぎ、今回の薄い検証ツールの境界を越える。 |
| Claude Desktop / ChatGPT の操作を完全自動化する | UI selector 保守が主目的になり、Remote MCP/OAuth の診断精度に寄与しない。 |
| 開発クラスタ全体を復旧・運用する | Kubernetes / Elasticsearch / DB の一般管理は別ツールの責務。devkit は検証に必要な最小診断だけを見る。 |
| public endpoint を常時ホストする | 検証後に速やかに閉じることが価値なので、常時公開は逆方向。 |
| 本番設定を永続変更する | IaC とレビューの責務。devkit は一時 patch と差分報告まで。 |

## 採否判断 (Single Source of Truth)

### 採用する機能

最新の優先度・状態・採用理由を 1 つの表に集約する。未着手 (`❌`) / 部分実装 (`△`) の acceptance criteria は `## 次のジョブ詳細` を参照。

| 優先度 | 機能 | 状態 | 採用理由 / 仕様メモ |
| --- | --- | --- | --- |
| P0 | `smoke` | ✅ 完了 | PRM / 401 / AS metadata / authorize redirect の外形を最短で検証する。 |
| P0 | `oauth-code` | ✅ 完了 (listener / manual 両モード) | 実 AS の authorization code + token exchange を `smoke` 以後で証跡化する。callback は redirect_uri のローカル listener で受けるか、`--callback-mode manual` で Playwright 等が捕捉した full callback URL を stdin から渡す (Claude CIMD `http://localhost/callback` のように port 80 listener を立てられないケース向け)。 |
| P0 | HAR / curl / report artifact bundle | ✅ 完了 | `report.md` / `network.json` / `network.har` / `curl-equivalent.sh` を session 単位で出力。redacted token summary / JWT payload も含む。 |
| P0 | `cleanup` / stale 検出 | ✅ 完了 | session state / cert / port の片付け、broken state best-effort recovery、`doctor` advisory による `ngrok` / `kubectl port-forward` 検出まで。kubectl 必須項目は `kubernetes-traefik adapter` へ移動。 |
| P1 | `tunnel run` | ❌ 未着手 | Claude Desktop / Web / ChatGPT 実機検証で必要な短時間 HTTPS public URL。ngrok / cloudflared、single endpoint、TTL、SIGINT cleanup 必須。 |
| P1 | `k8s doctor` | ❌ 未着手 | 実機検証前に control-plane / target deployment / service endpoint の壊れ方を早期検出する。read-only、復旧はしない。`controller-manager down` / `endpoint empty` / `rollout stuck` を分類する。 |
| P1 | `kubernetes-traefik adapter` (`k8s snapshot` / `k8s restore`) | ❌ 未着手 | dev cluster の IngressRoute / Middleware / env patch / rollout を snapshot + restore で安全に戻す。owner label で自分が作った変更だけを対象にする。 |
| P1 | `provider doctor zitadel` | ❌ 未着手 | OAuth 失敗を実装側か IdP 設定側かに切り分ける。provider 固有処理は trait / adapter 境界に閉じる。 |
| P1 | client handoff report | ❌ 未着手 | Claude Desktop に貼る時に迷わないよう、`mcp_url` / 期待されるブラウザ遷移 / 成功・失敗時の確認点 / stop コマンドを 1 つの artifact にまとめる。 |
| P2 | mock AS state / seed | ✅ 完了 | MCP server 単体 CI で OAuth dance を省略するための `/_devkit/state` / `/_devkit/clients` / `/_devkit/tokens`。実機検証の代替ではない。 |
| P2 | Playwright CLI recipe artifact | ❌ 未着手 | core に Chromium 依存を入れない代わりに、外部 Playwright CLI の実行例・保存先・期待 event を artifact にまとめる recipe を提供する。 |

### 採用しない機能

| 機能 | 不要な理由 | 代替 |
| --- | --- | --- |
| OAuth2/OIDC Authorization Server 本体 | token 発行・refresh rotation・JWT 署名・永続 token store を持つと、開発補助ツールではなく AS 実装になる。セキュリティ責務が過大。 | mock AS は local test 用に限定。 |
| core 内蔵 Playwright / Chromium runner | Node / Chromium / selector 保守が core の依存最小化と衝突する。UI selector の追従が主目的を侵食する。 | `oauth-code --callback-mode manual` と外部 Playwright CLI recipe を組み合わせる。 |
| Claude Desktop / ChatGPT Web UI 完全自動操作 | UI は頻繁に変わり、selector 保守が主目的を侵食する。 | 貼る URL と HTTP / OAuth 証跡を出し、最終 UI 確認は人間に任せる。 |
| Claude Desktop 設定ファイルの自動編集 | OS / バージョン差が大きく、誤編集時の復旧責任が重い。 | `client handoff report` に手順と URL を出す。 |
| 本番 IdP の client 登録 / redirect URI 永続変更 | IaC / 運用手順の責務。devkit が直接変更すると差分管理と監査が壊れる。 | 一時 patch は snapshot/restore 付き adapter、恒久変更は IaC。 |
| 常時公開 tunnel hosting / 複数 tunnel provider 同時起動 | 検証 session 用の短時間公開だけでよい。固定 domain・監視・再起動・証明書運用は別カテゴリ。複数同時起動は ngrok free single endpoint 制約と衝突する。 | 1 session = 1 public endpoint。provider は差し替え可能。 |
| 汎用 Kubernetes 管理 UI / CLI | port-forward / temporary routing / env patch / restore だけで十分。Pod 管理・deploy 管理の一般化はスコープ外。 | 一般 Pod / deploy 管理は kubectl / 専用ツール。 |
| secret vault / credential manager | password や token を長期保存すると攻撃面が増える。 | env / file から読み、一時使用し、artifact では redaction。 |
| API data seeding / Elasticsearch 管理 | MCP OAuth 接続性の検証ツールであり、アプリのデータ基盤復旧ツールではない。 | 別の dev environment tool で扱う。devkit は下流 endpoint が空かどうかだけ検出する。 |
| Production conformance monitor | 継続監視は SRE / monitoring 基盤の責務。 | 必要なら別プロジェクトで probe 化する。 |

### 保留する機能

| 機能 | 保留理由 | 着手条件 |
| --- | --- | --- |
| ChatGPT `private_key_jwt` full diagnostics | Claude の URL 貼り付け UX 確立が先。ChatGPT は `jwks_uri` / assertion / clock skew など診断面が広い。 | Claude profile で `smoke` + `oauth-code` + 実機検証が安定した後。 |
| DCR full flow | 今回の主経路は CIMD translation。DCR は別 client 互換性のための追加検証。 | DCR-only client をサポート対象に入れる時。 |
| multi-tenant matrix runner | まず 1 tenant の深い artifact 品質を固めるべき。 | single tenant artifact が安定し、検収で複数 tenant 確認が必要になった時。 |
| docker-compose adapter | Kubernetes / Traefik が現在の主要ジョブ。 | docker-compose 運用の Remote MCP 案件が出た時。 |

## 仕様メンテナンス課題

実装と仕様の言い回しがずれている箇所を、追加機能とは別ジョブとして畳む。

- ~~`docs/development-spec.md` に `remote-mcp-devkit playwright` が独立コマンドとして残っているが、現在の方針 (`採用しない機能 / core 内蔵 Playwright runner`) と矛盾する。~~ → 2026-05-22 解消。`oauth-code --callback-mode manual` + 外部 Playwright CLI recipe を組合せる手順として「ブラウザ観点の証跡 (core 非提供)」節へ書き換え済み。
- ~~`docs/development-spec.md` の CLI 仕様は `start` / `stop` で書かれている一方、実装と README は `up` / `down`。~~ → 2026-05-22 解消。spec を `up` / `down` に統一済み。alias は追加しない。
- `remote-client` mode は未着手なので、`tunnel run` / `k8s snapshot` / `k8s restore` / `k8s doctor` のコマンド名と責務を README / spec / TODO の 3 ファイルで揃える。今は本 TODO の `## 採否判断` を single source of truth とする。

## Arrove Duo MCP OAuth 検証で必要になった追加要件 (2026-05-22)

今回のゴールは、Claude Desktop の `claude_desktop_config.json` / `npx` / `mcp-remote` を使う回避策ではなく、Remote Custom Connector UI に `https://<tenant>.duo.arrove.jp/mcp` を貼り付けてブラウザ認証後に使えること。devkit はこの実機 UX を完全代替するのではなく、貼る前にローカル / dev cluster の HTTP・OAuth・routing 不備を潰すために使う。

### P0: QA 確定条件を profile に固定する

Arrove Duo の検収条件が更新されたため、devkit の Arrove Duo profile は次を前提条件として扱う。これは設定ファイル型の開発者向け workaround ではなく、最終的な URL 貼り付け UX を守るためのチェックリストである。

受け入れ条件:

- Claude Desktop の必達 UX は `https://<tenant>.duo.arrove.jp/mcp` を Remote Custom Connector に貼り付け、ブラウザ認証後に MCP が使えること。
- `claude_desktop_config.json` 編集、ローカル `npx`、`mcp-remote` を検収対象に含めない。
- 本番/検収用のユーザー接点は `/mcp` の URL だけとし、`/.well-known/*` と `/oauth/*` は技術的に必要な補助 endpoint として扱う。
- Claude Desktop は P0、Claude Web / ChatGPT Web は同じ CIMD 経路で確認できる範囲のベストエフォートにする。
- `tools/list` 成功だけでは不足。Arrove Duo が公開する全 tool を OAuth bearer で呼び出せることを検証対象にする。
- `mcp-remote` 互換や既存 workaround 互換は残さない。現在の MCP 利用者が内部開発者のみであり、将来 UX を汚す互換層を持つ価値が低いため。
- `aud` は project id と resource URI の両方を検証できる前提で report に表示する。どちらかしか取れない場合は profile failure ではなく `needs-design-decision` として分類する。

不要なこと:

- Claude Desktop の local config を自動編集しない。
- `mcp-remote` 用の callback 互換を追加しない。
- 開発者向け workaround が通ったことを最終 UX の合格として表示しない。

### P0: Claude Desktop URL 貼り付け検収パック

今回の検収は `claude_desktop_config.json` / `npx` / `mcp-remote` ではなく、Claude Desktop の Remote Custom Connector へ `https://<tenant>.duo.arrove.jp/mcp` を貼り付ける UX が合格線になる。devkit は Desktop UI を代替しないが、検収者が見るべき事実と、貼る前に壊れている箇所を 1 つの artifact にまとめる。

受け入れ条件:

- `remote-mcp-devkit handoff --profile arrove-duo --client claude-desktop` が、検収者向けに以下を出す:
  - 貼り付ける MCP URL は `/mcp` だけであること
  - ブラウザ認証で期待される issuer / login domain / tenant domain
  - Desktop に戻った後に確認する `tools/list` と全 tool call の状態
  - 失敗時に確認する PRM / AS metadata / redirect_uri / resource / audience の順番
  - ngrok を使っている場合は検証用 URL であり、本番手順ではないこと
- `smoke` / `oauth-code` / `verify-tools` / `negative-replay` の既存 artifact を読み、検収前ステータスを次の粒度で表示する:
  - `not_ready`: PRM / AS metadata / OAuth round-trip のどこかが未確認または失敗
  - `ready_for_desktop_manual_check`: HTTP / OAuth / tools/list までは通っているが Desktop UI は未確認
  - `desktop_verified`: Desktop UI で URL 貼り付け、browser auth、tool call まで確認済み
  - `blocked_by_client`: Desktop 側の UI / connector 制約で止まっている
- Desktop UI で確認した結果は、手動入力の evidence として追記できる:
  - connector URL
  - 検証日時
  - tenant
  - 成功した tool 数 / 失敗した tool 名
  - 失敗分類
- Desktop UI の結果を入れるときも access token / refresh token / cookie / authorization code は保存しない。

不要なこと:

- `claude_desktop_config.json` を編集しない。今回の最終 UX と違うため。
- `npx mcp-remote` の設定例を検収パックに出さない。内部回避策が合格条件に見えるため。
- Claude Desktop UI を selector で完全自動操作しない。UI 自動化が壊れても Remote MCP / OAuth の実装品質とは別問題になるため。

### P0: CIMD redirect_uri 互換性診断

検証中に `redirect_uri is not allowed by client_id metadata document` が発生した。これは OAuth shim や Zitadel token exchange 以前に、CIMD の `redirect_uris` と実際の authorize request が合っていないときに起きるため、devkit が貼る前に明示診断する。

受け入れ条件:

- `smoke --profile arrove-duo --client-profile claude` は、Claude CIMD URL を取得し、authorize request の `redirect_uri` が metadata の `redirect_uris` に含まれるか検査する。
- `redirect_uri` が不一致の場合、次を report に出す:
  - requested redirect URI
  - metadata に含まれていた redirect URI の redacted list
  - どの component がその redirect URI を決めているかの候補 (`client profile`, `override flag`, `shim config`, `Zitadel app`)
  - 修正先が devkit ではなく、MCP OAuth 実装 / client profile / IdP 設定であること
- Claude と ChatGPT の client profile を分けて診断する。ChatGPT では `private_key_jwt` / `jwks_uri` 診断も別項目として表示する。
- metadata fetch では redirect を追わず、HTTPS / size limit / content-type / SSRF guard を検査する。診断ツールが攻撃面にならないようにする。
- dev / local profile で一時的な redirect URI を許可した場合は、report に `dev-only override` と明示する。

不要なこと:

- devkit が Claude / ChatGPT の公開 CIMD を書き換えたように扱わない。CIMD は外部 client の事実として読むだけ。
- 不一致を自動で握りつぶして fallback redirect URI に変更しない。Desktop 実機で再発するため。
- 本番手順として `localhost` redirect を要求しない。Claude Desktop の公開 client metadata に従う。

### P0: staging debug session handoff

ローカルで大半を確認できても、Claude Desktop の URL 貼り付け UX は public HTTPS / 実 client 到達性が残る。開発者が staging 環境でデバッグさせてもらう場合、devkit は「何を一時変更し、誰が apply / rollback し、どの証跡を残すか」を短く出す。

受け入れ条件:

- `remote-mcp-devkit staging-plan --profile arrove-duo` が、staging 作業依頼に貼れる checklist を生成する:
  - 使う tenant / user / 初期データ
  - 反映対象 component (`arrove-duo-mcp`, `mcp-oauth-as`, `auth-gateway`, routing)
  - 必要な config / secret / env の有無
  - apply 担当者、log 閲覧担当者、rollback 担当者
  - 検証開始時刻と終了目安
  - rollback 条件
  - 検証後に削除・無効化する一時設定
- local artifact から staging で再確認すべき項目だけを抽出する:
  - local で PASS したものは証跡リンクとして添付
  - local で `requires-public-https` / `requires-staging` になったものだけを staging checklist に上げる
- staging で必要な最小ログを明示する:
  - `/mcp` 401 challenge
  - PRM / AS metadata response
  - authorize redirect
  - token endpoint 4xx / 5xx
  - auth-gateway JWT verification failure
  - direct replay 401
- staging 用 plan は IaC 変更を生成しない。人間がレビューできる依頼文と確認観点だけを生成する。

不要なこと:

- devkit から staging/prod に直接 apply しない。
- staging を ngrok の代替として扱わない。staging は public HTTPS 実環境検証、ngrok は一時ローカル公開検証。
- staging tenant のデータ作成や業務データ投入を devkit の責務にしない。

### P1: ngrok single endpoint session の厳格化

ngrok は無料アカウント前提で 1 endpoint だけを使い、検証後は速やかに閉じる。URL が変わるため、devkit は古い connector URL / token cache による誤判定を避ける注意を artifact に出す。

受け入れ条件:

- `tunnel run --provider ngrok --single-endpoint` は、1 つの public origin で以下がすべて同居していることを検査する:
  - `/mcp`
  - `/.well-known/oauth-protected-resource`
  - `/.well-known/oauth-protected-resource/mcp`
  - `/.well-known/oauth-authorization-server`
  - `/oauth/authorize`
  - `/oauth/token`
  - `/oauth/revoke`
- session 起動時に `expires_at` / `stop_command` / `public_url` を artifact に出す。
- session 終了時に ngrok process / port-forward / temporary route が残っていないか doctor を走らせる。
- public URL が前回 session と変わった場合、handoff report に次を明記する:
  - Claude Desktop の古い connector URL を使わない
  - 古い token cache / connector state が残る場合は削除してから再追加する
  - ただし `claude_desktop_config.json` を検収手順にしない
- ngrok interstitial / warning page が出る場合は `blocked_by_tunnel_provider` として分類する。

不要なこと:

- 複数 ngrok endpoint を前提にした route 設計をしない。
- ngrok URL を staging / production のURLとして文書化しない。
- tunnel を常駐化しない。session TTL と cleanup を必須にする。

### P0: local E2E feasibility report

ローカル環境で end-to-end の開発・確認まで実施できることが望ましいが、Claude Desktop Remote Connector の実機 UX は public HTTPS と Anthropic 側から到達可能な URL を要求する可能性が高い。devkit は「ローカルでどこまで確認済みか」と「技術的にローカルだけでは残る制約」を機械的に文章化する。

受け入れ条件:

- `remote-mcp-devkit arrove-duo local-e2e-report` などで、次を段階別に出す:
  - local dev cluster の Ready 状態
  - Traefik port-forward 経由の `/mcp`
  - root PRM と path-aware PRM
  - 未認証 401 challenge
  - AS metadata
  - authorize redirect
  - Zitadel login 到達
  - token exchange
  - OAuth bearer での `tools/list`
  - OAuth bearer での全 tools verifier
- 各段階を `verified-local` / `not-run` / `blocked-by-localhost` / `requires-public-https` / `requires-staging` のいずれかに分類する。
- `requires-public-https` の理由は具体的に書く:
  - Claude Desktop Remote Connector は `http://*.localhost` をユーザーが貼る本番形 URL として扱えない。
  - self-signed / private CA HTTPS は Anthropic 側の fetch とブラウザ認証の信頼境界を満たせない。
  - ngrok free endpoint は検証用であり、URL 変更・interstitial・single endpoint 制約があるため本番手順に含めない。
- staging で必要な確認項目を checklist として出す:
  - URL 貼り付け
  - browser auth
  - Claude Desktop への復帰
  - 全 tool call
  - tenant 分離
  - direct replay / missing resource / wrong audience の 401
- 出力は issue / Slack / PR description にそのまま貼れる短文版と、artifact 向け詳細版の両方を持つ。

不要なこと:

- 「ローカルだけで Claude Desktop Remote Connector の最終検収が完了した」と表示しない。
- ngrok を staging / production の代替として扱わない。
- local password / cookie / authorization code / access token を report に保存しない。

### P0: Arrove Seria local smoke script 取り込み

`arrove-seria` 側に追加した local smoke script は、devkit の汎用機能として吸収できる。目的は shell script を増やすことではなく、Remote MCP OAuth の検証手順を artifact 化し、他の MCP 実装にも流用できるようにすること。

受け入れ条件:

- devkit から、既存の `scripts/mcp-oauth-local-tools-list-smoke.sh` 相当の流れを profile として実行できる:
  - PRM 取得
  - AS metadata 取得
  - PKCE authorize
  - Zitadel login form 到達
  - token exchange
  - `/mcp` `tools/list`
- user id / password は env か secure prompt からのみ受け取り、artifact へ出さない。
- authorization code / access token / refresh token / cookie / csrf token は stdout と artifact に出さない。
- `expected_tool_count` を profile で指定でき、差分が出たら failure として report する。
- 任意の `tool_fixture` を指定した場合だけ、token exchange 後の bearer token を stdout / artifact へ出さず、子プロセス env 経由で `verify:tools` 相当へ渡す。
- `allow_skips=false` を既定にし、fixture 内に `skip` が残っている場合は failure にする。ローカル配線確認だけ `allow_skips=true` を許可し、report には「検収証跡ではない」と明記する。
- `verify:tools` 実行時も `tools/list` の tool count を先に確認し、catalog 差分と tool call 失敗を分けて report する。
- `local-only` mode では port-forward が既に起動しているか、devkit が session state 付きで起動したものだけを使う。
- 実行後に port-forward / tunnel / temporary route の残存を doctor が検出する。

不要なこと:

- アプリ固有の fixture data 作成を devkit に持たせない。
- token を CLI 引数で子プロセスへ渡さない。process list や shell history に漏れるため。
- login UI selector を devkit core に固定しない。必要なら provider adapter または external recipe に閉じる。
- shell script を単にラップするだけの機能にしない。report / redaction / cleanup / failure classification まで devkit 側の価値として持つ。

### P0: Arrove Duo 用 conformance profile

`remote-mcp-devkit smoke --profile arrove-duo` のように、Arrove Duo MCP OAuth で毎回必要な検査を 1 profile にまとめる。

受け入れ条件:

- `mcp_path=/mcp`
- PRM は root と path-aware alias の両方を検査する:
  - `/.well-known/oauth-protected-resource`
  - `/.well-known/oauth-protected-resource/mcp`
- 未認証 `/mcp` は `401` と `WWW-Authenticate: Bearer error="invalid_token", resource_metadata=".../.well-known/oauth-protected-resource/mcp", scope="mcp:read"` を返すことを検査する。
- AS metadata は同一 origin の `/.well-known/oauth-authorization-server` で、少なくとも以下を検査する:
  - `authorization_endpoint`
  - `token_endpoint`
  - `revocation_endpoint`
  - `code_challenge_methods_supported` に `S256`
  - `client_id_metadata_document_supported=true`
  - `token_endpoint_auth_methods_supported` に `none`
- Claude profileでは `client_id=https://claude.ai/oauth/claude-code-client-metadata`、`redirect_uri=http://localhost/callback`、`scope=mcp:read`、`resource=<base-url>/mcp` を送る。
- ChatGPT profileでは `client_id=https://chatgpt.com/oauth/client.json`、`redirect_uri=https://chatgpt.com/connector_platform_oauth_redirect`、`scope=mcp:read`、`resource=<base-url>/mcp` を送る。
- authorize redirect先が Zitadel `/oauth/v2/authorize` で、translation 後の upstream `client_id` と tenant primary domain scope (`urn:zitadel:iam:org:domain:primary:<tenant>`) を report へ出す。
- token / authorization code / cookie / shared secret は artifact に出さない。

不要なこと:

- `mcp-remote` 起動や `claude_desktop_config.json` 編集はしない。今回のUX目標と違うため。
- Claude Desktop UI の完全自動操作はしない。UIの最終確認は人間が行い、devkitは貼る前のHTTP/OAuth証跡を作る。

### P0: local dev cluster adapter

Arrove Seria の kind dev cluster に対して、Traefik port-forward / temporary route / env patch / cleanup を devkit 側で安全に扱えるようにする。今は `arrove-seria/scripts/mcp-oauth-ngrok.sh` に閉じているが、汎用化できる部分は devkit に寄せたい。

受け入れ条件:

- `remote-mcp-devkit arrove-duo local-up` などで、次を自動確認する:
  - current kube context が想定 context か
  - `traefik`, `arrove-duo`, `arrove-sistema`, `zitadel` namespace が存在するか
  - `arrove-duo-mcp`, `mcp-oauth-as`, `auth-gateway`, `api-duo-external`, `zitadel`, `traefik` の Pod が Ready か
  - target Service endpoints が空でないか
- Traefik への local port-forward を session state に記録し、`down` / `cleanup` で確実に閉じる。
- temporary IngressRoute / Middleware / env patch を作る場合は、変更前 snapshot を保存し、owner label (`remote-mcp-devkit/session`) を付ける。
- cleanup は自分が作った temporary resource だけを削除し、既存 manifest 管理の resource は消さない。
- controller-manager / API server / endpoint empty / rollout stuck を分類して report に出す。

不要なこと:

- Kubernetes 全般の復旧はしない。再起動、scale、Elasticsearch/DB復旧は別ツールの責務。
- staging/prod へ恒久変更を apply しない。devkit は一時検証と差分報告まで。

### P0: all-tools verification bridge

Remote MCP の OAuth 接続が成功しても `tools/list` だけでは検収にならない。Arrove Duo では全ツール呼び出しを要求されているため、devkit から `arrove-duo-mcp` の fixture-driven verifier を呼びやすくする。

受け入れ条件:

- `oauth-code` で取得した bearer tokenを stdout / artifact へ露出せず、子プロセス env (`MCP_BEARER_TOKEN`) 経由で verifier に渡せる。
- `local-smoke` / `oauth-code` / `tunnel run` のどの入口から取得した token でも、同じ verifier bridge へ渡せる。入口ごとに tool verification 実装を重複させない。
- `verify:tools:oauth` 相当の結果を取り込み、以下を report へ要約する:
  - tool count
  - ok / failed / skipped
  - skipped が 0 か
  - response body は常に redacted
- report は `tools/list` だけが成功した状態と、fixture-driven `tools/call` まで成功した状態を別ステータスにする:
  - `catalog_verified`: `tools/list` まで
  - `tool_calls_verified_with_skips`: fixture 実行はできたが skip が残る
  - `tool_calls_verified`: 全 tool call が skip なしで成功
- fixture path は絶対 path に正規化して report に残すが、fixture 内容と tool response 本文は保存しない。
- negative replay は tenant root `/` ではなく、具体的な非MCP Connect API routeを要求する。
  - 例: `https://<tenant>.duo.arrove.jp/duo.branch.v1.BranchService/GetBranch`
- `skip` が残っている場合は exit code 1 にする。途中確認だけ `--allow-skips` を許可する。

不要なこと:

- fixture data seeding はしない。staging-safe data の準備はアプリ側・検収側の責務。
- production tenant / production user / production data は扱わない。
- 成功レスポンス本文を artifact に保存しない。個人情報・業務データ漏洩を避ける。

### P0: local OAuth round-trip evidence

Arrove Duo の local kind cluster では、Traefik port-forward 経由で PRM / AS metadata / authorize redirect までは確認できた。一方で、token exchange までの証跡を devkit 側で一貫して残せないと、staging へ上げる前の切り分けがまだ弱い。

受け入れ条件:

- `smoke --profile arrove-duo` の report に、以下を段階別に記録する:
  - PRM root / path-aware PRM の取得結果
  - 未認証 `/mcp` の `WWW-Authenticate` challenge
  - AS metadata
  - `client_id` が CIMD URL から Zitadel 内部 client id に変換されたこと
  - `authorize` redirect が Zitadel `/oauth/v2/authorize` を経由して login UI へ到達したこと
- redirect chain の証跡では、authorization code / cookie / token / shared secret を保存しない。
- `oauth-code` は Arrove Duo profile と連携し、`resource=<tenant>/mcp` と `scope=mcp:read` を自動設定できる。
- loopback callback は `http://127.0.0.1:<port>/callback` と `http://localhost:<port>/callback` の両方を扱える。
- local dev では `http://*.duo.arrove.localhost:<port>/mcp` を許容しつつ、handoff report では「Claude Desktop Remote Connector の最終検収には public HTTPS / staging が必要」と明記する。
- token exchange に成功した場合、JWT payload の redacted summary だけを report に出す。
- token exchange に失敗した場合、原因候補を次のどれかに分類する:
  - redirect URI 不一致
  - Zitadel client 設定不一致
  - tenant primary domain scope 不一致
  - resource parameter 不一致
  - callback listener 未到達
  - token endpoint 4xx / 5xx

不要なこと:

- Zitadel のログイン UI を完全自動操作しない。必要なら外部 Playwright CLI recipe か manual callback mode に委ねる。
- devkit が dev user の password や cookie を保存しない。
- Claude Desktop UI の成功を local token exchange 成功だけで代替したと表示しない。

### P0: OAuth artifact redaction bug

現状の `oauth-code` report は callback payload に `code` をそのまま含める。authorization code は短命でも credential 扱いなので、PR / issue / Slack に貼る artifact としては不適切。Arrove Duo の local token exchange 検証では、実行後に `/tmp` の artifact を削除して回避したが、tool 側で直す必要がある。

受け入れ条件:

- `OAuthCodeReport.callback.code` は保存しない、または `has_code=true` / `code_len=<len>` の summary に置き換える。
- `raw_query` からも `code`, `state`, `session_state` など credential になり得る値を redaction する。
- stdout JSON、`report.md`、`network.json`、`network.har`、`curl-equivalent.sh` の全 artifact で同じ redaction policy を適用する。
- redaction 後も、state mismatch / callback missing / OAuth error の診断に必要な情報は残す。
- regression test で `code=` や access token らしき値が artifact に出ないことを確認する。

不要なこと:

- code や token を暗号化して保存する機能は不要。保存しないのが正しい。

### P1: local-only limitation report

Claude Desktop Remote Connector は public HTTPS endpoint を前提にするため、ローカル閉域だけでは最終UXを完全再現できない。この制約を毎回口頭で説明しないで済むよう、devkit が検証結果に応じた文章を出す。

受け入れ条件:

- `remote-mcp-devkit handoff --client claude-desktop` が以下を生成する:
  - ローカルで確認済み: PRM / 401 challenge / AS metadata / authorize redirect / token exchange / tool calls
  - ローカルで未確認: Claude Desktop UI での connector 作成、Anthropic cloud からの public HTTPS fetch
  - 未確認の理由: `http://*.localhost` と self-signed HTTPS は Remote Connector から信頼されない、ngrok free は interstitial を返す場合がある
  - staging で確認すべき項目: URL貼り付け、browser auth、Desktop復帰、全ツール呼び出し、direct replay 401
- report はそのまま issue / PR / Slack に貼れる短文にする。

不要なこと:

- 「ローカルだけで最終検収完了」と誤解させる表示はしない。
- ngrok を本番・staging 手順に含めない。使った場合は session TTL と cleanup を必須にする。

## 次のジョブ詳細

`採用する機能` 表で `❌ 未着手` / `△` のものだけ、acceptance criteria を残す。完了済み (`✅`) は `## 完了した改善` を参照。

### P1: tunnel run

Claude Desktop / Claude Web / ChatGPT Web の実機検証だけに使う。通常のローカル検証には混ぜない。

想定 CLI:

```sh
remote-mcp-devkit tunnel run \
  --upstream http://127.0.0.1:18080 \
  --provider ngrok \
  --single-endpoint \
  --setup ./scripts/setup-ngrok-ingress.sh \
  --cleanup ./scripts/cleanup-ngrok-ingress.sh \
  --max-duration 30m
```

受け入れ条件:

- ngrok / cloudflared provider を起動し、公開 URL を検出する。
- single endpoint 前提で `/.well-known/oauth-protected-resource` / `/.well-known/oauth-authorization-server` / `/oauth/*` / `/mcp` が同一 host で動くことを検査する。
- `X-Forwarded-Proto=https` 補正が必要な場合の setup / cleanup script フックを受け取れる。
- `MCP_RESOURCE_HOST_ALIASES` / `ZITADEL_AUTHORIZE_URL` のような一時 env 差し替えを開始時に適用し終了時に復元する。
- `--max-duration` 到達時に自動 stop、SIGINT / SIGTERM 時にも cleanup を走らせる。
- 起動時に公開 URL / 検証用 Claude connector URL / 終了予定時刻を stderr に表示し、stdout には machine-readable な 1 行 JSON event を出す。
- artifact には `public_base_url` と `local_upstream_url` を両方記録する。

### P1: kubernetes-traefik adapter

dev cluster 操作を毎回 shell 手順で行うと検証後の戻し忘れが起きやすい。snapshot / restore 境界を持つ adapter として分離する。

- `kubectl port-forward` の起動・health check・終了管理。
- namespace / deployment / env var の一時 patch と元値復元。
- Traefik `IngressRoute` / `Middleware` の一時 apply / delete。
- rollout status 待ち。

受け入れ条件:

- `remote-mcp-devkit k8s snapshot` で変更前状態を保存できる。
- `remote-mcp-devkit k8s restore` で差し替えた env / resource を戻せる。
- kube context / namespace / deployment / resource 名は config に書き、プロジェクト固有名をコードに埋め込まない。

### P1: k8s doctor

`kubernetes-traefik adapter` の前段として、read-only の状態診断を独立させる。devkit が一時 patch を当てる前に「そもそも cluster が健康か」を分類する。

- `kube-controller-manager` / `kube-scheduler` の running 状態
- target deployment の `.status.replicas` と `.spec.replicas` の整合
- target service の endpoint subset が空でないこと
- 直近の rollout が `stuck` か否か (deployment condition の `Progressing` を見る)

受け入れ条件:

- 復旧は行わない (read-only)。分類結果と推奨アクション (例: `kubectl describe deployment ...` の suggestion) を stderr / report に出す。
- `controller-manager down` / `endpoint empty` / `rollout stuck` / `healthy` のいずれかに分類する。
- 依存は `kubectl` CLI のみ。client-go 等を埋め込まない。
- adapter を当てる前に doctor が `controller-manager down` を返した場合、`k8s snapshot` / `k8s restore` 系は明示的に拒否する (失敗を invisibility にしない)。

### P1: provider doctor zitadel

OAuth で詰まったとき、実装バグか IdP データ不整合かを切り分けるために必要。provider 固有なので core へ混ぜない。

想定 CLI:

```sh
remote-mcp-devkit provider doctor zitadel \
  --base-url http://login.example.localhost:18080 \
  --service-account-key-file /secrets/zitadel-client.json \
  --org-domain demo.example.localhost \
  --user admin
```

受け入れ条件:

- org domain から Zitadel organization を解決する。
- candidate login names を列挙する。
- user state / human profile / email verification / password state / org membership / project grant / role を確認する。
- `oauth-code` の失敗 artifact を読み、該当 user / org の診断を追記できる。
- `login name not found` / `wrong organization` / `password unusable` / `missing project grant` / `redirect_uri/app config mismatch` を区別して表示する。
- service account key や access token は report に残さない。
- Zitadel 以外の IdP (keycloak 等) に広げられるよう、provider 固有処理は trait / adapter 境界に閉じる。

### P1: client handoff report

Claude Desktop / Claude Web / ChatGPT Web に MCP URL を貼る人が迷わないよう、必要な情報を 1 つの artifact にまとめる。

想定 CLI:

```sh
remote-mcp-devkit handoff \
  --base-url https://<public-or-local>/ \
  --client-profile claude \
  --out artifacts/handoff
```

成果物:

- `handoff.md` — 貼る URL、ブラウザで期待される遷移 (consent / callback)、成功時の確認 (PRM/AS metadata/token JWT)、失敗時の next step (`smoke` / `oauth-code` を再実行)、stop コマンド。
- `handoff.json` — 同じ情報を agent-readable に。

受け入れ条件:

- `up` の session state があれば自動でそれを参照する。`--base-url` で上書き可。
- secret 値は出さない。token / password / cert content は redact。
- 既存の `smoke` / `oauth-code` artifact があれば、その PASS/FAIL を参照リンクとして埋める。

### P2: Playwright CLI recipe artifact

core に Chromium 依存を入れない代わりに、外部 Playwright CLI で実機ブラウザ到達を確認する recipe を出力する。devkit 自身は spawn しない。

成果物:

- `playwright-recipe.md` — `npx playwright open <authorize-url>` 系のコマンド例、capture 対象の event (`requestfailed` / `navigated`)、callback URL の取り出し方、`oauth-code --callback-mode manual` への引き渡し例。
- `playwright-script-template.ts` — そのまま `npx playwright test` で動かせる雛形 (PKCE verifier は env から、redirect URI は引数から受け取る)。
- `playwright-results/` ディレクトリは Playwright CLI 側に書かせる。devkit はディレクトリ名と README だけ提供する。

受け入れ条件:

- devkit から Node や npx を一切 spawn しない。recipe を**生成するだけ**。
- recipe には `oauth-code --callback-mode manual` への接続手順が含まれる。
- `--client-profile claude|chatgpt` で redirect URI 既定値が変わることを recipe にも反映する。

## 完了した改善

### `smoke` に OAuth authorize request の設定を追加 (2026-05-21)

CLI: `--client-profile claude|chatgpt|generic` / `--client-id` / `--redirect-uri` / `--scope` / `--resource auto|omit|<url>` / `--expected-upstream-client-id`。

- Claude profile: `client_id=https://claude.ai/oauth/claude-code-client-metadata`, `redirect_uri=http://localhost/callback`, `scope=mcp:read`。
- ChatGPT profile: `client_id=https://chatgpt.com/oauth/client.json`, `redirect_uri=https://chatgpt.com/connector_platform_oauth_redirect`, `scope=mcp:read`。
- `--expected-upstream-client-id` で CIMD translation shim 後の upstream `client_id` を assert できる。
- 実 dev cluster に対する end-to-end 検証 (Zitadel + CIMD translation shim): local port-forward / ngrok single endpoint いずれでも PRM / 401 / AS metadata / authorize redirect が全 PASS。authorize redirect 先が Zitadel の `/oauth/v2/authorize` に向き、upstream `client_id` が CIMD translation 後の値、`resource` が `<base-url>/mcp`、Zitadel の `urn:zitadel:iam:org:domain:primary:<host>` scope を含むことを確認済み。

### `curl-equivalent.sh` を実 request と一致 (2026-05-21)

smoke で使った authorize URL をそのまま `curl-equivalent.sh` に書き出し、`resource` / `client_id` / `redirect_uri` / `code_challenge` が report と一致する。

### HAR (HTTP Archive 1.2) を session 単位で保存 (2026-05-21)

- `artifacts/<session>/network.har` を `report.md` / `network.json` / `curl-equivalent.sh` と並べて生成。
- HAR 1.2 必須フィールド (`log.version`, `log.creator`, `log.entries[]`) を満たし、各 entry に `request.method/url/headers/queryString`、`response.status/headers/content`、`startedDateTime`、`time` がある。
- 各 entry に `_devkit_check_name` / `_devkit_check_passed` / `_devkit_check_messages` を追加注釈し、smoke のどの check が出した request か HAR 上でわかる。
- DevTools "Import HAR" で開けて全リクエストが見える (`integration::smoke_writes_har_with_entries_for_each_check`)。
- secret 値 (Bearer tokens / authorization code / cookie 等) は redaction する。request の形は HAR に残すが、credential 本体は保存しない。

### mock AS introspection endpoint (2026-05-21)

- `GET /_devkit/state` — 登録 client / 未交換 auth code / 発行済 access_token を即時 dump。
- `POST /_devkit/clients` — DCR を経由せずに `(client_id, redirect_uris[, client_id_metadata_document])` を直接 seed。
- `POST /_devkit/tokens` — authorize / token を経由せずに access_token を直接発行 (`mldk_seed_` prefix)。発行 token はそのまま `Authorization: Bearer ...` で MCP path を通過する (`integration::devkit_introspection_dumps_state_and_seeds_token`)。
- mock AS モードでのみ意味を持つ。pass-through モードでは upstream に転送される。

### `oauth-code` real AS token exchange (2026-05-21)

- PRM → AS metadata → PKCE authorize URL 生成 → browser click待ち → `code` + `state` 捕捉 → `/oauth/token` 交換まで 1 コマンドで実行。
- callback capture mode:
  - `--callback-mode listener`: `redirect_uri` が指すloopback host/portにローカル axum listener を立てる。`redirect_uri` がportを省略した場合はscheme default (`http=80`, `https=443`) を使い、bindできなければ即エラーにする。
  - `--callback-mode manual`: browserやPlaywright等で捕捉したfull callback URLをstdinから受け取り、同じPKCE verifier/stateでtoken exchangeする。Claude実CIMDの `http://localhost/callback` のように、手元プロセスがport 80をlistenできないケースに使う。
- Playwright / Chromium / Node に依存しない。ユーザ (または agent が指示する別 client) がブラウザを開くだけ。OS の `open` / `xdg-open` は `--open-browser` flag を渡したときだけ起動 (agent default は false)。
- `/oauth/token` へも authorize と同じ `resource` parameter を送る。Remote MCP shimがtoken endpointでresource bindingを検査するため。
- `access_token` / `refresh_token` / `id_token` は redact し、TokenSummary に `has_*` と `*_len` だけ残す。JWT は header / payload を decode するが署名検証はしない (JWKS を取りに行かないため依存ゼロを維持)。
- 成功時: `oauth-code-report.md` + `oauth-code-report.json`、失敗時: 追加で `failures.json`。
- 検証: `integration::oauth_code_captures_callback_exchanges_token_and_redacts` で timeout 経路の artifact、`integration::oauth_code_full_flow_against_devkit_mock_as` で PKCE と token exchange の組合せ。
- **意図的にやらない**: login form selector 設定、screenshot、page text 取得、retry、IdP 固有処理。ブラウザを開いてクリックする部分は人間 (または agent が別 client に指示) の責務。

2026-05-21 追加検証:

- `--redirect-uri http://localhost/callback --callback-mode listener` はmacOS通常ユーザーでは `127.0.0.1:80` bindが `Permission denied` になり、即エラーになることを確認した。
- `--redirect-uri http://localhost/callback --callback-mode manual` により、Claude実CIMD redirect URIを変えずに、Playwrightの `requestfailed` eventで得たcallback URLをstdinへ渡してtoken exchangeまで成功した。
- 実 dev cluster での結果: `token_type=Bearer`, `expires_in=43199`, access token length 1416。JWT payload に CIMD translation 後の `client_id`、tenant 系 custom claims、`urn:zitadel:iam:org:domain:primary=<host>` の各値を含むことを確認。

### `cleanup` / stale 外部 process 検出 (2026-05-21)

- `cleanup::run` を `(state_dir, artifact_dir, session_id, force)` 4 引数に拡張し、broken state best-effort recovery を実装。`--force` 時は corrupt な state JSON でも削除し、session の artifact dir も合わせて回収する。`up` の graceful exit / SIGINT 経路は `force=false` で artifacts を残す。
- `doctor` に `advisories` フィールドを追加。`stale ngrok process` と `stale kubectl port-forward process` を `pgrep -af <pattern>` で検出し、見つかった PID を informational に報告する。advisory は `passed()` に影響しない (running ngrok は意図かもしれないため落とさない)。
- `pgrep` 出力は内部関数 `parse_pgrep_output` でパース、unit test で typical / empty / malformed 入力を検証。
- 依存追加ゼロ — Unix 標準 `pgrep` のみ使用。Windows 等では advisory が空になる。
- kubectl 必須項目 (IngressRoute / Middleware / env patch diff) は `kubernetes-traefik` adapter (P1) へ移動。本機能の scope は外部 CLI 依存を持たない範囲に限定する。
- 検証: `cleanup::tests::force_cleanup_removes_corrupt_state_file_and_artifact_dir`、`doctor::tests::advisories_report_*`、`doctor::tests::parse_pgrep_*` ほか計 9 unit tests。

### dev cluster e2e で見つかった環境診断不足 (2026-05-21)

実データ tool call まで含めた end-to-end 検証の過程で、kind dev cluster 側に以下のような状態が発生した。

- `kube-controller-manager` が CrashLoopBackOff になり、Deployment の `.spec.replicas=1` が ReplicaSet へ反映されない。
- 一時的に重い依存サービス (Elasticsearch 等) を立ち上げると control-plane が `TLS handshake timeout` を返し、kubectl 操作が不安定になる。
- 下流の MCP server が、依存サービスの endpoint が空の間に起動すると疎通確認エラーで落ちる。

必要な追加仕様:

- `remote-mcp-devkit k8s doctor` を追加し、検証前に control-plane / scheduler / controller-manager / target deployment / required service endpoints を確認する。
- `k8s doctor` は Remote MCP OAuth 検証に必要な最小依存だけを見る。アプリ依存サービスの管理・復旧そのものは不要。
- `k8s adapter` は一時 Pod / Ingress / Env patch を作る前に snapshot を取り、controller-manager が落ちている場合は変更を拒否する。
- `cleanup` は owner label (`remote-mcp-devkit/session`) で一時 Pod / PVC / CR を検出して削除できる。

### エージェント向け stdout / stderr 規律 (2026-05-21)

- すべてのサブコマンドで stdout は単一の JSON / NDJSON、stderr は人間向け banner と進捗。`up` は session 開始時に 1 行 JSON event を出す。
- agent は `2>/dev/null` で stderr を捨てて `jq` に通せる。
- exit code: `0` = 成功、`1` = report `passed()==false`、anyhow error は非ゼロで stderr に出る。
- interactive 要素は `oauth-code` の browser click 待ちのみ。`--open-browser` を渡さない限り自動でブラウザを開かない。
- Node / Chromium / kubectl / ngrok など外部 CLI 依存はゼロ。

## 実装メモ

- crate 名・bin 名・README・仕様は `remote-mcp-devkit` に揃えた。state dir / artifact dir も `.remote-mcp-devkit/` に統一。local-only mode と remote-client mode を同じ bin の中で `--upstream-oauth` / 将来の `--tunnel` で切り替える方針。
- `smoke::check_authorize_redirect` は real AS / shim の upstream redirect も検証できる (CIMD 変換後の `client_id`、`resource`、Zitadel `urn:zitadel:iam:org:domain:primary:*` scope を assert)。token exchange まで含む end-to-end は `oauth-code` で扱う。
- `smoke` は Playwright / Chromium に依存させない。browser automation は `oauth-code` のような real AS 実機検証コマンドにだけ閉じ込める。crate 全体で Node / npx 依存は持たない。
- ngrok / k8s integration は当初 out-of-scope と判定したが、Claude Desktop 実機検証で HTTPS 公開 URL が必須となるため、短時間 session 限定の `tunnel run` / `k8s adapter` として P1 で取り込み直した。常時公開 hosting や汎用 K8s 管理はあくまで `## 採否判断 / 採用しない機能` のまま。
