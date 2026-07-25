# ISUCON14 Rust チューニング TODO

公式 Rust 実装へ最初の INDEX と通知 polling の改善を加えた現在の作業ツリーを、正当性を維持したまま最大スコアまで段階的に改善するためのバックログです。

最終ソース監査日: 2026-07-25

## 最適化の目的と制約

最終目的は単純なHTTPリクエスト数ではなく、60秒ベンチを `pass=true` で完走させ、完了ライド数とスコアを増やすことです。

- ベンチマーカーは30msを1tickとして進むため、全エンドポイントの理想値を30ms以内とする
- スコアは「空車で乗車地点へ移動した距離×0.1 + 乗車中の移動距離 + 完了ライド数×5」で評価する
- 空車移動より乗車中の移動の価値が10倍なので、単なる処理件数だけでなく乗車地点に近い椅子の割当を優先する
- 通知は全状態遷移を順番どおり返す。厳密なat-least-onceはresponse ACKがない現行APIでは
  未達であり、`*_sent_at` commit後からclient受信前の切断を故障注入して残余riskを追う
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
- [x] `reqwest::Client` を `AppState` で共有し、当時の決済POST・確認GET・retryで再利用
  - 60秒3走: 76,761 / 88,638 / 80,354点
  - 観測範囲76,761–88,638点、推定代表値の中央値80,354点
  - 直前中央値60,102点から+20,252点、約+33.7%
  - 全run `pass=true`、error map空
  - 詳細: [`tuning/14-payment-client-reuse.md`](./tuning/14-payment-client-reuse.md)
  - 後続Benchmark 25で確認GETを削除し、共有clientは冪等POSTとretryで継続利用
- [x] `coupons(used_by)` を追加し、rideに適用済みのcoupon検索をB-tree lookup化
  - 60秒3走: 88,805 / 93,606 / 100,606点
  - 観測範囲88,805–100,606点、推定代表値の中央値93,606点
  - 直前中央値80,354点から+13,252点、約+16.5%
  - 対象SQLの平均0.928ms→0.060ms、全run `pass=true`・error map空
  - 詳細: [`tuning/15-coupon-used-by-index.md`](./tuning/15-coupon-used-by-index.md)
- [x] nearbyの未完了判定を `rides.evaluation IS NULL` へ単純化し、全status writerをride row lockで直列化
  - queryだけの60秒3走は96,546 / 108,073 / 100,310点だったが、完了後に遅延した `ENROUTE` / `ARRIVED` を追記できる競合反例があり不採用
  - 全座標をlockする安全版は3走中央値90,523点へ悪化したため、pickup / destination候補だけlock後に再読
  - 最終版のエラー0の60秒3走: 98,628 / 98,311 / 98,580点
  - 観測範囲98,311–98,628点、推定代表値の中央値98,580点
  - 直前採用版中央値93,606点から+4,974点、約+5.3%
  - 初期状態、負荷中3時点、最終run終了時で旧判定との差0件
  - 詳細: [`tuning/16-nearby-evaluation-filter.md`](./tuning/16-nearby-evaluation-filter.md)
- [x] 座標更新の通常経路から最新status相関subqueryを除去
  - 遷移候補だけlockする直前版の中央値92,484点から98,580点へ+6,096点、約+6.6%
  - current ride query平均0.288ms→0.112ms、遷移候補は全座標の約4.5%
  - `PICKUP` / `ARRIVED` はride row lock取得後、statusをlocking readし、期待値の場合だけ追加
  - pickupとdestinationが同一のrideも `PICKUP -> CARRYING -> ARRIVED` へ進むことを統合確認
  - 2本の並行座標更新を同じride lockで待たせても `PICKUP` は1行だけ
  - 詳細: [`tuning/17-coordinate-transition-query.md`](./tuning/17-coordinate-transition-query.md)
- [x] nearby用の最新座標をcurrent-state表とprocess内cacheへ分離
  - 変更前の `LATERAL` は候補42台ごとに平均166履歴を読みsortし、単発約26.4ms
  - active状態と割当可否はcacheせず、DBから毎回最新値を取得
  - 履歴とcurrent rowを同じtransactionで更新し、cacheをcommit後と2秒間隔で同期
  - canonical orderは全経路で `(created_at DESC, location_id DESC)`
  - 評価response bodyまで保持するtrackerを含む当時のエラー0の3走: 96,888 / 96,926 / 98,483点
  - 観測範囲96,888–98,483点、推定代表値の中央値96,926点
  - 直前採用版中央値98,580点から-1,654点、約-1.7%。write amplificationを次のP0へ残す
  - 最終run例のnearby SQL平均8.079ms、current UPDATE平均0.744ms
  - DB直接更新の故障注入は最終再実行で1.693秒、同時刻tieは1.651秒でcacheへ収束
  - busy-chair `CODE=30` のWARN本文を特定し、評価response bodyのpoll / dropまでprocess trackerで除外
  - 後続の認証cacheで処理量が増えると再発したため、body lifecycleだけではclient受信境界を閉じないことをBenchmark 23で再診断
  - 既存volumeの欠損rowと古いrowを起動時の冪等backfillで修復する故障注入も通過
  - `rides.updated_at` 起点の500ms / 1秒cooldownは評価処理時間に正しさが依存するため不採用
  - initializeは全APIと再同期を共通maintenance gateで排他
  - 詳細: [`tuning/18-latest-location-cache.md`](./tuning/18-latest-location-cache.md)
- [x] status通知と最新状態の順序を `created_at` ではなくENUMの状態遷移順へ統一する
  - 診断runで `CARRYING` 配信後に古い `PICKUP` を返すCODE=11を1件再現
  - app / chair両通知へ時刻逆転データを投入するHTTP回帰テストを追加
  - 未送信通知INDEXを `(ride_id, *_sent_at, status)` へ変更し、`Using filesort` を除去
  - 60秒3走89,539 / 98,338 / 99,895点、中央値98,338点、全run `pass=true`、CODE=11は0件
  - 詳細: [`tuning/19-status-semantic-order.md`](./tuning/19-status-semantic-order.md)
- [x] chair statsを1 chair 1 rowのcurrent-state表へ事前集計する
  - 旧履歴集計46,876回・22.299秒・平均0.476msを、主キーread
    39,326回・2.477秒・平均0.063msへ短縮
  - 評価確定と同じtransactionの差分writeは868回・0.119秒・平均0.137ms
  - 初期500 chair、公式prevalidation、60秒終了時の旧履歴集計との差はすべて0件
  - 欠損・誤値・余分なrowの再起動repairと、決済失敗rollback・再送非加算を回帰確認
  - 最終3走98,386 / 98,452 / 99,944点、中央値98,452点、全run `pass=true`・エラー0
  - 直前通常3走中央値101,984点から-3,532点のため、スコア寄与は未確定
  - 検証済みの再起動は単一webappのstop-then-start
  - [ ] 複数instanceやrolling restartを使う前に、起動repairと評価のlock順を
    DB advisory lockまたはdeadlock限定retryで安全にする
  - 詳細: [`tuning/20-chair-stats-current-state.md`](./tuning/20-chair-stats-current-state.md)
- [x] 通知の未送信status検索と最新status fallbackをCTEで1 SQLへまとめて比較する
  - 60秒runは`pass=true`、94,573点、error map空
  - SQL呼出しは減ったが、app / chair CTE版の累積が53.756秒、平均約0.56msへ増加
  - 変更前の関連query全体は約32秒だったため、実装は元へ戻して不採用
  - 単発`EXPLAIN ANALYZE`と高並行時の累積costが逆転した理由を記録
  - 詳細: [`tuning/21-notification-status-query.md`](./tuning/21-notification-status-query.md)
- [x] user / owner / chairの認証主体をprocess内cacheへ置く
  - 60秒3走109,454 / 102,887 / 104,612点、中央値104,612点
  - 直前中央値98,452点から+6,160点、約+6.3%
  - 認証SQLは139,690回・9.761秒から657回・0.069秒へ削減
  - 起動・initialize全置換、動的userの1回fallback、stale token削除をHTTP回帰確認
  - 全run`pass=true`だが`CODE=30`が6 / 15 / 20件あり、次のP0で原因を再計測
  - 詳細: [`tuning/22-authentication-cache.md`](./tuning/22-authentication-cache.md)
- [x] 認証cache後に増えたnearbyの`CODE=30`をresponse配送境界まで再診断する
  - 診断instrumentationでbody guardのみの27件すべてが、評価HTTPレスポンス待ちのphaseにあることを確認
  - 同一rideのserver body dropからclient受信完了まで、約55–677msの差を実測
  - nearby開始時snapshot、単調なcompletion revision、body drop起点の1秒delivery leaseを実装
  - initialize前後はgenerationで分離し、期限切れcompletionはlive snapshotの最小revisionを見て回収
  - generation/pruneを含む最終60秒3走105,002 / 103,046 / 96,542点、観測範囲96,542–105,002点
  - 推定代表値の中央値103,046点、全run`pass=true`・error map空、`CODE=30`は3走合計0件
  - generation/prune前の候補runで出た`CODE=17` 1件は別経路として再現情報の採取を継続
  - 詳細: [`tuning/23-code30-response-delivery.md`](./tuning/23-code30-response-delivery.md)
- [x] owner salesの`CODE=24`候補を評価完了時刻の逆転として再現し、決済後に完了を確定する
  - 修正前はpending rideの時刻が既知完了rideより約151ms古く、同じ`until`で売上が436,200円から436,900円へ過大化
  - 決済成功後に既存のevaluation / `COMPLETED` / chair stats writeを実行し、DBとresponseへ同じ`completed_at`を使用
  - 決済前の冗長なride再SELECTを削除し、完了時刻だけの追加UPDATEは不採用
  - 最終60秒3走94,173 / 104,048 / 93,408点、観測範囲93,408–104,048点
  - 推定代表値の中央値94,173点、全run`pass=true`・error map空、`CODE=24`は3走合計0件
  - 詳細: [`tuning/24-owner-sales-completion-boundary.md`](./tuning/24-owner-sales-completion-boundary.md)
- [x] 決済POSTをride IDで冪等化し、owner売上の評価response重複境界をride単位で狭める
  - すべてのretryで同じ `Idempotency-Key`、token、amountを送る
  - エラー時の`GET /payments`とuserのride全件SELECTを削除
  - owner requestと重なった評価rideだけをrevision付きtrackerで除外し、既知の完了rideは除外しない
  - 最終60秒3走95,596 / 101,037 / 115,968点、観測範囲95,596–115,968点
  - 推定代表値の中央値101,037点、全run`pass=true`・error map空
  - 詳細: [`tuning/25-payment-idempotency.md`](./tuning/25-payment-idempotency.md)
- [x] 状態不変のapp / chair通知payloadをrevision付きprocess cacheから返す
  - writer commit後にuser / chair revisionを進め、読み取り中のstale payload再挿入を防止
  - app payloadが参照するchair statsにもrevisionを持ち、別userの評価によるcross-keyの
    stale payloadをlookupとinsertの両方で拒否
  - 未送信status中は30ms、全status送信後またはrideなしの定常cacheだけ100ms
  - 30ms固定cacheは3走中央値88,757点へ悪化し、closed-loop request増加として不採用
  - dependency追加前の3走114,996 / 103,957 / 112,156点は途中結果として保持
  - cross-user chair stats修正後の最終60秒3走111,798 / 103,727 / 109,443点、
    観測範囲103,727–111,798点
  - 推定代表値の中央値109,443点、Benchmark 25比+8.3%、全run`pass=true`・error map空
  - 診断runでapp / chair通知の平均を113 / 130msから37 / 51msへ短縮
  - 詳細: [`tuning/26-notification-payload-cache.md`](./tuning/26-notification-payload-cache.md)
- [x] coordinateを1/64 samplingでphase分解し、current UPDATEのrow lock仮説を検証する
  - 1,185 sampleのcurrent writeは平均1.633ms、p95 4.185ms
  - `pool.begin()` は平均32.452ms、p95 93.651ms、handler内total p95 105.296ms
  - current-state write全75,834回は平均0.812ms、row lock待ちは2,914回・平均約16.6ms
  - row lockは存在するが支配的phaseではないため、current row queue化を直近の実装対象から外す
  - `pool.begin()` はacquire + SQL `BEGIN` の合算なので、次は2区間を分離する
  - 詳細: [`tuning/27-coordinate-phase-diagnostics.md`](./tuning/27-coordinate-phase-diagnostics.md)
- [x] 再発した`CODE=17`のHTTP経路、MySQL error、deadlock履歴を同時採取する
  - `POST /api/app/users` のMySQL 1062、`users.username='Kulas4628'` の重複
  - 同名rowは同じrunで約16秒前に作成。InnoDB deadlock履歴はなく、過去のcoupon競合とは別原因
  - nginxにrequest IDがないため、UTC時刻 + endpoint + HTTP status + DB error + usernameで相関
  - OpenAPIがusernameを一意と定義するためUNIQUE INDEXは維持し、衝突時の限定retryを別施策で検証する
  - 詳細: [`tuning/27-coordinate-phase-diagnostics.md`](./tuning/27-coordinate-phase-diagnostics.md)
- [x] `users.username` のMySQL 1062だけを内部usernameで1回再試行する
  - 修正前は同名の2回目がHTTP 500、修正後は別userとして2回ともHTTP 201
  - UNIQUE制約を維持し、通常経路にはSELECTも追加しない
  - ID、access token、invitation codeなど別の一意制約違反は再試行しない
  - 60秒3走103,738 / 107,508 / 104,263点、観測範囲103,738–107,508点
  - 推定代表値の中央値104,263点、Benchmark 26比-4.7%のため高速化とは扱わない
  - 全run `pass=true`、`CODE=17`は0件
  - `CODE=26`は0 / 136 / 142件で、後半2走はerror予算の68% / 71%を消費
  - 詳細: [`tuning/28-username-collision-retry.md`](./tuning/28-username-collision-retry.md)
- [x] 招待登録のcoupon gap deadlockとreward code衝突を分離して修正する
  - 追加診断runの `CODE=17` は `POST /api/app/users` のMySQL 1213だった
  - `SHOW ENGINE INNODB STATUS` で、異なる `INV_...` codeのtransaction同士が
    `idx_coupons_code` の同じgapを保持し、互いのinsert intentionを待つcycleを確認
  - 招待者の `users.invitation_code` UNIQUE行を `FOR UPDATE` し、同一codeの上限判定だけを直列化
  - coupon全rowの `SELECT *` を `COUNT(*)` へ、招待者全列をID 1列へ縮小
  - 最初の修正版で `NOW(3)` が同じミリ秒になったreward couponの1062を再現し、
    一意部分を新規user IDへ変更
  - 異なる24 codeの同時登録は全件201。同一codeの4並行登録は201が3件、400が1件
  - 回帰テスト区間の `ER_DUP_ENTRY` と `ER_LOCK_DEADLOCK` は増分0
  - 60秒3走99,775 / 105,304 / 102,569点、観測範囲99,775–105,304点
  - 推定代表値の中央値102,569点、Benchmark 28比-1.6%のため高速化とは扱わない
  - 全run `pass=true`、error map空、終了後のMySQL 1062 / 1213は0件
  - 詳細: [`tuning/29-invitation-concurrency.md`](./tuning/29-invitation-concurrency.md)
- [x] coordinateの `pool.begin()` をconnection取得待ちとSQL `BEGIN`へ分離する
  - 診断runは `pass=true`、124,064点、error map空。診断n=1なのでscoreは未推定
  - 成功1,173 sampleでpool acquireは平均43.657ms、p95 113.156ms
  - SQL `BEGIN` は平均0.611ms、p95 2.327ms
  - 916 sample、約78.1%で取得直前がpool size 50 / idle 0 / in use 50
  - size 50 / idle 0群のacquire phaseは平均54.762ms・p95 117.398ms、
    idleあり群は平均3.968ms・p95 16.138ms
  - current-state write p95は5.007msで、row write支配仮説を棄却
  - 同runの評価APIは1,795回、平均403ms、p95 769ms。外部決済をtransaction内で待つ
    connection保持時間を次にphase分解する
  - pool上限は変更せず、長いtransactionを先に短縮する
  - 詳細: [`tuning/30-coordinate-pool-acquisition.md`](./tuning/30-coordinate-pool-acquisition.md)
- [x] 評価APIをpool、DB準備、決済、完了write、COMMITへ分解する
  - 診断runは `pass=true`、114,109点、error map空。診断n=1なのでscoreは未推定
  - 成功203 sampleでconnection所有は平均319.754ms、p95 695.556ms
  - 決済は平均302.507msで、connection所有平均の約94.6%
  - 内訳は決済HTTP平均100.785ms、retry sleep平均201.719ms
  - 203 sampleで608 attempts、途中5xx 405回、すべて最終204
  - 最大active評価38、同じrideの並行評価sampleは0
  - 172 sample、約84.7%でacquire直前がpool size 50 / idle 0
  - この群のacquire平均58.172msに対し、idleあり群は平均4.379ms
  - pool上限は変更せず、決済中のconnectionとride row lockを先に解放する
  - 詳細: [`tuning/31-evaluation-phase-diagnostics.md`](./tuning/31-evaluation-phase-diagnostics.md)
- [x] 評価APIを準備transaction、transaction外決済、完了transactionへ分割する
  - 診断runは `pass=true`、118,204点、error map空。診断n=1なのでscoreは未推定
  - connection所有平均319.754ms→19.241ms（-94.0%）、p95
    695.556ms→36.764ms（-94.7%）
  - 初回pool acquire平均49.957ms→27.773ms、p95 123.528ms→78.674ms
  - 決済平均は302.507ms→308.947msでほぼ同じ。DB資源の保持だけを短縮できた
  - 遅延決済中のride row lock 0件を確認
  - 2 requestがともに準備transactionを抜けて決済barrierへ到達してから204を返し、
    200 / 400各1件、`COMPLETED`とchair statsの加算1回を確認
  - 通常3走99,689 / 106,035 / 99,633点、推定代表値の中央値99,689点
  - 全run `pass=true`・error map空。Benchmark 29中央値比-2.8%で得点寄与は未確定
  - 初回sampleの66.5%、完了sampleの66.0%はpool size 50 / idle 0だったため、
    次は上限50 / 75 / 100を比較する
  - 詳細: [`tuning/32-evaluation-transaction-split.md`](./tuning/32-evaluation-transaction-split.md)
- [x] 評価分割後のSQLx pool上限50 / 75 / 100を比較する
  - 同じhot-path実装による通常3走中央値は107,234 / 105,867 / 103,720点
  - 全9 run `pass=true`・error map空。75は50比-1.3%、100は-3.3%で既定50を維持
  - 診断上の初回acquire平均は32.447 / 24.173 / 20.848msと単調に短縮
  - 一方、connection所有平均は18.637 / 26.527 / 30.410ms、InnoDBの1 wait平均は
    18 / 23 / 26msと増え、上限追加とDB内競合悪化の兆候が整合する
  - `ISUCON_DB_MAX_CONNECTIONS`を追加し、正の整数だけを許可。未指定時は50
  - CPU / memory / diskは4 CPU / 4 GiB / 100 GiBから変更していない
  - 詳細: [`tuning/33-sqlx-pool-capacity.md`](./tuning/33-sqlx-pool-capacity.md)
- [x] app / chair通知cache missをpool取得、SQL、connection所有へphase分解する
  - 診断runは`pass=true`、131,491点、error map空。instrumentation付き1走なので未推定
  - cache hitはapp 80.7%、chair 75.9%。cache hitのp95はいずれも1µs
  - cache missでは初回acquire平均40.051 / 41.001msに加え、transaction acquire平均
    37.788 / 41.512msを同じrequestで待っていた
  - rideありの同じ母数では2区間のconnection所有合計が平均10.540 / 10.021msで、
    SQLより再acquireが支配的
  - rideありでは最初の`PoolConnection`をtransactionへ引き継ぐ施策を次に比較する
  - 連続所有で他endpointへ待ちを移す可能性もあるため、全endpoint p95と通常scoreで判定する
  - 詳細: [`tuning/34-notification-phase-diagnostics.md`](./tuning/34-notification-phase-diagnostics.md)
- [x] 通知の存在確認connectionをtransactionへ引き継ぐ候補を診断する
  - rideありの成功sampleでは2回目のpool acquireをapp 153件、chair 126件すべてで削除
  - 診断runは52,564点、`pass=false`、`CODE=26` 60件、`CODE=29` 142件で
    エラー上限200件へ達したため不採用
  - 失敗後DBで、`updated_at`最大rideは送信済みなのに別rideへ未送信statusが残る
    hidden pending状態を25 chairで確認
  - connection再利用のソースはBenchmark 34へ戻し、次はride選択だけを独立修正する
  - 詳細: [`tuning/35-notification-connection-reuse.md`](./tuning/35-notification-connection-reuse.md)
- [x] chair通知のride選択を`updated_at`最大から配送状態機械へ変更する
  - hidden pending fixtureを固定し、期待ride / user / statusとcursor更新をHTTPで確認
  - 単純な未送信優先はdelivery gapで別rideへ切り替わり、診断runで`CODE=12` 4件を
    起こしたため不採用
  - `MATCHING`送信済み・`COMPLETED`未送信のcurrent ride、新規`MATCHING`未送信、
    完了履歴の順に選ぶ
  - `idx_rides_chair_created_at`と`idx_ride_statuses_ride_status`の利用を確認。
    2 ride fixtureの`EXPLAIN ANALYZE`は0.182msの点観測
  - レビュー前候補の診断runは113,046点、通常3走91,603 / 94,301 / 112,819点だが、
    `COMPLETED`後の終端反例を含むため採用値には使わない
  - 終端反例を修正した最終3走は86,532点`pass=true`、43,980 / 44,825点`pass=false`
  - 最終3走の`CODE=12/29`は0件だが、`CODE=32`が2走で発生したため推定代表値は出さない
  - hidden pendingとride取り違えを防ぐ変更は保持し、全体の正当性gateは未通過として
    `CODE=32`を次の最優先へ移す
  - 詳細: [`tuning/36-chair-notification-delivery-state.md`](./tuning/36-chair-notification-delivery-state.md)
- [ ] `CODE=32` の長時間MATCHINGを再現し、rideとmatcher候補を同じtickで追う
  - Benchmark 36最終3走のrun 2 / 3で各1件発生し、両方`pass=false`
  - pending ride ID、作成時刻、地域、空きchair数、matcherが選んだbatchとUPDATE件数を採取する
  - `internal_get_matching`の64件batch、地域間距離、500ms pollのどこでstarvationしたか分ける
  - critical errorなので、通常3走がすべて`pass=true`になるまで次の性能施策を重ねない
- [ ] `CODE=8` の未依頼userへ状態通知される経路を再現する
  - Benchmark 36最終run 2だけ24件、run 1 / 3は0件
  - app通知のride ID / user ID、ベンチ側current request、DBのuser_idとapp cursorを相関する
  - chair側の配送状態修正とはendpointが異なるため、同じ根本原因と決めつけず分離する
- [ ] `CODE=26` のowner累積距離が座標responseの受信境界より先へ進む競合を検証する
  - 期待値より実値が4–40程度大きく、直近1回の移動距離に近い例を確認
  - ベンチマーカーのcoordinate POST、world更新、owner検証の順序を同じchairで追う
  - server側はowner request開始時の座標watermarkを固定できるか検討する
  - username再試行が実行されなかったrun 3でも142件出たため、Benchmark 28の分岐とは分離する
  - Benchmark 29前の診断3走と通常3走では再現しなかった。解決とは扱わず、
    再発時に座標request / responseとowner集計を同じchair IDで採取する
  - Benchmark 30診断runもerror map空。再現待ちだけで他のP0計測を止めない
  - Benchmark 36レビュー前は診断153件、通常130 / 136 / 151件。
    最終実装でも144 / 85 / 80件と毎回再現したため、`CODE=32`の次に調べる
  - `got`が`want`を大きく上回る例もあり、直近1移動分という仮説に限定せず、
    location IDの採用範囲とowner requestのsnapshot境界から再確認する
- [ ] `CODE=27` のnearby椅子が最新の指定範囲から外れる競合を検証する
  - Benchmark 36レビュー前は診断4件、通常10 / 0 / 49件。最終実装は3 / 0 / 0件
  - DBの`chair_current_locations`、process cache revision、
    nearby応答を同じchair IDとrequest時刻で採取する
  - `CODE=26`のwatermarkを直した後に、共通の座標可視性問題か独立したcache staleかを判定する
- [x] nearbyの集合SQL、chair statsの集約SQL、batch matcherを実装
- [x] 上記3変更を別々のBenchmarkとして正当性・性能検証する

## ソース監査で見つかった主要ボトルネック

| 優先度 | 対象 | 現在の処理 | 主な問題 |
|---|---|---|---|
| P0 | `internal_get_matching` | 64件batch + 近傍優先、外部pollは500ms | 空き定義の集約、500msの最小待ち |
| P0 | `app_get_nearby_chairs` | 最新座標はcurrent-state表 + process cache、active / 割当可否はDB、評価はsnapshot + revision + delivery leaseで除外 | 最終run例8.079ms。ride antijoinとtracker確認の内訳を測る |
| P0 | 通知2経路 | recipient + chair stats dependency revision付きpayload cache。chairは配送状態機械でcurrent rideを維持。未送信statusは30ms、定常cacheは100ms | connection再利用、response ACKなしの配送loss、long pollingを順に検証 |
| P0 | `get_chair_stats` | 評価時差分更新 + 主キーreadへ変更 | SQL累積は約89%減、スコア中央値は約3.5%低下したため次の通知施策と合わせて再評価 |
| P0 | `app_post_ride_evaluation` | 準備transaction、transaction外の冪等決済、ride再lock付き完了transactionへ分割済み | connection所有平均は94.0%短縮。完了時の追加acquireとprocess crash後の自動回収を検討する |
| P0 | `chair_post_coordinate` | 履歴INSERT + current UPDATE、遷移候補だけride lock + 最新status再読 | pool 50維持。上限追加で通常中央値は改善しなかったため、connection取得後のDB滞在をquery別に減らす |
| P1 | `app_get_rides` | rideごとにstatus、coupon、chair、ownerを取得 | 履歴増加に比例するN+1 |
| P1 | `owner_get_sales` | ownerのchairごとに完了rideを取得し、評価responseと重なったride IDを除外 | N+1、read transactionが暗黙ROLLBACK。複数processではtracker共有が必要 |
| P1 | `app_post_rides` | userの全rideとride別最新statusを取得 | ライド作成ごとに履歴全体を再走査 |
| P2 | `app_post_users` | 招待者UNIQUE行を直列化地点にし、couponは `COUNT(*)`、rewardは新規user IDで一意化 | 現在の上限3では十分。上限や同一code集中が増えた場合だけcounterの条件付きUPDATEを比較 |
| P1 | 認証middleware | 初期tokenはprocess cache、動的主体は最初のmissだけDB検索 | DB外のtoken失効と複数processのcache invalidationは未対応 |
| P1 | `payment_gateway` | process共有client + ride IDの冪等POST。エラー時の履歴GETは削除済み | TCP connect回数、retry status別回数、connection再利用率の直接計測は未実施 |
| P2 | nginx / Rustログ | stock設定のまま全リクエストを処理 | 高頻度経路のログI/Oとproxy overheadが未計測 |
| P2 | MySQL / sqlx pool | stock MySQL、SQLx pool上限50。50 / 75 / 100の通常各3走で維持を決定 | 上限追加でrow-lock待ちとDB内滞在が増えたため、query・lock保持を減らす |

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
- [x] 評価APIは認証ユーザー本人のrideだけを更新する
  - locking readを `id + user_id` 条件にし、他ユーザーrideの存在はHTTP 404として扱う
  - 別ユーザーcookieでevaluation、`COMPLETED`、chair statsが不変であることをHTTP確認
  - 詳細: [`tuning/81-evaluation-authorization.md`](./tuning/81-evaluation-authorization.md)
- 3秒cacheを許せるowner情報は `/owner/chairs` の累積距離だけ
  - `/owner/sales` はリクエスト直前snapshotを下限に検証されるため、遅延cacheしない

## スコア構造から導いた追加仮説

- [x] INDEX、nearbyのN+1解消、owner椅子一覧の事前絞り込みを実装済みであることを確認する
- [x] `users(access_token)` と `users(invitation_code)` は既存の `UNIQUE` INDEXで検索できることを確認する
- [x] nearbyの「割当済み判定には `evaluation IS NULL` を利用できる」という前提を、同一transactionだけでなく全status writerのride row lockとlock後再読で保証する
- [x] `coupons.used_by` 全走査をprepared statement統計で特定し、単独INDEX追加を3走比較する
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
- [x] エンドポイント別のp50 / p95 / p99を診断overlayで採取する
  - 変更前app通知96 / 274 / 344ms、chair通知119 / 289 / 352ms
  - Benchmark 26後app通知2 / 166 / 257ms、chair通知5 / 181 / 269ms
  - 通常スコアrunとは分離し、詳細は [`tuning/26-notification-payload-cache.md`](./tuning/26-notification-payload-cache.md)
- [ ] 各エンドポイントの30ms超過率と、1tick中に完了できなかった回数を記録する
- [x] sqlx poolの `size` / `idle` / `in_use` と取得待ち時間をcoordinate sampleで採取する
  - 1/64 samplingの1,173件中916件、約78.1%でsize 50 / idle 0 / in use 50
  - acquire平均43.657ms・p95 113.156ms、SQL BEGIN平均0.611ms・p95 2.327ms
  - 1秒ごとの全pool時系列は未実装。まず接続保持元のhandler phaseを測る
- [x] MySQLのprepared statement統計をSQL本文別に回数、累積、平均、最大、走査行数で並べる
  - SQLxの個別queryはdigest表で `statement/com/Execute` へ集約されるため、`prepared_statements_instances` をSQL本文でgroup化
  - `coupons.used_by` 変更前: 60,993回、56.615秒、平均0.928ms、61,616,755行走査
- [x] prepared statement計測で終了時の `Connections` と
  `Performance_schema_prepared_statements_lost` も保存する
  - 終了前に閉じたconnectionのinstanceは集計から消えるため、現在値だけでは全期間を保証しない
  - Benchmark 26診断run終了時: `Connections=88`、`Performance_schema_prepared_statements_lost=0`
- [x] `docker stats` でwebapp、MySQL、nginx、ベンチマーカーのCPU・メモリ・I/Oを同時に採取する
  - 診断run中の2 snapshotでMySQL 188.32–239.10%、webapp 70.65–89.16%
- [x] 採用したBenchmark 26の全エラー件数と、200件のエラー予算に対する消費率を記録する
  - 最終3走は各0件、消費率0%
- [x] 座標・status系APIのp99を評価遅延予算と比較する
  - matching評価: 100tick = 3秒未満
  - pickupの余分な遅延: 15tick = 450ms未満
  - driveの余分な遅延: 5tick = 150ms未満
  - Benchmark 26診断run: coordinate p99 234ms、chair status p99 266ms
  - 150msを超えるため、通知cache後もcoordinate / statusをP0として継続
- [x] 結果を [`tuning/02-notification-transactions.md`](./tuning/02-notification-transactions.md) と [`TUNING.md`](./TUNING.md) に反映する
- [x] `tuning/02-notification-transactions.md` の説明を現在の実装へ修正する
  - 全面autocommit化ではなく、rideなしの分岐だけをtransaction外へ出している
  - `chair_get_notification` の `FOR SHARE` は残っている

## Phase 0.5: 計測ツールを選定・検証する

ツールの導入自体は高速化ではありません。ボトルネックの層を特定し、変更前後の差を同じ条件で説明できたツールだけを残します。

### 運用ルール

- [x] HTTP timingの診断runと、access logを追加しない最終スコアrunを分離する
- [ ] 各ツールのversion、実行コマンド、sampling間隔、開始・終了時刻を記録する
- [ ] macOSホスト、Colima Linux VM、Docker containerのどこで採取した値かを必ず明記する
- [x] nginx timing logを `ISUCON_DIAGNOSTIC=1` のCompose overlayへ分離する
- [ ] ツールあり／なしで同一revisionを各3回走らせ、スコア中央値とCPU使用率の差から計測overheadを確認する
- [x] nginx診断logへ認証token、Cookie、request body、決済情報を残さない
- [ ] artifactはrun IDでまとめ、HTTP、SQL、CPU、I/Oを同じ時刻範囲で照合できるようにする

### 現在の利用可否

| 優先度 | ツール | 現在の状態 | 主な用途 |
|---|---|---|---|
| P0 | `alp 1.0.21` | macOSホストへ導入済み。診断overlayのJSON timing logと集計scriptを追加 | endpoint別件数、p50 / p95 / p99、総処理時間 |
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

- [x] nginx診断用JSON `log_format` を追加する
  - method、URI、status、`request_time`、`upstream_response_time`
  - `upstream_connect_time`、request / response bytes、connection ID、`connection_requests`
- [x] path parameterのride IDを `alp --matching-groups` でまとめ、同一endpointを別行へ分散させない
- [x] `alp` でcount、sum、avg、p50、p95、p99、max、5xx / 499件数をendpoint別に出す
- [x] `DIAGNOSTIC_SINCE` でrun開始時刻を固定し、同じnginx containerの過去logを混ぜない
- [x] `compose logs` 失敗と診断JSON 0件を集計成功として扱わない
- [ ] `request_time - upstream_response_time` からnginx・socket・client側の待ちを推定する
- [ ] `alp --dump` と `alp diff` で変更前後を機械比較できる形にする
- [ ] access logをtmpfsまたは診断用volumeへ出す場合と無効化した場合を比較し、log I/Oのscore overheadを測る
- [ ] localhostまたはDocker network内だけから参照できる`stub_status` endpointを診断構成へ追加する
- [ ] `stub_status`のactive、reading、writing、waiting、accepts、handled、requestsを1秒間隔で採取する
- [ ] `handled < accepts`、active connection上限、writingの継続増加をnginx側の飽和兆候として扱う
- [x] 採用条件: 30ms超過を作るendpointを特定し、診断runを最終スコアrunから分離する
  - 変更後もapp / chair通知p95は166 / 181ms、coordinate p99は234ms

### SQL: `performance_schema`、`sys` schema、`pt-query-digest`

- [x] benchmarkごとのMySQL再起動で統計をresetし、終了直後に回数、合計、平均、最大、rows examinedを保存する
  - prepared statementはdigest表では本文別にならないため、`prepared_statements_instances` も保存する
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
- [x] `CODE=26` 1件との因果を切り分けるため、同一revisionを3回以上走らせる
  - 変更前の通常3走101,984 / 102,498 / 98,444点と、Performance Schema履歴を
    有効にした100,732点ではCODE=26を再現しなかった
  - 終了時87,107座標で同時刻、ID順の時刻逆転、順序による距離差、
    current-state不整合はすべて0件
  - chair stats変更後の6走でもCODE=26は0件。推測修正は入れず再発時のID採取を残す
- [x] 未完了ride判定のstatus相関subqueryを `rides.evaluation IS NULL` へ置き換え、実行計画と結果を比較する
  - 旧新のride判定とnearby結果を負荷中3時点で比較し、差分0件
  - `EXPLAIN ANALYZE`: 28.2ms→10.1ms、status subquery 1,671 loopsを除去
  - queryだけの中央値100,310点は競合反例があるため不採用
  - 全status writerをride row lockへ合流させ、statusもcurrent/locking readする最終版はエラー0の3走中央値98,580点
  - 詳細: [`tuning/16-nearby-evaluation-filter.md`](./tuning/16-nearby-evaluation-filter.md)
- [x] 座標をcurrent-state表とprocess内cacheへ置き、`is_active` と割当可否は毎回合成する
  - `LATERAL` の単発約26.4msに対し、座標を外した候補queryは約4.79ms
- [x] 評価response bodyまで保持する当時のtracker版3走は中央値96,926点、最終run例のnearby SQL平均8.079ms
  - 起動、initialize、動的chair、座標更新、2秒再同期、process再起動を確認
  - 高負荷時にclient受信前の再掲載が再発したため、現在はBenchmark 23のdelivery lease版へ更新
  - 詳細: [`tuning/18-latest-location-cache.md`](./tuning/18-latest-location-cache.md)
- [ ] nearbyレスポンス全体の3秒cacheは割当済み椅子を返すため採用しない
- [x] `CODE=30` のWARN本文を保存し、評価commitとbenchmarker状態更新の競合と特定する
- [x] 評価後cooldownを500ms / 1秒で比較し、処理時間依存のため両方を不採用に戻す
- [x] 評価handler開始からresponse bodyのpoll / dropまでchairをprocess trackerへ登録し、RAII guardで全終了経路から解除する
- [x] 認証cache後の高い処理量でbody trackerだけを再診断し、client受信前に解除される反例を確認する
- [x] nearby開始snapshot、completion revision、body drop起点1秒leaseを組み合わせる
- [x] 最終60秒3走で`CODE=30`がすべて0件であることを確認する
  - generation/pruneを含む105,002 / 103,046 / 96,542点、中央値103,046点、全run error map空
  - 詳細: [`tuning/23-code30-response-delivery.md`](./tuning/23-code30-response-delivery.md)
- [x] `CODE=24` owner sales過大値候補を、評価commitとbenchmarker計上の境界として決定的に再現する
  - pending / knownの時刻逆転と、owner salesの+700円を同じfixtureで確認
  - 決済後にevaluationと完了時刻を同じUPDATEで保存し、同じ時刻をresponseへ返す
  - `./scripts/test-owner-sales-response-boundary.sh` でInnoDB行ロックを条件pollして赤/緑を確認
  - 詳細: [`tuning/24-owner-sales-completion-boundary.md`](./tuning/24-owner-sales-completion-boundary.md)
- [x] `CODE=17` が再発したrunで、登録HTTP経路、MySQL error、`SHOW ENGINE INNODB STATUS`を同時採取する
  - request IDは現行logにないため、UTC時刻、endpoint、status、DB error、重複usernameで相関
  - MySQL 1062のusername重複であり、deadlockではなかった
- [x] initializeのtable再作成中は、全API requestと定期再同期をmaintenance gateで待たせる
- [ ] latest-coordinate cacheの `RwLock` read / write待機時間と保持時間を計測する
- [x] 共有current-state表を追加し、複数processも2秒以内に収束できる経路を作る
- [x] current UPDATEのrow-lock待機時間とcoordinate transaction p95 / p99を計測する
  - current write平均1.633ms / p95 4.185ms / p99 23.184ms
  - coordinate handler内total平均40.089ms / p95 105.296ms / p99 138.956ms
  - `pool.begin()` p95 93.651msが最大であり、acquireとSQL `BEGIN`の分離を継続
- [ ] current row更新のcoalescing / queue化で3秒収束と履歴完全性を維持できるか比較する

### JSON通知の短期改善

- [x] chair通知のride選択を配送状態機械へ変更する
  - Benchmark 35失敗後DBでhidden pendingを25 chair確認
  - 未送信だけの優先はdelivery gapで別rideへ切り替わるため、
    `MATCHING`送信済み・`COMPLETED`未送信を最優先にする
  - 最終通常3走の`CODE=12/29`は0件だが、2走は`CODE=32`で`pass=false`
  - 合格実測`n=1`は86,532点、推定代表値なし
  - 詳細: [`tuning/36-chair-notification-delivery-state.md`](./tuning/36-chair-notification-delivery-state.md)
- [ ] ride存在確認とtransaction内の最新ride再取得を1回へまとめる
- [ ] 未送信statusがない場合は高価なpayloadを再構築せず `data: null` を返せるかprevalidationで確認する
  - status追加なしで `rides.chair_id` が変わり、同じ `MATCHING` のpayloadへchair情報を
    追加する必要があるため、未送信行の有無だけで短絡する案は不採用
  - status ID、chair割当、評価、statsを含むpayload versionまたは明示的なinvalidateが必要
- [ ] 未送信status、ride、user/chair、fareを1 SQLで取得する
- [x] 未送信statusと送信済み時の最新status fallbackだけをCTEで1 SQLへまとめて比較する
  - run 1は94,573点、app / chair SQL累積53.756秒で変更前の関連query約32秒より悪化
  - SQL数ではなく、候補集合・sort・rows examinedを含む累積costで不採用と判断
  - 詳細: [`tuning/21-notification-status-query.md`](./tuning/21-notification-status-query.md)
- [x] `get_chair_stats` を集約SQL1回へ置き換える
- [x] 初期データの全椅子で旧履歴集約とcurrent-state表の結果を比較する
  - `./scripts/test-chair-stats-consistency.sh` で500 chairの件数・評価合計の差0件
- [x] `ARRIVED` / `CARRYING` / `COMPLETED` の一部が欠けるrideを同じように除外する
  - 評価差分SQLも `CARRYING` の存在を要求し、backfillと完了条件を統一
  - `./scripts/test-chair-stats-transitions.sh` で欠損ride、決済rollback、再送をHTTP検証
  - `./scripts/test-chair-stats-consistency.sh` で故障注入後の再起動repairを検証
- [ ] 通知対象のclaimとsent時刻更新は、まず条件付きUPDATEで競合安全にする
- [ ] 同一recipientへの並行pollingが発生する構成になった場合だけ `FOR UPDATE SKIP LOCKED` を比較する
- [ ] transactionは未送信statusのclaimからsent更新までの最短区間だけにする
- [ ] app/chairそれぞれで、状態遷移の順序とat least onceを並行リクエストでも確認する
- [ ] `*_sent_at` commit後・response受信前の接続切断を故障注入し、現状は未受信statusを
  replayできないことを固定テストで示す
- [ ] 厳密なat-least-onceが必要なら、client ACKまたは次回pollで前回statusをACKしてから
  cursorを進めるprotocolを設計し、公式client互換性と追加DB負荷を比較する
- [x] wall-clockが逆転した履歴でもapp/chairが状態遷移順に配信することをHTTPで確認する
  - `MATCHING -> ENROUTE -> PICKUP -> CARRYING` を両endpointで確認
  - 実行: `./scripts/test-status-notification-order.sh`
- [x] `retry_after_ms` を30 / 50 / 100msで比較し、通知遅延とDB負荷の交点を測る
  - 全pollを50 / 100msにする案はCOMMIT回数を減らしたがスコアを改善せず不採用
  - Benchmark 26では未送信statusを30msに残し、状態不変cacheだけ100msを採用
  - 詳細は [`tuning/10-notification-retry-interval.md`](./tuning/10-notification-retry-interval.md)
- [x] 同じ利用者・椅子への直前payloadをcacheし、状態不変時のSQLとJSON再構築をなくす
- [x] cache keyをrecipient ID、valueをrevision / generation / JSON bytesとし、ride割当・status追加・評価確定で明示的にinvalidateする
- [x] app payloadにchair stats dependency revisionを持たせ、別userの評価後に同じchairを
  参照するcache hitとstale insertを拒否する
- [x] TTLに依存せず、cache missとプロセス再起動時はDB履歴から復元する
- [ ] JSON APIのまま最大60秒のlong pollingを実装し、状態変化時に `Notify` / channelで即時wakeする案をSSEより先に比較する
- [ ] version確認 → waiter登録 → version再確認の順にして、確認と待機開始の間に発生した通知を取りこぼさない
- [ ] long polling中はDB connectionとtransactionを保持せず、切断・timeout・再接続時もat least onceを維持する
- [x] cacheはpayload生成の高速化だけに使い、`app_sent_at` / `chair_sent_at` の配信cursorと混同しない
- [x] 未配信statusが複数ある場合はcacheせず、状態遷移順で1件ずつ送る
- [ ] JSON polling、JSON long polling、SSEを同一条件で比較し、protocol変更だけではなくDB query数と通知遅延が減った案を採用する

### 決済と評価

- [x] すべての決済POSTへride IDを `Idempotency-Key` として付与する
- [x] 同じkey・token・amountでretryし、エラー応答後も二重決済しない
  - RustのTCP unit testで500→204の2 requestが同じkeyのPOSTであることを確認
  - 400 / 422など回復しない4xxはretryせず、409 / 5xx / network errorだけをretry
  - 公式決済handlerとtestで、同じkey・同じpayloadが処理済み決済を再利用することを確認
- [x] 現行の `GET /payments` による照合を除去し、retry時のuserのride全件取得をなくす
- [x] `reqwest::Client` を `AppState` に1個保持し、POST/GETとretryでconnectionを再利用する
  - 3走中央値80,354点、直前中央値比約+33.7%、全runエラー0
- [ ] 診断runで決済先のTCP connect回数、connection再利用率、TIME_WAITを採取する
- [x] 評価handlerをpool取得、SQL、外部HTTP、retry sleepへ分け、p95 / p99を採取する
  - connection所有平均319.754ms・p95 695.556ms、決済平均302.507ms・p95 691.875ms
  - retry sleep平均201.719ms・p95 502.523ms、完了write平均6.417ms・p95 20.904ms
  - 詳細: [`tuning/31-evaluation-phase-diagnostics.md`](./tuning/31-evaluation-phase-diagnostics.md)
- [ ] 決済URLをinitialize時にメモリへ読み込み、評価ごとのsettings検索をなくす
- [x] ride、payment token、fareの読取りを短い準備transactionへまとめる
- [x] 外部決済HTTPとretry sleep中はDB transactionを保持しない
  - 8秒の遅延決済mockがrequestを受理した後、500msにわたり対象rideのrow lockが
    0件であることを確認
- [x] 評価と `COMPLETED` 追加を短いwrite transactionへ分離する
  - 完了時にrideを再lockし、所有user、未評価、`ARRIVED`、chair IDを再検証する
- [x] 決済成功後にだけ評価、chair stats、`COMPLETED` を同じwrite transactionで確定する
- [x] write transaction成功後にだけ評価APIの200を返す
- [x] 同じrideへの並行評価を完了transactionのrow lockと再検証で1件へ収束させる
  - 2並行requestを決済barrierで同期し、両方が準備確認を通過したことをassert
  - barrier解放後はHTTP 200 / 400各1件、評価、`COMPLETED`、chair statsは1回分
  - process内mutexと異なり、複数processでもMySQLの同じlock境界を利用できる
- [x] 「HTTP 500の後に決済成功」は同じhandler内のretryを同じ決済keyへ収束させる
  - RustのTCP unit testで500→204の2 requestが同じride ID keyであることを確認
- [ ] 「決済成功後にDB更新失敗」を故障注入し、次のhandlerが同じ決済keyで完了することを確認する
  - 現在の実装は毎回ride IDをkeyにするため再送可能だが、204後に完了transactionを
    意図的に失敗させるHTTP回帰は未実施
  - process crash後に未完了rideを自動再開する回収処理も未実装
- [x] 正常系、決済retry、重複評価、遅延決済のテストを追加する
  - network timeout時のhandler cancelとprocess crash後の自動回収は別途検証する
- [ ] `CODE=6`、`CODE=34`、`CODE=35` と評価APIのp99を比較する
- [x] 評価分割後のSQLx pool上限50 / 75 / 100を各3走で比較する
  - scoreだけでなく初回・完了acquire p95、MySQL `Threads_running`、
    `Max_used_connections`、row lock waitを同時に記録する
  - ホストのCPU / memoryは4 CPU / 4 GiBのまま変更しない
  - 同じhot-path実装の中央値107,234 / 105,867 / 103,720点から50を維持
  - 詳細: [`tuning/33-sqlx-pool-capacity.md`](./tuning/33-sqlx-pool-capacity.md)

### 座標更新

- [x] INSERT直後の `chair_locations` 再SELECTをなくす
- [x] `recorded_at` はINSERTへ渡した時刻をそのままレスポンスへ使う
- [x] rideと最新statusを別々に取得せず、現在rideだけを1 SQLで取得する
- [x] 座標がpickup/destinationと一致しない通常経路では、status INSERTなしで早くcommitする
- [x] pickup / destination候補だけride rowをlockし、lock後の最新statusを再読する
- [x] MySQL `REPEATABLE READ` の古いsnapshotを避けるため、遷移判定のstatusを `FOR UPDATE` でcurrent readする
- [x] `ENROUTE -> PICKUP` / `CARRYING -> ARRIVED` の期待する直前状態だけを条件付き遷移する
- [x] 通常座標のcurrent ride queryから最新status相関subqueryを除去する
- [x] 同じ `ENROUTE` の再送を追加INSERTなしの204にする
- [x] 通常の1座標更新あたりのSQL回数を4回から2回へ削減する
- [x] 遷移候補だけlockする版と、通常座標のstatus取得も除いた版を60秒3走で比較する
  - 直前版: 90,858 / 107,091 / 92,484点、中央値92,484点
  - 最終版: 98,628 / 98,311 / 98,580点、中央値98,580点
  - 詳細: [`tuning/17-coordinate-transition-query.md`](./tuning/17-coordinate-transition-query.md)
- [ ] 座標更新のtransaction保持時間、p95 / p99を比較する
- [ ] 座標更新をper-chair順序付きのbounded queueへ投入し、HTTP応答と永続化・status判定を分離する実験を行う
- [ ] 最新座標をメモリ上では即時更新し、`chair_locations` を30 / 50 / 100ms単位でbulk INSERTする
- [ ] queue内の中間座標は累積距離と乗車地点・目的地への到達判定に必要なので、最新1件へ無条件にcoalesceしない
- [ ] nearby向けlatest-coordinate cacheだけを上書きし、永続化対象の全座標列とは分離する
- [ ] queue full時のbackpressure、DB失敗時の再試行、initialize / shutdown時のflushを定義する
- [ ] HTTP 200をqueue投入時とDB commit後のどちらで返すか比較し、応答p99と再起動時の座標欠落リスクを記録する
- [ ] 非同期化後も座標は3秒以内、割当可否と到着statusは通知評価を落とさない時間内に反映する
- [ ] 同じ椅子の座標順序、累積距離、`PICKUP` / `ARRIVED` の一度だけの遷移を並行負荷で検証する
- [ ] 今回の手動再現（同一pickup / destination、ride lock後ろの並行座標2本）を自動integration testへ移す
- [ ] `PICKUP` 後にpickupへ滞在中、`ARRIVED` 後から評価前にdestinationへ滞在中のlocking read回数と待機時間を分離計測する

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

- [x] 評価responseがbenchmarkerの既知集合へ入る前に、古い`updated_at`のrideを`until`へ含める競合を除去する
- [ ] chairごとのride取得をowner単位の集約SQL1回へ置き換える
- [ ] `COMPLETED` 判定はstatus履歴JOINではなく、`evaluation IS NOT NULL` またはcurrent statusを使えるか検証する
- [ ] `(chair_id, updated_at)` を利用して `since` / `until` を先に絞る
- [ ] chair別、model別、totalを同じ入力集合から計算する
- [ ] read transactionと暗黙ROLLBACKをなくす
- [ ] 0売上の椅子とモデルもレスポンスへ残す

### 招待とcoupon

- [x] `SELECT * FROM coupons WHERE code = ?` を `COUNT(*)` 1値へ縮小する
  - 招待者のUNIQUE行を先に `FOR UPDATE` し、同じcodeの並行登録だけを直列化
  - 異なる24 codeと同一code 4件のbarrier付き回帰テストで1213増分0を確認
  - 詳細: [`tuning/29-invitation-concurrency.md`](./tuning/29-invitation-concurrency.md)
- [ ] 招待回数をcoupon全件から数えず、inviterのcounterを条件付きUPDATEする案を比較する
  - 上限3の現在は最大3 rowのCOUNTなので優先度をP2へ下げる
  - schema変更、列名なし初期dump、backfill、coupon INSERT失敗時rollbackを先に設計する
- [x] reward coupon codeのミリ秒時刻を新規user IDへ置き換える
  - row-lock版の並行テストで同一ミリ秒の主キー1062を再現し、変更後は1062増分0
- [ ] 先行追加した `coupons(code)` の利用回数とwrite costを再評価する
- [ ] 未使用coupon検索用 `(user_id, used_by, created_at)` を比較する
- [x] `WHERE used_by = ?` 用の非unique INDEXを比較する
  - 1,331行のtable scan・0.551msから1行のB-tree lookup・0.025msへ変化
  - 制約変更を混ぜないため `UNIQUE(used_by)` は別検証にする
- [x] `coupons(used_by)` を単独追加し、ride履歴の実行計画と60秒ベンチを比較する
  - 変更後run 3: 56,383回、3.386秒、平均0.060ms、37,398行走査
- [ ] `coupons(used_by)` のUPDATE回数・latency、INDEX byte数、buffer pool I/Oを変更前後で測る
  - 今回は総合ベンチでnet positiveを確認したが、write costの内訳は未計測
- [ ] coupon書き込みコストを含め、不要・重複INDEXを残さない

### 認証

- [x] middlewareのtoken検索回数と累積時間を利用者・椅子・owner別に計測する
  - 変更前合計139,690回・9.761秒、cache版run 3は657回・0.069秒
- [ ] queryはレスポンスに必要な列だけ取得する
- [x] tokenから認証主体を引くプロセス内cacheを導入する
- [x] initialize時に初期tokenをcacheへ再構築する
- [x] 動的登録された主体を最初のcache missで追加する
- [ ] activityやowner情報更新で古いsnapshotを使わないよう、可変属性は分離する
- [x] cache miss時だけDBへfallbackし、再起動後も正しく復元する
  - 初期userはDB queryなし、動的userは最初の1回だけDB、initialize後は旧tokenを401
  - initialize失敗時もcacheを空のままにして、前世代だけのtokenを認証しない
  - 複数processのinitialize invalidationは未対応なので、単一webappを検証範囲とする

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

- [x] chairの完了ride数と評価合計を保持し、完了時に差分更新する
- [x] 通知のchair statsをO(1)で返す
- [ ] ownerのchair別・model別売上を完了時に差分更新する案を比較する
- [ ] 更新失敗時に履歴から再構築できる手順を用意する
- [x] chair statsは評価・`COMPLETED` と同時commitし、通知遷移点で厳密に一致させる
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
  - 100ms: 54,172 / 53,715点、実測n=2の記述上の中央値53,943.5点（推定代表値には不使用）
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
- [x] poolの `max_connections` を50 / 75 / 100で比較し、50を維持する
  - 同じhot-path実装の3走中央値107,234 / 105,867 / 103,720点、全run
    `pass=true`・error map空
- [ ] poolの `min_connections`、`acquire_timeout` を必要な症状が出た場合に調整する
  - 起動時handshakeではなく定常時size 50 / idle 0が主要状態なので、`min_connections` は
    直近の解決策にしない
- [x] pool上限比較でMySQL接続数とrow-lock悪化を同時採取する
  - `Max_used_connections`は51 / 77 / 101
  - InnoDBの1 wait平均は18 / 23 / 26msで、上限を増やさない根拠にした
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
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test --all --all-targets`
- [x] `cargo build --release --locked`

### API・正当性

- [x] `./scripts/smoke-test.sh`
- [x] 公式prevalidation
- [ ] 通知の全遷移・順序・重複許容・取りこぼし
- [ ] chair statsが走行中は固定され、`COMPLETED` で当該評価を含むこと
- [x] nearbyの空車判定（初期状態、負荷中3時点、response配送境界修正後の60秒3走）
- [ ] nearbyの座標・3秒猶予
- [ ] ownerの距離・売上・0件行
- [ ] 並行ride作成と並行matching
- [ ] 決済retryとexactly-once相当の結果
- [x] 招待登録の異なる24 code同時実行、同一codeの3成功・1拒否、
  coupon件数、MySQL 1062 / 1213増分0
- [ ] `rides.updated_at` と履歴 `completed_at` が完全一致すること
- [ ] 既存表へ列を追加しても列名なし初期ダンプをロードできること
- [ ] initialize直後とwebapp再起動後

### 性能

- [x] 同じCPU・メモリ・走行時間で変更前後を比較する
- [x] 最低3回実行し、中央値とばらつきを残す
- [x] `pass`、スコア、全エラーコードを記録する
- [ ] 完了ride数を独立して記録する
- [ ] 空車移動距離×0.1、乗車中移動距離、完了ride数×5の各スコア寄与を記録する
- [ ] 全APIの30ms超過率とmatching / pickup / driveのtick遅延を記録する
- [ ] matcherは地域別pending数、available chair数、starvationした最古rideの待ち時間を記録する
- [ ] 通知はcache hit率、wake latency、recipientあたりSQL数、再接続時replay件数を記録する
  - [x] 1/64 samplingでcache hit率、path別total、pool acquire、SQL、connection所有を記録
  - [ ] long polling実装時にwake latencyと再接続時replay件数を記録
- [ ] 座標queueはdepth、最古未flush時間、batch件数、drop / retry数、status反映遅延を記録する
- [ ] エンドポイント件数とp50 / p95 / p99を記録する
- [x] SQL回数、累積時間、走査行数を記録する
- [ ] pool待ち、MySQL CPU、webapp CPU、block I/Oを記録する
- [ ] 改善しなければ変更を重ねずrevert候補として記録する

## 推奨する直近の実行順

1. `CODE=32`を再現し、pending ride、地域、空きchair、matcher batchとUPDATE結果を
   同じtickで採取する。critical errorが0件の通常3走へ戻す
2. `CODE=26` を再現し、ベンチマーカーがresponseを受信済みの座標と
   `owner_get_chairs` が集計する座標のwatermark差を同じchairで特定する。
   再現しない間も以下のP0計測は止めない
3. `CODE=8`が再発したらapp通知のride / user / cursorを同一requestで保存する
4. 評価APIのphase計測は完了。connection所有平均319.754msの約94.6%が決済で、
   retry sleepだけで平均201.719msと確認した
5. 短い準備transaction、transaction外の冪等決済、rideを再lockする短い完了transactionへ
   分ける。決済成功後のDB失敗は同じkeyで再開し、二重status・stats更新を防ぐ
6. app / chair通知cache missのphase計測は完了。connection所有平均は約10msだが、
   同じrequestの2回のacquire平均合計がapp 77.839ms、chair 82.513msだった
7. pool上限50 / 75 / 100の比較は完了し、通常3走中央値が最も高い50を維持した
8. chair通知のride選択は配送状態機械へ変更済み。hidden pendingとdelivery gapの
   固定回帰、通常3走の`CODE=12/29` 0件を確認したが、全体gateは未通過
9. owner request開始時に既知の座標までを集計する方法を設計し、決定的な赤・緑テストと
   通常3走で`CODE=26`のerror予算・scoreを比較する
10. `CODE=27`を同じchairのDB current row、process cache、nearby応答で追跡する
11. rideあり通知のconnection再利用は、`CODE=26/27`を解消してerror mapを安定させてから
   再比較する
12. latest-coordinate cacheの `RwLock` とmaintenance gateの待機・保持時間を計測し、
   2秒再同期時のglobal stallを定量化する
13. 最新statusをcurrent-state化し、未送信statusがない通知pollの履歴lookupとpayload再構築をなくす
14. matcherへ地域間の距離上限を追加し、500 / 100 / 30msの実行間隔と組み合わせて比較する
15. app history、owner sales、ride作成のN+1を順に除去する
16. current-state別表で最新statusをO(1)化する
17. JSON long pollingで不足する場合だけSSEへ移し、状態変更時の即時pushまで実装する
18. 貪欲matcherと最小費用二部マッチングを比較する
19. 最後にMySQL、nginx、compiler設定をprofileに基づいて調整する

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
