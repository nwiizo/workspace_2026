# Benchmark 27: 座標更新のphase分解とCODE=17再診断

[チューニング目次へ戻る](../TUNING.md)

## 結論

`POST /api/chair/coordinate` の遅延を、64 requestに1件だけphase別に計測しました。
公式60秒ベンチは `pass=true`、117,989点でした。ただし、このrunは診断logと
Performance Schemaを有効にした実測 `n=1` です。通常スコアrunの推定代表値には混ぜません。

当初の仮説は「`chair_current_locations` の同一chair rowを更新するlock待ちが、
coordinateのtail latencyを支配する」でした。実測では次のようになりました。

- current-state writeはp95 4.185ms、平均1.633ms
- `pool.begin()` はp95 93.651ms、平均32.452ms
- handler全体はp95 105.296ms、平均40.089ms
- current-state UPDATE単体の全75,834回集計は平均0.812ms
- MySQLのrow lock待ちは2,914回、平均約16.6ms、最大157ms

row lock待ちは実在しますが、sampled handlerの主な待ちはcurrent-state writeではなく
`pool.begin()` の区間でした。したがって「current UPDATEを最初にqueue化すれば
tail latencyの大半が消える」という仮説は棄却します。

ただし `pool.begin()` は、SQLx poolからconnectionを借りる待ちとMySQLへ `BEGIN` を送る
時間をまとめて計測しています。終了時の `Max_used_connections=51` は、root診断接続を
除くと上限50のapplication poolを使い切ったことと整合しますが、次の診断では
`pool.acquire()` と `connection.begin()` を分離して因果を確定します。

同じrunで `CODE=17` が1件再現しました。今回はdeadlockではありません。

```text
2026-07-24T21:13:51Z
POST /api/app/users -> HTTP 500
MySQL 1062 (23000):
Duplicate entry 'Kulas4628' for key 'users.username'
```

同名の既存rowは同じrunの `2026-07-24 21:13:35.682592` に作られており、約16秒後に
benchmarkerが同じusernameをもう一度生成していました。InnoDBのdeadlock記録はなく、
Performance Schemaの `ER_DUP_ENTRY` も1件でした。過去の `CODE=17` で直した
`coupons(code)` の広いlockとは別原因です。

## 計測条件

| 項目 | 内容 |
|---|---|
| 日時 | 2026-07-25 JST |
| revision | Benchmark 26採用版 `f980ac0d` + 診断instrumentation |
| Colima | 4 CPU / 4 GiB。変更なし |
| 走行時間 | 60秒 |
| 診断 | nginx時刻log、coordinate 1/64 sampling、Performance Schema |
| score | 117,989 |
| pass | `true` |
| error map | `CODE=17` 1件 |
| 推定値 | 実測 `n=1` のため未推定 |

診断は通常処理へ時刻取得とJSON logを追加するため、最終採否用の通常runと分けます。
1回の高いscoreを「診断で高速化した」とは解釈しません。

DB metricの時間境界は次の順でした。

| boundary | UTC |
|---|---|
| 診断command直前 | 2026-07-24T21:12:39Z |
| MySQL process起動 | 2026-07-24T21:13:11Z |
| 最初のcoordinate sample | 2026-07-24T21:13:32.702054733Z |

`benchmark.sh` が既存DB processを停止し、このrun用に再起動したことを時刻順で確認しました。
したがってInnoDB metricは、この新しいprocessの起動からrun後のreportまでの累積です。
prepared statementはreport時点で生存するconnectionだけのlossy live snapshotであり、
終了したconnectionの実行は含みません。

## はじめに知っておく用語

### phase

phaseは、1つのhandlerを意味のある区間へ分けたものです。今回のcoordinateは次の順です。

```text
cache lookup
  -> pool.begin
  -> chair_locations INSERT
  -> chair_current_locations UPDATE / UPSERT
  -> current ride SELECT
  -> 必要な場合だけstatus遷移
  -> COMMIT
  -> process cache更新
```

handler全体が100msだったという情報だけでは、Rustの計算、pool待ち、SQL、lock、commitの
どこを修正すべきか分かりません。phaseごとのp95 / p99を比較すると、大きな待ちを含む
境界から先に追加計測できます。

### sampling

samplingは、すべてを記録せず一定割合だけを観測する方法です。今回はrequest sequenceが
64の倍数になる1件だけを記録し、75,834 requestから1,185 sampleを得ました。

全件へ `Instant::now()` とJSON serialization、stdout writeを加えると、計測自体が
hot pathを遅くします。samplingはoverheadを抑えますが、1,185件が全requestを完全に
表す保証はありません。そのため、SQL単体はPerformance Schemaの全実行集計でも補います。

今回の実装はglobal sequenceが64の倍数になるrequestを選ぶ固定周期samplingです。実装が
単純でsample比率を説明しやすい一方、benchmarker tickやchair送信順にも64の周期があると、
特定のbatch位置へ偏るaliasの可能性があります。今回の実測方法は後から変えずそのまま記録し、
後続診断でwrite pathやchair IDの偏りが見えた場合は、sequenceを軽量hashして選ぶ方式または
processごとのrandom offsetと比較します。

### connection pool

connection poolは、MySQL接続を毎request作り直さず再利用する仕組みです。上限50なら、
同時に50 transactionが接続を使っている間、51番目以降は空きが返るまで待ちます。

```text
request A ... connection 1を使用
request B ... connection 2を使用
...
request AX .. connection 50を使用
request AY .. pool内部の待ちqueue
```

MySQL queryが1msでも、接続取得に90ms待てばhandlerは90msを超えます。逆にpool上限を
無条件に増やすと、MySQLへ同時queryを押し込み、CPU、row lock、buffer pool、commit I/Oの
競合を悪化させる場合があります。まず取得待ちとDB実行時間を分離します。

### row lock待ち

InnoDBは同じrowを同時更新すると、先にlockを取ったtransactionのCOMMITまたはROLLBACKまで
後続を待たせます。coordinateでは同じchairのcurrent-state rowを順序付きで更新するため、
同一chairの並行requestには待ちが起こり得ます。

`lock_row_lock_time` はrow lock待ち時間の累積、`lock_row_lock_waits` は待った回数です。
ただしtableやSQL別ではありません。current UPDATEのprepared statement時間とhandler phaseを
合わせ、単一metricから原因を断定しないようにします。

### UNIQUE INDEXとduplicate key

`users.username` のUNIQUE INDEXは、同じusernameを持つrowを2件保存させないDB制約です。
B-tree lookupを速くする用途だけでなく、データの一意性をtransaction間でも保証します。

並行または逐次INSERTで同じ値が来ると、MySQLはerror 1062を返します。これはdeadlockの
1213とは異なり、同じSQLをそのままretryしても成功しません。入力をrejectするか、
別の一意な値へ変えるか、APIのidentity設計自体を変える必要があります。

OpenAPIにはusernameが「ユニーク」と明記されています。そのため、errorを消すだけのために
UNIQUE INDEXを外す案は採用しません。

## 仮説と反証条件

### 仮説A: current UPDATEのrow lockが支配的

予想は次のとおりでした。

> coordinateの大半が同じ500 chairのcurrent rowを更新するため、row lock待ちが
> transaction p95 / p99を支配している。

反証条件は、current writeのp95がhandler全体より十分小さく、別phaseのp95が明確に大きい
ことです。実測はcurrent write 4.185msに対して `pool.begin()` 93.651msだったため、
この仮説は反証されました。

### 仮説B: CODE=17はcoupon deadlockの再発

過去のCODE17は、INDEXなしの `coupons WHERE code = ? FOR UPDATE` が広くlockした
deadlockでした。今回も同じならMySQL 1213、InnoDB deadlock履歴、対象coupon SQLが
同時に観測されるはずです。

実際はMySQL 1062、`users.username`、`ER_DUP_ENTRY` 1件で、deadlock履歴はありません。
したがって過去原因の再発という仮説も棄却しました。

## 実装した診断

### 診断overlayだけで有効化する

`compose.diagnostics.yaml` からwebappへ `ISUCON_DIAGNOSTIC=1` を渡します。Rust側は
`OnceLock<bool>` でprocess中に1回だけ環境変数を読み、値が厳密に `1` の場合だけ有効に
します。通常構成と `ISUCON_DIAGNOSTIC=0` ではsampling counter、時刻取得、JSON生成を
実行しません。

### 64件に1件をphase計測する

sampleには次を記録します。

| field | 境界 |
|---|---|
| `cache_lookup_us` | 最新座標cacheの存在確認 |
| `pool_begin_us` | `pool.begin()` 全体 |
| `history_insert_us` | `chair_locations` INSERT |
| `current_write_us` | current UPDATE / UPSERT |
| `ride_lookup_us` | 椅子のcurrent ride検索 |
| `transition_us` | candidateのride lockとstatus追加 |
| `commit_us` | COMMIT |
| `cache_update_us` | commit後のprocess cache更新 |
| `total_us` | handler内の計測開始からresponse作成まで |

write pathとstatus遷移候補も同じsampleへ残し、遅い値がfallbackや遷移に偏るかを確認できます。

最初のレビューでは、成功末尾だけでlogを出すと、SQL errorやtask cancellationになった
sampleが統計から消える問題を検出しました。このrunのcoordinateはnginx log上75,833件
すべてHTTP 200だったため記録値の欠落はありませんでしたが、将来の診断で障害を見落とします。
修正後はDrop guardが `outcome=error_or_cancelled` と中断時の `terminal_phase` を出し、
成功時は `outcome=success`、`terminal_phase=complete` を出します。

error sampleでは未到達phaseの値が0のままなので、成功sampleのphase percentileへ混ぜると
障害が多いほど後半phaseが速く見える集計バイアスになります。集計scriptはphase表を
成功sampleだけに限定し、error / cancellationはterminal phase別の件数とtotal latencyへ
分けます。

stdoutへの書き込みは診断時だけ64件に1件ですが、response返却前の同期I/Oである点は残ります。
`println!` によるwrite errorのpanicは避け、`writeln!` の失敗を診断log欠落として扱います。
内部 `total_us` はJSON serializationとlog I/Oを含まないため、nginx latencyと併読します。

### 集計script

`scripts/report-coordinate-phases.sh <run開始時刻>` はwebapp logから診断JSONだけを抽出し、
各phaseの平均、p50、p95、p99、最大を出します。加えて次を取得します。

- InnoDB row lock metrics
- `chair_current_locations` UPDATE / UPSERTのprepared statement集計
- write path別sample数
- status遷移候補と実際のINSERT数

一時ファイルは `mktemp` で作り、正常終了・signalのどちらでも削除します。MySQL passwordは
command line引数へ出さず、container内の `MYSQL_PWD` として渡します。

`DIAGNOSTIC_SINCE` はwebapp logを切る境界で、DB tableへ直接は作用しません。scriptは
run開始、MySQL process起動、最初のsampleの順序を検証し、DBを使い回した場合やrun後に
再起動した場合はDB metricを表示せず停止します。prepared statement表はlive snapshotである
限界も見出しへ表示します。

## 計測結果

### coordinate phase

次の表は、全phaseを完了した成功sampleだけの分布です。このrunはnginx log上のcoordinateが
すべてHTTP 200だったため、1,185 sampleすべてが対象です。

| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |
|---|---:|---:|---:|---:|---:|---:|
| cache lookup | 1,185 | 3 | 0 | 0 | 0 | 3,078 |
| `pool.begin()` | 1,185 | 32,452 | 23,503 | 93,651 | 126,495 | 161,822 |
| history INSERT | 1,185 | 780 | 394 | 2,709 | 5,520 | 20,089 |
| current write | 1,185 | 1,633 | 674 | 4,185 | 23,184 | 88,459 |
| ride lookup | 1,185 | 1,025 | 632 | 3,317 | 6,445 | 16,018 |
| status transition | 1,185 | 113 | 0 | 0 | 3,270 | 12,305 |
| COMMIT | 1,185 | 4,064 | 2,971 | 11,768 | 17,383 | 37,015 |
| cache update | 1,185 | 15 | 0 | 1 | 37 | 7,671 |
| total | 1,185 | 40,089 | 33,308 | 105,296 | 138,956 | 194,090 |

sampleのwrite pathは通常UPDATE 1,176件、初回UPSERT 9件でした。status遷移候補54件は
すべてstatus INSERTへ進みました。通常UPDATEが99%以上なので、fallbackだけがtailを
作ったとは説明できません。

このrunはレビュー前のschemaでsampleを出しましたが、nginx log上のcoordinate
75,833件はすべてHTTP 200でした。したがって成功末尾に到達しないcoordinate sampleは
ありません。後続runでは、集計scriptが成功・失敗とterminal phaseの件数も表示します。

### MySQL

| metric | 値 |
|---|---:|
| `lock_row_lock_waits` | 2,914 |
| `lock_row_lock_time` | 48,332ms |
| 1 waitあたり単純平均 | 約16.6ms |
| `lock_row_lock_time_max` | 157ms |
| 終了時current wait | 0 |
| current-state write回数 | 75,834 |
| current-state write累積 | 61.581秒 |
| current-state write平均 | 0.812ms |
| current-state write最大 | 158.334ms |
| `Max_used_connections` | 51 |
| application pool上限 | 50 |

InnoDB metricsはSQL別ではなく、prepared statement集計も終了時に生存するconnectionの
snapshotです。値の限界を明記したうえで、sampled phaseと同じ方向かを確認しています。

### endpoint

| endpoint | count | average | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| chair coordinate | 75,833 | 62ms | 47ms | 183ms | 284ms | 1.211秒 |
| app notification | 110,975 | 37ms | 2ms | 191ms | 311ms | 未記録 |
| chair notification | 76,089 | 54ms | 3ms | 216ms | 346ms | 未記録 |
| evaluation POST | 1,706 | 381ms | 未記録 | 752ms | 未記録 | 未記録 |

Rust内のcoordinate total p95 105msとnginx p95 183msは同じ値ではありません。
Rust instrumentationは1/64 sampleかつmiddlewareやnginx待ちを含まず、nginxは全requestを
集計します。差を「未計測の93ms」と単純に引き算せず、それぞれの観測境界として扱います。

## CODE=17をどう特定したか

確認順は次のとおりです。

1. benchmarkerのerror mapで `CODE=17` 1件を確認
2. nginx access logで同時刻の `POST /api/app/users` 500を確認
3. webapp logでMySQL error code 1062と重複値を確認
4. `users` を検索し、同名rowが16秒前に同じrunで作られたことを確認
5. Performance Schemaで `ER_DUP_ENTRY` 1件を確認
6. `SHOW ENGINE INNODB STATUS` に新しいdeadlockがないことを確認

現行nginx logにはrequest IDとbodyを保存していないため、登録request IDは取得できませんでした。
body全件をlogすると個人情報と高頻度I/Oを増やします。今回はUTC時刻、endpoint、HTTP status、
DB error、重複usernameで同じ1件を相関しました。

benchmarkerは `gofakeit.Username()` を使い、同一性を集合で検査せず登録requestを作ります。
このrunでは偶然同じ値を2回生成しました。serverはOpenAPIどおりUNIQUE制約を持つため、
2回目だけ500になりました。

## 判断

### 採用するもの

- 診断overlay限定の1/64 phase sampling
- 再利用可能な集計script
- error / cancellation時もterminal phaseを残すDrop guard
- current UPDATE支配仮説の棄却
- CODE17をdeadlockと決めつけず、error code別に追う方針

### まだ採用しないもの

- current rowのqueue / coalescing
  - 全履歴と3秒収束の設計前に、主要なp95待ちではないと分かった
- pool上限の増加
  - acquire待ちと `BEGIN` 自体をまだ分離していない
  - 長い評価transactionがconnectionを保持するため、上限増加だけではMySQL競合を増やし得る
- usernameのUNIQUE INDEX削除
  - OpenAPIの一意性と、同名を別identityとして扱う保証を失う

## 次の仮説

1. username 1062だけを識別し、別の一意な保存名へ限定retryすれば、CODE17を避けられる
2. `pool.acquire()` と `connection.begin()` を分けると、`pool.begin()` p95の大半が
   application側のconnection待ちとして観測される
3. pool待ちが確定した場合、先に外部決済HTTPをtransaction外へ出して保持時間を短くする方が、
   pool上限だけを増やすより根本原因へ効く

usernameの限定retryは、requestのusernameと保存値が一致しなくなるtrade-offがあります。
benchmarkerは登録後のidentityをIDとcookieで追い、usernameを検証していませんが、
一般の会員登録APIなら勝手な改名は望ましくありません。採用時はISUCON用の耐障害策であること、
409を返す一般的なAPI設計、benchmarker側で一意生成する選択肢も併記します。
