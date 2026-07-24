# ISUCON14 Rust チューニング TODO

公式 Rust 実装へ最初の INDEX と通知 polling の改善を加えた現在の作業ツリーを、正当性を維持したまま最大スコアまで段階的に改善するためのバックログです。

最終ソース監査日: 2026-07-24

## 最適化の目的と制約

最終目的は単純なHTTPリクエスト数ではなく、60秒ベンチを `pass=true` で完走させ、完了ライド数とスコアを増やすことです。

- ベンチマーカーは30msを1tickとして進むため、全エンドポイントの理想値を30ms以内とする
- スコアは「空車で乗車地点へ移動した距離×0.1 + 乗車中の移動距離 + 完了ライド数×5」で評価する
- 空車移動より乗車中の移動の価値が10倍なので、単なる処理件数だけでなく乗車地点に近い椅子の割当を優先する
- 通知は全状態遷移を順番どおり、at least onceで返す
- 通知状態は変化から3秒以内に反映する
- nearbyの座標とownerの累積距離は3秒以内のずれに収める
- nearbyで3秒のずれが許されるのは座標だけで、椅子の割当可否は即時に反映する
- ライドを30秒以内にマッチさせ、1台の椅子へ複数ライドを割り当てない
- 評価成功時の決済を重複・欠落させない
- 動的に追加される利用者、オーナー、椅子を初期データと同様に扱う
- `POST /api/initialize` 後やプロセス再起動後も正しい状態へ戻る
- 1回のベンチでは性能仮説を1つだけ変更し、改善量と副作用を分離する
- ベンチ中のソフトエラーも合計200件へ達するとFAILするため、クリティカルエラー以外も予算として追跡する
- `rides.updated_at` は完了時刻として検証されるため、評価完了後の状態更新で変更しない

## 現在までの実装・計測

- [x] 初期状態の10秒ベンチ: `pass=true`、スコア394
- [x] 初期状態の60秒ベンチ（共有負荷あり）: `pass=false`、スコア0、`CODE=32`
- [x] `jj` で初期revision `ef9265541d50` だけを開いて静穏時に再計測
  - `pass=true`、スコア5,906、`CODE=26` 1件
  - マッチング不満足度85.9%、最終評価数64
  - 初回との差はコード改善ではなく実行環境差を含むため、別条件として記録
- [x] 初期データ件数を確認
  - `chairs`: 500
  - `chair_locations`: 21,209
  - `rides`: 750
  - `ride_statuses`: 4,496
- [x] 初期状態では上記4テーブルに主キー以外の索引がないことを確認
- [x] 高コスト SQL を `EXPLAIN ANALYZE`
- [x] [`webapp/sql/1-schema.sql`](./webapp/sql/1-schema.sql) へ初回の INDEX を追加
  - `chairs`: `access_token`、`owner_id`、`is_active`
  - `chair_locations`: `(chair_id, created_at)`
  - `rides`: `(user_id, created_at)`、`(chair_id, created_at)`、`(chair_id, updated_at)`
  - `ride_statuses`: `(ride_id, created_at)`、未送信通知検索用の2本
- [x] INDEX追加後の60秒ベンチ: `pass=false`、スコア364
- [x] INDEX追加後のtransaction境界を計測
  - `BEGIN`: 12,979回、累積87.784秒
  - `COMMIT`: 7,468回、累積88.516秒
  - `ROLLBACK`: 5,534回、累積108.967秒
- [x] ライドが1件もない通知 polling をtransaction開始前に返す処理を実装
  - `app_get_notification`
  - `chair_get_notification`
- [x] 空polling改善後の60秒ベンチ: `pass=true`、スコア2,357、CODE=33なし
- [x] `owner_get_chairs` を対象ownerの位置履歴へ先に絞る
- [x] owner SQL改善後の60秒ベンチ: `pass=true`、スコア5,601、error map空
- [x] nearbyを1 SQLへ集約した60秒ベンチ: `pass=true`、スコア4,116、`CODE=26` 1件
- [x] `get_chair_stats` を1 SQLへ集約した60秒ベンチ: `pass=false`、スコア4,460、`CODE=32` 2件
- [x] matcherを最大64件のbatchへ変更した60秒ベンチ: `pass=true`、スコア2,393、error map空
- [x] 乗車地点に近い空き椅子を優先した60秒ベンチ: `pass=true`、スコア16,909、error map空
- [x] 座標更新を通常4 SQLから2 SQLへ削減した60秒ベンチ: `pass=true`、スコア11,599、`CODE=17` 2件
- [x] `SHOW ENGINE INNODB STATUS` で `coupons.code` 全走査に起因する登録deadlockを特定
- [x] `coupons(code)` 追加後の60秒ベンチ: `pass=true`、スコア15,415、error map空
- [x] 通知の `retry_after_ms` を30 / 50 / 100msで比較し、30msを維持
  - 30ms: `pass=true`、15,415、エラー0、通知GET 34,360回
  - 100ms: `pass=true`、14,611、エラー0、通知GET 21,140回
  - 50ms: `pass=true`、6,986、`CODE=31` 1件、通知GET 13,552回
  - DB負荷は減ったが評価数とスコアが改善せず、50msはエラー0も満たさないため不採用
- [x] BuildKit、release incremental、LLDでRust source変更後の再buildを高速化
  - Cargo: 7.03秒
  - Docker build壁時計: 11.02秒
  - ホストおよびColima: 4 CPU / 4 GiBのまま
- [x] nearbyの集合SQL、chair statsの集約SQL、batch matcherを実装
- [x] 上記3変更を別々のBenchmarkとして正当性・性能検証する

## ソース監査で見つかった主要ボトルネック

| 優先度 | 対象 | 現在の処理 | 主な問題 |
|---|---|---|---|
| P0 | `internal_get_matching` | 64件batch + 近傍優先、外部pollは500ms | 空き定義の集約、500msの最小待ち |
| P0 | `app_get_nearby_chairs` | `LATERAL` + `NOT EXISTS` の集合SQL | 最新statusの相関subquery、複数回中央値未計測 |
| P0 | 通知2経路 | 30ms pollingごとに認証、最新ride、status、表示データを取得 | 60秒で通知GET 34,360回、同じレスポンスの再計算 |
| P0 | `get_chair_stats` | 1集約SQLを実装・ベンチ検証済み | 初期データ全件の旧実装との照合は未実施 |
| P0 | `app_post_ride_evaluation` | DB transaction中に外部決済HTTPと最大5回の100ms sleep | connection・snapshot・lockを外部I/O中も保持 |
| P0 | `chair_post_coordinate` | INSERT後に同じ位置を再SELECTし、rideとstatusも個別取得 | 全椅子が高頻度で通る書き込み経路の往復過多 |
| P1 | `app_get_rides` | rideごとにstatus、coupon、chair、ownerを取得 | 履歴増加に比例するN+1 |
| P1 | `owner_get_sales` | ownerのchairごとに完了rideを取得 | N+1、read transactionが暗黙ROLLBACK |
| P1 | `app_post_rides` | userの全rideとride別最新statusを取得 | ライド作成ごとに履歴全体を再走査 |
| P1 | `app_post_users` | `coupons(code)` INDEXは追加済みだが、招待回数確認で該当行全体を取得 | `COUNT` / counter化とlock範囲の縮小が未実施 |
| P1 | 認証middleware | 全APIリクエストでtokenからDB検索 | pollingと座標送信のたびに追加SQL |
| P1 | `payment_gateway` | retryごとに `reqwest::Client::new()` | HTTP connection poolを再利用できない |
| P2 | nginx / Rustログ | stock設定のまま全リクエストを処理 | 高頻度経路のログI/Oとproxy overheadが未計測 |
| P2 | MySQL / sqlx pool | stock MySQL、pool上限50固定 | 実負荷に対するbuffer・接続数が未調整 |

## Fable独立レビューで追加した正当性不変条件

- `rides.updated_at` は利用者履歴の `completed_at` とowner売上の期間判定に使われる
  - current statusは別表へ置くか、評価確定と同じ更新以外で `rides` を変更しない
  - 初期データbackfillで既存rideを更新する場合は `updated_at` を保存する
- 初期ダンプは `INSERT INTO rides VALUES (...)` のように列名を省略している
  - 既存表への列追加はダンプ投入後のALTERにするか、別のcurrent-state表を作る
- 「空き椅子」は用途で2種類に分ける
  - nearby掲載可能: 最後のrideが評価済み
  - matcher再割当可能: `COMPLETED` が椅子へ送達済み
- chair statsに3秒猶予はない
  - 走行中の通知では値を固定し、`COMPLETED` 通知では当該評価を含める
  - stats更新は評価と `COMPLETED` 追加と同じtransactionへ入れる
- 評価APIは決済成功前に200または `COMPLETED` を公開しない
  - 決済成功後に短いwrite transactionで完了状態を確定し、その後レスポンスする
- 3秒cacheを許せるowner情報は `/owner/chairs` の累積距離だけ
  - `/owner/sales` はリクエスト直前snapshotを下限に検証されるため、遅延cacheしない

## スコア構造から導いた追加仮説

- [x] INDEX、nearbyのN+1解消、owner椅子一覧の事前絞り込みを実装済みであることを確認する
- [x] `users(access_token)` と `users(invitation_code)` は既存の `UNIQUE` INDEXで検索できることを確認する
- [x] nearbyの「割当済み判定には `evaluation IS NULL` を利用できる」という前提が、評価と `COMPLETED` を同じtransactionで確定する仕様に基づくことを確認する
- [ ] 新しい施策は1つずつ単独ベンチし、現在のRust実装で改善することを確認してから採用する
- [ ] 完了数だけでなく、空車移動距離、乗車中移動距離、matching / pickup / drive評価を記録してスコアの増減理由を分解する
- [ ] 空車移動距離の0.1点を単独で稼ごうとせず、pickup遅延と完了数への悪影響を含む総スコアでpolicyを比較する
- [ ] スコア増加が完了数、乗車距離、空車距離のどれによるものかをrunごとに説明できる集計scriptを用意する

## Phase 0: 現在の変更を確定する

- [x] 現在のコードを再ビルドし、`./scripts/smoke-test.sh` を通す
- [x] `./scripts/benchmark.sh 60` を実行する
- [x] `pass`、スコア、全エラーコードを記録する
- [ ] 完了ライド数を独立した指標として記録する
- [x] `BEGIN` / `COMMIT` / `ROLLBACK` の回数と累積時間を再計測する
  - 30ms走行: BEGIN 50,643回、COMMIT 50,526回・累積452.757秒、ROLLBACK 114回
  - 100ms走行: COMMIT 35,194回・累積412.027秒
  - 50ms走行: COMMIT 20,108回・累積410.209秒
- [x] `CODE=33` など通知内容の不整合が発生していないことを確認する
- [x] nginx access logから30ms走行のエンドポイント別件数を採取する
  - app通知18,382、chair通知15,978、座標更新14,644
- [ ] エンドポイント別のp50 / p95 / p99を一時的な計測で採取する
- [ ] 各エンドポイントの30ms超過率と、1tick中に完了できなかった回数を記録する
- [ ] sqlx poolの `size` / `idle` / `in_use` と取得待ち時間を1秒ごとに採取する
- [ ] MySQLのstatement digestを回数、累積時間、平均時間で並べる
- [ ] `docker stats` でwebapp、MySQL、nginx、ベンチマーカーのCPU・メモリ・I/Oを同時に採取する
- [ ] 全エラー件数の合計と、200件のエラー予算に対する消費率を記録する
- [ ] 座標・status系APIのp99を評価遅延予算と比較する
  - matching評価: 100tick = 3秒未満
  - pickupの余分な遅延: 15tick = 450ms未満
  - driveの余分な遅延: 5tick = 150ms未満
- [x] 結果を [`tuning/02-notification-transactions.md`](./tuning/02-notification-transactions.md) と [`TUNING.md`](./TUNING.md) に反映する
- [x] `tuning/02-notification-transactions.md` の説明を現在の実装へ修正する
  - 全面autocommit化ではなく、rideなしの分岐だけをtransaction外へ出している
  - `chair_get_notification` の `FOR SHARE` は残っている

## Phase 0.5: 計測ツールを選定・検証する

ツールの導入自体は高速化ではありません。ボトルネックの層を特定し、変更前後の差を同じ条件で説明できたツールだけを残します。

### 運用ルール

- [ ] profile採取run、ツールのoverhead測定run、最終スコアrunを分離する
- [ ] 各ツールのversion、実行コマンド、sampling間隔、開始・終了時刻を記録する
- [ ] macOSホスト、Colima Linux VM、Docker containerのどこで採取した値かを必ず明記する
- [ ] 計測用package・capability・debug symbolは通常imageへ入れず、profile用DockerfileまたはCompose overrideへ分離する
- [ ] ツールあり／なしで同一revisionを各3回走らせ、スコア中央値とCPU使用率の差から計測overheadを確認する
- [ ] 認証token、Cookie、決済情報をaccess log、packet capture、profile artifactへ残さない
- [ ] artifactはrun IDでまとめ、HTTP、SQL、CPU、I/Oを同じ時刻範囲で照合できるようにする

### 現在の利用可否

| 優先度 | ツール | 現在の状態 | 主な用途 |
|---|---|---|---|
| P0 | `alp 1.0.21` | macOSホストへ導入済み。現在のnginx logには時間項目なし | endpoint別件数、p50 / p95 / p99、総処理時間 |
| P0 | MySQL `performance_schema` / `sys` schema | `performance_schema=ON` | SQL fingerprint別の回数、累積時間、lock・I/O |
| P0 | `docker stats` / `docker top` | 利用可能 | container別CPU、memory、block I/O、process |
| P0 | `hyperfine 1.20.0` | macOSホストへ導入済み | build、initialize、起動、補助scriptの反復比較 |
| P0 | `vegeta` | macOS arm64版を導入済み | 単一endpointのrate別micro load |
| P0 | `k6 2.1.0` | macOS arm64版を導入済み | 複数endpointをまたぐstateful scenario、threshold、custom metric |
| P0 | nginx `stub_status` | nginx 1.27.5へ組込み済み、endpoint未設定 | active / reading / writing / waiting connection、accept / request数 |
| P1 | `pt-query-digest` | 未導入。MySQL slow logもOFF | slow queryのfingerprint、p95、lock time、rows examined |
| P1 | `pt-stalk` | 未導入 | MySQLのThreads_running急増や短いstall発生時の自動証拠採取 |
| P1 | `sysbench` | 未導入、aarch64対応 | MySQL設定やdisk / mutexの合成負荷比較 |
| P1 | `perf` / `cargo-flamegraph` | Colima VMへ未導入、`perf_event_paranoid=4` | release binaryのCPU sample、cycles、cache miss |
| P1 | `pprof-rs` | 未導入、診断featureの組込みが必要 | `perf`権限を使わないin-process CPU sampling |
| P1 | `samply` / macOS Instruments | `samply`未導入、`xctrace`等はホストに存在 | native実行時のsampling profile |
| P1 | `tokio-console` / `tokio-metrics` | 未導入、診断用buildが必要 | taskのbusy / idle、poll時間、wake、resource待ち |
| P1 | `tracing-chrome` / Perfetto | 未導入、既存`tracing`へLayer追加が必要 | handler、pool取得、SQL、外部HTTPの時系列表示 |
| P1 | `vmstat` / `pidstat` / `iostat` | VMには`vmstat`のみ、macOSには`iostat` | CPU runnable、context switch、process I/O、disk待ち |
| P1 | Criterion | 未導入 | matcher、距離計算、payload生成のRust microbenchmark |
| P1 | [Gungraun](https://github.com/gungraun/gungraun) | 未導入。旧Iai-Callgrind、Linux診断imageへValgrindが必要 | instructions、estimated cycles、cache miss、branchの安定した相対比較 |
| P2 | `sccache` | 未導入、BuildKit cacheとrelease incrementalは利用中 | source変更後のRust再build短縮 |
| P2 | [`cargo-pgo`](https://github.com/Kobzol/cargo-pgo) / `llvm-profdata` | 未導入。Rust 1.83 toolchainに`llvm-tools-preview`なし | 代表負荷で学習したPGO binaryの生成 |
| P2 | `cargo build --timings` / `rustc -Z self-profile` | `--timings`は実施済み、self-profile用nightlyは未導入 | Cargo依存graphとrustc内部のcompile時間 |
| P2 | `strace -c` | VM・containerへ未導入、ptrace権限が必要 | syscall回数と時間、connect / write / fsync / futex |
| P2 | `tcpdump` / `tshark` / `ss` | macOSに`tcpdump`あり。Docker bridge内captureは未検証 | connection再利用、再接続、TIME_WAIT、再送 |
| P2 | `bpftrace` / BCC tools | VMへ未導入、kernel capability検証が必要 | off-CPU、block I/O、TCP、scheduler待ち |
| P2 | `heaptrack` / Valgrind Massif | 未導入 | heap allocation量、peak memory、allocation hotspot |
| P2 | `dhat-rs` | 未導入、実験的な診断crate | Rust関数単位のallocation回数、bytes、peak heap |
| P2 | `cargo-show-asm` | 未導入 | CPU hotspotのassembly / LLVM IR確認 |

### Rust performance tuning How To

現在のwebappは`rustc 1.83.0 (aarch64-unknown-linux-gnu)`でbuildされ、macOSホストは別version・別targetです。`Cargo.toml`に独自profileはなく、通常releaseはCargo既定の`opt-level=3`、`lto=false`、`codegen-units=16`です。Docker buildだけ`CARGO_INCREMENTAL=1`とLLDを追加しています。ホスト上の結果は補助情報とし、採否はwebappと同じLinux target、依存version、compiler、RUSTFLAGSで判断します。

#### 0. 計測契約を固定する

- [ ] 変更前に次をrun manifestへ保存する
  - Git revisionとdirty diffの有無
  - `rustc -vV`、`cargo -V`、target triple、`Cargo.lock`のhash
  - `RUSTFLAGS`、Cargo profile、binaryのSHA-256、image ID
  - Colima CPU / memory、MySQL設定、benchmark seedと実行時刻
- [ ] 正当性を満たす同一revisionを3回実行し、score、成功request数、endpoint別p95、webapp CPU・RSSの中央値をbaselineにする
- [ ] Rustだけの改善目標を「score中央値」「CPU time / request」「対象endpoint p95」「allocation回数」のうち少なくとも1つで数値化する
- [ ] compiler、依存更新、SQL、Rustロジック、build flagを同じrunで同時変更しない
- [ ] 先に次の分岐で使うツールを1つに絞る

| 観測した症状 | 最初のRust向けツール | 確認する値 | 次の判断 |
|---|---|---|---|
| webapp CPUが先に飽和 | `perf` / `pprof-rs` | on-CPU symbol、cycles、IPC | hot symbolをmicrobenchmarkへ切り出す |
| CPUに余裕があるのにp95が高い | `tokio-console` / `tracing-chrome` | busy / idle、poll、wake、pool・I/O待ち | scheduler、lock、DB、networkを分離する |
| `malloc` / `free`やRSSが目立つ | `dhat-rs` / heaptrack | allocation回数、bytes、peak、stack | hotな所有・clone・bufferだけ直す |
| pure Rust関数の候補比較 | Criterion | wall time、throughput、95% CI | 明確な差だけ全体benchmarkへ進める |
| 小差がnoiseへ埋もれる | Gungraun | instructions、estimated cycles、cache / branch event | wall timeと同方向か確認する |
| CPU hotspotの理由が不明 | `cargo-show-asm` | bounds check、分岐、vectorization、clone / format call | source上の仮説を1つ作る |
| 実装を詰めた最終binary | `cargo-pgo` | non-PGO対PGOのscore・CPU・p95 | 代表負荷でのみ改善する偏りを除外する |
| 反復buildが遅い | `cargo build --timings` / `sccache` | critical path、重複crate、cache hit率 | runtime改善と別に管理する |

#### 1. release相当の診断binaryを作る

- [ ] 通常releaseを変更せず、次のcustom profileを診断用branchまたはprofile用patchに追加する

  ```toml
  [profile.profiling]
  inherits = "release"
  debug = "line-tables-only"
  strip = "none"
  ```

- [ ] stackが欠ける場合だけ`RUSTFLAGS`へ`-C force-frame-pointers=yes`を追加し、現在の`-C link-arg=-fuse-ld=lld`を消さない
- [ ] `cargo build --profile profiling --locked --frozen`の実際のrustc引数をverbose logで保存する
- [ ] 通常releaseとprofiling buildを各3回比較し、debug line / frame pointer自体のscore・CPU overheadを記録する
- [ ] symbols、profiler、診断crate、追加capabilityを通常imageと最終binaryへ残さない

#### 2. 全体profileからhot pathを1つ選ぶ

- [ ] 60秒benchmark全体をprofileし、warm-up直後やinitializeだけでなく定常区間を含める
- [ ] flame graphの上位を次の5分類で集計する
  - SQL / sqlx decodeとconnection pool待ち
  - Tokio scheduler、wake、lock、channel
  - serde、JSON、文字列format
  - allocation、clone、collection growth
  - matcher、距離・料金・集計などpure Rust計算
- [ ] sample数が少ないsymbol、inlined frame、unknown frameはすぐ最適化せず、sampling時間・debug line・frame pointerを見直す
- [ ] CPU samplingはI/O待ち時間を直接示さないため、CPUが低い遅延を`tokio-console`なしで「Rust計算が遅い」と結論付けない
- [ ] 上位1〜3 symbolの合計CPU比率とrequest当たり呼出回数を記録し、最大寄与の1件だけを次の検証へ進める

#### 3. pure Rust処理をCriterionで比較する

- [ ] `calculate_distance`、`calculate_fare`、`sum_sales`、`get_chair_stats`の純粋部分、matcher候補生成・rankingをDB / networkから分離してbenchから呼べる形にする
- [ ] 実データ分布を匿名化したfixtureを固定し、matcherは1 / 8 / 32 / 64 rides × chairs、疎・密・同距離・地域偏りを分ける
- [ ] `BenchmarkId`で実装と入力sizeを識別し、`Throughput::Elements`で1秒当たり候補数またはride数を出す
- [ ] 入出力を`std::hint::black_box`へ通し、compilerが処理全体を消すのを防ぐ
- [ ] fixture生成を測らないbenchは`iter_batched`でsetupとmeasurementを分け、allocation込み／buffer再利用の2種類を意図的に分ける
- [ ] 最初はwarm-up 3秒、measurement 10秒、sample 100以上を基準にし、実行時間が長いmatcherはsample数を下げた理由を記録する
- [ ] 変更前baselineを保存し、同じtarget・compilerで変更後と比較する

  ```bash
  cargo bench --bench matcher -- --save-baseline before
  cargo bench --bench matcher -- --baseline before
  ```

- [ ] p値だけで採用せず、95%信頼区間が実用上のnoise幅を越え、3回の実行で方向が一致することを確認する
- [ ] microbenchmarkで改善しても、profile上の寄与率から全体改善上限を見積もり、60秒benchmarkでscoreと正当性を再確認する

#### 4. 小差をGungraunで命令・cache単位に分解する

- [ ] Linux profile imageへValgrindを追加し、通常imageには入れない
- [ ] Gungraun libraryと`gungraun-runner`を同一versionへ固定し、Rust 1.83でcompileできるversionを診断image内で確認する
- [ ] Criterionと同じpure function・fixtureを`cargo bench --bench matcher-callgrind`で1回実行する
- [ ] instructions、estimated cycles、L1 / LL cache event、branch eventを変更前後で保存する
- [ ] Valgrindのsimulated eventはwall timeではなく安定した相対指標として使い、network・DB込みのAxum endpoint速度を代用させない
- [ ] 命令数が減ってもCriterionのwall timeと全体benchmarkが悪化した変更は採用しない

#### 5. allocationを関数単位で削る

- [ ] CPU profileでallocatorがhot、またはRSS / allocation rateが高い場合だけ`dhat-rs`用の診断testを作る
- [ ] 通知payload、matcher候補Vec、chair stats集計ごとにtotal allocation、total bytes、peak bytesを固定fixtureで保存する
- [ ] `SELECT *`削減、`Vec::with_capacity`、buffer再利用、不要な`clone` / `format!`の削減を1件ずつ比較する
- [ ] `SmallVec`、別allocator、unsafe、複雑なborrow化はallocation減少だけで採用せず、CPU profileと全体scoreの改善を必須にする
- [ ] `dhat-rs`は実験的な診断crateとしてfeatureまたはtestへ閉じ込め、最終binaryからglobal allocatorとprofilerを除く

#### 6. asyncの「遅い」と「待っている」を分離する

- [ ] `diagnostics` featureを作り、`tokio_unstable`、Tokio `tracing`、`console-subscriber`、`tracing-chrome`を診断buildだけで有効にする
- [ ] notification long polling、coordinate更新、matcher、payment retryへ固定名のspan / task名を付ける
- [ ] taskごとのbusy時間、scheduled時間、poll回数・時間、wake回数を採取する
- [ ] handler spanを「pool acquire」「SQL」「serde」「外部HTTP」へ分け、長いspan内にどの待ちがあるかをPerfettoで確認する
- [ ] 過剰wakeはpoll loop・短すぎるtick、長いbusy pollはCPU処理・blocking処理、長いidleはI/O・resource待ちとして別仮説にする
- [ ] `spawn_blocking`はCPU処理を消さず別threadへ移すだけなので、Tokio worker starvationを解消し全体throughputも上がる場合だけ採用する
- [ ] instrumentation有無を比較し、final runでは`diagnostics` featureをOFFにする

#### 7. 生成コードとrelease設定を最後に比較する

- [ ] profilerでhotと判明した関数だけを`cargo asm --lib --rust <function>`または`--llvm`で確認する
- [ ] 小さすぎてinlineされた関数はbench用のmonomorphic wrapperで確認し、製品コードへ`#[inline(never)]`を残さない
- [ ] bounds check、panic path、vectorization、不要なcopy / allocation callをsource上の仮説と対応させる
- [ ] 現在のreleaseを対照に、`opt-level=2`、`codegen-units=1`、`lto="thin"`をまず1項目ずつ比較する
- [ ] 単独で改善した設定だけを組み合わせ、build時間、binary size、起動時間、CPU、60秒scoreを各3回記録する
- [ ] `target-cpu=native`は本番サーバー上でbuildして同じCPUだけで実行する場合に限り比較し、Apple Siliconで作ったbinaryをamd64へ持ち込まない
- [ ] `panic="abort"`はhandler panic時にprocess全体が終了する意味変更を伴うため、速度目的の候補から外す
- [ ] `cargo clippy --release --all-targets --all-features -- -W clippy::perf -D warnings`を低コストな候補抽出に使い、lint解消を速度改善の証拠にしない

#### 8. PGOを最終候補binaryへ適用する

- [ ] SQL回数、pool待ち、ログ、上位CPU hotspotを解消した後だけPGOへ進む
- [ ] Rust 1.83のprofile imageへ`llvm-tools-preview`を追加し、`llvm-profdata`のpathとversionを保存する
- [ ] `cargo-pgo`を使う場合は`cargo pgo info`で環境を確認し、現在のLLD用RUSTFLAGSとPGO flagが両方rustcへ渡ることをverbose logで確認する
- [ ] instrumented binaryを通常stackへ組み込み、native rustc方式では`LLVM_PROFILE_FILE`を診断volume上の絶対pathへ設定する。`cargo-pgo`方式ではtool管理のprofile directory自体を永続化する
- [ ] initialize、主要app / chair / owner API、notification、matcher、paymentを含む代表的な60秒benchmarkでprofileを収集する
- [ ] `.profraw`を`llvm-profdata merge`し、`llvm-profdata show`でprofileが空でなく主要経路を含むことを確認する
- [ ] 最適化buildでは`-Cprofile-use=<absolute-path>`と`-Cllvm-args=-pgo-warn-missing-function`を使い、profile欠落を記録する
- [ ] 簡易手順を使う場合も「instrument → 代表負荷で学習 → optimize」の3段階を分ける

  ```bash
  cargo pgo info
  cargo pgo build
  # instrumented isurideを通常stackで起動し、代表benchmarkを実行する
  cargo pgo optimize
  ```

- [ ] 学習に使ったrunとは別の3回でnon-PGO対PGOを比較し、score中央値、最小値、p95、CPU、全エラーコードを確認する
- [ ] 特定seedだけ速く別seed・別初期状態で悪化する場合はprofile偏りとして不採用にする
- [ ] BOLTはDocker対応が実験的で導入・symbol要件も増えるため、PGO単独の改善を確認するまで追加しない

### HTTP: `alp` とnginx access log

- [ ] nginxへ診断用LTSVまたはJSON `log_format` を追加する
  - method、正規化可能なURI、status、`request_time`
  - `upstream_response_time`、`upstream_connect_time`
  - request / response bytes、connection ID、`connection_requests`
- [ ] path parameterのride IDなどを `alp --matching-groups` でまとめ、同一endpointを別行へ分散させない
- [ ] `alp` でcount、sum、avg、p50、p95、p99、max、5xx / 499件数をendpoint別に出す
- [ ] `request_time - upstream_response_time` からnginx・socket・client側の待ちを推定する
- [ ] `alp --dump` と `alp diff` で変更前後を機械比較できる形にする
- [ ] access logをtmpfsまたは診断用volumeへ出す場合と無効化した場合を比較し、log I/Oのscore overheadを測る
- [ ] localhostまたはDocker network内だけから参照できる`stub_status` endpointを診断構成へ追加する
- [ ] `stub_status`のactive、reading、writing、waiting、accepts、handled、requestsを1秒間隔で採取する
- [ ] `handled < accepts`、active connection上限、writingの継続増加をnginx側の飽和兆候として扱う
- [ ] 採用条件: 30ms超過を作るendpointと回数を特定でき、ツール有効時のスコア低下が許容範囲または別runへ分離できる

### SQL: `performance_schema`、`sys` schema、`pt-query-digest`

- [ ] benchmark直前にstatement digestをresetし、終了直後に回数、合計、平均、最大、rows examinedを保存する
- [ ] `sys.statement_analysis`、`sys.statements_with_full_table_scans`、`sys.schema_table_statistics` を同じrunで保存する
- [ ] transaction、data lock、metadata lock、pool待ちを同じ時刻軸へ載せる
- [ ] slow logを診断runだけ有効化し、30ms目標より短い `long_query_time=0.01` から開始する
- [ ] [`pt-query-digest`](https://docs.percona.com/percona-toolkit/pt-query-digest.html) でfingerprint別のQuery time、Lock time、Rows examined、p95を集計する
- [ ] slow logのthresholdを10 / 30 / 100msで比較し、log量とMySQL CPU・I/O overheadを測る
- [ ] `performance_schema`のdigestと`pt-query-digest`で上位SQLが一致するか照合する
- [ ] [`pt-stalk`](https://docs.percona.com/percona-toolkit/pt-stalk.html) をまず`--no-stalk`の1回採取で試し、収集ファイル・時間・負荷を確認する
- [ ] baselineの`Threads_running`分布からtrigger thresholdと連続cycle数を決め、瞬間的なDB stallだけを捕捉する
- [ ] `pt-stalk`のGDB、strace、tcpdump collectorはMySQLを停止・減速させ得るため既定で無効にする
- [ ] MySQL認証情報はcommand lineへ渡さず、診断containerだけのoption fileを使う
- [ ] `sysbench`はISURIDE表を直接更新せず、別schemaでbuffer pool、redo、fsync設定の候補を比較する
- [ ] 合成OLTPの改善だけでは採用せず、同じMySQL設定を公式benchmarkで再検証する
- [ ] final runではslow logをOFFへ戻し、`performance_schema`も必要なconsumerだけに絞る
- [ ] 採用条件: SQLの「単発の遅さ」と「回数による累積負荷」を分離し、改善対象のfingerprintを一意に示せる

### 資源: `docker stats`、`vmstat`、`pidstat`、`iostat`

- [ ] 1秒間隔でwebapp、MySQL、nginx、matcher、benchmarkerのCPU、memory、block I/O、PID数を保存する
- [ ] Colima VMの`vmstat 1`を同時取得し、run queue、context switch、CPU idle / iowaitを記録する
- [ ] profile用VMへ`sysstat`を導入できるか確認し、`pidstat -urd`と`iostat -xz 1`を採取する
- [ ] macOSの`iostat`はホスト全体、Linuxの`iostat`はColima VM内の値として分けて記録する
- [ ] cgroupのCPU throttlingとmemory pressureを確認し、単なるCPU使用率100%とquota待ちを区別する
- [ ] 採用条件: webapp CPU、MySQL CPU、I/O、CPU quotaのどれが先に飽和したかを時系列で説明できる

### CPU profile: `perf`、`cargo-flamegraph`、`samply`、Instruments

- [ ] Colima VMへ対応kernelの`perf`を導入できるか確認する
- [ ] `kernel.perf_event_paranoid=4`、Docker seccomp、`CAP_PERFMON` / `CAP_SYS_PTRACE`のどれが採取を阻害するか切り分ける
- [ ] 通常構成の権限を広げず、profile用Compose overrideだけへ必要最小限のcapabilityを付与する
- [ ] release binaryへline tableとframe pointerを付けた診断用buildを作り、通常releaseとの速度差を記録する
- [ ] `perf stat`でcycles、instructions、IPC、context switches、cache missを採取する
- [ ] [`cargo-flamegraph`](https://github.com/flamegraph-rs/flamegraph) または[`samply`](https://github.com/mstange/samply) で60秒runのCPU flame graphを作る
- [ ] `perf`権限の準備が高コストなら、profile featureで[`pprof-rs`](https://github.com/tikv/pprof-rs)を組み込み、SIGPROF samplingを先に試す
- [ ] `pprof-rs`はsampling frequencyを100 / 250 / 500Hzで比較し、drop sample、CPU overhead、stack解決率を記録する
- [ ] signal handlerのunwind riskを避けるblocklistを設定し、診断endpointを外部公開せずprofile artifactをvolumeへ出す
- [ ] macOS Instrumentsはcontainerへ直接attachせず、webappをnative起動した補助実験だけに使い、Docker runと同一視しない
- [ ] flame graphでDB/socket待ち、serde、allocation、tracing、matcher計算の比率を分類する
- [ ] pure Rust関数がCPU hotspotと判明した場合だけ`cargo-show-asm`でbounds check、vectorization、不要なclone / formatを確認する
- [ ] assemblyの短さだけで採用せず、Criterionと全体benchmarkの両方で改善を確認する
- [ ] 採用条件: wall timeの大きいsymbolまたは待ち境界を特定でき、profileから直接1つの改善仮説を作れる

### Rust async: `tokio-console` と `tokio-metrics`

- [ ] `tokio_unstable`、Tokioの`tracing` feature、`console-subscriber`をprofile buildだけで有効にする
- [ ] [`tokio-console`](https://tokio.rs/tokio/topics/tracing-next-steps) でtaskのpoll時間、busy時間、wake回数、resource待ちを確認する
- [ ] notification long polling、coordinate queue、background matcherをtask名付きでinstrumentする
- [ ] sqlx pool取得待ちとTokio scheduler starvationを区別できるspan / metricを追加する
- [ ] `tracing-chrome`をprofile buildへ追加し、handler → pool acquire → SQL → payment HTTPを同一traceへ出す
- [ ] span名はendpoint / operation単位に固定し、ride IDやtokenを名前へ含めずartifactの高cardinalityと秘密情報を防ぐ
- [ ] Chrome trace JSONをPerfetto UIで開き、30ms tickを越えるcritical pathと並行taskの重なりを確認する
- [ ] flush guardをshutdown時まで保持し、途中で欠けたtraceを正常な結果として扱わない
- [ ] instrumentationあり／なしでCPU、memory、スコアを比較し、最終binaryから診断featureを外す
- [ ] 採用条件: 長時間poll、過剰wake、mutex / semaphore待ち、scheduler遅延のいずれかを再現可能に示せる

### Microbenchmark: Criterion、`hyperfine`、`vegeta`

- [ ] Criterion用bench targetを追加し、64×64 matcher候補生成、貪欲法、二部マッチングを同じfixtureで比較する
- [ ] 距離 / speed cost、地域bucket、通知payload生成、chair stats集計の純粋処理をmicrobenchmark化する
- [ ] microbenchmarkはDB・network・lock待ちを含まないため、公式benchmarkの採否を置き換えない
- [ ] `hyperfine`でclean / warm build、initialize、container起動時間をprepare / cleanup付きで比較する
- [ ] `vegeta`では読み取りendpointから始め、固定Cookieを含むtarget fileでrateを段階的に上げる
- [ ] `k6`ではuser登録 → ride作成 → notificationのような複数API scenarioを、setupで毎run初期化して再現する
- [ ] endpoint名をtagへ固定し、動的IDをmetric tagへ入れてcardinalityを増やさない
- [ ] `http_req_failed == 0`、endpoint別p95 < 30msをthresholdとして明示する
- [ ] constant-arrival-rateと固定VUを比較し、server遅延でload generatorの送信率まで落ちる問題を区別する
- [ ] k6 / Vegeta自体のCPUがColima 4 CPUを奪わないようhost実行とbenchmark container実行を比較する
- [ ] nginx経由とDocker network内のwebapp直結を比較し、proxyとアプリのlatencyを分離する
- [ ] 更新endpointは毎run initializeし、同じuser / chairへの非現実的な並行更新で仕様を壊さないload modelにする
- [ ] 採用条件: 変更前後の差が分布として再現し、60秒benchmarkの改善・悪化と方向が一致する

### 深掘り用: `strace`、packet capture、eBPF、heap profiler

- [ ] `strace -c -f`をprofile containerだけで実行し、`connect`、`write`、`fsync`、`futex`、`epoll_wait`の回数と時間を見る
- [ ] `tcpdump`はColima VMまたは同一network namespaceで採取し、Host側interfaceだけのcaptureで結論を出さない
- [ ] `ss -s` / `ss -tanp`でnginx↔webapp↔MySQLのESTABLISHED、TIME_WAIT、再接続を確認する
- [ ] packetにはtokenやpayloadが含まれ得るため、port・header中心の最小captureにしてartifactを公開しない
- [ ] `bpftrace --info`とprobe一覧でColima kernelのBTF、tracepoint、uprobe対応を確認してから導入判断する
- [ ] eBPFではoff-CPU、block I/O latency、TCP retransmitのうち、既存指標で説明できない1項目だけを調べる
- [ ] allocationがflame graphへ現れた場合だけ`heaptrack`またはMassifを使い、bytes、回数、peak、stackを採取する
- [ ] 採用条件: P0 / P1ツールで原因を絞れない待ちやallocationを説明し、権限追加とoverheadに見合う情報が得られる

### Allocation: `dhat-rs` とheap profiler

- [ ] CPU flame graphまたはmemory増加でallocationが疑われた場合だけallocation profileへ進む
- [ ] [`dhat-rs`](https://github.com/nnethercote/dhat-rs) は実験的なため、全アプリではなく通知payloadやmatcher fixtureの診断testから試す
- [ ] allocation回数、総bytes、peak heapをbaselineとして固定し、`SELECT *`削減やbuffer再利用前後を比較する
- [ ] `heaptrack` / Massifは診断containerで短いloadだけに使い、60秒スコアrunへ混ぜない
- [ ] leak判定ではinitializeによるcache世代切替後に古い世代が解放されることを確認する
- [ ] 採用条件: allocation stackと対象処理を結び付け、変更後に回数またはbytesと全体CPUの両方が減る

### 反復開発: `sccache`

- [ ] `cargo build --timings`の`target/cargo-timings/cargo-timing.html`からcritical path、同時実行数、codegen時間、build script時間を保存する
- [ ] `cargo tree --duplicates`で複数versionのcrateを抽出し、feature・target差による正当な重複と、依存整理で統合できる重複を分ける
- [ ] isuride自身のcompileがcritical pathなら、別nightly toolchainの`rustc -Z self-profile`とmeasureme toolsでquery / codegen / LLVM時間を診断する
- [ ] self-profileのために`RUSTC_BOOTSTRAP`で通常Rust 1.83 buildへunstable flagを混ぜず、nightlyの結果はcompile-time仮説だけに使う
- [ ] BuildKit target cacheだけの現在値と、[`sccache`](https://github.com/mozilla/sccache)追加時のclean / warm / 1ファイル変更buildを比較する
- [ ] Rustのincremental compilation artifactはsccache対象外なので、`CARGO_INCREMENTAL=1`の現状と`CARGO_INCREMENTAL=0 + sccache`を別条件で比較する
- [ ] `sccache --show-stats`でcompile request、hit率、cache write、non-cacheable reasonを保存する
- [ ] linkerと最終binaryはcacheされない前提で、現在11.02秒のDocker buildを実測で下回る場合だけ採用する
- [ ] cache volumeの上限と削除手順を決め、Colima disk圧迫でruntime benchmarkを悪化させない
- [ ] build高速化の結果はruntime score改善と分けて記録する

### 現時点では追加しないツール

- [ ] `oha` / `wrk` / `hey`: Vegetaとk6で単一endpoint・stateful scenarioを覆えるため、必要なload modelが不足した場合だけ再検討する
- [ ] Prometheus + Grafana、cAdvisor、PMM: 60秒runには構築・常駐overheadが大きいため、CLI採取で時系列を説明できない場合だけ再検討する
- [ ] `mysqltuner`: 一般的な設定推奨をそのまま採用せず、`performance_schema`と同一workloadの根拠を優先する
- [ ] `cargo-bloat` / `dive`: binary・image sizeは起動や配布には効くが現在のruntime bottleneckを直接示さないため後回しにする
- [ ] OpenTelemetry collector: `tracing-chrome`のローカルtraceでcritical pathを確認できない場合だけ再検討する

### 最初に実施するツール検証順

1. nginxへ時間付き診断logを追加し、導入済み`alp`でendpoint別p95 / p99を取得する
2. 同じrunの`performance_schema` / `sys` schemaと`docker stats` / `vmstat`を保存する
3. 上位endpointとSQLが決まった後に、slow log + `pt-query-digest`を短い診断runで比較する
4. 再現しにくいDB stallがあれば`pt-stalk`、statefulな局所負荷が必要ならk6を使う
5. CPUが律速なら`pprof-rs`または`perf`、async critical pathが不明なら`tracing-chrome` / `tokio-console`を使う
6. matcher algorithmだけはCriterionで候補を絞り、小差だけGungraunで命令・cache eventへ分解してから全体benchmarkで判断する
7. SQL・待ち・CPU hotspotを解消した最終候補だけPGOを学習用とは別runで比較する
8. `strace`、packet capture、eBPF、heap profilerは既存計測で説明できない場合だけ使う

## Phase 1: 小さな変更で待ち行列を減らす

### owner椅子一覧

- [x] `owner_get_chairs` の対象ownerの椅子IDをwindow関数より内側で絞る
- [x] 現在計測済みの候補SQLを改めて `EXPLAIN ANALYZE` する
  - window関数対象: 22,078行 → 253行
  - 単発時間: 約246ms → 約25.5ms
- [x] レスポンスに不要な `owner_id`、`access_token`、`updated_at` を取得しない
- [x] この変更だけで60秒ベンチを行い、`CODE=25` を比較する
  - 改善後: `pass=true`、スコア5,601、CODE=25を含む全エラー0
- [ ] owner SQL変更前後のMySQL一時表作成数を独立計測する

### nearby椅子検索

- [x] `is_active = TRUE` を最初のSQL条件へ移し、非稼働椅子をRustへ転送しない
- [x] 椅子ごとの全rideを調べず、未完了rideが存在しないことを `NOT EXISTS` で判定する
- [x] 椅子ごとの最新位置を `LATERAL` subqueryで1回にまとめる
- [x] SQLは候補椅子と最新座標だけを返し、マンハッタン距離の最終判定だけをRustに残す
- [x] 1リクエストのSQL回数を `1 + C + C×R + C` から1回へ減らす
- [x] read transactionを廃止する
- [x] `retrieved_at` 用の `SELECT CURRENT_TIMESTAMP(6)` をなくす
- [x] `.timestamp_millis()` を使ってRust側の時刻を返す
- [ ] `retrieved_at` は現行benchmarkerの判定に未使用なので、性能施策ではなく仕様準拠として検証する
- [x] 集合SQLだけをBenchmark 04として60秒計測する
- [x] nearbyの内容不一致とtimeoutがないことを60秒ベンチで確認する
- [ ] `CODE=26` 1件との因果を切り分けるため、同一revisionを3回以上走らせる
- [ ] 未完了ride判定のstatus相関subqueryを `rides.evaluation IS NULL` へ置き換え、実行計画と結果を比較する
- [ ] 座標だけを最大3秒cacheし、`is_active` と割当可否は毎回最新状態を合成する案を比較する
- [ ] nearbyレスポンス全体の3秒cacheは割当済み椅子を返すため採用しない

### JSON通知の短期改善

- [ ] ride存在確認とtransaction内の最新ride再取得を1回へまとめる
- [ ] 未送信statusがない場合は高価なpayloadを再構築せず `data: null` を返せるかprevalidationで確認する
- [ ] 未送信status、ride、user/chair、fareを1 SQLで取得する
- [x] `get_chair_stats` を集約SQL1回へ置き換える
- [ ] 初期データの全椅子で旧loop実装と集約SQLの結果を比較する
- [ ] `ARRIVED` / `CARRYING` / `COMPLETED` の一部が欠けるrideを同じように除外する
- [ ] 通知対象のclaimとsent時刻更新は、まず条件付きUPDATEで競合安全にする
- [ ] 同一recipientへの並行pollingが発生する構成になった場合だけ `FOR UPDATE SKIP LOCKED` を比較する
- [ ] transactionは未送信statusのclaimからsent更新までの最短区間だけにする
- [ ] app/chairそれぞれで、状態遷移の順序とat least onceを並行リクエストでも確認する
- [x] `retry_after_ms` を30 / 50 / 100msで比較し、通知遅延とDB負荷の交点を測る
  - 50 / 100msはCOMMIT回数を減らしたがスコアを改善せず、実装は30msへ戻した
  - 詳細は [`tuning/10-notification-retry-interval.md`](./tuning/10-notification-retry-interval.md)
- [ ] 同じ利用者・椅子への直前payloadと最新ride状態をcacheし、状態不変時のSQLとJSON再構築をなくす
- [ ] cache keyをrecipient ID、valueを `last_status_id` / ride version / payloadとし、ride割当・status追加・評価確定で明示的にinvalidateする
- [ ] TTLだけに依存せず、cache missとプロセス再起動時はDB履歴から復元する
- [ ] JSON APIのまま最大60秒のlong pollingを実装し、状態変化時に `Notify` / channelで即時wakeする案をSSEより先に比較する
- [ ] version確認 → waiter登録 → version再確認の順にして、確認と待機開始の間に発生した通知を取りこぼさない
- [ ] long polling中はDB connectionとtransactionを保持せず、切断・timeout・再接続時もat least onceを維持する
- [ ] cacheはpayload生成の高速化だけに使い、`app_sent_at` / `chair_sent_at` の配信cursorと混同しない
- [ ] 未配信statusが複数ある再接続では、cacheの最新1件だけを返さず `created_at, id` 順で全遷移を送る
- [ ] JSON polling、JSON long polling、SSEを同一条件で比較し、protocol変更だけではなくDB query数と通知遅延が減った案を採用する

### 決済と評価

- [ ] すべての決済POSTへride IDを `Idempotency-Key` として付与する
- [ ] 同じkey・token・amountでretryし、エラー応答後も二重決済しない
- [ ] 現行の `GET /payments` による照合を除去し、固定300ms待ちとuserのride全件取得をなくす
- [ ] `reqwest::Client` を `AppState` に1個保持し、POST/GETとretryでconnectionを再利用する
- [ ] 決済URLをinitialize時にメモリへ読み込み、評価ごとのsettings検索をなくす
- [ ] ride、payment token、fareの読取りを短い区間へまとめる
- [ ] 外部決済HTTPとretry sleep中はDB transactionを保持しない
- [ ] 評価と `COMPLETED` 追加を短いwrite transactionへ分離する
- [ ] 決済成功後にだけ評価、chair stats、`COMPLETED` を同じwrite transactionで確定する
- [ ] write transaction成功後にだけ評価APIの200を返す
- [ ] 同じrideへの並行評価を防ぐ状態またはride単位mutexを設ける
- [ ] 「決済成功後にDB更新失敗」と「HTTPエラーだが決済成功」の両方で二重決済しない設計にする
- [ ] 正常系、決済retry、重複評価、タイムアウトのテストを追加する
- [ ] `CODE=6`、`CODE=34`、`CODE=35` と評価APIのp99を比較する

### 座標更新

- [x] INSERT直後の `chair_locations` 再SELECTをなくす
- [x] `recorded_at` はINSERTへ渡した時刻をそのままレスポンスへ使う
- [x] rideと最新statusを別々に取得せず、現在rideだけを1 SQLで取得する
- [x] 座標がpickup/destinationと一致しない通常経路では、status INSERTなしで早くcommitする
- [ ] 同じstatusを重複INSERTしない条件付き遷移へする
- [x] 通常の1座標更新あたりのSQL回数を4回から2回へ削減する
- [ ] 座標更新のtransaction保持時間、p95 / p99を比較する
- [ ] 座標更新をper-chair順序付きのbounded queueへ投入し、HTTP応答と永続化・status判定を分離する実験を行う
- [ ] 最新座標をメモリ上では即時更新し、`chair_locations` を30 / 50 / 100ms単位でbulk INSERTする
- [ ] queue内の中間座標は累積距離と乗車地点・目的地への到達判定に必要なので、最新1件へ無条件にcoalesceしない
- [ ] nearby向けlatest-coordinate cacheだけを上書きし、永続化対象の全座標列とは分離する
- [ ] queue full時のbackpressure、DB失敗時の再試行、initialize / shutdown時のflushを定義する
- [ ] HTTP 200をqueue投入時とDB commit後のどちらで返すか比較し、応答p99と再起動時の座標欠落リスクを記録する
- [ ] 非同期化後も座標は3秒以内、割当可否と到着statusは通知評価を落とさない時間内に反映する
- [ ] 同じ椅子の座標順序、累積距離、`PICKUP` / `ARRIVED` の一度だけの遷移を並行負荷で検証する

### 招待couponのINDEX

- [x] `coupons(code)` をPhase 2より先に単独追加する
- [x] `SELECT * FROM coupons WHERE code = ? FOR UPDATE` の全走査とlock範囲を変更前後で比較する
  - 変更前: coupon 698行をtable scan、約6.41ms、対象2行
  - 変更後: coupon 766行中3行をindex lookup、約0.389ms
- [x] `SHOW ENGINE INNODB STATUS` でcoupon検索同士のlock待ち・deadlock原因を確認する
- [x] このINDEXだけで60秒ベンチを行い、`CODE=17` が2件から0件になることを確認する
- [ ] 利用者登録とride作成のp95 / p99を比較する

## Phase 2: N+1と重複計算をなくす

### 利用者ライド履歴

- [ ] completed rideだけをSQL側で絞る
- [ ] ride、chair、owner、適用couponをJOINし、1 SQLでレスポンス行を返す
- [ ] 最新status取得をrideごとのqueryからJOINまたはcurrent status列へ置き換える
- [ ] fareをライド作成時に確定保存し、履歴表示ごとのcoupon検索をなくす
- [ ] ridesへ列追加する場合は初期ダンプ投入後にALTERし、初期rideの `updated_at` を変えずにbackfillする
- [ ] read-only transactionを廃止する
- [ ] 履歴0 / 1 / 多数件で内容と順序を検証する

### ライド作成

- [ ] 進行中rideの有無を `EXISTS` 1回で判定する
- [ ] userごとのactive rideを一意に表現できるcurrent-state表を検討する
- [ ] INSERT後の `COUNT(*)` とride再SELECTをなくす
- [ ] 使用couponの選択とclaimを1 SQLまたは条件付きUPDATEへまとめる
- [ ] fareとdiscountをrideへ保存する場合は、別表または初期ダンプ後のALTERを使う
- [ ] 同一userからの並行作成で2件のactive rideを作らない

### オーナー売上

- [ ] chairごとのride取得をowner単位の集約SQL1回へ置き換える
- [ ] `COMPLETED` 判定はstatus履歴JOINではなく、`evaluation IS NOT NULL` またはcurrent statusを使えるか検証する
- [ ] `(chair_id, updated_at)` を利用して `since` / `until` を先に絞る
- [ ] chair別、model別、totalを同じ入力集合から計算する
- [ ] read transactionと暗黙ROLLBACKをなくす
- [ ] 0売上の椅子とモデルもレスポンスへ残す

### 招待とcoupon

- [ ] `SELECT * FROM coupons WHERE code = ?` を `COUNT` または存在確認へ縮小する
- [ ] 招待回数をcoupon全件から数えず、inviterのcounterを条件付きUPDATEする案を比較する
- [ ] 先行追加した `coupons(code)` の利用回数とwrite costを再評価する
- [ ] 未使用coupon検索用 `(user_id, used_by, created_at)` を比較する
- [ ] `WHERE used_by = ?` 用INDEXまたは `UNIQUE(used_by)` を比較する
- [ ] `coupons(used_by)` を単独追加し、ride履歴・coupon claimの実行計画とwrite costを比較する
- [ ] coupon書き込みコストを含め、不要・重複INDEXを残さない

### 認証

- [ ] middlewareのtoken検索回数と累積時間を利用者・椅子・owner別に計測する
- [ ] queryはレスポンスに必要な列だけ取得する
- [ ] tokenからIDと最小属性を引くプロセス内cacheを導入する
- [ ] initialize時に初期tokenをcacheへ再構築する
- [ ] 動的登録時にcacheへ追加する
- [ ] activityやowner情報更新で古いsnapshotを使わないよう、可変属性は分離する
- [ ] cache miss時だけDBへfallbackし、再起動後も正しく復元する

## Phase 3: 現在状態を履歴検索から分離する

履歴テーブルは仕様検証用に残し、ホットパスは椅子・ライドごとに1行の現在状態を読む構造へ移します。

### 椅子の現在状態

- [ ] 椅子ごとの最新緯度・経度・更新時刻を1行で保持する
- [ ] 累積移動距離を座標更新時に差分加算する
- [ ] 現在割当中のride IDを保持する
- [ ] 空車・active・最新位置を1回で取得できる構造にする
- [ ] `chair_locations` への履歴INSERTと現在状態更新を同じtransactionで行う
- [ ] initialize時に初期履歴から現在状態と累積距離をbackfillする
- [ ] nearby掲載可能とmatcher再割当可能を別の状態として保持する
- [ ] `owner_get_chairs` とnearbyから位置履歴のwindow関数を除去する

### ライドの現在状態

- [ ] `rides.updated_at` を変えないよう、ride ID主キーのcurrent-state別表を作る
- [ ] status履歴INSERTとcurrent status更新を同じtransactionにする
- [ ] status遷移を期待する直前状態とのcompare-and-swapにする
- [ ] userのactive rideとchairのactive rideをO(1)で検索可能にする
- [ ] matcher、coordinate、ride作成、nearbyから最新status subqueryを除去する
- [ ] 履歴の全状態遷移が残ることを検証する

### 集計の事前計算

- [ ] chairの完了ride数と評価合計を保持し、完了時に差分更新する
- [ ] 通知のchair statsをO(1)で返す
- [ ] ownerのchair別・model別売上を完了時に差分更新する案を比較する
- [ ] 更新失敗時に履歴から再構築できる手順を用意する
- [ ] chair statsは評価・`COMPLETED` と同時commitし、通知遷移点で厳密に一致させる
- [ ] 3秒の整合性猶予はowner累積距離だけへ適用する

## Phase 4: pollingとmatcherの上限を外す

### SSE通知

- [ ] JSONのcache / long pollingで到達できるスコアを先に計測し、SSE移行の追加利益を見積もる
- [ ] app/chair通知を `text/event-stream` で返す実験ブランチを作る
- [ ] streamを開いたままDB connectionやtransactionを保持しない
- [ ] 接続直後に必要な最新状態または未送信状態列を順番どおり送る
- [ ] 状態変化時だけpayloadを生成してpushする
- [ ] 1クライアント1pollではなく、1つのdispatcherが未配信statusをbatch取得する
- [ ] user ID / chair IDごとの接続registryとbounded queueを用意する
- [ ] 遅いclient、切断、再接続、queue overflow時のat least onceを定義する
- [ ] sent cursorをDBへ持つか、既存 `app_sent_at` / `chair_sent_at` を使うか比較する
- [ ] nginxで `proxy_buffering off`、十分な `proxy_read_timeout`、keepaliveを設定する
- [ ] 60秒中の通知HTTPリクエスト数、DB query数、接続数、メモリをJSON pollingと比較する
- [ ] SSEへ形式だけ移行せず、status変更時の即時pushと接続単位cacheまで含めて評価する
- [ ] prevalidationと全通知エラーコードが通るまでJSON実装を削除しない

### matcherの再設計

- [x] 1呼び出し最大64件のbatch処理を実装する
- [x] batch matcherだけを独立したBenchmarkとして検証する
- [ ] matcherのcurl → nginx → Axum往復をTokio background taskまたはride作成時のenqueueへ置き換える
- [x] pending rideと空き椅子をそれぞれ一度だけ取得する
- [x] rideとchairを同じtransactionで `FOR UPDATE SKIP LOCKED` する
- [ ] `UPDATE ... WHERE chair_id IS NULL` のaffected rowsで競合負けを検出する
- [ ] chair側もcurrent rideがNULLの場合だけclaimする
- [ ] `COUNT(chair_sent_at) = 6` を「最新rideの `COMPLETED` が椅子へ送達済み」という明示状態へ置き換える
- [x] 1回のbatchで同じchairを2件へ割り当てない
- [x] oldest ride優先を維持する
- [x] `is_active = TRUE` をDB matcherの不変条件にする
- [x] matcher間隔を500 / 100 / 30msで比較し、matching latencyとDB負荷を測る
  - 500ms対照: 53,198点、matching不満足度9.5%
  - 100ms: 54,172 / 53,715点、中央値53,943.5点
  - 30ms: 41,016点、matching不満足度0.2%だが最終評価数560へ低下
  - 局所的な割当待ちは短縮しても総得点が下がるため、500msを維持
- [ ] CODE=32または未マッチ滞留が悪化したら、他のP1施策よりmatcherを繰り上げる
- [x] 乗車地点に近い椅子を優先するthroughput重視policyを計測する
- [ ] 2地域をまたぐ遠距離割当を避ける距離上限を設け、近隣椅子がないrideは次batchへ保留する
- [ ] 距離上限を100 / 200 / 400で比較し、地域ごとの未マッチ滞留と枯渇を監視する
- [ ] pickup座標とchair座標を地域bucketへ分類し、同一地域内だけを候補にする方式と単純な距離上限を比較する
- [ ] chair modelのspeedを候補取得時にJOINし、距離ではなく `ceil(distance / speed)` のpickup予測tickを最小化する
- [ ] matcherの目的関数を「割当件数最大化 → 期限超過ride最小化 → pickup予測tick最小化」の辞書順で定義する
- [ ] 64件batch内の貪欲法と最小費用二部マッチングを、計算時間・空車移動距離・完了数で比較する
- [ ] 二部マッチングではride待ち時間をcostへ加え、近い新規rideだけが選ばれて古いrideがstarvationしないようにする
- [ ] matcher自身の計算を30ms以内に収め、64×64の候補生成・最適化時間を独立計測する
- [ ] 走行中の椅子について「現在rideの完了予測時間 + 次の乗車地点までの時間」が空き椅子より短い場合の先行予約を高リスク実験として評価する
- [ ] dispatch評価を落とさない範囲で距離スコアを増やすpolicyを別ベンチで比較する
- [x] ID順と近傍優先について完了数、不満率、最終スコアを記録する
- [ ] 同一revisionを3回以上実行し、中央値・最小・最大を比較する

## Phase 5: Rust・DB・nginxの上限調整

### Rust / sqlx

- [x] BuildKit cache mountでCargo registry、Git、release targetを保持する
- [x] releaseの `opt-level=3` を維持し、incrementalとtoolchain同梱LLDを使う
- [x] `cargo build --timings` でtiming reportを生成する
- [x] build中だけ前回のISUCON stackを正常停止し、build後にhealthcheck付きで再開する
- [ ] `SELECT *` をhot pathから除き、転送・decode・allocationを減らす
- [ ] response件数が分かる `Vec` はcapacityを事前確保する
- [ ] `RUST_LOG=info` と `warn` でログ行数、byte数、CPU、スコアを比較する
- [ ] TraceLayerとエラーログの量を測り、成功requestログだけを抑制する
- [ ] poolの `min_connections`、`max_connections`、`acquire_timeout` を計測で調整する
- [ ] pool上限を増やす前にMySQL CPUと実行中thread数に余裕があることを確認する
- [ ] release binaryをperf / samply / Instrumentsでprofileする
- [ ] DB待ちが支配的でなくなった後だけLTO、codegen-units、`target-cpu` を比較する
- [ ] allocationがhotになった場合だけallocator変更を比較する

### MySQL

- [ ] `EXPLAIN ANALYZE` とstatement digestで使われていないINDEXを特定する
- [ ] 非正規化後に不要になった履歴検索用INDEXを削除し、INSERT/UPDATEのwrite amplificationを減らす
- [x] statusだけを読むqueryにcovering INDEXが有効か比較する
  - `Covering index lookup` へ変化し、単発計画は改善
  - 60秒ベンチは45,075点で対照53,198点を下回ったため不採用
  - schemaは `(ride_id, created_at)` へ復元
- [ ] ULIDやtoken列を `CHAR/VARCHAR ... CHARACTER SET ascii` またはbinary表現に変えた場合のINDEXサイズを比較する
- [ ] `chairs.model` の `TEXT` を上限付き `VARCHAR` へ変える
- [ ] buffer pool hit率、temporary table、sort、redo量を採取する
- [ ] datasetに合わせて `innodb_buffer_pool_size` を調整する
- [x] `SELECT @@log_bin` でbinary logが有効、`SHOW REPLICAS` が空であることを確認する
- [x] `innodb_flush_log_at_trx_commit=2` を単独比較する
  - fsyncは減ったが52,606点で対照53,198点を上回らず、redo側だけでは不十分
- [x] `sync_binlog=0` を追加し、完全同期と3回ずつ比較する
  - 完全同期は30,710–60,200点・中央値53,198点、`2 / 0` は58,220–66,167点・中央値60,102点
  - `COMMIT` 平均の中央値は3.349msから1.722msへ48.6%低下
- [x] MySQL 8.4.10のimage digestをComposeへ固定し、走行条件へ記録する
- [ ] binary logが有効かつ複製・復旧に不要な環境だけ、`--skip-log-bin` を別runで比較する
- [ ] binary log停止は運用・復旧特性を変えるため、通常のSQL改善と分離する
- [ ] durabilityを変える設定は通常施策と分け、再起動試験とデータ損失リスクを明記する
- [ ] `performance_schema` やslow logの計測overheadを最終スコアrunでは外す

### nginx・静的ファイル

- [ ] APIと静的ファイルのrequest数・転送byte数を分けて計測する
- [ ] notificationやcoordinateのaccess logを抑制した場合のI/Oとスコアを比較する
- [ ] upstream keepaliveとHTTP/1.1を明示する
- [ ] SSEではproxy bufferingを無効化する
- [ ] 静的ファイルへ `open_file_cache`、適切なCache-Control、事前圧縮が有効か比較する
- [ ] 小さいJSON APIへ圧縮を掛けてCPU負荷を増やしていないか確認する

## Phase 6: 最大スコア用の高リスク実験

次はSQL回数と待ち時間を十分に減らした後だけ試します。通常の改善結果と混ぜません。

- [ ] active chair、最新位置、current ride、statsをプロセス内メモリへ保持し、nearbyと通知をDBなしで返す
- [ ] 空間をgridへ分割し、nearby候補を全椅子走査せず近隣bucketだけから取得する
- [ ] pending rideと空き椅子をメモリqueueでmatchingする
- [ ] initialize時に全cacheを再構築し、初期化世代を切り替えて古い状態を捨てる
- [ ] 単一process前提を明記し、複数process化する場合は状態共有方式を再設計する
- [ ] `/owner/chairs` の累積距離だけを3秒の許容範囲内で短時間cacheする
- [ ] `/owner/sales` は遅延cacheせず、常にリクエスト時点の許容範囲を満たす
- [ ] MySQLを別CPU/ホストへ分離できる環境では、アプリ同居構成と比較する
- [ ] Axum processの複数化はSSE・cache・matcherの状態共有を解決してから比較する
- [ ] 複数webappでmatcherを動かす場合はleaderを1つに限定するか、rideとchairの条件付きclaimで二重割当を防ぐ
- [ ] PGOやCPU固有最適化は最終候補binaryだけで比較する

## 各変更の検証

### Rust

- [x] `cargo fmt -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test --all --all-targets`
- [x] `cargo build --release --locked`

### API・正当性

- [x] `./scripts/smoke-test.sh`
- [x] 公式prevalidation
- [ ] 通知の全遷移・順序・重複許容・取りこぼし
- [ ] chair statsが走行中は固定され、`COMPLETED` で当該評価を含むこと
- [ ] nearbyの空車・座標・3秒猶予
- [ ] ownerの距離・売上・0件行
- [ ] 並行ride作成と並行matching
- [ ] 決済retryとexactly-once相当の結果
- [ ] `rides.updated_at` と履歴 `completed_at` が完全一致すること
- [ ] 既存表へ列を追加しても列名なし初期ダンプをロードできること
- [ ] initialize直後とwebapp再起動後

### 性能

- [x] 同じCPU・メモリ・走行時間で変更前後を比較する
- [ ] 最低3回実行し、中央値とばらつきを残す
- [x] `pass`、スコア、全エラーコードを記録する
- [ ] 完了ride数を独立して記録する
- [ ] 空車移動距離×0.1、乗車中移動距離、完了ride数×5の各スコア寄与を記録する
- [ ] 全APIの30ms超過率とmatching / pickup / driveのtick遅延を記録する
- [ ] matcherは地域別pending数、available chair数、starvationした最古rideの待ち時間を記録する
- [ ] 通知はcache hit率、wake latency、recipientあたりSQL数、再接続時replay件数を記録する
- [ ] 座標queueはdepth、最古未flush時間、batch件数、drop / retry数、status反映遅延を記録する
- [ ] エンドポイント件数とp50 / p95 / p99を記録する
- [ ] SQL回数、累積時間、走査行数を記録する
- [ ] pool待ち、MySQL CPU、webapp CPU、block I/Oを記録する
- [ ] 改善しなければ変更を重ねずrevert候補として記録する

## 推奨する直近の実行順

1. 現在の近傍優先matcherを同一revisionで3回計測し、16,909点の再現性とスコア内訳を確認する
2. matcherへ地域間の距離上限を追加し、500 / 100 / 30msの実行間隔と組み合わせて比較する
3. JSON通知のpayload cacheとlong pollingを実装し、30ms pollingよりDB負荷と通知遅延が減るか確認する
4. 座標更新の非同期queueとbulk INSERTを単独実験し、3秒制約とstatus遷移を検証する
5. 決済へ `Idempotency-Key` を導入してGET照合をなくす
6. 外部決済HTTPをDB transactionの外へ出し、Clientを共有する
7. nearbyの未完了判定を `evaluation IS NULL` へ単純化し、座標だけの短時間cacheを比較する
8. app history、owner sales、ride作成のN+1を順に除去する
9. current-state別表で最新位置・status・statsをO(1)化する
10. JSON long pollingで不足する場合だけSSEへ移し、状態変更時の即時pushまで実装する
11. 貪欲matcherと最小費用二部マッチングを比較する
12. 最後にpool、MySQL、nginx、compiler設定をprofileに基づいて調整する

## 記録ルール

確定した結果は [`TUNING.md`](./TUNING.md) から参照できる個別記録へ移します。

| 項目 | 内容 |
|---|---|
| 条件 | 日時、commit、ホストCPU/メモリ、走行時間 |
| 症状 | `pass`、スコア、完了数、エラーコード |
| 証拠 | endpoint latency、SQL、実行計画、query数、資源使用量 |
| 仮説 | なぜその処理が律速なのか |
| 変更 | SQL、INDEX、Rust、設定の差分 |
| 正当性 | prevalidation、通知、決済、並行実行の結果 |
| 性能 | 同条件での変更前後と3回のばらつき |
| 判断 | 採用、保留、revertとその理由 |
