# Benchmark 20: chair statsを完了時に差分更新する

## 結論

`GET /api/app/notification` が返すchairの完了ライド数と平均評価を、通知pollingの
たびに全履歴から集計する処理をやめました。`chair_stats` という1 chair 1 rowの
集計表を追加し、評価確定と同じtransactionで件数と評価合計を差分更新します。

正当性とSQL時間は改善しました。

- 公式prevalidation: `pass=true`
- 初期データの全500 chairで旧履歴集計との差: 0件
- 60秒走行終了時の動的chairを含む差: 0件
- stats readの累積時間: 22.299秒から2.477秒
- stats readの平均時間: 0.476msから0.063ms
- 評価時のstats更新: 868回、累積0.119秒、平均0.137ms

一方、完了条件と再起動repairを厳密化した最終3走の中央値は、変更前101,984点から
98,452点へ約3.5%低下しました。3走ともエラー0で正当性は上がりましたが、この変更
だけでスコアが改善したとは判断しません。通知の最新status・ride・fareの重複readを
次の施策で除くためのcurrent-state基盤として採用し、組み合わせ後も再評価します。

## なぜこの項目を優先したか

Benchmark 19後の60秒診断runで、MySQLの
`performance_schema.prepared_statements_instances` をSQL本文ごとに集約しました。
上位は次のとおりでした。

| SQL | 回数 | 累積 | 平均 |
|---|---:|---:|---:|
| `chair_current_locations` UPDATE | 46,437 | 32.040秒 | 0.690ms |
| chair stats履歴集計 | 46,876 | 22.299秒 | 0.476ms |
| 最新status取得 | 119,911 | 14.600秒 | 0.122ms |
| app未送信status取得 | 71,862 | 10.284秒 | 0.143ms |
| chair未送信status取得 | 50,946 | 7.206秒 | 0.141ms |

座標current rowの更新が単独では最大でしたが、通知関連を合計するとそれを上回ります。
statsは「完了したときだけ変わる値」を30ms pollingのたびに再集計していたため、
更新頻度と参照頻度の差が特に大きい箇所でした。また、評価確定transactionへ
差分更新を追加すれば履歴との整合性を明確に保てるため、先に切り出しました。

## はじめに知っておく用語

### 集計と事前集計

集計は、多数の明細行から `COUNT`、`SUM`、`AVG` などを計算する処理です。旧実装は
通知が来るたびに `rides` と `ride_statuses` をJOINし、完了条件を確認してから
件数と平均を求めていました。

事前集計は、値が変わる時点で集計結果も更新し、read時には結果だけを読む方法です。
今回の件数と評価合計は評価確定時にしか増えません。readが数万回、writeが千回弱
という比率では、writeを少し増やしてreadを小さくする効果が期待できます。

### read amplification

利用者が欲しいのは「件数」と「平均」の2値だけなのに、そのたびに複数tableの
多数行を読む現象です。必要な出力に対して読み込む行が増えるほどread amplificationが
大きいと考えます。

旧SQLは1 chairのライドごとに6 status前後を読み、temporary tableを作って
`GROUP BY` / `HAVING` しました。chairの利用回数が増えるほど1 pollの仕事も増えます。

### write amplification

readを減らす代わりに、評価確定時の書き込みが1回増えます。これをwrite amplification
と呼びます。今回は868回、累積0.119秒だったため、削減したread累積時間に比べて
小さいことを確認しました。ただし、書き込みはrow lockとtransaction commitにも
影響するため、SQL単体時間だけで無条件に安全とは判断しません。

### current-state table

履歴tableは過去の全eventを保持します。current-state tableは、現在必要な値だけを
entityごとに1行で保持します。

`chair_stats` はchair IDを主キーにして、次だけを保存します。

```text
chair_id -> 完了ライド数、評価合計
```

平均値そのものは保存しません。平均を毎回丸めて保存すると丸め誤差が積み重なるため、
整数の合計と件数からread時に割り算します。

### atomicity

複数の更新が「全部成功するか、全部失敗するか」を保証する性質です。今回、
`rides.evaluation`、`COMPLETED` status、`chair_stats` は同じtransactionで更新します。
決済処理など後続処理が失敗してrollbackされれば、3つとも戻ります。

statsだけ先にprocess内cacheへ反映すると、DBがrollbackしてもcacheだけ増える危険が
あります。同じ理由で、commit後に非同期更新する方式も採りませんでした。

### backfill

新しいcurrent-state tableを追加したとき、既存の履歴から初期値を作る処理です。
初期データにはすでに完了済みrideがあるため、空tableから開始すると最初の通知から
値がずれます。

今回はinitializeの最後に履歴を1回集計します。process再起動時はtransaction内で
既存のprojectionを全削除し、履歴から再構築します。これにより旧versionのDocker
volumeにtableがない場合だけでなく、欠損、誤値、履歴に存在しない余分なrowも修復します。
InnoDBではcommitまで他connectionから旧commit済み状態が見えるため、`DELETE` と
`INSERT` の途中に空の集計を公開しません。

## 変更前の処理

旧実装はapp通知のたびに次の形のSQLを実行していました。

```sql
SELECT COUNT(*),
       AVG(completed_rides.evaluation)
FROM (
  SELECT rides.id, rides.evaluation
  FROM rides
  INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
  WHERE rides.chair_id = ?
    AND rides.evaluation IS NOT NULL
  GROUP BY rides.id, rides.evaluation
  HAVING ARRIVEDが存在
     AND CARRYINGが存在
     AND COMPLETEDが存在
) AS completed_rides;
```

単に `evaluation IS NOT NULL` だけを数えない理由は、初期データや故障時に一部statusが
欠けたrideを完了扱いしないという既存仕様を維持するためです。

利用回数が多いchairでの `EXPLAIN ANALYZE` は次でした。

- `rides(chair_id)` のindex lookup: 19行
- `ride_statuses(ride_id, status)` のlookup: 18 loops、108行
- temporary tableによる集約
- 完了ride: 18行
- 実測: 約1.19ms

これは単発では短く見えます。しかし60秒で46,876回呼ばれるため、累積22.299秒に
なりました。性能調査では「1回の遅いSQL」だけでなく「短いSQLの回数×時間」も
確認する必要があります。

## 仮説

仮説は次でした。

> statsは完了時にしか変わらない。完了時に件数と評価合計を1 rowへ加算すれば、
> 30ms pollingは主キー1行だけを読み、JOIN・GROUP BY・temporary tableを除去できる。

採用条件は次のように置きました。

1. 初期全chairで旧集計と一致する
2. `COMPLETED` 通知では今回確定した評価を含む
3. 走行中は同じrideの各通知でstatsが変わらない
4. 決済失敗時にstatsだけ増えない
5. process再起動とinitialize後に履歴から復元できる
6. 60秒ベンチが `pass=true`
7. read削減が追加writeを上回る

## 実装

### schema

```sql
CREATE TABLE chair_stats (
  chair_id             VARCHAR(26) NOT NULL,
  total_rides_count    INTEGER     NOT NULL,
  total_evaluation_sum BIGINT      NOT NULL,
  PRIMARY KEY (chair_id)
);
```

`chair_id` を主キーにした理由は、通知がchair IDを完全一致で指定するためです。
B-tree主キーではrootから対象leafまでを辿り、該当する1行だけを取得できます。
範囲検索や並び替えはないので、別の複合INDEXは不要です。

評価合計は長時間の加算余地を持たせるため `BIGINT` にしました。評価1〜5だけなら
当面 `INTEGER` でも足りますが、件数より先に合計が上限へ達するためです。

### initialize時のbackfill

旧SQLと同じ完了条件を使い、rideごとの重複statusを `GROUP BY` で1件へまとめてから
chair単位に集計します。`ARRIVED` / `CARRYING` / `COMPLETED` のどれかが欠けるrideは
含めません。

通常の通知pollではこの重い集計を実行せず、initializeと旧volumeからの起動時だけに
限定します。

### 評価確定時の差分更新

```sql
INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
)
SELECT ?, 1, ?
WHERE EXISTS (
  SELECT 1
  FROM ride_statuses
  WHERE ride_id = ?
    AND status = 'CARRYING'
)
ON DUPLICATE KEY UPDATE
  total_rides_count = total_rides_count + 1,
  total_evaluation_sum = total_evaluation_sum + VALUES(total_evaluation_sum);
```

最初の完了rideならINSERT、既存rowがあれば同じ主キーrowへ加算します。このSQLは
`rides.evaluation` と `COMPLETED` statusの更新と同じtransaction内です。

評価APIが受け付ける直前状態は `ARRIVED` なので、`ARRIVED` の存在はそこで確認済みです。
同じtransactionで `COMPLETED` を追加し、上の `EXISTS` で `CARRYING` の存在も確認します。
これにより、差分更新とbackfillがどちらも「3 statusがすべて存在するrideだけ」という
同じ完了条件を使います。履歴から復元できない状態をcurrent-stateだけへ書かないことが
重要です。

同じrideを二重評価すると二重加算になるため、既存のride row `FOR UPDATE` と
「最新statusがARRIVEDであること」の検査を維持しています。最初の評価がcommitすると
最新statusはCOMPLETEDになり、2回目は差分更新へ到達しません。

### 通知時のread

通知側は `chair_stats.chair_id = ?` の1行から件数と合計を読みます。rowがなければ
完了数0、平均0として返します。新規chairは完了するまでstats rowを持たなくても
正しく扱えます。

新SQLの `EXPLAIN ANALYZE` は定数の主キーlookupとして実行前に1行取得され、
履歴JOIN、temporary table、filesortはありませんでした。

## 正当性検証

### 全初期chairの旧新比較

```sh
./scripts/test-chair-stats-consistency.sh
```

scriptはinitialize後、500 chairすべてについて次を比較します。

- 旧履歴SQLから求めた完了ライド数
- 旧履歴SQLから求めた評価合計
- `chair_stats` の完了ライド数
- `chair_stats` の評価合計

さらに次の故障を注入してwebappを再起動し、同じ比較を繰り返します。

- 集計rowを1件削除
- 別の集計rowを誤った大きな値へ変更
- chairに対応しない余分な集計rowを追加
- `CARRYING` が欠けた評価済みrideを追加

再起動後は差分0件、余分なrowも0件でした。失敗・signal終了時もtrapでinitializeを
行い、fixtureや変更中の状態を残さないようにしています。

### 評価transactionの回帰検証

```sh
./scripts/test-chair-stats-transitions.sh
```

成功と失敗を切り替えられる一時的な決済HTTP serverをCompose network内だけに起動し、
実際の評価APIで次を確認しました。

- `CARRYING` 欠損rideは評価を完了してもstatsへ含めない
- 通常rideは完了時に件数を1、評価合計を送信値だけ増やす
- 同じrideの再送はHTTP 400となり二重加算しない
- 決済失敗はHTTP 502となり、evaluation、`COMPLETED`、statsをすべてrollbackする

結果はすべて成功でした。一時containerとfixtureはtrapで削除・初期化します。

### 公式prevalidation

```text
pass=true
score=0
error map={}
```

prevalidationは新しいrideを `ARRIVED` まで進め、評価5を送信し、その後の
`COMPLETED` 通知で次を検査します。

- `total_rides_count = 1`
- `total_evaluation_avg = 5.0`

したがって、初期backfillだけでなく評価transactionの差分更新と通知readまで通っています。

### 既存回帰テスト

- status通知: `MATCHING -> ENROUTE -> PICKUP -> CARRYING`
- app / chairの最新status fallback: `CARRYING`
- 座標による `CARRYING -> ARRIVED`
- smoke test
- 60秒ベンチ6走: 全run `pass=true`
- 最終run終了時の旧履歴集計との差: 0件

## SQL計測

代表的な変更前runと変更後runを比較します。回数はrunの完了数で変わるため、
平均時間と累積時間の両方を見ます。

| 項目 | 変更前 | 変更後 |
|---|---:|---:|
| stats read回数 | 46,876 | 39,326 |
| stats read累積 | 22.299秒 | 2.477秒 |
| stats read平均 | 0.476ms | 0.063ms |
| rows examined | 163,108 | 78,652 |
| stats write回数 | 0 | 868 |
| stats write累積 | 0 | 0.119秒 |
| stats write平均 | - | 0.137ms |

readの累積は約88.9%減りました。新readのrows examinedが2行/回なのは、stats rowが
ないchairも0として1行返すための定数seedとLEFT JOINを使っているためです。
履歴行数には比例しません。

## 60秒ベンチ

条件はApple Silicon macOS、Colima 4 CPU / 4 GiB、同一Dockerホスト、
公式ベンチマーカー、静的ファイル検証ありです。CPU / memoryは変更していません。

### 変更前の直近通常3走

| run | pass | スコア | error map |
|---:|---:|---:|---|
| 1 | true | 101,984 | `30:1` |
| 2 | true | 102,498 | `30:1` |
| 3 | true | 98,444 | 空 |

- 観測範囲: 98,444–102,498点
- 推定代表値: 中央値101,984点

### 完了条件とrepairを厳密化した最終3走

| run | pass | スコア | error map |
|---:|---:|---:|---|
| 1 | true | 98,386 | 空 |
| 2 | true | 98,452 | 空 |
| 3 | true | 99,944 | 空 |

- 観測範囲: 98,386–99,944点
- 推定代表値: 中央値98,452点
- 変更前中央値との差: -3,532点、約-3.5%
- 全run: `pass=true`、error mapは空
- 最終run終了時: 811 chair、履歴との差0件、孤立stats 0件

レビュー前の暫定実装は99,318 / 107,042 / 95,880点、中央値99,318点でした。しかし、
この版は `CARRYING` 欠損rideの差分更新とbackfillが一致せず、再起動でも孤立rowを
除去できなかったため、採否の代表値には使いません。問題を修正した最終3走だけを
現在値とします。

暫定runで観測した `CODE=17` は座標送信失敗、`CODE=30` は評価直後のnearby警告です。
最終3走ではどちらも0件でしたが、再発し得るsoft errorとして今後も追跡します。

## 判断

この変更単体のスコア中央値は改善していません。しかし、次の理由でrevertせず
次の通知施策へ進みます。

- 正当性検証で旧集計との差が0
- 最も多い通知集計の累積時間を約89%削減
- 追加writeは累積0.119秒
- 最終3走はすべてエラー0
- 最新statusとpayloadのcurrent-state化に必要な更新点が明確になった

ただし「SQLが速くなったのでスコアも改善した」とは記録しません。現在のスコアは
matcher間隔、移動tick、通知遅延、決済など他の待ちにも左右されます。次の施策後も
中央値が戻らなければ、stats writeを含む評価transactionのlock / commit時間を測り、
この変更もrevert候補として再評価します。

## 他の選択肢

### process内cache

readは最も速くできますが、commit失敗、process再起動、複数process、initializeとの
同期が必要です。statsは厳密一致を検査されるため、まずDB current-stateを正としました。

### 平均値を直接保存

`new_avg = (old_avg * count + evaluation) / (count + 1)` でも更新できます。ただし
浮動小数点の丸めが完了ごとに積み重なります。整数の評価合計を保存する方が再構築と
照合が容易です。

### generated column

件数と合計から平均をgenerated columnにできますが、通知readの単純な除算は十分短く、
schemaの責務を増やす利益がありません。

### trigger

`rides.evaluation` 更新をtriggerで検知する方法です。全writerへ適用できますが、
statusが3種類揃ったことの確認や二重更新の扱いがDBへ隠れ、調査が難しくなります。
このrepositoryでは評価handlerのtransactionへ明示しました。

### 毎秒cache

最大1秒古いstatsを許せば簡単ですが、benchmarkerは同じrideの通知中は値が固定され、
`COMPLETED` では今回の評価を含むことを検査します。時間依存のcacheは採りません。

## 次のTODO

1. app / chair通知のlatest statusを1 ride 1 rowのcurrent-stateへ移す
2. 未送信statusがないpollで履歴status lookupをなくす
3. ride、fare、chair/user payloadの再構築をstatus version単位で再利用する
4. 同一recipientの並行pollでも未送信statusを二重claimしない条件付きUPDATEを検証する
5. notification SQL数、cache hit率、status反映遅延を3走で比較する
