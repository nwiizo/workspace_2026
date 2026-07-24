# Benchmark 31: 評価APIのDB接続保持時間を決済待ちまで分解

## 結論

`POST /api/app/rides/:ride_id/evaluation` が長い理由は、完了writeや
`COMMIT`ではなく、DB transactionを開いたまま待っている外部決済でした。

診断runの成功203 sampleでは、SQLx connectionをhandlerが所有した時間が平均
319.754ms、そのうち決済phaseが平均302.507msでした。平均値同士の比では約94.6%です。
決済phaseの内側は、決済HTTP requestが平均100.785ms、5xx後の100ms retry sleepが
平均201.719msでした。

```text
評価API total 平均 369.719ms
├─ pool acquire              49.957ms
└─ connection所有           319.754ms
   ├─ BEGIN                   0.671ms
   ├─ ride lock + status      2.103ms
   ├─ token/fare/settings     2.932ms
   ├─ 外部決済              302.507ms
   │  ├─ HTTP request合計   100.785ms
   │  └─ retry sleep合計    201.719ms
   ├─ 完了write               6.417ms
   └─ COMMIT                  5.114ms
```

各平均値は別々に丸めているため、表示値の合計はtotalと完全には一致しません。
また `pool acquire` はconnection取得前なので、`connection_owned_us` には含みません。

次の改善対象はpool上限ではありません。短い準備transactionで必要な値を読み、
DB connectionを返してから冪等な決済を実行し、成功後に短い完了transactionを開始します。
決済成功後のDB失敗は同じride IDの冪等keyで再実行できるようにし、完了transactionでは
rideを再lockして二重のstatus・stats更新を防ぎます。

## この計測で答えたい問い

Benchmark 30では、coordinateのSQL `BEGIN` はp95 2.327msに対し、pool acquire phaseが
p95 113.156msでした。pool size 50・idle 0も高頻度に観測したため、接続を長時間
保持するendpointを短縮する必要があります。

nginx logでは評価APIが平均403ms・p95 769msでした。ソース上では次の全処理が1つの
transactionに入っています。

1. rideを `FOR UPDATE` でlockする
2. 最新statusを確認する
3. payment token、fare、決済URLを読む
4. 外部決済へPOSTする
5. 5xxなどでは100ms待って最大5回再試行する
6. `COMPLETED`、chair stats、evaluationを書き込む
7. commitする

HTTP時間だけでは、pool待ち、SQL、決済、retry sleepのどれが支配的か分かりません。
そこで次を個別に測りました。

- connection取得前のpool状態
- pool acquire
- SQL `BEGIN`
- ride lockとstatus確認
- response配送用tracker登録
- token、fare、settingsの準備
- 決済phase全体
- 決済HTTP requestの合計
- retry sleepの合計
- 完了write
- `COMMIT`
- cache invalidationとresponse生成
- connectionをhandlerが所有した時間
- 決済試行回数とerror分類
- 同時に処理中の評価数と、同じrideの重複評価数

## 計測条件

| 項目 | 値 |
|---|---|
| 日時 | 2026-07-25 |
| revision | Benchmark 30後の `main` + 評価診断instrumentation |
| ホスト | Apple Silicon macOS / Colima |
| Colima | 4 CPU / 4 GiB / 100 GiB |
| app | Rust release build |
| DB | MySQL、runごとに停止・再起動 |
| pool上限 | 50、変更なし |
| 走行時間 | 60秒 |
| 診断sampling | 評価APIの8 requestに1件 |
| 診断run開始 | `2026-07-24T22:37:43Z` |
| MySQL process開始 | `2026-07-24T22:38:18Z` |
| 最初の評価sample | `2026-07-24T22:38:42.076516527Z` |

開始時刻、MySQL process開始、最初のsampleを並べるのは、別runの累積metricを混ぜない
ためです。集計scriptはMySQLが指定時刻以降、最初のsample以前に起動したことを検証し、
条件を満たさなければ失敗します。

診断runの結果は次のとおりです。

| pass | score | error map | tick 1980のevaluation request |
|---|---:|---|---:|
| true | 114,109 | 空 | 1,548 |

nginx logでは評価APIが1,624件あります。tickの値は特定時点のbenchmarker内部集計、
nginxはrun中に受信したHTTP logの集計なので、用途の異なる値として分けます。

score 114,109は実測値ですが、診断JSON出力付きの1走だけです。通常構成の推定代表値や
高速化の効果には使いません。

review修正後はDocker releaseを再buildし、10秒の短い診断runでも `pass=true`・
score 6,287・error map空を確認しました。成功11 sampleで全phase、決済attempt、
pool状態、tracker数を再集計できました。この10秒値はserializationと集計scriptの
sanity checkであり、60秒scoreの比較や推定には加えません。

## 実装した計測

### 1/8 sampling

診断は `ISUCON_DIAGNOSTIC=1` のときだけ有効です。評価requestにprocess内の連番を振り、
8件に1件だけ `EVALUATION_DIAGNOSTIC` JSONを標準出力へ書きます。通常Composeでは
環境変数を設定しないため、JSON生成と出力は行いません。

1/8にした理由は、評価APIが60秒で約1,600件あり、約200 sampleあれば分布とretry回数を
確認できる一方、全requestの同期的なstdout書込みを避けられるためです。

今回の成功sampleは203件で、nginxの評価API 1,624件のちょうど1/8でした。

### RAIIでerrorとcancelも残す

診断objectは最初のoutcomeを `error_or_cancelled`、terminal phaseを `validation` として
作成します。phaseを通過するたびにterminal phaseを次へ進めます。正常終了だけ
`success / complete` に変更します。

Rustではscopeを抜けると `Drop` が呼ばれます。途中の `?`、timeout、futureのcancelでも、
まだ出力していなければそれまでの値を1回だけ出します。この仕組みにより、成功request
だけを見て遅い失敗を見落とすことを避けます。

今回の203 sampleはすべて `success / complete` で、診断対象内の途中失敗はありません。

### 未観測値を0にしない

pool size、idle、in-use、active evaluation数、決済terminal statusは `Option` で持ちます。
たとえばpool acquire前にrequestが終了した場合、pool状態は `null` です。

0は「測って0だった」という事実です。未到達を0にすると、error sampleを高速な正常処理と
誤認するため、未観測と実測0を分けます。

### 決済の内訳

決済関数へsampled requestだけの診断objectを渡し、次を加算します。

- `attempts`: POSTを開始した回数
- `request_us`: 各POSTの送信からresponse status判定までの合計
- `retry_sleep_us`: 100ms sleepの実経過時間の合計
- `network_errors`
- `conflict_errors`: HTTP 409
- `server_errors`: HTTP 5xx
- `other_status_errors`: retryしないその他のstatus
- `terminal_status`: 最後に観測したHTTP status

`payment_us` は決済関数全体のwall timeです。`request_us + retry_sleep_us`との差には、
loopやerror分類などの小さな処理時間と計測の丸めが入ります。

決済診断objectは評価診断object自身が所有します。決済HTTPまたはretry sleepの途中で
futureがcancelされても、評価診断の `Drop` が同じobjectから開始済みattempt、分類済みerror、
進行中phaseの経過時間をsampleへ反映します。決済関数の `.await` が正常に戻った後だけ
別objectからコピーする方式にはしません。

## phase別結果

成功203 sampleの結果です。percentileは値を昇順に並べ、
0始まりの `floor((n - 1) × p)` 番目を選ぶlower order statisticです。

| phase | avg | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|
| pool acquire | 49.957ms | 46.562ms | 123.528ms | 133.619ms | 138.875ms |
| SQL BEGIN | 0.671ms | 0.302ms | 2.114ms | 4.904ms | 9.917ms |
| ride lock + status | 2.103ms | 1.294ms | 6.808ms | 9.237ms | 21.772ms |
| tracker begin | 0.003ms | 0.001ms | 0.001ms | 0.003ms | 0.327ms |
| token / fare / settings | 2.932ms | 2.590ms | 7.193ms | 8.211ms | 11.948ms |
| payment | 302.507ms | 184.741ms | 691.875ms | 714.058ms | 759.083ms |
| payment HTTP request合計 | 100.785ms | 79.939ms | 200.995ms | 223.657ms | 308.896ms |
| payment retry sleep合計 | 201.719ms | 103.914ms | 502.523ms | 507.979ms | 522.100ms |
| completion write | 6.417ms | 4.513ms | 20.904ms | 26.368ms | 29.511ms |
| COMMIT | 5.114ms | 3.320ms | 15.537ms | 25.475ms | 53.361ms |
| cache + response | 0.007ms | 0.004ms | 0.009ms | 0.068ms | 0.346ms |
| connection所有 | 319.754ms | 221.974ms | 695.556ms | 734.037ms | 779.825ms |
| handler total | 369.719ms | 308.535ms | 733.946ms | 814.502ms | 852.311ms |

### 平均値から見た割合

| 境界 | 比較 | 割合 |
|---|---|---:|
| payment / connection所有 | 302.507 / 319.754 | 約94.6% |
| retry sleep / connection所有 | 201.719 / 319.754 | 約63.1% |
| payment HTTP / connection所有 | 100.785 / 319.754 | 約31.5% |
| completion write / connection所有 | 6.417 / 319.754 | 約2.0% |
| COMMIT / connection所有 | 5.114 / 319.754 | 約1.6% |

割合は個々のrequestの比率を平均した値ではなく、phase平均をconnection所有平均で
割った記述統計です。それでも決済待ちが桁違いに大きく、改善の優先順位を決めるには
十分な差があります。

## retry回数

すべてのsampleは最終的に204で成功しました。

| attempts | sample | 平均payment | 平均HTTP合計 | 平均sleep |
|---:|---:|---:|---:|---:|
| 1 | 48 | 31.769ms | 31.768ms | 0ms |
| 2 | 56 | 167.703ms | 66.582ms | 101.118ms |
| 3 | 20 | 305.929ms | 103.546ms | 202.380ms |
| 4 | 19 | 435.103ms | 132.524ms | 302.575ms |
| 5 | 48 | 574.932ms | 170.644ms | 404.282ms |
| 6 | 12 | 709.199ms | 202.168ms | 507.025ms |

203 sampleの合計は608 attempts、途中の5xxは405回でした。network error、409、
retryしないstatusは0でした。1 requestあたりの平均attempt数は約3.0回です。

現在の実装は初回失敗後に最大5回retryするため、最大は初回を含む6 attemptsです。
`retry < 5` という変数だけを見ると合計5回と誤読しやすいため、attempt数とretry数を
区別します。

決済mockは負荷に応じて5xxを返します。5xxをretryしない変更は正当性を壊します。
retry回数を減らすのではなく、retry中にDB connectionとride row lockを保持しないことが
今回の改善対象です。

## pool状態との関係

acquire直前の状態は次のとおりでした。

| 状態 | sample | 構成比 |
|---|---:|---:|
| pool size 50 / idle 0 | 172 | 約84.7% |
| idleあり | 31 | 約15.3% |

| acquire直前の群 | sample | avg | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| 観測最大size 50・idle 0 | 172 | 58.172ms | 54.821ms | 123.731ms | 135.168ms | 138.875ms |
| idleあり | 31 | 4.379ms | 0.159ms | 5.631ms | 51.867ms | 64.895ms |

`pool.size()` と `pool.num_idle()` は別々の時点で読むため、完全に原子的なsnapshotでは
ありません。またacquire phaseにはqueue待ちだけでなく、SQLxのhealth checkや接続返却
処理が含まれ得ます。したがって58.172msを純粋なqueue時間とは呼びません。

それでも同一runでidle 0群とidleあり群に大きな差があり、外部決済中の長いconnection
所有とpool枯渇が同時に観測されています。

MySQL側も次の状態でした。

| metric | 値 |
|---|---:|
| `Connections` | 87 |
| `Max_used_connections` | 51 |
| `Threads_connected` | 51 |
| `Threads_running` | 2 |

アプリpool上限50に管理接続を加えた51接続が最大です。`Threads_running=2`なのに
50接続を使い切るのは、多数のconnectionがCPUでSQLを実行しているのではなく、外部HTTPや
sleepを待ちながらtransactionを保持しているというphase計測と整合します。

## 同時評価

response配送trackerへ登録した後に、process内でactiveな評価数を読みました。

| metric | 値 |
|---|---:|
| sample | 203 |
| 最大active evaluation | 38 |
| 同じrideが2件以上active | 0 |

この38件はsample開始時に観測したtracker guard数です。guardはtransactionだけでなく
response body dropまで残るため、同時にconnectionを所有した件数やrun全期間の厳密な最大値
ではありません。現在のhandlerが決済中にconnectionを1本所有することはコードと
`connection_owned_us` から確認できますが、38をそのままconnection本数には変換しません。

同じrideの並行評価が0だったことは今回の負荷の観測であり、将来も起きない保証では
ありません。transaction分割後の完了writeではrideを再lockし、`evaluation` と最新statusを
再確認して二重加算を防ぎます。

## endpoint log

同runのnginx JSON logを `alp` で集計しました。

| endpoint | count | avg | p50 | p95 | p99 | max | 累積 |
|---|---:|---:|---:|---:|---:|---:|---:|
| coordinate | 76,106 | 65ms | 53ms | 173ms | 269ms | 524ms | 4,916.089秒 |
| app notification | 109,566 | 32ms | 2ms | 197ms | 289ms | 629ms | 3,501.029秒 |
| chair notification | 81,638 | 43ms | 3ms | 218ms | 307ms | 629ms | 3,475.912秒 |
| evaluation | 1,624 | 392ms | 360ms | 762ms | 843ms | 1,082ms | 636.978秒 |
| nearby | 13,121 | 34ms | 7ms | 131ms | 185ms | 398ms | 447.998秒 |

評価sampleのhandler totalは平均369.719ms・p95 733.946ms、nginxでは平均392ms・
p95 762msです。sampling差、nginxまでの処理、proxy、log精度を含むため同値には
なりませんが、約20–30ms差で同じ分布を示しています。

通知のHTTP 499はapp 166件、chair 139件でした。評価APIの2xxは1,624件、4xx / 5xxは
0件です。

## InnoDB row lock

run用に再起動したMySQLの累積値は次のとおりでした。

| metric | 値 |
|---|---:|
| `lock_row_lock_waits` | 3,285 |
| `lock_row_lock_time` | 50,907ms |
| `lock_row_lock_time_max` | 172ms |
| 平均 | 約15.5ms |

これは評価APIだけでなく、同じMySQL processの全transactionの累積です。評価sampleの
`ride_lock_status_us` は平均2.103ms・p95 6.808msなので、評価ride lock自体を
今回の主因とはしません。

transactionを決済前後へ分ければ、評価がride row lockを保持する約300msも解消します。
pool acquireだけでなく、coordinateやstatus writerが同じrideを待つ時間にも効果が
ある可能性があります。採否は変更後のInnoDB累積値と通常ベンチで確認します。

## 用語

### connection

アプリとMySQLの通信路です。SQLx poolは作成済みconnectionを再利用します。上限50なら、
同時に借りられるconnectionは最大50本です。

### transaction

複数のSQLを1つの整合性単位としてcommitまたはrollbackする範囲です。`FOR UPDATE` で
取得したrow lockは通常commit / rollbackまで保持されます。

### connection所有時間

この計測では、`pool.acquire()` 成功からtransactionをcommitし、handler側の
`PoolConnection` をdropするまでです。SQLx 0.8.2はdrop後にpingやprotocol flushを行う
非同期の返却taskを開始するため、dropした瞬間に `num_idle()` が増えるとは限りません。
したがって「handlerが所有した時間」と呼び、poolで再利用可能になるまでの完全な時間とは
区別します。

### 外部I/O

このAPIにとってのMySQL以外の通信です。決済HTTPは相手serviceの応答を待ちます。
`.await` 中はRust worker threadを占有しませんが、transaction objectとDB connectionは
scope内に残るため、pool資源とrow lockは保持します。

### retry sleep

再試行前に一定時間待つbackoffです。相手が一時的に失敗したとき即時再送の集中を
避けます。しかしDB transaction内でsleepすると、SQLを1つも実行しない100msの間も
connectionとlockを保持します。

### 冪等性

同じ操作を複数回実行しても、結果が1回分に収束する性質です。現在はride IDを
`Idempotency-Key` として決済POSTへ付けています。決済成功後にアプリが落ち、
clientが評価を再送しても同じ決済へ収束させる土台です。

## 仮説と実際

| 仮説 | 実際 | 判断 |
|---|---|---|
| `BEGIN` またはride lockが評価p95の主因 | BEGIN p95 2.114ms、ride lock + status p95 6.808ms | 棄却 |
| 完了writeとCOMMITが主因 | p95 20.904ms / 15.537ms | 主因ではない |
| 外部決済がconnection所有時間の大半 | 平均302.507 / 319.754ms、約94.6% | 支持 |
| retry sleepが特に大きい | 平均201.719ms、5xx 405回 | 支持 |
| 評価の長い保持とpool枯渇が同時に起きる | connection所有平均319.754ms、84.7%がsize 50 / idle 0 | 関連を支持。trackerの38はconnection数には使わない |
| 同じrideの重複評価が通常負荷で多い | 今回は0 sample | 今回は棄却。ただし再lockは残す |
| pool上限を増やすのが最初の解決 | 50接続の多くが外部I/O待ち | 採用しない |

## 次の実装

次は評価処理を3区間へ分けます。

```text
短い準備transaction
  rideをuser_id付きでlock
  ARRIVED確認
  token / fare / settings取得
  commitしてconnectionを返す

transaction外
  ride IDの冪等keyで決済
  5xx retryとsleep

短い完了transaction
  rideをuser_id付きで再lock
  evaluation未確定・ARRIVEDを再確認
  COMPLETED / chair stats / evaluationを同時write
  commit
```

守る不変条件は次のとおりです。

1. 決済成功前に `COMPLETED`、evaluation、chair statsを公開しない
2. DB完了前にHTTP 200を返さない
3. 完了transactionでrideを再lockし、二重加算しない
4. 決済成功後にDB更新が失敗しても、再送は同じride IDの決済へ収束する
5. `rides.updated_at` は完了writeの最後に更新し、owner sales境界を保つ
6. response配送trackerは準備開始からresponse body dropまで椅子を除外する
7. 別userのrideは引き続き404とし、存在を漏らさない

## 他の選択肢

### pool上限を増やす

acquire待ちは一時的に減る可能性があります。しかし外部決済中にconnectionを保持する
構造のまま上限を増やすと、待ち行列をSQLxからMySQLへ移し、memory、context switch、
row lock競合を増やす可能性があります。長い保持を短縮した後もacquire待ちが残る場合に
だけ比較します。

### retry回数を減らす

平均時間は短くなりますが、一時的5xxから回復できず評価APIのerrorが増えます。今回も
203 sample中155件は1回以上のretry後に204へ到達しました。正当性と完了数を落とすため
採用しません。

### transactionを開かず全準備を読む

connection保持はさらに短くできますが、ride、status、couponを別snapshotで読み、
並行評価との境界が曖昧になります。まず短い準備transactionで一貫した条件を確認し、
commit後に決済へ進みます。

### process内のride mutexだけを使う

単一processでは同じrideの並行評価を抑えられますが、複数processや再起動をまたげません。
冪等な決済と、完了transactionでのDB row lock・再確認を正本にします。process mutexは
必要性を計測してから待ち削減の補助として検討します。

### 永続claim表を追加する

`ride_evaluation_claims` のような別表へclaim状態と時刻を保存すれば、重複評価を決済前に
拒否できます。一方、process crash後のstale claim回収、lease期限、決済済みか不明な
状態の再開処理が必要です。

現在はride IDによる決済冪等性があり、今回の同一ride並行数は0でした。まず完了transaction
で二重writeを防ぐ最小の分割を検証し、故障注入で不足が見つかった場合にclaim表を追加します。

### 決済URLをmemoryへcacheする

毎回のsettings SELECTを削減できますが、準備phaseは平均2.932msで、決済302.507msより
2桁小さい値です。transaction分割後に残るSQLとしては候補ですが、今回の支配項を先に
解消します。

## 実行コマンド

```sh
# 開始UTCを保存し、診断Composeで60秒実行
date -u +%Y-%m-%dT%H:%M:%SZ
ISUCON_DIAGNOSTIC=1 ./scripts/benchmark.sh 60

# 同じrunの評価phase
./scripts/report-evaluation-phases.sh 2026-07-24T22:37:43Z

# 同じrunのnginx endpoint
./scripts/report-endpoint-latency.sh 2026-07-24T22:37:43Z

# ローカル固定資源
colima list --json
```

## 次の判定条件

transaction分割後は通常60秒ベンチを3回実行し、次を同時に確認します。

- 全run `pass=true`、error map空
- evaluationのconnection所有時間とpool acquire p95が低下
- coordinate / notificationのp95も低下
- `Max_used_connections`、`Threads_running`、row lock累積
- 同じrideの二重 `COMPLETED` が0
- chair statsの二重加算が0
- 決済失敗ではDB完了状態を公開しない
- 決済成功後のDB失敗を同じ冪等keyで回復できる
- 通常3走の中央値と範囲

診断値が改善してもscoreと正当性が悪化すれば採用しません。逆にscoreの単発上振れだけで、
connection保持が変わっていなければ今回の仮説を確認できたとは扱いません。
