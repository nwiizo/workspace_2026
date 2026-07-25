# Benchmark 52: 利用者のライド履歴を1回のSQLで取得する

[チューニング目次へ戻る](../TUNING.md)

## 結論

`GET /api/app/rides` に残っていた、rideごとのstatus、coupon、chair、owner検索を、
1回のSQLへまとめました。通常60秒の3走は124,205–133,737点、推定代表値の中央値は
128,584点です。直前の同期実装3走の中央値127,499点に対して+1,085点、約+0.85%でした。
全走 `pass=true`、error mapは空です。

点差はrun間の幅9,532点より小さく、スコアだけでは大きな改善と断定できません。一方で、
次の内部仕事量は確実に削減できました。

- 利用者の履歴1件ごとに発生していたRustとMySQL間の往復を除去
- 一覧専用のchair / owner個別SELECTを除去
- 一覧読み取り用の明示的な `BEGIN` / `COMMIT` を除去
- `SELECT *` をやめ、response作成に必要な列だけをdecode
- 料金計算を純粋関数へ分離し、通常割引と過大な割引の下限を単体テスト

固定fixture、同じ終了DBに対する変更前後JSONの完全一致、SQL実行計画、負荷中の
prepared statement統計を確認したため、この変更は採用します。

## 比較revisionと再現手順

| 役割 | jj commit ID |
| --- | --- |
| 同期対照 | `18f73352f07e2ba50aea8cfcbed496155132afb8` |
| 1 SQL化した実装 | `78cd698211c643a30b4c6e95c4cbeb5299c1d632` |

同じ4 CPU / 4 GiB、60秒、通常設定で対照と候補を各3走しました。候補の固定fixture、
Rust検証、診断run、通常runは次の順で再現できます。

```sh
./scripts/test-app-rides-batch.sh
(cd webapp/rust && cargo fmt --check)
(cd webapp/rust && cargo test --all --all-targets)
(cd webapp/rust && cargo clippy --all-targets --all-features -- -D warnings)

diagnostic_since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
ISUCON_DIAGNOSTIC=1 \
  BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b52-diagnostic.txt \
  ./scripts/benchmark.sh 60
ISUCON_DIAGNOSTIC=1 ./scripts/report-endpoint-latency.sh "$diagnostic_since"

BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b52-normal-1.txt ./scripts/benchmark.sh 60
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b52-normal-2.txt ./scripts/benchmark.sh 60
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b52-normal-3.txt ./scripts/benchmark.sh 60
```

prepared statementの終了時snapshotは、診断run直後かつwebapp connectionが残っている間に
次のqueryで採取しました。終了済みconnectionのstatementは表から消えるため、
nginx request数との完全一致ではなく、SQL形・実行回数・累積時間・examined / sentの
相対関係を読みます。

```sh
./scripts/compose.sh exec -T db mysql -uroot -pisucon performance_schema \
  -e 'SELECT SQL_TEXT,
             SUM(COUNT_EXECUTE) AS executions,
             ROUND(SUM(SUM_TIMER_EXECUTE) / 1000000000000, 3) AS total_s,
             ROUND(
               SUM(SUM_TIMER_EXECUTE) /
               NULLIF(SUM(COUNT_EXECUTE), 0) / 1000000000,
               3
             ) AS avg_ms,
             ROUND(MAX(MAX_TIMER_EXECUTE) / 1000000000, 3) AS max_ms,
             SUM(SUM_ROWS_EXAMINED) AS rows_examined,
             SUM(SUM_ROWS_SENT) AS rows_sent
      FROM performance_schema.prepared_statements_instances
      GROUP BY SQL_TEXT
      ORDER BY total_s DESC'
```

## はじめに知っておく用語

### N+1

最初の1回のSQLで親となる行をN件取得し、その後に各行へ追加SQLを実行する形です。
今回の変更前は概念的に次の処理でした。

```text
1回: 利用者のrideをすべて取得
N回: 各rideの最新statusを取得

完了ride M件について:
M回: couponを取得
M回: chairを取得
M回: ownerを取得
```

全rideが完了済みなら、概ね `1 + 4N` 回のSQLになります。利用者の履歴が1件のときは
問題が小さく見えますが、履歴が増えるほどSQL回数が直線的に増えます。

N+1の問題はMySQL内の検索時間だけではありません。1回ごとに次が繰り返されます。

1. RustがSQLとbind値をdriverへ渡す
2. SQLxがMySQL protocolのmessageを送る
3. MySQLがprepared statementを実行する
4. 結果をnetwork経由で返す
5. Rust taskがwakeされ、行をdecodeする

単発の主キー検索が0.1ms未満でも、poolが混雑した状態で多数の往復を順番に待てば、
handler全体は1tickの30msを超えます。

### JOIN

複数tableを関係するキーで結び、1つの結果集合を作るSQL操作です。今回は次の関係を
利用します。

```text
rides.chair_id  -> chairs.id
chairs.owner_id -> owners.id
coupons.used_by -> rides.id
```

`chairs.id` と `owners.id` は主キーなので、MySQLは各rideに対応する1行へ
B-treeから到達できます。

### 相関subquery

外側の行の値を使って評価される内側のqueryです。couponは次の形で取得します。

```sql
COALESCE((
  SELECT coupons.discount
  FROM coupons
  WHERE coupons.used_by = rides.id
  LIMIT 1
), 0)
```

これはMySQL内部ではrideごとのINDEX lookupです。しかし、RustからSQLをM回送る方法と
異なり、client/server間の往復は一覧SQLの1回だけです。

### current stateと履歴

`ride_statuses` は状態遷移の履歴です。`rides.evaluation` は評価完了後にだけ値が入る
現在状態です。

履歴から完了を判定するには、rideごとに最新statusを復元する必要があります。現在状態の
`evaluation IS NOT NULL` なら、ridesの行だけで評価完了を判定できます。ただし、この
置換には「`COMPLETED` とevaluationが同じtransactionで確定する」という不変条件が
必要です。

### statement-consistent snapshot

InnoDBの通常のSELECTは、1つのstatementの途中で別transactionがcommitしても、同じ
読み取りviewに基づく結果を返します。

変更前は複数のSELECTを同じsnapshotで読むため明示transactionが必要でした。変更後は
関連情報を1つのSELECTへまとめたため、1 statementのsnapshotだけで整合したresponseを
作れます。明示的なread transactionを外しても、途中でchairだけ新旧が混ざるわけでは
ありません。

## 変更前に何を確認したか

### OpenAPI

`webapp/openapi.yaml` では、このendpointを「ユーザーが完了済みのライド一覧を取得する」
ものと定義しています。各要素には次が必要です。

- ride ID
- 乗車地点と目的地
- 割引後料金
- chair ID、名前、model、owner名
- 評価
- 要求時刻と完了時刻

変更前のRust実装は `rides.created_at DESC` で並べ、最新statusが `COMPLETED` のrideだけを
返していました。この列、意味、並び順を変更しないことを正当性条件にしました。

### writerの原子性

評価handlerは、決済成功後の完了transactionで次を行います。

```text
ride rowをFOR UPDATEで再確認
COMPLETED statusをINSERT
chair_statsを更新
rides.evaluationとrides.updated_atをUPDATE
COMMIT
```

`COMPLETED` とevaluationは同じcommitで外部へ見えるため、正常なデータでは次が成立します。

```text
COMPLETEDが存在する
    ⇔ rides.evaluation IS NOT NULL
```

終了DBでも両方向の不一致をSQLで確認しました。

| 確認 | 件数 |
| --- | ---: |
| `COMPLETED` あり、evaluationなし | 0 |
| evaluationあり、`COMPLETED` なし | 0 |
| 同じ `used_by` を持つcouponが複数 | 0 group |

この確認だけで将来のwriterを保証することはできません。そのため、status writerを追加・
変更するときは、ride row lock、evaluation再確認、完了後に状態を戻さない条件を
回帰確認する必要があります。

### 変更前のprepared statement

SQLxはMySQLのprepared statement protocolを使います。この構成では
`events_statements_summary_by_digest` が本文を `statement/com/Execute` にまとめるため、
`performance_schema.prepared_statements_instances` をSQL本文で集計しました。

直前の60秒run終了時snapshotは次でした。

| SQL | 実行回数 | 累積 | 平均 | examined | sent |
| --- | ---: | ---: | ---: | ---: | ---: |
| ride別の最新status | 45,333 | 9.780秒 | 0.216ms | 45,379 | 45,333 |
| `coupons.used_by` | 39,790 | 3.762秒 | 0.095ms | 29,516 | 29,516 |
| chair主キー | 31,735 | 2.201秒 | 0.069ms | 31,735 | 31,735 |
| user別ride一覧 | 10,048 | 1.321秒 | 0.131ms | 13,527 | 13,527 |
| owner主キー | 13,527 | 0.761秒 | 0.056ms | 13,527 | 13,527 |

status、coupon、chair queryは他endpointも使うため、全回数を
`app_get_rides` だけの負荷とは扱いません。一方、source上で一覧handlerがrideごとに
これらを直列実行することと、一覧の親queryが10,048回動いたことから、N+1を優先して
検証する根拠にはなります。

`prepared_statements_instances` は終了したconnectionの行を失うため、厳密な全期間counter
ではありません。絶対件数ではなく、queryの形、倍率、累積時間を調べる終了時snapshotです。

## 仮説

`app_get_rides` を1 statementにすれば、関連rowのINDEX lookup自体は残っても、
RustとMySQL間の逐次往復、各queryのdecode、明示transactionのcommitを削減できます。

このendpointは診断runで約1.2万回呼ばれるため、1requestあたりの小さな削減でも、
共有poolを使うcoordinate、通知、評価の待ちを間接的に減らせると考えました。

反証条件は次です。

- response JSON、料金、順序が変わる
- `pass=false` またはerrorが出る
- query単体が重くなり、通常3走の中央値が明確に悪化する
- 新しいINDEXのwrite amplificationが読み取り削減を上回る

## 実装

### 1回のSQLへ集約

主要部分は次です。

```sql
SELECT
  rides.id,
  rides.pickup_latitude,
  rides.pickup_longitude,
  rides.destination_latitude,
  rides.destination_longitude,
  rides.evaluation,
  rides.created_at AS requested_at,
  rides.updated_at AS completed_at,
  chairs.id AS chair_id,
  chairs.name AS chair_name,
  chairs.model AS chair_model,
  owners.name AS owner_name,
  COALESCE((
    SELECT coupons.discount
    FROM coupons
    WHERE coupons.used_by = rides.id
    LIMIT 1
  ), 0) AS discount
FROM rides
INNER JOIN chairs ON chairs.id = rides.chair_id
INNER JOIN owners ON owners.id = chairs.owner_id
WHERE rides.user_id = ?
  AND rides.evaluation IS NOT NULL
ORDER BY rides.created_at DESC
```

### なぜ `SELECT *` を使わないか

一覧responseに不要なaccess token、ownerの登録token、各tableの更新時刻をMySQLから
転送する必要はありません。列を明示すると次が明確になります。

- networkへ送る列
- SQLxがdecodeする列
- `AppRideRow` が表すqueryの契約
- 将来covering INDEXを検討するときの必要列

tableへ列が追加されても、このqueryのdecode対象は変わりません。

### query専用の `AppRideRow`

DB schema全体を表す `Ride`、`Chair`、`Owner` を順番に組み立てず、JOIN結果を表す
`AppRideRow` を定義しました。

```text
DB row
  -> AppRideRow
  -> GetAppRidesResponseItem
  -> JSON
```

永続化modelとAPI用のprojectionを分けることで、nullableなDB列をresponseの必須列へ変換する
地点と、時刻をミリ秒へ変換する地点が1か所になります。

### 料金計算を純粋関数へ分離

変更前の割引ロジックはDB検索と料金計算が同じasync関数にありました。計算部分を
`calculate_fare_with_discount` へ分けました。

```text
走行料金 = Manhattan距離 × 100
割引後の走行料金 = max(走行料金 - 割引額, 0)
最終料金 = 500 + 割引後の走行料金
```

割引は初乗り料金500円を減らしません。割引額が走行料金を上回っても最終料金は500円です。
純粋関数ならDB fixtureなしで境界値をテストでき、履歴一覧と決済準備が同じ式を使えます。

### couponを通常のLEFT JOINにしなかった理由

`coupons.used_by` には通常INDEXがありますが、UNIQUE制約ではありません。単純な
`LEFT JOIN coupons` は、壊れたデータで同じrideにcouponが2行あればride自体を2行へ
増やします。

scalar subqueryへ `LIMIT 1` を付けることで、変更前の `fetch_optional` と同じく、
responseのride cardinalityを増やしません。

### 明示read transactionを外した理由

変更前は親ride、status、coupon、chair、ownerを複数statementで読むため、
同じsnapshotを保つ明示transactionに意味がありました。変更後は1 statementです。

明示transactionを外すと、次がなくなります。

- `BEGIN` のprotocol往復
- row loop中のconnection占有
- readだけの `COMMIT` とその待ち

InnoDB内部でSELECTがtransactionと無関係になるわけではありません。ここで削除したのは
アプリが複数statementを束ねる明示的なtransaction境界です。

## INDEXの仕組みと今回の判断

### ridesの複合INDEX

既存schemaには次があります。

```sql
INDEX idx_rides_user_created_at (user_id, created_at)
```

B-treeの複合INDEXは左端から並びます。まず `user_id = ?` で利用者の範囲へ移動し、
その範囲を `created_at` の逆順に走査できます。

```text
user A: old -> new
user B: old -> new
                 ↑ user Aの範囲だけを逆向きに読む
```

`EXPLAIN ANALYZE` でも `idx_rides_user_created_at` のreverse index lookupを確認しました。
10 ride中、evaluationがある9 rideを返す例は次でした。

```text
Index lookup on rides using idx_rides_user_created_at (reverse)
Filter: rides.evaluation is not null
Single-row index lookup on chairs using PRIMARY
Single-row index lookup on owners using PRIMARY
Index lookup on coupons using idx_coupons_used_by
```

実測点では9行を約1.18msで返しました。変更前の親ride SELECTだけは10行約0.16msなので、
新SQL単体が親SELECTより速いわけではありません。新SQLは、その後にRustから発行していた
関連queryと往復を含んだ結果です。

### なぜ新しいINDEXを追加しなかったか

候補として `(user_id, evaluation, created_at)` が考えられます。しかしevaluationへ
`IS NOT NULL` の範囲条件を置くと、その後のcreated_at順をそのまま利用しにくくなります。

`(user_id, created_at, evaluation)` は並び順を使えますが、evaluationは末尾なので、
現在のINDEXと同じく利用者の範囲を読んでからfilterします。今回の終了DBでは利用者あたり
最大10 rideであり、追加INDEXの効果は小さい一方、ride INSERTと評価UPDATEのたびに
別B-treeも更新します。

responseに必要な座標、chair ID、evaluation、時刻をすべて含むcovering INDEXは幅が広く、
write amplificationとbuffer pool使用量を増やします。現行INDEXで対象が最大10行まで
絞れるため、まず既存INDEXを再利用しました。

INDEXは「検索列をすべて追加する」ものではありません。次を同時に比較します。

1. 先頭列で何行まで絞れるか
2. ORDER BYをINDEX順で満たせるか
3. table rowへのlookupを何回減らせるか
4. INSERT / UPDATE時に増えるB-tree更新
5. INDEXが占有するmemoryとstorage

## 正当性検証

### 同じ終了DBで変更前後JSONを比較

変更前コンテナで履歴9件の利用者を選び、responseを保存しました。DBを初期化せず、
webappだけを変更後binaryへ差し替えて同じ利用者を再取得しました。

```text
変更前 SHA-256:
73deb03240a6ff0d4d7db42143405b499b6604b9e19838b10993ad637aa9b6fb

変更後 SHA-256:
73deb03240a6ff0d4d7db42143405b499b6604b9e19838b10993ad637aa9b6fb
```

byte単位で一致したため、JSONの配列順、料金、時刻、chair / owner情報も同じです。

### 固定HTTP fixture

`scripts/test-app-rides-batch.sh` は次を固定します。

- 完了ride 3件と未完了ride 1件
- 作成時刻と完了時刻のmicrosecond
- couponなしの通常料金
- 通常の200円割引
- 走行料金を上回る5,000円割引
- 日本語のowner、chair、model

確認内容は次です。

- 未完了rideを返さない
- 完了rideを作成時刻の降順で返す
- couponがないrideを割引0円として返す
- 距離5、割引200円を800円として返す
- 距離2、割引5,000円でも初乗り料金500円を返す
- `DATETIME(6)` をUNIX millisecondへ変換するときmicrosecond以下を切り捨てる
- UTF-8文字列を壊さない
- 公式初期データのstatus / evaluation不一致とcoupon重複が0件である
- 終了時に `POST /api/initialize` で公式初期データへ戻す

結果は次です。

```text
app rides batch regression: PASS
```

### Rust検証

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

54 testが成功しました。

## 診断run

診断付き60秒runは `pass=true`、138,231点、error map空でした。

### endpoint

`GET /api/app/rides` は12,100回でした。

| 件数 | 平均 | p50 | p95 | p99 | 最大 | 2xx |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 12,100 | 96ms | 8ms | 373ms | 529ms | 778ms | 12,100 |

新SQLの負荷中snapshotは次です。

| 実行回数 | 累積 | 平均 | 最大 | examined | sent |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 11,956 | 3.855秒 | 0.322ms | 34.972ms | 67,958 | 17,451 |

1回あたり約5.7行を調べ、約1.46行を返しています。SQL平均0.322msに対してHTTP p95が
373msなので、このendpointの長いtailはquery実行時間だけでは説明できません。
general admission、pool取得、共有CPU、response送信の待ちを含みます。

`prepared_statements_instances` の11,956回とnginxの12,100回が一致しないのは、
前者が終了済みconnectionのstatementを失うsnapshotだからです。

旧owner個別queryは候補runの集計から消えました。旧status、coupon、chair query自体は
通知、ride作成、評価など他endpointも使うため、システム全体では残ります。

## 通常60秒の結果

Colimaは全runで4 CPU / 4 GiBのままです。公式Goベンチマーカー、Rust、MySQL、nginx、
matcherは同じDockerホストを共有します。

| run | pass | score | error | matching不満 | pickup不満 | drive不満 |
| ---: | --- | ---: | --- | ---: | ---: | ---: |
| 1 | true | 128,584 | 空 | 55.8% | 34.2% | 63.0% |
| 2 | true | 124,205 | 空 | 58.6% | 34.6% | 61.0% |
| 3 | true | 133,737 | 空 | 52.9% | 31.6% | 62.8% |
| 推定代表値 | - | 128,584 | - | 55.8% | 34.2% | 62.8% |

直前の同期実装は124,230 / 127,499 / 128,061点、中央値127,499点でした。

```text
推定差 = 128,584 - 127,499 = +1,085
推定改善率 = 1,085 / 127,499 ≒ +0.85%
```

今回の観測範囲は9,532点あり、中央値差1,085点より大きいので、+0.85%を精密な将来値とは
扱いません。matching不満の中央値は64.2%から55.8%へ下がりましたが、pickupとdriveは
悪化しており、この変化を履歴endpointだけへ因果帰属することもできません。

採用理由は、scoreが明確に悪化せず、source上とprepared statement統計の両方でN+1の
仕事量削減を確認し、正当性fixtureが通ったためです。

このrunでは完了ride数、空車距離、乗車距離のscore内訳を独立保存していません。
score増加を特定の内訳へ推定配分せず、TODOへ計測不足を残します。

## 確認したログと判断の対応

| ログ・証拠 | 確認したこと | 判断 |
| --- | --- | --- |
| OpenAPI | 完了ride、必須field、料金、時刻 | response契約を固定 |
| Rust source | 1 + 4N形の逐次query | N+1を施策対象に選択 |
| schema | 主キーと既存複合INDEX | INDEX追加なしで実装 |
| 不一致確認SQL | status / evaluation / coupon cardinality | current state利用の前提を検証 |
| `EXPLAIN ANALYZE` | user INDEX、reverse scan、PK lookup | 全表走査・filesortなし |
| prepared statement | query回数、累積、examined / sent | Rust-MySQL往復削減を確認 |
| nginx timing log | endpoint件数とtail latency | 残る待ちはSQL単体以外と判断 |
| 旧新JSON hash | byte単位のresponse一致 | 既存終了DBでの回帰なし |
| 固定fixture | 未完了除外、順序、割引、時刻、UTF-8 | 境界値の正当性を確認 |
| 60秒3走 | pass、score、error、不満率 | 採用時の全体影響を確認 |

## 検討した他の選択肢

### Rust側でqueryを並行実行する

`join_all` や多数の `tokio::spawn` でN個のqueryを同時に実行しても、SQL総数は減りません。
共有poolが飽和している現在の構成では、他endpointのconnection取得待ちを増やします。

### responseを利用者単位でcacheする

完了履歴は変化頻度が低いため候補になります。ただし評価完了時のinvalidaton、initialize世代、
動的利用者、複数processの整合性が必要です。まずDB仕事量を1 SQLへ減らす変更を行い、
cacheはhit率と無効化条件を独立して検証します。

### couponを別のbatch queryで取得する

完了ride IDを集めて `WHERE used_by IN (...)` とする2 query案です。scalar subqueryの
INDEX lookupが支配的になった場合は有効ですが、現在はqueryが1回増え、動的placeholderの
組み立ても必要です。診断では新SQL全体が平均0.322msなので優先しません。

### `coupons.used_by` をUNIQUEにしてLEFT JOINする

意味として1 ride 1 couponは自然です。しかしUNIQUE制約追加は性能変更だけでなく、
既存データと並行writerの許容範囲を変えます。制約追加は全初期データと同時ride作成の
検証を独立して行うべきです。

### JSONをMySQLで組み立てる

`JSON_ARRAYAGG` ならRustのrow mappingを減らせますが、MySQL CPUへserializationを移し、
数値、時刻、空配列、順序の扱いも変わります。現在はMySQLが共有資源の中心なので、
明確なprofileなしには選びません。

### 完了ride専用tableを作る

読み取りは単純になりますが、評価transactionのwrite amplification、起動時backfill、
initialize、repair、二重書込みの不変条件が増えます。利用者あたり最大10 rideの現状では
過剰です。

## 残る課題

1. endpoint内をgeneral admission、pool acquire、SQL、row mapping、JSON送信へphase分解する
2. 30ms超過率を平均・p95 / p99とは別に保存する
3. 完了ride数、空車距離、乗車距離のscore寄与をrunごとに保存する
4. 将来status writerを変更したときに、evaluationとの同値性をCIで再確認する
5. owner / chair参照の外部キーがないため、孤児rowの監査を初期化後と走行後に継続する
6. 履歴が現在の最大10件を大きく超えた場合、INDEXとresponse sizeを再評価する

次の優先対象は、`app_post_rides` が利用者の全rideとride別statusを読み直すN+1です。
一覧endpointと異なり、こちらは並行ride作成を拒否する正当性条件があるため、current-state
への置換前に同時requestのfixtureが必要です。
