# Benchmark 18: 最新座標をcurrent-state表とprocess内cacheへ分離する

[チューニング目次へ戻る](../TUNING.md)

## 結果

このBenchmarkでの採用対象は、current-state表、2秒ごとの再同期、評価response bodyの
lifecycleまでchairを保持するtracker、initialize用maintenance gateをすべて含む
60秒ベンチ3走です。後続の高負荷ではbody lifecycle後にもclient受信まで競合窓があると
分かり、trackerはBenchmark 23でsnapshot・revision・delivery leaseへ拡張しました。
現在の最終結果は[Benchmark 23](./23-code30-response-delivery.md)を参照してください。

| run | pass | score | 最終評価数 | matching不満 | pickup不満 | drive不満 | error map |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | true | 96,888 | 1,294 | 28.2% | 42.9% | 71.7% | 空 |
| 2 | true | 96,926 | 1,357 | 20.8% | 39.7% | 74.7% | 空 |
| 3 | true | 98,483 | 1,390 | 23.8% | 36.7% | 77.1% | 空 |

- 観測範囲: 96,888–98,483点
- 推定代表値: 中央値96,926点
- 直前採用版: 中央値98,580点
- 差: -1,654点、約-1.7%
- handler-scope tracker版: 中央値97,790点
- handler-scope版との差: -864点、約-0.9%
- 時間依存の1秒cooldown版: 中央値95,095点
- 1秒cooldown版との差: +1,831点、約+1.9%

スコアだけなら改善を確認できませんでした。一方でnearby SQLから最新位置履歴の
`LATERAL` を除き、matcherも1 chair 1 rowのcurrent-state表へ移せました。さらに、
レビューで見つかった「commit後にrequest taskが中断するとprocess cacheが永久に古い」
問題には2秒再同期を追加しました。実際に出たbusy-chairの `CODE=30` には固定時間に
依存しないresponse body guardを追加し、最終3走では再発しませんでした。

この版は読み取り高速化だけでなく、最新座標について複数processがDBへ収束する経路を
含む土台として採用します。評価response trackerは後述のとおり単一process用です。
current rowのwrite amplificationは、スコア低下の有力な仮説として次のP0計測対象に
残します。

最初のprocess cacheだけの暫定版は、エラー0の3走で93,991–104,114点、
中央値103,683点でした。しかし別runに `CODE=30` があり、commit後キャンセルで
永続的にstaleになる反例も独立レビューで確認されたため、最終採用値には使いません。

## 何を優先したか

Benchmark 17の最終runで、`app_get_nearby_chairs` のSQLは次の負荷でした。

| 指標 | 値 |
|---|---:|
| 実行回数 | 1,838 |
| 累積実行時間 | 82.451秒 |
| 平均実行時間 | 44.859ms |
| 最大実行時間 | 460.403ms |
| rows examined | 1,492,575 |
| rows sent | 32,882 |

60秒より累積時間が長いのは、複数connectionの同時実行時間を合計しているためです。
通知や決済にもP0候補はありますが、今回は次の理由でnearbyを先に選びました。

1. 累積時間と平均時間が大きい
2. `EXPLAIN ANALYZE` で履歴走査とsortを直接確認できる
3. 仕様上、座標には最大3秒の遅れが許される
4. `is_active` と割当可否はDBから毎回読む構成を維持できる

全responseをcacheするのではなく、遅延が許される座標だけを分離します。

## 変更前のquery

```sql
SELECT chairs.id,
       chairs.name,
       chairs.model,
       latest_location.latitude,
       latest_location.longitude
FROM chairs
INNER JOIN LATERAL (
    SELECT latitude, longitude
    FROM chair_locations
    WHERE chair_id = chairs.id
    ORDER BY created_at DESC
    LIMIT 1
) AS latest_location ON TRUE
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND rides.evaluation IS NULL
  )
```

`LATERAL` は外側の `chairs.id` を内側queryで参照し、椅子ごとの最新位置を返します。
SQLは1回でも、内部では候補椅子ごとに履歴検索を繰り返します。

## 実行計画から立てた仮説

ベンチ後のDBは次の規模でした。

| table | 観測値 |
|---|---:|
| chairs | 810行 |
| active chairs | 300行 |
| chair_locations | 86,270行 |
| 位置を持つchair | 687台 |
| 1台あたり位置履歴 | 平均125.6行、最大846行 |

`EXPLAIN ANALYZE` の主要部分は次のとおりです。

```text
Nested loop antijoin
  active chairs: 300 rows
  空き候補: 42 rows

42 loops:
  Index lookup on chair_locations
    actual rows=166
  Sort: created_at DESC
  Limit: 1 row

query全体: 26.4ms
```

`chair_locations(chair_id, created_at)` があるため、最初は末尾から1行だけ読めると
予想しました。しかし実際のplanは候補42台それぞれで平均166行を読み、sortしてから
1行へ絞っていました。

INDEXが存在することと、optimizerが期待どおりの順序で読むことは同じではありません。
外側rowごとにderived tableをmaterializeする形では、複合INDEXの順序をそのまま
`LIMIT 1` に利用できず、履歴範囲のlookupとsortが残りました。

そこで仮説を次のように置きました。

> nearbyのたびに履歴から現在状態を再構成するのをやめれば、座標履歴のloopとsortを
> request経路から外せる。履歴はownerの累積距離に必要なので削除せず、現在座標だけを
> 別の読み取り形へ射影する。

座標取得をqueryから外した実行計画は約4.79msでした。残る主な仕事はactive chairと、
そのchairに未評価rideがないかのantijoinです。

## INDEXの仕組みと採らなかった案

### scalar subqueryを2本使う

緯度と経度を別々の相関subqueryで取得すると、各列で166行を読み、2回sortしました。
query全体は約31msで、元の `LATERAL` より仕事が重複するため不採用です。

### 降順covering INDEX

診断用DBへ一時的に次のINDEXを追加しました。

```sql
INDEX idx_chair_locations_latest_covering
    (chair_id, created_at DESC, latitude, longitude)
```

B-treeのleafに緯度・経度まで含むため、table rowへ戻らずINDEXだけで値を読めます。
単発時間は約26.4msから6.1msへ短縮しましたが、166行のscanとsortは残りました。

不採用理由は次です。

- requestごとの履歴走査を根本的に除去しない
- 高頻度INSERTのたびに太いsecondary INDEXを更新する
- 緯度・経度をleafへ追加し、buffer poolとstorageを消費する
- 既存 `(chair_id, created_at)` と役割が重なる

readだけを見ると有望でも、座標INSERTのwrite amplificationを含めて判断する必要があります。

### `MAX(created_at)` とjoinする

椅子ごとの最大時刻を求め、その時刻のrowへjoinする案は約12.4msでした。相関した
`MAX` の内側で履歴を読むplanが残り、同一時刻rowのtie-breakも必要です。

全chairを1回だけ再構築するqueryではindex skip scanが使われ、86,270履歴から687台分を
約14msで取得できました。高頻度requestと初期化1回では、同じquery costでも意味が違います。

### nearby response全体をcacheする

不採用です。座標には最大3秒の猶予がありますが、次は即時に反映する必要があります。

- `chairs.is_active`
- rideが割り当て済みか
- 評価が完了して再掲載可能になったか

全responseをTTL cacheすると、割当済みchairをTTL中も返す可能性があります。

## process cacheだけでは不足した理由

最初は追加DB writeを避け、`Arc<RwLock<HashMap<...>>>` だけを実装しました。しかし
独立レビューで次の反例が見つかりました。

```text
chair_locations INSERTをcommit
  ↓
client切断やtask cancellation
  ↓
commit後のprocess cache更新へ到達しない
  ↓
initializeまたはprocess再起動まで古いまま
```

座標に3秒猶予があっても、この反例は3秒で収束しません。定期的に履歴全体を集約する
試作は、60秒runで107回・累積2.920秒・平均27.285msでした。正しさは回復しますが、
8万行の履歴を繰り返し集約する固定費になります。

そこで `chair_current_locations` を追加し、1 chair 1 rowだけを保持します。

```sql
CREATE TABLE chair_current_locations (
  chair_id    VARCHAR(26) PRIMARY KEY,
  location_id VARCHAR(26) NOT NULL,
  latitude    INTEGER NOT NULL,
  longitude   INTEGER NOT NULL,
  created_at  DATETIME(6) NOT NULL
)
```

履歴INSERTとcurrent UPDATEは同じtransactionです。process cacheは2秒ごとにこの表から
全置換し、request futureの完走に依存せず収束します。

代償もあります。最終run 3ではcurrent UPDATEが39,013回、累積29.033秒、
平均0.744msでした。readを短縮してもwriteは増えるため、無料の高速化ではありません。

## 実装

### canonical order

cacheする値は次です。

```text
chair_id -> latitude, longitude, recorded_at, location_id
```

並行requestは記録時刻順にcommitするとは限りません。新しい座標Bが先にcacheへ入り、
古いAが後から到着しても、Aで上書きしてはいけません。

```text
A: recorded_at=10 ───────── commit ─ cache update
B: recorded_at=20 ─ commit ─ cache update
```

DB、process cache、matcherの「最新」をすべて
`(created_at DESC, location_id DESC)` に統一しました。同一microsecondならIDの辞書順で
決定します。初期backfillも次のwindow関数で同じ順序を使います。

```sql
ROW_NUMBER() OVER (
  PARTITION BY chair_id
  ORDER BY created_at DESC, id DESC
)
```

### `Arc<RwLock<HashMap<...>>>`

`AppState` はrouter、middleware、handlerへcloneされます。HashMap本体をcloneせず、
`Arc` で同じ所有物を共有します。

`RwLock` は複数nearby requestのreadを同時に許し、座標更新の短い区間だけwriteを
排他にします。呼び出し側へguardやHashMapを公開せず、必要なchair IDの
`Vec<Option<Coordinate>>` だけをcopyします。handlerがDB queryやHTTPのawait中まで
read lockを保持できないAPIにしています。

起動・initializeは通常requestとmaintenance gateで排他したうえで全置換します。
定期再同期はMySQL query中にcacheのwrite guardを持ちません。先にDB snapshotを取得し、
write guardを取ってから、その間にcommit後cache更新された新しいversionをsnapshotへ
mergeします。これにより更新を消さず、MySQL待ちの間に全nearby readを止めません。

current-state表は履歴8万行ではなくchair数だけです。最終run 3で観測した再同期SELECTは
37回、累積0.038秒、平均1.039ms、最大6.502msでした。

### 初期化、既存volume、process再起動

正解データと派生データの関係は次です。

1. `chair_locations` は全履歴を永続化する
2. `chair_current_locations` は同じtransactionで最新1件へ更新する
3. commit後、同processのcacheを即時更新する
4. 起動時と `POST /api/initialize` 後にcurrent-state表から全置換する
5. 2秒ごとにcurrent-state表から全置換する
6. cache missのchairはnearbyへ含めない

`4-current-data.sql` が初期履歴をbackfillします。既存Docker volumeでは起動時に表を
作成し、canonical latestを `ON DUPLICATE KEY UPDATE` で冪等に投入します。表が空の
場合だけでなく、一部chairのrowが欠けた状態や古いrowが混ざった部分移行も修復します。
webappだけを再起動してもcurrent-state表からcacheを復元できます。

定期再同期により、commit後cache更新が欠けても、健康なDBとprocessであれば次の2秒tickと
query時間で修復されます。

### current rowの更新

既存current rowは主キーUPDATE、新規chairはatomic upsertを使います。cache確認なしで
「まずUPDATEし、0行ならINSERT」とした試作は不採用です。存在しないkeyへのUPDATEが
REPEATABLE READのgap lockを取り、複数の初座標transactionが後続INSERTでdeadlockしました。

cacheにcurrent rowがないchairは最初からupsertします。既存と分かるchairだけUPDATEし、
古いversionや稀なcache不一致で0行だった場合はconditional upsertへフォールバックします。

### nearbyとmatcher

nearby SQLは座標を返しません。

```sql
SELECT chairs.id,
       chairs.name,
       chairs.model
FROM chairs
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND rides.evaluation IS NULL
  )
```

返ったchair IDの座標をHashMapから読み、マンハッタン距離をRustで判定します。
active状態と割当可否はcacheしません。さらに、評価中または評価response bodyを送信中の
chair IDを `ActiveRideEvaluationTracker` のsnapshotと照合し、DB commit後からbodyが
消費またはdropされるまでnearbyへ再掲載しません。

matcherは `chair_current_locations` を主キーjoinし、最新位置のLATERALを除去しました。
`FOR UPDATE SKIP LOCKED` によるrideとchairのclaim範囲は変えていません。

## SQL実測

process cacheだけの暫定版では、nearby SQLが次まで短縮しました。

| run | 回数 | 累積 | 平均 | 最大 | rows examined | rows sent |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1,687 | 14.048秒 | 8.327ms | 60.204ms | 1,502,951 | 32,388 |
| 2 | 2,052 | 17.722秒 | 8.637ms | 72.482ms | 1,757,886 | 36,175 |
| 3 | 1,400 | 12.464秒 | 8.903ms | 72.775ms | 1,148,911 | 21,295 |

変更前平均44.859msに対し、暫定版中央値は8.637msで約80.7%減です。

最終response body tracker版run 3の `prepared_statements_instances` snapshotは
次のとおりです。

| SQL | 回数 | 累積 | 平均 | 最大 |
|---|---:|---:|---:|---:|
| nearby候補 | 1,376 | 11.117秒 | 8.079ms | 74.339ms |
| current UPDATE | 39,013 | 29.033秒 | 0.744ms | 97.494ms |
| current upsert | 210 | 0.088秒 | 0.420ms | 11.358ms |
| matcher current join | 66 | 1.838秒 | 27.853ms | 112.053ms |
| current-state再同期 | 37 | 0.038秒 | 1.039ms | 6.502ms |

prepared statement snapshotは実行時間と回数を接続ごとに集計できますが、
`events_statements_summary_by_digest` のようなrows examined / sentは持ちません。
同じrunの異なる観測器の列を混ぜず、未取得列は無理に推定していません。

nearby平均は短くなりましたが、current UPDATEが新しいhot SQLです。得点中央値が前段より
1.7%下がったこととも整合します。runごとの処理量が違うため因果の断定はせず、次回は
row-lock待機とtransaction p95 / p99を直接測ります。

## 正当性確認

### 履歴、current-state、再起動

動的chairの座標APIを実行し、履歴とcurrent-state表が同じtransactionで `(314, 159)` へ
更新され、nearbyへ即時反映されることを確認しました。

履歴をwindow関数でcanonical orderへ並べた結果とcurrent-state表の全件比較は次です。

```text
history_latest=693
current_rows=693
mismatches=0
```

webappだけを再起動した後も、同じchairが `(314, 159)` でnearbyへ返りました。
さらにcurrent rowを1件削除し、別の1件を古い値へ書き換えてからwebappを再起動し、
起動時の冪等backfillが両方を修復することを確認しました。

### commit後cache更新欠落の故障注入

`scripts/test-latest-location-reconciliation.sh` はAPIのcache更新を通さず、DB transactionへ
履歴とcurrent rowだけを書きます。「DB commitは成功したがrequest futureがcache更新前に
止まった」状態の再現です。

```text
OK: startup repaired missing and stale current-location rows
OK: direct DB update converged through reconciliation in 1.693s (limit: 3.000s)
OK: equal timestamps select the lexicographically greatest location ID (1.651s)
```

loop回数ではなく、curl時間も含めたmonotonic clockの実時間をassertします。
各curlには0.5秒のtimeoutを設定し、3秒を超えた成功を見逃しません。

### unit test

```text
cargo test --all --all-targets
7 passed
```

古いcache更新の無視、同一時刻のtie-break、再同期snapshotと並行cache更新のmerge、
共通insert helperのversion規則に加え、
評価中trackerが複数guardを参照数で扱うこと、正常なbody消費とclient切断相当のbody
dropで最後のguardが必ず解除されることを確認しています。

## `CODE=30` をどう特定して修正したか

最終検証前のrunで警告全文を保存しました。

```text
取得した付近の椅子情報に不備があります (CODE=30):
ID:01KYAAFXG20T860NNWBX33NTSPの椅子は既にライド中です
```

該当chairの直前rideは15:04:45.856に完了し、警告は15:04:46.202でした。DBで評価を
commitした後、benchmarkerがHTTP responseを受けて自身の `Evaluated` flagを更新する前に
nearbyが走ると、DBだけが先に空きと判断します。

最初は評価更新後の再掲載を遅らせ、比較しました。

| 条件 | 結果 |
|---|---|
| cooldownなし | 87,176点、`CODE=30` 1件 |
| 500ms | 1走目91,301点・エラー0、2走目87,366点・`CODE=30` 1件。不採用 |
| 1秒 | 3走すべてエラー0、中央値95,095点。一度は採用候補 |

しかし独立レビューで、基準時刻に反例が見つかりました。`rides.updated_at` はevaluation
UPDATE時に決まりますが、その後も同じtransaction内で外部決済HTTP、失敗時のGET確認、
最大5回の100ms retryを行います。

```text
evaluation UPDATE（updated_atが決まる）
  -> 外部決済HTTP / retry
  -> COMMIT
  -> HTTP response
```

決済が1秒より遅ければ、commitした瞬間にはcooldownが既に切れています。反対に処理が
速ければ、response後の正常なnearbyまで不要にchairを隠します。固定時間は観測runには
合っても、処理時間を変えるチューニングの途中で正しさが変わるため、最終版では
500msと1秒の両方を不採用に戻しました。

### 明示的な評価response tracker

最終版は `ActiveRideEvaluationTracker` にchair IDを登録します。

1. ride rowをlockし、最新statusが `ARRIVED` だと確認する
2. chair IDの参照数をtrackerで増やし、RAII guardを作る
3. evaluation更新、COMPLETED追加、外部決済、commitを行う
4. nearbyはDBの `evaluation IS NULL` に加え、trackerにあるchairを除外する
5. 成功responseを `IntoResponse` でJSON bodyへ変換し、guardの所有権をbody wrapperへ移す
6. bodyの送信処理が終わるか、client切断でbodyがdropされるとguardもdropする
7. commit前の失敗や早期returnではhandler側のguardがdropする

ここでは時刻を比較しません。外部決済が30msでも1秒を超えても、handlerが実行中なら
同じ状態です。重複guardがあっても最初のdropだけで消えないよう、
`HashMap<chair_id, usize>` の参照数にしました。

最初のtracker版はguardをhandlerのローカル変数に置き、97,790 / 101,213 /
90,228点、中央値97,790点、エラー0でした。しかしAxumはhandlerが `Json` を返した後に
response変換とbody送信を行います。handler scope末尾のdropでは、DB commitから
benchmarkerのresponse受信までの競合窓を最後まで保持できません。3走で再発しなかった
ことは競合が消えた証明ではないため、この版も最終候補から外しました。

最終版の `ActiveRideEvaluationBody` は `http_body::Body::poll_frame`、
`is_end_stream`、`size_hint` を内側のAxum bodyへ委譲しながらRAII guardを所有します。
正常なbody消費とbody dropを別のunit testで固定しました。60秒3走は
96,888 / 96,926 / 98,483点、中央値96,926点で、prevalidationを通過し、
`CODE=30`を含むerror mapがすべて空でした。

trackerは1 Rust process内の整合性です。現在のComposeはwebapp 1 processなので条件と
一致します。複数processへ水平分割する場合は、評価中状態をDBやRedisへ共有し、
process crash後のlease回収も含めて再設計する必要があります。

body wrapperが説明できる境界は「Axum/Hyperがresponse bodyを消費またはdropするまで」
です。TCP peerでのJSON decodeとbenchmarkerのatomic flag更新をserverがACKとして
観測するprotocolではないため、その数命令のscheduleまで数学的に閉じるものでは
ありません。今回は元の長いtransaction/handler窓をbody lifecycleまで狭め、
3走で再発しなかったことを採用根拠とします。完全なACKが必要ならAPIへack endpointを
追加するか、DB/Redisの期限付きleaseを比較する必要があります。

### 後続計測で分かったこと

この節の3走は当時の処理量では再発しなかったという履歴です。Benchmark 22で認証SQLを
cache化して評価・nearbyの並行数が増えると、`CODE=30` が6–20件再発しました。追加した
phase診断では、body guardのみのrunで出た27件すべてが、benchmarker側ではまだ評価HTTP
レスポンスを待っていました。serverのbody dropからclient受信完了までは約55–677msの差が
あり、「body lifecycleまで保持すればclient観測まで閉じる」という仮説は棄却しました。

最終版はnearby開始snapshot、completion revision、body drop起点の1秒delivery leaseを
組み合わせています。公式3走の`CODE=30`は0件でした。新しい因果・実装・スコアは
[Benchmark 23](./23-code30-response-delivery.md)を参照してください。

### response extensionへguardを移す案を採らなかった理由

commitからnetwork response完了までをさらに狭めるため、RAII guardをAxumのresponse
extensionへ載せる試作も行いました。短時間prevalidationは通り、60秒3走もすべて
エラー0でしたが、スコアは88,393 / 94,155 / 91,620点、中央値91,620点でした。

Axum 0.7の公式資料はextensionをresponseへ値を付加する方法として説明しますが、
「response bodyの送信完了まで値を保持するcompletion hook」というlifetime保証は
示していません。解放時点を仕様として説明できず、handler-scope版中央値97,790点より
6,170点、約6.3%低かったため不採用です。最終版ではextensionではなく、lifecycle契約を
実装・unit testできるBody wrapperを採用しました。

### initializeを通常requestと排他する

`init.sh` はtableをdropして再作成します。古いcacheを持つcoordinate requestや定期再同期が
この途中へ入ると、current rowがないのに既存row用UPDATEを選んだり、空snapshotを
公開したりします。最終版は全通常APIがmaintenance gateのread guard、initializeがwrite
guardを取り、reset開始からcache再読込までを排他します。定期再同期も同じread guardを
取るため、lock順序は次に統一されます。

```text
maintenance gate
  -> reconciliation mutex
  -> latest-location cache write lock
```

## 失敗したcurrent UPDATE最適化とdeadlock

cache確認なしで「主キーUPDATE、0行ならupsert」とした短時間runでは `CODE=1` が24件
発生しました。webapp logはMySQL error 1213、InnoDB statusは次の循環待ちでした。

```text
missing keyへのUPDATE
  -> REPEATABLE READでPRIMARY supremumのgap lock
複数transactionが別chairのgap lockを保持
  -> 続くINSERT intentionが相互待ち
  -> deadlock victimをrollback
```

新規chairでは最初からatomic upsertを使う修正版の短時間runは5,648点・全エラー0、
deadlock 0でした。短時間のcurrent UPDATEは平均0.226msでしたが、60秒負荷では
最終run 3で平均0.744ms、最大97.494msまで伸びました。低負荷の値を高負荷へ
一般化しないことも、この実験の学びです。最終run 3終了時の
`information_schema.INNODB_METRICS.lock_deadlocks` は0でした。

## 効果と限界

```text
chair_locations:
  全履歴、owner累積距離、初期backfill

chair_current_locations:
  process間で共有する最新1件、matcher、cache再同期元

process cache:
  nearbyの座標lookup

chairs / rides:
  active状態、割当可否

ActiveRideEvaluationTracker:
  評価handler開始からresponse body dropまでのprocess内availability
```

利点は、高頻度readのために履歴を毎回並べ直さないことです。限界もあります。

- 全座標更新が1個の `RwLock` writeを通る
- 全座標更新で履歴INSERTに加えてcurrent UPDATEが発生する
- DB commit後からcache更新前まで短い古い座標を返し得る
- DBまたはprocessが停止すれば2秒再同期も動かない
- trackerは単一process内だけで、複数processへ即時共有されない
- body終了からclient側のatomic flag更新までをserverはACKとして観測できない
- maintenance gateは通常時のread lock取得を全APIへ1回追加する

複数processでもDB current-state表から2秒ごとに収束しますが、即時共有ではありません。
より短い同期が必要なら更新event、Redis、DBだけでのreadを比較します。

## 次に測ること

1. current UPDATEのrow-lock待機時間とcoordinate transaction p50 / p95 / p99
2. current row更新を独立transaction、順序付きqueue、一定間隔のcoalescingにした場合の
   3秒収束、累積距離、crash整合性
3. `RwLock` とmaintenance gateのread / write待機時間、保持時間
4. nearby SQLに残るride antijoinをcurrent ride表で減らせるか
5. webappを複数process化する場合の共有availability state、lease、crash回収
