# Benchmark 21: 通知statusの1 SQL化は不採用

## 結論

app / chair通知で行っていた「未送信status検索」と「未送信がない場合の最新status検索」を、
CTEと `UNION ALL` を使う1 SQLへまとめました。しかし、60秒runでは
`pass=true`、94,573点だったものの、対象SQLの累積実行時間が変更前の約32秒から
約54秒へ増えました。

SQLの呼出回数は減りましたが、MySQLが1回のqueryで調べる行と一時的な集合処理が増え、
4 CPUを全containerで共有する現在の環境では逆効果でした。実装は元へ戻し、この記録と
TODOだけを残します。1走だけなので、この案の代表スコアは推定しません。

## 確認した処理

変更前の通知は、最初に次のようなqueryで状態遷移順の最初の未送信statusを調べます。

```sql
SELECT *
FROM ride_statuses
WHERE ride_id = ?
  AND app_sent_at IS NULL
ORDER BY status ASC
LIMIT 1;
```

未送信行がなければ、次に最新statusを調べます。

```sql
SELECT status
FROM ride_statuses
WHERE ride_id = ?
ORDER BY status DESC
LIMIT 1;
```

chair側は `app_sent_at` が `chair_sent_at` になる以外は同じ構造です。
2 SQLになるのは「すべて送信済み」のpollです。変化がないときも30msごとにpollするため、
network round tripを1回減らせるという仮説を立てました。

## 試したSQL

検証版は、最初の未送信行をCTEへ置き、未送信行がなければ最新status側だけを候補へ
加えました。概略は次のとおりです。

```sql
WITH unsent AS (
  SELECT id,
         status,
         status + 0 AS status_rank
  FROM ride_statuses
  WHERE ride_id = ?
    AND app_sent_at IS NULL
  ORDER BY status ASC
  LIMIT 1
)
SELECT id, status, is_unsent
FROM (
  SELECT id, status, status_rank, TRUE AS is_unsent
  FROM unsent
  UNION ALL
  SELECT id, status, status + 0, FALSE AS is_unsent
  FROM ride_statuses
  WHERE ride_id = ?
    AND NOT EXISTS (SELECT 1 FROM unsent)
) AS candidates
ORDER BY is_unsent DESC, status_rank DESC
LIMIT 1;
```

### CTEとは

CTE（Common Table Expression）は、`WITH 名前 AS (...)` でquery内の中間結果へ名前を
付ける仕組みです。ここでは未送信行を `unsent` として、候補の選択と
`NOT EXISTS` の両方から参照しました。読みやすさには役立ちますが、単純な2本の
INDEX検索より常に速いわけではありません。実行計画によっては中間結果の扱い、
候補集合の作成、並べ替えが追加されます。

### `UNION ALL`とは

`UNION ALL` は2つの結果を重複排除せず連結します。`UNION` の重複排除は不要なので
避けましたが、連結後の候補へ `ORDER BY` と `LIMIT` を適用する処理は残ります。

### `status + 0` とENUM順

`ride_statuses.status` は次のENUMです。

```text
MATCHING < ENROUTE < PICKUP < CARRYING < ARRIVED < COMPLETED
```

MySQLのENUMは定義順に内部番号を持ちます。`status + 0` はその番号を明示的に取り出す
式です。`UNION ALL` の結果で型が文字列寄りに扱われても、`CARRYING` と `PICKUP` を
辞書順で誤って並べないために使いました。

## 変更前の計測

Benchmark 20終了時の `performance_schema.prepared_statements_instances` では、
関連queryは次の値でした。

| SQL | calls | 累積 | 平均 |
|---|---:|---:|---:|
| app未送信status | 71,862 | 10.284秒 | 0.143ms |
| chair未送信status | 50,946 | 7.206秒 | 0.141ms |
| 最新status | 119,911 | 約14.6秒 | 約0.122ms |

最新statusには通知以外のhandlerからの呼出しも含まれます。したがって約32秒は通知だけの
厳密な値ではありませんが、同じprepared statementを含むhot path全体の上限寄りの
比較値として使えます。

単発の `EXPLAIN ANALYZE` では、選んだCTE版は送信済みfixtureで約0.014–0.034ms、
未送信fixtureでは約0.0002msでした。この時点では有望に見えました。

比較の途中で試した次の案は単発でも遅かったため、60秒runへ進めていません。

| 案 | 単発の目安 | 不採用理由 |
|---|---:|---|
| 1 SELECT + `CASE` sort | 約0.105ms | 最大6行を毎回並べ替える |
| 条件なしの単純な `UNION ALL` | 約0.174ms | 未送信があっても最新側を評価する |

## 60秒runとSQL統計

固定条件はColima 4 CPU / 4 GiBで、ホストのCPU・memory設定は変更していません。

| run | pass | スコア | error map |
|---|---|---:|---|
| CTE版 run 1 | true | 94,573 | 空 |

終了直後のprepared statement snapshotは次のとおりです。

| SQL | calls | 累積 | 平均 | 最大 | rows examined |
|---|---:|---:|---:|---:|---:|
| app CTE版 | 53,896 | 30.365秒 | 0.563ms | 21.498ms | 201,418 |
| chair CTE版 | 40,710 | 23.391秒 | 0.575ms | 34.580ms | 173,311 |
| 他handlerに残る最新status | 8,511 | 1.593秒 | 0.187ms | 11.493ms | 8,520 |
| locking read版の最新status | 1,741 | 0.321秒 | 0.184ms | 8.109ms | 1,741 |

appとchairのCTE版だけで累積53.756秒です。SQL呼出しは大きく減りましたが、
1回あたりの平均が約0.56msとなり、対象2本だけで変更前の関連3本全体より重くなりました。
`rows examined / calls` もapp約3.74行、chair約4.26行です。

## 単発計測と負荷計測が逆になった理由

`EXPLAIN ANALYZE` は1回の実行経路を理解するために有効ですが、次の要素を同時には
再現しません。

- 多数connectionから30ms間隔で繰り返されるpoll
- MySQL、webapp、matcher、benchmarkが4 CPUを共有する競合
- CTEと候補集合を約9万回処理する累積CPU
- 状態数が1行から6行まで変化する実データ分布

network round tripの削減はwebappとMySQLが同一Docker networkにあるこの構成では小さく、
MySQL側の追加処理が勝ちました。「SQL数が少ない」ことと「総処理が軽い」ことは同じでは
ありません。採否には単発計画、累積時間、全体スコアの3つが必要です。

## 正当性の確認

性能不採用とは別に、CTE版が通知の意味を変えていないかは確認しました。

```text
./scripts/test-status-notification-order.sh
  app:   MATCHING -> ENROUTE -> PICKUP -> CARRYING
  chair: MATCHING -> ENROUTE -> PICKUP -> CARRYING
  送信済みfallback: app / chairともCARRYING

./scripts/benchmark.sh 0
  pass=true

./scripts/smoke-test.sh
  GET / = 200
  POST /api/initialize = {"language":"rust"}
```

正しくても遅い変更は採用しません。ソースを変更前へ戻したので、この記録には
再利用するためのRust実装は残していません。

## `data: null` をすぐ返さなかった理由

「未送信statusがなければ `data: null`」は、現在の仕様ではそのまま適用できません。
prevalidationでは利用者が `MATCHING` を一度受信したあと、matcherが
`rides.chair_id` を更新します。この割当では新しいstatus行を追加しません。その後の
同じ `MATCHING` 応答には、割り当てられたchair情報が含まれる必要があります。

つまり、status IDが変わらなくてもpayloadは変わります。

```text
MATCHING / chair未割当
  -> rides.chair_id更新（status追加なし）
  -> MATCHING / chair情報あり
```

未送信行の有無だけをversionとして使うと、2回目を `data: null` にして椅子情報を
取りこぼします。短絡するには、statusだけでなく `chair_id`、評価確定、
chair statsなどpayloadへ影響する値を含むversionまたは明示的なcache invalidationが
必要です。

## 次の選択肢

1. 2本の単純なcovering INDEX lookupは維持し、認証SQLなど独立した重複readを先に減らす
2. `ride_current_status` のような1 ride 1行のcurrent-state表を作り、最新statusを主キーで読む
3. recipientごとのpayload cacheへ、status IDだけでなくride割当versionも持たせる
4. JSON long pollingで状態変化まで待ち、変化がない30ms pollそのものを減らす
5. 同一recipientの並行pollに備え、送達claimを条件付きUPDATEへする

次はBenchmark 21の実装を残さず、60秒runで約13.9万回・累積約9.7秒だった認証SQLを
process cacheで削減できるか、initialize・動的登録・cache miss fallbackを含めて
単独検証します。
