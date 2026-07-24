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
- [ ] statusだけを読むqueryにcovering INDEXが有効か比較する
- [ ] ULIDやtoken列を `CHAR/VARCHAR ... CHARACTER SET ascii` またはbinary表現に変えた場合のINDEXサイズを比較する
- [ ] `chairs.model` の `TEXT` を上限付き `VARCHAR` へ変える
- [ ] buffer pool hit率、temporary table、sort、redo量を採取する
- [ ] datasetに合わせて `innodb_buffer_pool_size` を調整する
- [ ] `SELECT @@log_bin` でbinary logの実状態を確認する
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
