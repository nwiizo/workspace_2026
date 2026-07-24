# ISUCON14 Rust Docker 環境

ISUCON14 の公式リポジトリを基に、Rust リファレンス実装と公式ベンチマーカーを Docker Compose だけで動かすローカル環境です。

- 取得元: <https://github.com/isucon/isucon14>
- 取得コミット: `53f8b627e040c30ebec600457c6c97da008b84b0`
- アプリ: Rust 1.83 / Axum
- データベース: MySQL 8
- 公開サーバー: nginx
- ベンチマーカー: Go 1.23

公式の `development/compose-rust.yml` を土台に、フロントエンドのコンテナビルドとローカル用ベンチマーカーを追加しています。公式の競技環境は 3 台の競技者 VM と専用ベンチマーカーであり、この 1 ホスト構成のスコアは本番スコアと直接比較できません。

性能改善の計測手順と残タスクは [TODO.md](./TODO.md)、変更理由と結果は [TUNING.md](./TUNING.md) で管理します。

## 初期状態

このディレクトリは、公式リポジトリの特定コミットを基準に構築しました。次の表はチューニング前の基準状態です。現在のRustアプリと初期SQLには、後述のベンチマークで検証した改善が段階的に入っています。

| 項目 | 初期状態 |
|---|---|
| ソース | 公式 `isucon/isucon14` のコミット `53f8b627e040c30ebec600457c6c97da008b84b0` |
| アプリ | 公式 Rust/Axum リファレンス実装 |
| DB | 公式 `webapp/sql/` の初期データ。初回起動時に MySQL ボリュームへ投入 |
| フロントエンド | 公式ソースを Docker ビルド時に pnpm でビルド |
| ベンチマーカー | 公式 Go 実装。フロントエンドと同時生成した静的ファイルハッシュを使用 |
| チューニング | 基準時点ではインデックス追加、SQL変更、キャッシュ導入などは未実施 |
| 起動前 | コンテナ、ネットワーク、MySQL ボリュームは未作成 |

公式スナップショットは、作業時に次の方法で入れ子の `.git` を含めず取り込みました。通常の利用時にこの操作をやり直す必要はありません。

```sh
source_dir=$(mktemp -d)
git clone https://github.com/isucon/isucon14.git "$source_dir/isucon14"
git -C "$source_dir/isucon14" checkout 53f8b627e040c30ebec600457c6c97da008b84b0
mkdir -p contests/isucon14
git -C "$source_dir/isucon14" archive HEAD | tar -x -C contests/isucon14
```

公式スナップショットへ追加したローカル用ファイルは次のとおりです。構成の正本は各ファイルに置き、この README では役割だけを示します。

| ファイル | 役割 |
|---|---|
| `compose.yaml` | Rust、MySQL、nginx、matcher、benchmark のサービス定義 |
| `docker/Dockerfile` | フロントエンド、nginx、公式ベンチマーカーのコンテナビルド |
| `docker/nginx.conf` | 静的ファイル配信と Rust API へのプロキシ |
| `docker/client-config/config.json` | 公開イメージ取得とHomebrew CLI plugin検出用のプロジェクト専用 Docker 設定 |
| `scripts/compose.sh` | Compose plugin / standalone Compose の差を吸収 |
| `scripts/up.sh` / `down.sh` | 起動、停止、DBを含む完全初期化 |
| `scripts/smoke-test.sh` | トップ画面と初期化 API の疎通確認 |
| `scripts/test-auth-cache.sh` | 初期token、動的登録、initialize失敗・成功後の認証cacheをHTTPとSQL回数で確認 |
| `scripts/test-latest-location-reconciliation.sh` | commit後のcache更新欠落と同時刻tie-breakの故障注入 |
| `scripts/test-status-notification-order.sh` | 時刻逆転時もapp / chair通知が状態遷移順になることをHTTPで確認 |
| `scripts/test-chair-stats-consistency.sh` | 全初期chairを照合し、欠損・誤値・余分なrowを再起動で修復 |
| `scripts/test-chair-stats-transitions.sh` | 評価の所有者認可、完了条件、決済rollback、再送時の非加算をHTTP検証 |
| `scripts/test-owner-sales-response-boundary.sh` | 遅い決済中の評価完了時刻とowner salesの`until`境界をHTTP・決済TCP accept・InnoDB行ロック・response JSON・SQLで確認 |
| `scripts/test-username-collision.sh` | 同じusernameを2回登録し、別user・別認証・招待couponを維持した限定再試行を確認 |
| `scripts/test-invitation-concurrency.sh` | 異なる招待コードと同一招待コードをbarrier付きで並行登録し、上限、coupon件数、MySQL 1062 / 1213の増分0を確認 |
| `scripts/benchmark.sh` | 決済モックを含む公式ベンチマーカーの実行 |
| `scripts/report-endpoint-latency.sh` | 診断runのnginx timing logをendpoint別に集計 |
| `scripts/report-coordinate-phases.sh` | 診断runのcoordinate phase、row lock、current-state writeを集計 |
| `.dockerignore` / `webapp/rust/.dockerignore` | Dockerへ不要なソース・`target/` を送らない |

## 初期構築方法

### 1. 必要なもの

- Docker Engine または Docker Desktop
- Docker Compose v2（`docker compose` または `docker-compose`）
- `curl`（疎通確認）、`jq`とTime::HiResを含むPerl（最新位置の故障注入テスト）
- 任意: `alp 1.0.21`（endpoint latencyの診断runだけで使用）
- 初回ビルド用のインターネット接続

Rust、Go、Node.js、pnpm をホストへインストールする必要はありません。
ただし、後述のPlaywright CLIによる任意の画面確認にはNode.jsとnpmの
`npx`を使用します。Docker環境の起動とベンチマークだけなら不要です。

この環境が取得するコンテナイメージはすべて公開イメージです。操作スクリプトは `docker/client-config/config.json` を使うため、ホスト側のレジストリ認証情報を読み込んだり変更したりしません。

### ローカル実行環境の確認と最適化

Docker Desktop以外にColimaを使う場合は、最初に割り当て資源を確認します。

```sh
colima status --extended
```

今回の初期ベンチ条件は4 CPU / 4 GiB / 100 GiB diskです。アプリ、MySQL、nginx、matcher、ベンチマーカーを同じVMへ入れるため、CPUとメモリは全サービスで共有されます。

将来、別条件として資源を増やす場合は、実行中のCompose環境を安全に停止してからColimaを再起動します。次は8 CPU / 12 GiBを使えるホストでの参考例です。今回の検証では実行していません。

```sh
./scripts/down.sh
colima stop
colima start --cpu 8 --memory 12 --disk 100
./scripts/up.sh
colima status --extended
```

Colimaの停止・再起動ではVMのdiskは保持されますが、実行中コンテナは停止します。別プロジェクトのコンテナも同じColimaを使っている場合は、先に影響を確認してください。ホストOSが使うCPU・メモリも必要なので、搭載資源のほぼ全量は割り当てません。

資源を変えるとスコアの比較条件も変わります。変更前後を比較するときは、CPU・メモリ・走行時間を同じにし、結果ファイルへ記録してください。

#### Docker build context

Dockerはイメージをビルドする前に、build contextをdaemonへ送ります。Rustの `target/` にはコンパイル済み成果物が数百MB生成されますが、Dockerfileはコンテナ内で改めてビルドするため、ホストの `target/` は不要です。

この環境では次の2段階で対象を絞っています。

- ルート `.dockerignore`: nginx・ベンチイメージへ `frontend/`、`bench/`、`docker/` だけを送る
- `webapp/rust/.dockerignore`: Rustイメージへ `target/` を送らない

実測ではローカルテスト後のRust build contextが約467MBから32.5KBへ減りました。これはアプリのベンチスコアではなく、変更後の再ビルド待ち時間とColimaへのI/Oを減らす改善です。

通常の反復ではDocker cacheを使用します。依存関係や生成物の不整合を調査するとき以外は `--no-cache` を付けません。

#### Rust release再ビルド

プロジェクト専用Docker設定は、Apple Silicon Homebrewの `/opt/homebrew/lib/docker/cli-plugins` とIntel Homebrewの `/usr/local/lib/docker/cli-plugins` を参照します。これにより、ホストの認証設定を読み込まずComposeとBuildxを使用できます。

Rust DockerfileはBuildKitのcache mountへ次を保存します。

- Cargo registry
- Cargo Git checkout
- releaseの `target/` とincremental情報

さらにRust toolchain同梱のLLDをlinkerとして使います。`opt-level=3` は変更していません。

4 CPU / 4 GiBを固定した実測は次のとおりです。

| 状態 | 時間 |
|---|---:|
| legacy builderでRust source変更後 | 30分52秒 |
| BuildKit cacheの初回作成（Cargo全依存を含む） | Cargo 4分08秒、全体6分15.24秒 |
| owner SQL変更後のincremental再build | Cargo 7.03秒、全体11.02秒 |

`scripts/benchmark.sh` は前回のISUCON stackを正常停止してからbuildし、`up.sh` で再開します。古いmatcherのpollingやMySQLのmemory保持を、Rust buildと競合させないためです。他のCompose projectは停止しません。

詳しいログ、失敗した方法、cacheの仕組みは [Rust / sqlx実装から学べること](./tuning/80-rust-implementation.md) を参照してください。

#### 同居ベンチの注意

公式競技環境は競技者VMとベンチマーカーが別です。このローカル構成では同じColima VM内で動くため、ベンチマーカー自身のCPU・メモリ・I/Oもアプリと競合します。

- 本番スコアとの絶対比較には使わない
- バックグラウンドの重いコンテナを止める
- 同じColima資源で変更前後を比較する
- 負荷終了後にDBが詰まっている場合は、再起動または初期化してから次を測る

詳細な理由と計測値は [ローカル環境の最適化記録](./tuning/90-local-environment.md) に記載しています。

#### MySQLのcommit耐久性

ローカル既定値は `innodb_flush_log_at_trx_commit=2`、`sync_binlog=0` です。
replicationを使わず、`POST /api/initialize` で初期データへ戻せる競技環境で
commit待ちを減らす設定です。各設定を60秒ベンチで3回ずつ比較したところ、
完全同期 `1 / 1` の中央値53,198点に対して `2 / 0` は60,102点で、約13.0%
改善しました。得点範囲は一部重なるため、確定的な差ではなく同じホストでの
採用判断です。MySQLは8.4.10のimage digestへ固定しています。

この設定ではOS・電源障害時に直近のcommit済みtransactionやbinary logを失う
可能性があります。業務データを保持するDB向けの既定値ではありません。
完全なcommit耐久性を優先して起動する場合は次のように上書きします。

```sh
MYSQL_INNODB_FLUSH_LOG_AT_TRX_COMMIT=1 \
MYSQL_SYNC_BINLOG=1 \
./scripts/up.sh
```

設定値、COMMITログ、リスク、他の選択肢は
[MySQLのCOMMIT永続化](./tuning/13-mysql-commit-durability.md) に記録しています。

### 2. ビルドと起動

```sh
cd contests/isucon14

# Rust アプリ、MySQL、nginx、マッチャーをビルドして起動
./scripts/up.sh
```

初回は Rust とフロントエンドをビルドし、MySQL ボリュームへ公式初期データを投入するため時間がかかります。2 回目以降は Docker のビルドキャッシュと既存の DB ボリュームが使われます。

起動後は次のサービスが作成されます。

```sh
./scripts/compose.sh ps
```

`db`、`webapp`、`nginx`、`matcher` が起動し、アプリは <http://localhost:8080/> で確認できます。

### 3. 初期状態の検証

```sh
# フロントエンドと Rust 初期化 API の疎通確認
./scripts/smoke-test.sh

# 初期token、動的user、initialize失敗・成功後の認証cacheを確認
# 注意: DBを初期化し、故障注入中だけwebapp/sql/init.shを一時名へ退避する
./scripts/test-auth-cache.sh

# DBだけが更新された状態から2秒再同期で復旧できることを確認
# 注意: 開始時と終了時にPOST /api/initializeを呼び、ローカルデータを初期化する
./scripts/test-latest-location-reconciliation.sh

# created_atが逆転しても通知と最新statusが状態遷移順になることを確認
# 注意: POST /api/initializeを呼び、ローカルデータを初期化する
./scripts/test-status-notification-order.sh

# chair statsを照合し、故障注入後のwebapp再起動repairを確認
# 注意: 開始時と終了時にPOST /api/initializeを呼び、ローカルデータを初期化する
./scripts/test-chair-stats-consistency.sh

# 評価APIの所有者認可、差分更新、決済失敗rollback、再送の非加算を確認
# 注意: 一時決済mock containerを起動し、終了時にローカルデータを初期化する
./scripts/test-chair-stats-transitions.sh

# 決済待ち中の評価をowner salesの既知watermarkへ早く含めないことを確認
# 注意: 8秒遅延する一時決済mock containerを起動し、終了時にローカルデータを初期化する
./scripts/test-owner-sales-response-boundary.sh

# 同じusernameの再登録を別userとして継続できることを確認
# 注意: 開始時と終了時にPOST /api/initializeを呼び、ローカルデータを初期化する
./scripts/test-username-collision.sh

# 異なる24招待コードと同一招待コード4件を並行登録し、lockと3回上限を確認
# 注意: 開始時と終了時にPOST /api/initializeを呼び、ローカルデータを初期化する
./scripts/test-invitation-concurrency.sh

# 公式ベンチマーカーによる短い動作確認
./scripts/benchmark.sh 10

# 公式と同じ 60 秒で本計測
./scripts/benchmark.sh 60
```

endpoint別のp50 / p95 / p99を採るときだけ、診断overlayを有効にします。

```sh
diagnostic_since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
ISUCON_DIAGNOSTIC=1 ./scripts/benchmark.sh 60
DIAGNOSTIC_SINCE="$diagnostic_since" ./scripts/report-endpoint-latency.sh
DIAGNOSTIC_SINCE="$diagnostic_since" ./scripts/report-coordinate-phases.sh
```

`ISUCON_DIAGNOSTIC=1` は
[`compose.diagnostics.yaml`](./compose.diagnostics.yaml) から
[`docker/nginx.diagnostic.conf`](./docker/nginx.diagnostic.conf) をmountし、APIのmethod、
URI、status、request time、upstream response time、request / response bytesを
JSONでstdoutへ記録します。同じoverlayがRust webappのcoordinate phase samplingも
有効にします。64 requestに1件について、pool + `BEGIN`、履歴INSERT、current-state write、
ride検索、status遷移、COMMIT、cache更新を分け、成功・error / cancellationと最後のphaseを
JSONで記録します。
upstream connect time、connection ID、同じconnection上のrequest回数も含みます。
Cookie、認証token、request body本文、決済情報は記録しません。通常のスコアrunは
環境変数を付けずに実行し、access log、時刻取得、sample JSON、同期stdout writeの追加処理と
分離してください。

集計にはmacOSホストの `alp 1.0.21` を使います。ride IDを含むpathは
`/api/app/rides/[^/]+/evaluation` と `/api/chair/rides/[^/]+/status` へ正規化されます。
診断runは原因を調べる値で、通常runの3走中央値と混ぜません。
`DIAGNOSTIC_SINCE` は同じnginx containerに残る前回runのlogを混ぜないため必須です。
`compose logs` またはJSON抽出に失敗した場合も、空の集計を成功として扱わず停止します。
通常表の4xx合計とは別に、clientがresponse完了前に切断したHTTP 499をendpoint別に出します。

coordinate集計のInnoDB metricは `DIAGNOSTIC_SINCE` で時刻filterできず、MySQL process起動後の
累積値です。そこで集計scriptは「指定run開始 ≤ MySQL起動 ≤ 最初のcoordinate sample」を
検証し、再利用DBや走行後のDB再起動では停止します。上記の
`./scripts/benchmark.sh` はbuild前にDBを正常停止し、新しいrun用processとして再起動します。
run開始時刻はbenchmark commandより前に取得し、同じrunのcontainerを再起動する前に集計して
ください。

`prepared_statements_instances` はreport時点で生存するSQLx connectionだけを持つlive snapshotです。
終了済みconnectionの実行は消えるため、全期間の完全な回数ではありません。phase sampling、
nginx全request、InnoDB process累積を同じ境界の値として混ぜず、互いに同じ傾向かを確認する
補助情報として使います。

phase別p50 / p95 / p99は、すべてのphaseを完了した成功sampleだけで計算します。errorまたは
cancellationでは未到達phaseがあるため、初期値0を成功分布へ混ぜません。失敗sampleは
terminal phase別の件数とhandler内total latencyを別表へ出し、どこまで進んだかを確認します。

走行時間は引数または環境変数で指定します。省略時は公式と同じ 60 秒です。
上記の `test-*.sh` はDBを公式初期データへ戻すため、保持したいローカルデータが
ある環境では実行しないでください。使い捨てのISUCON検証stackを対象にします。

`test-auth-cache.sh` はinitialize失敗を再現する短い区間だけ、
`webapp/sql/init.sh` を `init.sh.auth-cache-test-<PID>` へ退避し、通常終了とsignal trapで
元へ戻します。`SIGKILL` やホスト停止ではtrapを実行できません。退避名が残った場合は、
そのファイルが本来の `init.sh` であることを確認してから元の名前へ戻し、webappを
再起動してください。退避ファイルを確認せず別の内容で上書きしないでください。

chair stats照合は初期500 chairで差分0、公式prevalidationと60秒終了時の動的chairでも
差分0でした。実装と計測は
[`tuning/20-chair-stats-current-state.md`](./tuning/20-chair-stats-current-state.md) に
記録しています。再起動repairの検証対象は、Composeの単一webappを停止後に起動する
stop-then-startです。複数instanceのrolling restartはこの結果へ含めません。

```sh
./scripts/benchmark.sh 10
BENCHMARK_DURATION=10 ./scripts/benchmark.sh
```

ベンチマーカーは実行開始時に `POST /api/initialize` を呼ぶため、DB は初期データへ戻ります。フロントエンドの静的ファイル検証だけを省略したい場合は、公式オプションを次のように有効化できます。

```sh
SKIP_STATIC_SANITY_CHECK=1 ./scripts/benchmark.sh
```

### 4. 完全な初期状態へ戻す

アプリケーションデータだけを戻す場合は `./scripts/smoke-test.sh` またはベンチマーカーが呼び出す `POST /api/initialize` を使用します。MySQL ボリュームから作り直す場合は次を実行します。

```sh
RESET=1 ./scripts/down.sh
./scripts/up.sh
./scripts/smoke-test.sh
```

`RESET=1` はこの Compose プロジェクトの MySQL ボリュームを削除します。次回起動時に `webapp/sql/` から公式初期データが再投入されます。

## ローカルでの検証結果

2026-07-24〜25 に Colima（Apple Silicon、4 CPU / 4 GiB）で次を確認しました。スコアはホスト性能に依存します。

| 確認 | 結果 |
|---|---|
| `./scripts/smoke-test.sh` | `GET /` が 200、`POST /api/initialize` が `{"language":"rust"}` |
| `./scripts/benchmark.sh 10` | `pass=true`（最終確認時のスコア 394） |
| 初期状態の `./scripts/benchmark.sh 60`（共有負荷あり） | `pass=false`、スコア0、`CODE=32` |
| 同じ初期revisionの静穏時再計測 | `pass=true`、スコア5,906、`CODE=26` 1件 |
| INDEX追加後 | `pass=false`、スコア364 |
| 空通知polling改善後 | `pass=true`、スコア2,357 |
| owner距離集計改善後 | `pass=true`、スコア5,601、エラー0 |
| nearby N+1集約後 | `pass=true`、スコア4,116、`CODE=26` 1件 |
| 椅子統計集約後 | `pass=false`、スコア4,460、`CODE=32` 2件 |
| matcherバッチ化後 | `pass=true`、スコア2,393、エラー0 |
| 近傍優先matcher後 | `pass=true`、スコア16,909、エラー0 |
| 座標更新のDB往復削減後 | `pass=true`、スコア11,599、`CODE=17` 2件 |
| coupon code INDEX追加後 | `pass=true`、スコア15,415、エラー0 |
| 通知polling間隔比較 | 30msを維持。100msは14,611、50msは6,986・`CODE=31` 1件 |
| matcher間隔比較 | 500msを維持。100msの実測n=2は54,172 / 53,715点（記述上の中央値53,943.5点・推定代表値には不使用）、30msは41,016点 |
| 最新statusのcovering INDEX | 実行計画は改善したが45,075点のため不採用 |
| MySQL commit同期の緩和 | 3走中央値53,198→60,102点（+13.0%）、`COMMIT`平均中央値48.6%減 |
| 決済HTTP clientの再利用 | 3走76,761–88,638点、中央値80,354点（直前中央値比+33.7%）、全runエラー0 |
| `coupons(used_by)` INDEX | 3走88,805–100,606点、中央値93,606点（直前中央値比+16.5%）、全runエラー0 |
| nearbyの未完了ride判定 + 競合安全化 | エラー0の3走98,311–98,628点、中央値98,580点（直前採用版比+5.3%） |
| 座標遷移queryの絞り込み | 通常座標のstatus取得を除去し、候補だけlocking read。直前版中央値92,484→98,580点（+6.6%） |
| nearby最新座標cache + current-state表 | 評価response bodyまで保持するtrackerを含む当時の3走96,888–98,483点、中央値96,926点、全runエラー0。nearby SQL平均44.859→最終run例8.079ms |
| statusの状態遷移順 | 時刻逆転のapp / chair HTTP回帰テスト成功。3走89,539–99,895点、中央値98,338点、全run `pass=true`、CODE=11は0件 |
| 認証cache | 3走102,887–109,454点、中央値104,612点。認証SQL累積約99.3%減、`CODE=30`が6–20件再発 |
| 評価response配送競合の修正 | nearby開始snapshot + completion revision + body drop起点1秒lease + initialize generation。3走96,542–105,002点、中央値103,046点、全run `pass=true`・error map空、`CODE=30` 0件 |
| owner売上の完了時刻境界 | 決済成功後にevaluation / `COMPLETED` / chair statsを確定し、DBとresponseへ同じ完了時刻を使用。3走93,408–104,048点、推定代表値の中央値94,173点、全run `pass=true`・error map空、`CODE=24` 0件 |
| 決済冪等化 + owner配送境界 | ride IDを全決済POSTの `Idempotency-Key` にして確認GETを削除。owner requestと重なる評価rideだけを除外。最終3走95,596–115,968点、中央値101,037点、全run `pass=true`・error map空 |
| 通知payload cache | recipient revisionとchair stats dependency revisionでstale hit / 再挿入を防ぎ、未送信statusは30ms、状態不変cacheは100ms。最終3走103,727–111,798点、中央値109,443点（Benchmark 25比+8.3%）、全run `pass=true`・error map空 |
| username衝突の限定再試行 | `users.username` のMySQL 1062だけを内部usernameで1回再試行。3走103,738–107,508点、中央値104,263点、全run `pass=true`、`CODE=17` 0件。直前中央値比-4.7%のため高速化ではなく正当性修正として採用 |
| 招待登録の並行安全化 | 招待者UNIQUE行を直列化地点にし、couponを `COUNT(*)`、reward codeを新規user IDで一意化。3走99,775–105,304点、中央値102,569点、全run `pass=true`・error map空。並行回帰テストのMySQL 1062 / 1213増分0 |

初回の初期60秒走行ではMySQLのqueryが十数秒以上へ遅延し、ベンチマーカーの期限を
超えました。同じ初期revisionを外部コンテナの大きな共有負荷がない条件で再計測
すると5,906点で完走しました。この差をコード改善の効果とは扱いません。INDEX、
空通知polling、owner距離集計、N+1削減、matcherを1変更ずつ計測しました。
最新改善版はMySQL `2 / 0`、決済用 `reqwest::Client` のprocess内共有、
`coupons(used_by)` INDEXを維持し、nearbyの未完了ride判定から最新statusの
相関subqueryを除いています。完了後の遅延status追記を防ぐため、状態を変更する
全writerをride row lockへ合流させ、座標更新はpickup / destination候補だけlock後に
最新statusをlocking readで再読します。さらに、全位置履歴はDBへ残したまま、
最新座標を1 chair 1 rowの `chair_current_locations` とprocess内cacheへ分離しました。
履歴とcurrent rowは同じtransactionで更新し、cacheはcommit後に即時更新したうえで
2秒ごとにDBから再同期します。active状態と割当可否は毎回DBから読みます。
評価transactionが外部決済を待っている間はprocess内trackerで該当chairを明示的に
nearbyから除外します。RAII guardはhandlerのローカル変数ではなく評価response bodyへ
移しました。ただし、認証cacheで処理量が増えた後の診断では、Axum/Hyperのbody dropから
benchmarkerのresponse受信まで約55–677msの差があり、body lifecycleだけでは
`CODE=30`を閉じないことが分かりました。現在はnearby開始snapshot、単調なcompletion
revision、body drop起点の1秒delivery leaseを組み合わせています。
Benchmark 23時点では `rides.updated_at` 起点の固定cooldownが外部決済時間を途中で
消費するため不採用でした。generationと期限切れ記録の安全なpruneを含む最終3走は
105,002 / 103,046 / 96,542点、中央値103,046点、全run error map空、
`CODE=30` 0件でした。

Benchmark 24では、長い決済待ちによるowner salesのwatermarkと処理順の逆転を
なくすため、評価の完了writeを決済成功後へ移しました。修正前の決定的な再現では、
pending rideが既知完了rideより約151ms古く、同じ`until`でowner salesが700円過大に
なりました。最終実装は完了時刻だけの追加UPDATEを使わず、決済前の冗長なride再SELECTも
削除しています。最終3走は94,173 / 104,048 / 93,408点、中央値94,173点で、
全run error map空、`CODE=24` 0件でした。直前中央値より低いため性能改善とは扱わず、
決定的な赤/緑テストを根拠にした正当性修正として記録します。

Benchmark 25では、公式決済serviceの冪等key実装を確認し、ride IDをすべての
決済POSTとretryへ付与しました。204以外の応答時に行っていた `GET /payments` と
userのride全件取得は削除し、network error、409、5xxだけを同じkey・token・amountの
POSTで再試行します。回復しない4xxはDB transactionを保持したまま再送せず即時に返します。
また、owner sales開始時のsnapshotとcompletion revisionを使い、owner requestと
評価response bodyの配送が実際に重なったride IDだけを売上から除外します。固定1秒の
除外は、既知の正しい売上を小さくするためowner経路には使いません。最終3走は
95,596 / 101,037 / 115,968点、中央値101,037点、すべて `pass=true`・error map空でした。
Benchmark 24中央値比では約+7.3%ですが、Benchmark 23中央値には約-1.9%のため、
最高点更新ではなく正当性改善とエラー時経路短縮として扱います。詳細は
[`tuning/25-payment-idempotency.md`](./tuning/25-payment-idempotency.md)を参照してください。

Benchmark 26では、変更前診断runでapp / chair通知の累積時間が
10,726.603 / 9,357.486秒だったため、決済transaction分割より先に状態不変pollを
対象にしました。全writerでrecipient revisionを進め、app payloadが含むchair statsには
別のdependency revisionを持たせます。同じchairを別userが評価しても、過去userの
stale stats payloadはlookupとinsertの両方で拒否します。未送信statusがない場合だけ
JSON bytesをprocess cacheへ保存します。30ms固定cacheはHTTP pollを増やして
3走中央値88,757点まで悪化したため不採用にし、定常cacheだけ100msへ変更しました。
dependency revision追加前の3走は114,996 / 103,957 / 112,156点、中央値112,156点、
全run error map空でした。レビュー修正版は別の3走111,798 / 103,727 / 109,443点で
再計測し、中央値109,443点、範囲103,727–111,798点、全run `pass=true`・error map空でした。
直前Benchmark 25の中央値101,037点より8.3%高く、dependency追加前より2.4%低い結果です。
cross-userの正当性を満たす修正版だけを現在実装の代表値として扱います。
変更後診断runではapp / chair通知の平均が113 / 130msから37 / 51ms、p50が96 / 119msから
2 / 5ms、累積が3,941.695 / 3,887.219秒へ減りました。p95は166 / 181msで30msを
超えているため、通知は引き続きP0です。詳細は
[`tuning/26-notification-payload-cache.md`](./tuning/26-notification-payload-cache.md)を
参照してください。

Benchmark 29では、招待登録の `SELECT ... FOR UPDATE` が異なる招待コードでも
`coupons(code)` B-treeの同じgapをlockし、互いのINSERTを待つdeadlockを診断しました。
招待者のUNIQUE行を同一コードの直列化地点に変更し、coupon全row取得を `COUNT(*)` へ
縮小しています。最初の修正版ではreward codeの `NOW(3)` が同じミリ秒になって主キー
1062を起こしたため、新規user IDを一意suffixにしました。barrier付き回帰テストでは
異なる24コードがすべて成功し、同一コード4件は3成功・1拒否、MySQL 1062 / 1213の
増分は0でした。通常3走は99,775 / 105,304 / 102,569点、中央値102,569点で
全run error map空です。最高中央値は更新していないため、性能改善ではなく高負荷時の
HTTP 500とerror budget消費を防ぐ正当性修正として扱います。lockの仕組み、赤・緑検証、
counter方式などの代替案は
[`tuning/29-invitation-concurrency.md`](./tuning/29-invitation-concurrency.md)に
記録しています。
過去のBenchmark 19では、診断runで `CARRYING` の後に古い `PICKUP` を返す
CODE=11を再現したため、
通知と最新statusの順序をwall-clockではなくENUMの状態遷移順へ変更しました。
時刻逆転のHTTP回帰テストを追加し、通常条件の3走は89,539 / 98,338 / 99,895点、
中央値98,338点、すべて `pass=true` でした。
queryだけを変えた暫定版の中央値100,310点は競合反例があるため、採用スコアには
含めていません。
process cacheだけの暫定版中央値103,683点も、`CODE=30` とcommit後cache更新欠落の
反例があるため、最終採用値には含めていません。
これは今回の3走から推定した代表値で、最小値–最大値は観測範囲であり、将来の
保証範囲ではありません。
同一条件が1走だけの過去スコアは実測値として残し、典型値は推定していません。
スコアには走行ごとの揺れがあるため、
変更の判断には実行計画、エラーログ、HTTP件数、transaction累積時間も併用して
います。詳細は [TUNING.md](./TUNING.md) からベンチマーク単位の記録を参照して
ください。

## 構成

```text
localhost:8080
      |
    nginx ───── static files (Remix/Vite)
      |
  Rust/Axum :8080 ───── MySQL :3306
      |                    ^
      |                    |
      +── payment ── benchmark
      ^
      |
   matcher (0.5 秒間隔)
```

| Compose サービス | 役割 | ホスト公開 |
|---|---|---|
| `nginx` | 静的ファイルと `/api/*` のリバースプロキシ | `127.0.0.1:8080` |
| `webapp` | Rust/Axum リファレンス実装 | なし |
| `db` | MySQL 8 | `127.0.0.1:13306` |
| `matcher` | `/api/internal/matching` の定期実行 | なし |
| `benchmark` | 公式 Go ベンチマーカーと決済モック | 実行時のみ |

公開ポートは変更できます。

```sh
APP_PORT=18080 MYSQL_PORT=23306 ./scripts/up.sh
APP_PORT=18080 ./scripts/smoke-test.sh
```

## 確認できるエンドポイント

ブラウザの入口は <http://localhost:8080/> です。主な画面は `/client`、`/owner`、`/simulator` にあります。

Rust 実装が提供する API は次のとおりです。

| Method | Path | 用途 | 認証 |
|---|---|---|---|
| `POST` | `/api/initialize` | DB 初期化、決済 URL 設定 | なし |
| `POST` | `/api/app/users` | 利用者登録 | なし |
| `POST` | `/api/app/payment-methods` | 決済トークン登録 | 利用者 Cookie |
| `GET` / `POST` | `/api/app/rides` | 配車履歴取得、配車依頼 | 利用者 Cookie |
| `POST` | `/api/app/rides/estimated-fare` | 料金見積もり | 利用者 Cookie |
| `POST` | `/api/app/rides/:ride_id/evaluation` | 乗車評価 | 利用者 Cookie |
| `GET` | `/api/app/notification` | 利用者向け通知polling | 利用者 Cookie |
| `GET` | `/api/app/nearby-chairs` | 周辺の椅子検索 | 利用者 Cookie |
| `POST` | `/api/owner/owners` | オーナー登録 | なし |
| `GET` | `/api/owner/sales` | 売上取得 | オーナー Cookie |
| `GET` | `/api/owner/chairs` | 所有する椅子一覧 | オーナー Cookie |
| `POST` | `/api/chair/chairs` | 椅子登録 | 登録トークン |
| `POST` | `/api/chair/activity` | 稼働状態更新 | 椅子 Cookie |
| `POST` | `/api/chair/coordinate` | 座標更新 | 椅子 Cookie |
| `GET` | `/api/chair/notification` | 椅子向け通知polling | 椅子 Cookie |
| `POST` | `/api/chair/rides/:ride_id/status` | 乗車状態更新 | 椅子 Cookie |
| `GET` | `/api/internal/matching` | 配車マッチング | ローカル環境内（nginx 制限） |

初期化 API は次のように直接確認できます。

```sh
curl -sS \
  -X POST \
  -H 'Content-Type: application/json' \
  -d '{"payment_server":"http://example.invalid"}' \
  http://localhost:8080/api/initialize

# {"language":"rust"}
```

## Playwright CLI での画面確認

`@playwright/cli` を使い、次の画面が描画できることを確認しました。

| 画面 | URL | 確認結果 |
|---|---|---|
| トップ | `/` | `Top | ISURIDE`、3種類の画面へのリンクを表示。console error 0 |
| 利用者 | `/client` | 未ログインの新規sessionでは `/client/register` へ遷移し、利用者登録フォームを表示 |
| オーナー | `/owner/login` | セッショントークン入力とログイン・新規登録導線を表示 |
| 椅子シミュレーター | `/simulator` | 利用者登録iframeとChair Simulatorを左右に表示 |

- [トップ画面](artifacts/playwright/top.png)
- [利用者画面（登録後）](artifacts/playwright/client.png)
- [オーナー登録画面](artifacts/playwright/owner.png)
- [椅子シミュレーター画面](artifacts/playwright/simulator.png)

2026-07-25の最終確認では4画面の静的resource 62 requestがすべて200でした。
未認証の利用者iframeと椅子シミュレーターから呼ばれた
`/api/app/notification` と `/api/chair/activity` の401だけがconsoleへ記録されました。
登録・ログイン前の想定どおりの認証拒否で、トップ画面ではconsole error 0件です。

Benchmark 24の最終確認では、トップの3リンクから `/simulator` を開き、
利用者登録フォームとChair Simulatorの描画、chair notification / coordinateのHTTP 200を
確認しました。利用者登録フォームを送信すると「決済トークン登録」へ遷移しました。
この確認でも、登録前に開始したapp notificationとchair activityのHTTP 401だけが
consoleへ残りました。

Benchmark 26修正版の最終確認でも、トップの静的resource 17件はすべてHTTP 200でした。
`/simulator` は利用者登録iframeとChair Simulatorを描画し、初回bootstrap時の
app notification / chair activity 401をconsoleからclearした後、1秒間に新しいconsole
errorは0件でした。chair notificationとcoordinateは継続してHTTP 200です。

再確認するときは、サービスを起動した状態で次を実行します。

```sh
npx --yes @playwright/cli -s=isucon14-check open http://localhost:8080
npx --yes @playwright/cli -s=isucon14-check snapshot
npx --yes @playwright/cli -s=isucon14-check console error
npx --yes @playwright/cli -s=isucon14-check requests --static
npx --yes @playwright/cli -s=isucon14-check screenshot \
  --filename=artifacts/playwright/top.png

npx --yes @playwright/cli -s=isucon14-check goto http://localhost:8080/client
npx --yes @playwright/cli -s=isucon14-check snapshot
npx --yes @playwright/cli -s=isucon14-check screenshot \
  --filename=artifacts/playwright/client-register.png

npx --yes @playwright/cli -s=isucon14-check goto http://localhost:8080/owner
npx --yes @playwright/cli -s=isucon14-check snapshot
npx --yes @playwright/cli -s=isucon14-check screenshot \
  --filename=artifacts/playwright/owner.png

npx --yes @playwright/cli -s=isucon14-check goto http://localhost:8080/simulator
npx --yes @playwright/cli -s=isucon14-check snapshot
npx --yes @playwright/cli -s=isucon14-check screenshot \
  --filename=artifacts/playwright/simulator.png
npx --yes @playwright/cli -s=isucon14-check close
```

`snapshot` で「Simulator / Client / Owner」の3リンクを確認し、`console error` が0件、
`requests --static` が17件すべて200であることを確認します。`screenshot` は実際の
描画崩れを目視するために使い、DOM snapshotだけで見た目まで正常とは判断しません。
既存の `client.png` は登録後のsessionで取得した画面です。上記の新規sessionでは
登録画面へ遷移するため、上書きせず `client-register.png` へ保存します。

## 日常操作

```sh
# 状態
./scripts/compose.sh ps

# ログ
./scripts/compose.sh logs -f webapp
./scripts/compose.sh logs -f db

# Rust コード変更後に再ビルド
./scripts/compose.sh up -d --build webapp nginx matcher

# 停止（DB データは保持）
./scripts/down.sh

# 停止して DB ボリュームも削除
RESET=1 ./scripts/down.sh
```

Rust 実装は `webapp/rust/`、初期 SQL は `webapp/sql/`、ローカル Compose 拡張は `compose.yaml` と `docker/` にあります。

## トラブルシューティング

### 8080 または 13306 が使用中

`APP_PORT` または `MYSQL_PORT` を変更してください。ベンチマーカーは Docker ネットワーク内の `http://nginx` を使うため、ホスト側の `APP_PORT` を変更しても動作します。

### DB を完全に作り直す

[完全な初期状態へ戻す](#4-完全な初期状態へ戻す)の手順で、この Compose プロジェクトの MySQL ボリュームを再作成してください。

### ベンチマーカーの静的ファイル検証に失敗する

まずキャッシュを使わずフロントエンドとベンチマーカーを再ビルドします。

```sh
./scripts/compose.sh --profile benchmark build --no-cache nginx benchmark
./scripts/benchmark.sh
```

調査中だけ検証を省略する場合は `SKIP_STATIC_SANITY_CHECK=1` を使用してください。静的ファイルへの負荷も省略されるため、通常のスコア計測では使用しません。
