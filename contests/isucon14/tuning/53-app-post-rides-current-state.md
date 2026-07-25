# Benchmark 53: ライド作成をcurrent stateとuser row lockで判定する

[チューニング目次へ戻る](../TUNING.md)

## 結論

`POST /api/app/rides` が利用者の全rideを読み、rideごとに最新statusを問い合わせる
N+1を除去しました。`rides.evaluation IS NULL` を進行中のcurrent stateとして集約し、
同一userの並行作成は `users` の主キー行を `FOR UPDATE` して直列化します。

診断runのendpoint latencyは次のように短縮しました。

| 指標 | 変更前 | 変更後 | 差 |
| --- | ---: | ---: | ---: |
| request数 | 2,613 | 2,493 | workload依存 |
| 平均 | 217ms | 163ms | -24.9% |
| p50 | 193ms | 159ms | -17.6% |
| p95 | 518ms | 384ms | -25.9% |
| p99 | 646ms | 477ms | -26.2% |
| 最大 | 770ms | 635ms | -17.5% |

通常60秒3走は124,346–129,832点、推定代表値の中央値125,536点でした。直前の
Benchmark 52は124,205–133,737点、中央値128,584点なので、中央値は3,048点、
2.37%低下しています。候補の範囲5,486点、対照の範囲9,532点のどちらよりも小さい差であり、
全体スコアが改善したとは判断しません。

一方、変更前の固定fixtureでは同じuserの8並行requestがすべて202となり、active rideも
8件作られました。変更後は202が1件、409が7件、active rideが1件です。全履歴N+1と
重複queryを除き、endpoint tailを約25%短縮しながらこの競合も修正できたため採用します。

## 比較revisionと再現手順

| 役割 | jj commit ID |
| --- | --- |
| 対照 | `e291f270b9862c73ec83a047eb1cb22f6f6d71b2` |
| 実装 | `57a2b354f3ba0785ad28eaa1a96991acbd387d2a` |

ホスト・Colimaの割当は全runで4 CPU / 4 GiBのままです。

```sh
./scripts/test-app-post-rides-concurrency.sh
./scripts/test-invitation-concurrency.sh
./scripts/test-app-rides-batch.sh
./scripts/smoke-test.sh

(cd webapp/rust && cargo fmt --check)
(cd webapp/rust && cargo test --all --all-targets)
(cd webapp/rust && cargo clippy --all-targets --all-features -- -D warnings)

diagnostic_since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
ISUCON_DIAGNOSTIC=1 \
  BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b53-candidate-diagnostic.txt \
  ./scripts/benchmark.sh 60
ISUCON_DIAGNOSTIC=1 ./scripts/report-endpoint-latency.sh "$diagnostic_since"

BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b53-normal-1.txt ./scripts/benchmark.sh 60
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b53-normal-2.txt ./scripts/benchmark.sh 60
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b53-normal-3.txt ./scripts/benchmark.sh 60
```

## はじめに知っておく用語

### current state

履歴を最初から再生しなくても、現在の判断に直接使える状態です。変更前は
`ride_statuses` の履歴を各rideについて読み、最新statusが `COMPLETED` か判断しました。

この実装では評価完了transactionが次を一緒にcommitします。

- `rides.evaluation` を設定する
- `ride_statuses` へ `COMPLETED` を追加する
- chair statsを更新する

初期データ、fixture、負荷後DBで次の不一致が0件であることを確認しました。

- `COMPLETED` があるのにevaluationがNULL
- evaluationがあるのに `COMPLETED` がない

したがって、ライド作成が必要とする「まだ完了していないrideがあるか」は
`rides.evaluation IS NULL` だけで判定できます。これはDB制約ではなくwriterの不変条件
なので、回帰テストで継続して監査します。

### serialization point

同じ論理資源への並行更新を、必ず同じ1行のlockへ合流させる地点です。今回は
`users.id` の主キー行を使います。

```sql
SELECT id
FROM users
WHERE id = ?
FOR UPDATE
```

同じuserの2 requestが来ると、先にlockを取ったtransactionだけが先へ進みます。後続は
先行transactionのcommitを待ってからride状態を読みます。異なるuserは異なる主キー行を
lockするため、全利用者を1本に直列化しません。

### lost updateではなくcheck-then-act競合

変更前は値を上書きして失う競合ではなく、次のcheckとactの間に穴がありました。

```text
request A: active rideなしと読む
request B: active rideなしと読む
request A: rideをINSERT
request B: rideをINSERT
```

どちらのINSERTも異なるride IDなので、主キーUNIQUEは違反しません。`rides(user_id)` にも
「activeは1件」というUNIQUE制約がないため、DBは両方を正常に受理します。読み取りを
速くするだけでは直らず、判定とINSERTを同じserialization pointの後ろへ置く必要があります。

### locking readとconsistent read

InnoDBの `SELECT ... FOR UPDATE` は最新commit済み行を読むlocking readです。通常の
SELECTはREPEATABLE READのconsistent readで、最初のconsistent read時にsnapshotを作ります。

今回の順序は次です。

1. user行をlocking readし、先行する同一user transactionのcommitを待つ
2. ride集約をこのtransaction最初のconsistent readとして実行する
3. 先行transactionが作ったactive rideを含むsnapshotで判定する

先にride集約を行ってからuser lockを待つと、古いsnapshotを保持する危険があります。
lock取得を先に置く順序に意味があります。

### 条件付きUPDATEと `rows_affected`

選んだcouponは次の条件でclaimします。

```sql
UPDATE coupons
SET used_by = ?
WHERE user_id = ?
  AND code = ?
  AND used_by IS NULL
```

更新行数が1なら未使用couponを1件だけclaimできました。0なら、選択後に前提が崩れています。
今回は `FOR UPDATE` とuser row lockがあるため通常0にはなりませんが、値を使ってfareを
返す前に `rows_affected() == 1` を確認し、未適用の割引をresponseへ載せません。

## 変更前の仕事量

概念的には次の順でした。

```text
1. userの全rideをSELECT
2. 各rideについて最新statusをSELECT
3. rideをINSERT
4. MATCHING statusをINSERT
5. userのrideをCOUNT
6. 初回か否かでcoupon SELECTを分岐
7. couponをUPDATE
8. INSERTしたrideをSELECT
9. used_byからcouponを再SELECT
10. fareを計算
```

対照診断runの終了snapshotは次のとおりです。

| query | 回数 | 累積 | 平均 | examined | sent |
| --- | ---: | ---: | ---: | ---: | ---: |
| userの全ride | 2,589 | 0.532s | 0.205ms | 5,967 | 5,967 |
| ride COUNT | 2,589 | 0.440s | 0.170ms | 8,556 | 2,589 |
| INSERT後ride再取得 | 2,589 | 0.212s | 0.082ms | 2,589 | 2,589 |
| 初回coupon locking read | 645 | 0.069s | 0.107ms | 645 | 645 |
| 古い未使用coupon locking read | 1,944 | 0.354s | 0.182ms | 2,568 | 872 |

全ride queryが返した5,967行に対してhandlerは必ず最新statusを1回ずつ呼ぶため、この
endpoint由来のstatus lookupも約5,967回です。prepared statement表のlatest status全体は
36,610回ですが、通知や評価も同じSQLを使うため、全回数をこのendpointへ帰属しません。

`SELECT * FROM coupons WHERE used_by = ?` も全体30,225回のうち、成功したride作成ごとに
1回発行されていました。他endpointとSQL本文を共有するので、合計全体と帰属分を区別します。

`prepared_statements_instances` は終了済みconnectionの行を失います。HTTP 2,613件と
prepared statement 2,589件の差は、失敗requestが24件あったという意味ではありません。
nginxでは2,613件すべて2xxでした。

## 仮説

仮説は3つに分けました。

1. 全履歴とride別statusを1集約へ変えれば、履歴数に比例する往復を除去できる
2. coupon選択後のride / coupon再読を消せば、固定回数の往復も減る
3. user row lockを先に取れば、同一user並行requestのactive重複を防げる

全体scoreの仮説は控えめです。このendpointは診断runで約2,600回ですが、通知は
10万回規模、座標は6万回規模です。endpoint単体が25%速くても、全体scoreの変化は
run間分散に埋もれる可能性があります。

## 実装

### user単位のlock順序

ride作成は次の順でlockします。

```text
users主キー行
  -> rides current stateをconsistent read
  -> ride / MATCHING INSERT
  -> couponをFOR UPDATE
  -> coupon claim
  -> COMMIT
```

招待登録は既に次の順です。

```text
招待者users主キー行
  -> 招待回数確認
  -> reward coupon INSERT
  -> COMMIT
```

両方が `users -> coupons` の同じ順序なので、相互に逆順でlockしてcycleを作りません。
固定fixtureでは同一userへのreward追加とride作成を同じuser lockで待たせ、どちらの
直列化順でもcouponとfareが整合することを確認しました。

### ride状態を1走査で集約

```sql
SELECT
  COUNT(*) AS ride_count,
  CAST(COALESCE(MAX(evaluation IS NULL), 0) AS SIGNED) AS has_active_ride
FROM rides
WHERE user_id = ?
```

`COUNT(*)` はINSERT前に0なら初回rideです。`MAX(evaluation IS NULL)` は1行でも未評価が
あれば1になります。別々のCOUNTとEXISTSではなく、同じuser範囲を1回走査します。

`CAST(... AS SIGNED)` はMySQLの式結果をSQLxの `i64` と明示的に合わせます。空集合では
`MAX` がNULLになるため `COALESCE(..., 0)` が必要です。

### coupon選択を1つのSQL形へ統合

```sql
SELECT code, discount
FROM coupons FORCE INDEX (PRIMARY)
WHERE user_id = ?
  AND used_by IS NULL
ORDER BY
  CASE WHEN ? AND code = 'CP_NEW2024' THEN 0 ELSE 1 END,
  created_at,
  code
LIMIT 1
FOR UPDATE
```

初回のbind値だけtrueにし、`CP_NEW2024` を先頭へ置きます。2回目以降は全行のCASE値が
1になり、付与時刻順です。同時刻はcodeで決定し、結果を再現可能にしました。

永続化modelの `Coupon` 全列ではなく、作成処理に必要な `code` と `discount` だけを
`RideCoupon` へdecodeします。claim後は選択済みdiscountを純粋関数へ渡すため、rideと
couponの再SELECTは不要です。

## INDEXをどう考えたか

### `users` の主キー

`users.id` はPRIMARY KEYです。InnoDBでは主キーB-treeのleafがrow本体を持つclustered
indexなので、user IDから1行へ到達してそのrecord lockを取れます。user数全体に対して
1行だけを直列化地点にできるため、新しいlock tableは不要です。

### `rides(user_id, created_at)`

既存の `idx_rides_user_created_at` は先頭列がuser IDです。MySQLは特定userの範囲だけを
range scanし、evaluationをrowから読んで集約します。

rideが11件ある実測点の `EXPLAIN ANALYZE` は次でした。

```text
Index lookup on rides using idx_rides_user_created_at
  actual rows=11
Aggregate
  actual time=0.0284ms
```

`evaluation` をINDEXへ追加すればcoveringにできますが、ride完了時のevaluation更新でも
secondary index更新が増えます。診断runの集約平均は0.207ms、1 user最大11件だったため、
write amplificationを増やす新INDEXは追加しません。

### `coupons` のPRIMARY KEY

couponの主キーは `(user_id, code)` です。先頭列user IDで、そのuserのcouponだけへ
範囲を限定できます。診断runでは2,473回で8,393行、1回平均約3.4行を調べました。

`idx_coupons_used_by(used_by)` もありますが、`used_by IS NULL` は未使用coupon全体に
多く一致します。optimizerの統計次第でこちらを選ぶと、user単位より広い範囲を読む可能性が
あります。今回は `FORCE INDEX (PRIMARY)` で探索境界を明示しました。

INDEX hintは将来の分布やschema変更へ自動適応しません。userあたりcoupon数が大きく増えたら、
`(user_id, used_by, created_at)` の複合INDEXとhintなしの計画を再比較します。

## 正当性fixture

`scripts/test-app-post-rides-concurrency.sh` は開始・終了時にinitializeし、次を確認します。

- 初期データのevaluation / `COMPLETED` 不一致が0
- 初回は古い別couponより `CP_NEW2024` を優先
- 2回目以降は最古の未使用couponを選択
- 通常割引と過大割引でもfare式を維持
- active rideがあれば409
- 招待reward追加とride作成を同じuser lockで競合させても整合
- 8並行ride作成は202が1件、409が7件
- DB上のactive rideとMATCHING statusも1件

並行fixtureでは、空の `rides(user_id)` 範囲を別transactionでgap lockします。ready markerを
mysql clientのunbuffered出力で確認してから8 requestを開始するため、単なるsleepによる
開始時刻の推測ではありません。

変更前は次の結果でした。

```text
parallel requests: accepted=8 conflict=0 db=8  8  8
parallel ride creation was not serialized per user
```

変更後は次です。

```text
parallel requests: accepted=1 conflict=7 db=1  1  1
app ride creation regression: PASS
```

招待の既存回帰も次を維持しました。

```text
distinct=24 shared_created=3 shared_rejected=1
duplicate_delta=0 deadlock_delta=0
```

## 診断run

対照と候補はいずれも `pass=true`、error map空でした。

| 指標 | 対照 | 候補 |
| --- | ---: | ---: |
| score | 142,315 | 140,616 |
| app ride作成request | 2,613 | 2,493 |
| endpoint平均 | 217ms | 163ms |
| endpoint p95 | 518ms | 384ms |
| endpoint p99 | 646ms | 477ms |
| MySQL 1062 | 0 | 0 |
| MySQL 1213 | 0 | 0 |
| userあたりactive最大 | 1 | 1 |

候補の新しい3 queryは次でした。

| query | 回数 | 累積 | 平均 | 最大 | examined | sent |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| user row lock | 2,473 | 0.196s | 0.079ms | 13.123ms | 2,473 | 2,473 |
| ride current state集約 | 2,473 | 0.511s | 0.207ms | 15.337ms | 5,375 | 2,473 |
| coupon選択 | 2,473 | 0.408s | 0.165ms | 19.891ms | 8,393 | 1,499 |

旧全履歴、ride COUNT、INSERT後ride再取得は候補snapshotから消えました。latest statusと
`used_by` coupon queryは他endpointも使うためシステム全体には残ります。

## 通常60秒3走

| run | pass | score | error | matching不満 | pickup不満 | drive不満 |
| ---: | --- | ---: | --- | ---: | ---: | ---: |
| 1 | true | 129,832 | 空 | 59.3% | 33.7% | 61.4% |
| 2 | true | 125,536 | 空 | 52.8% | 29.9% | 65.0% |
| 3 | true | 124,346 | 空 | 59.6% | 33.5% | 59.9% |
| 推定代表値 | - | 125,536 | - | 59.3% | 33.5% | 61.4% |

対照は124,205 / 128,584 / 133,737点、中央値128,584点です。

```text
推定差 = 125,536 - 128,584 = -3,048
推定変化率 = -3,048 / 128,584 ≒ -2.37%
```

matching不満中央値は55.8%から59.3%へ悪化し、pickupは34.2%から33.5%、driveは
62.8%から61.4%へ改善しました。対象endpointの変更だけを各不満率へ因果帰属できず、
完了ride数と距離内訳も独立保存していないため、スコア差を推定配分しません。

## 確認したログと判断

| 証拠 | 分かったこと | 判断 |
| --- | --- | --- |
| OpenAPI | 202 response、fare、active時409 | response契約を固定 |
| Rust source | 全履歴 + ride別status + 固定再読 | N+1と重複queryを対象化 |
| schema | user主キー、rides複合INDEX、coupon主キー | 追加INDEXなし |
| 不一致監査 | evaluationとCOMPLETEDの不一致0 | current state利用可 |
| 変更前並行fixture | 8 requestすべて202、active 8件 | row lockが必要 |
| 変更後並行fixture | 202が1件、409が7件 | 同一user直列化を確認 |
| reward競合fixture | どちらの直列化順でもfareとcoupon整合 | lock順序を確認 |
| `EXPLAIN ANALYZE` | user範囲11行、集約0.0284ms | 新INDEX不要 |
| prepared statement | 旧query消滅、新query各約1回/request | 往復削減を確認 |
| nginx timing | p95 / p99約26%短縮 | endpoint効果を確認 |
| MySQL error summary | 1062 / 1213が0 | UNIQUE / deadlock回帰なし |
| 通常3走 | 中央値-2.37%、全走合格 | 全体得点改善とはしない |

## 検討した他の選択肢

### `EXISTS` だけへ変える

履歴N+1は消せますが、並行requestが同時にfalseを読む競合は残ります。判定queryの
高速化と、判定からINSERTまでの排他は別の問題です。

### ridesのuser範囲を直接 `FOR UPDATE` する

既存rideがあればlockできますが、初回は行がありません。gap lockで空範囲を守る方法は
INDEXとisolation levelへの依存が強く、評価transactionがride行をlockする経路とも
競合します。必ず存在するusers行の主キーlockの方が境界を説明しやすく、招待rewardとも
同じ順序へ揃えられます。

### active ride専用table

`active_rides(user_id PRIMARY KEY, ride_id)` を作れば、INSERTのUNIQUE制約でDB自身が
1件だけを保証できます。完了時DELETE、initialize backfill、途中障害のrepairが必要です。
現在はuser row lock + evaluationで十分に短く、別tableの不変条件を増やしませんでした。

### generated column + UNIQUE INDEX

未評価時だけuser ID、完了時NULLになるgenerated columnへUNIQUE INDEXを付ける案です。
NULLは複数許容しactiveだけ一意にできますが、既存tableの列追加、列名なし初期dump、
evaluation更新時のINDEX保守を検証する必要があります。schema変更は独立Benchmarkにします。

### MySQL advisory lock

`GET_LOCK(user_id, timeout)` はrowではなくconnection単位の名前付きlockです。transaction
rollbackと自動で同じ境界にならず、connection poolへ返す前のrelease漏れも危険です。
既存の主キーrecord lockを優先します。

### 全体をSERIALIZABLEにする

全transactionのisolationを強くすると、無関係なrange readまでlock競合が増えます。
必要なuserだけを明示的に直列化する方が影響範囲を狭くできます。

### coupon専用複合INDEX

`(user_id, used_by, created_at)` なら未使用couponをさらに絞れます。ただし初回
`CP_NEW2024` 優先のCASE sortは残り、couponのINSERT / claimにもINDEX更新が増えます。
現状は1 request平均約3.4行、SQL平均0.165msなので追加しません。

## 残る課題

1. app ride作成のgeneral admission、pool acquire、user lock待ち、SQL、COMMITをphase分解する
2. 同一userへ招待rewardが集中したときのuser lock待ちp95 / p99を測る
3. current state不一致を初期化後・負荷後の自動監査として共通化する
4. userあたりrideが大きく増えたときにactive専用tableとgenerated UNIQUEを比較する
5. 完了ride数、空車距離、乗車距離をrunごとに保存し、score差を分解する
6. `FORCE INDEX (PRIMARY)` がcoupon分布の変化後も妥当か定期的にEXPLAINする

次の優先対象は、`GET /api/app/rides` のSQL平均0.322msに対してHTTP p95が数百msになる
general admission / pool acquire / response phaseの分解です。SQL以外の待ちを特定した後、
nearbyのride antijoinとowner sales N+1を順に比較します。
