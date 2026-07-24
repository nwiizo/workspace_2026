# Benchmark 30: coordinateのpool取得待ちとSQL BEGINの分離

## 結論

Benchmark 27で `POST /api/chair/coordinate` の `pool.begin()` が平均32.452ms、
p95 93.651msだったため、次の2区間へ分けて再計測しました。

1. SQLx connection poolから接続を取得する `pool.acquire().await`
2. 取得済み接続でMySQL transactionを開始する `connection.begin().await`

診断runの成功sample 1,173件では、pool acquire phaseが平均43.657ms・p95 113.156ms、
SQL `BEGIN` が平均0.611ms・p95 2.327msでした。合計p95 114.048msのほぼ全体を
acquire phaseが占めています。

取得開始直前のsnapshotでは、SQLx poolの上限50接続がすべて使用中だったsampleが
916 / 1,173件、約78.1%でした。idle接続が0だったsample全体では917件、約78.2%です。
size 50 / idle 0のsampleだけではacquire phaseが平均54.762ms・p95 117.398ms、
idleが1以上のsampleでは平均3.968ms・p95 16.138msでした。

したがって、次のP0は `BEGIN` 文やcurrent-state UPDATEの書き換えではありません。
DB connectionを長く保持しているtransactionを特定し、保持時間を短くすることです。
同じrunで評価APIは1,795回、平均403ms・p95 769msでした。現行実装はride rowを
`FOR UPDATE` した後、外部決済HTTPとretry sleepをtransaction内で待つため、
最初に評価APIの接続保持時間をphase分解します。

pool上限を50から増やす変更はまだ行いません。待ち行列をMySQL側へ移し、CPU競合、
row lock、COMMIT待ちを増やす可能性があるためです。ホストとColimaは
4 CPU / 4 GiB / 100 GiBのままです。

## 診断runの結果

`ISUCON_DIAGNOSTIC=1` を付けた60秒runを1回実行しました。診断用のJSON出力と
nginx timing logを有効にした値なので、通常runの推定代表値には混ぜません。

| 項目 | 結果 |
|---|---|
| `pass` | `true` |
| score | 124,064 |
| error map | 空 |
| tick 1980の評価request数 | 1,731 |
| 最終不満率 | 39.7% / 34.2% / 69.5% |
| coordinate成功sample | 1,173 |
| sample内error / cancellation | 0 |

1走だけなのでscoreの中央値、観測幅、改善率は推定しません。この変更は診断値を
分けるためのinstrumentationであり、124,064点を高速化の効果とも扱いません。

計測境界は次のとおりです。

| boundary | UTC |
|---|---|
| requested run start | 2026-07-24T22:19:40Z |
| MySQL process start | 2026-07-24T22:20:18Z |
| first coordinate sample | 2026-07-24T22:20:40.884364175Z |

MySQLはrun用に再起動されており、InnoDBの累積値はこのprocess lifetimeだけを含みます。

## coordinate phaseの実測

成功sampleだけを対象にしたpercentileです。

| phase | samples | avg | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| cache lookup | 1,173 | 1µs | 0µs | 0µs | 0µs | 1.172ms |
| pool acquire phase | 1,173 | 43.657ms | 40.181ms | 113.156ms | 145.391ms | 195.700ms |
| transaction BEGIN | 1,173 | 0.611ms | 0.290ms | 2.327ms | 5.061ms | 11.251ms |
| acquire + BEGIN | 1,173 | 44.268ms | 40.974ms | 114.048ms | 145.643ms | 195.949ms |
| history INSERT | 1,173 | 0.896ms | 0.394ms | 2.947ms | 8.447ms | 75.014ms |
| current-state write | 1,173 | 1.851ms | 0.687ms | 5.007ms | 29.727ms | 87.057ms |
| current ride lookup | 1,173 | 1.105ms | 0.679ms | 3.217ms | 7.673ms | 19.777ms |
| status transition | 1,173 | 0.156ms | 0ms | 0ms | 4.729ms | 17.599ms |
| COMMIT | 1,173 | 4.447ms | 3.502ms | 12.145ms | 17.141ms | 27.708ms |
| cache update | 1,173 | 0.012ms | 0ms | 0.002ms | 0.201ms | 2.848ms |
| handler内合計 | 1,173 | 52.742ms | 48.750ms | 124.240ms | 159.206ms | 201.199ms |

p95で比較すると、pool acquire phaseはcurrent-state writeの約22.6倍、
transaction BEGINの約48.6倍です。先にcurrent UPDATEをqueue化するより、
connectionを返せない時間を減らす方が大きな改善余地を持つという仮説を支持します。

このreportのpercentileは、昇順へ並べた0始まり配列の
`floor((n - 1) × p)` 番目を使うlower order statisticです。nearest-rankなど別方式では
境界の1 sampleが変わり得るため、異なるtoolの値を比較するときは計算方法も揃えます。

## pool snapshot

sample対象requestが `acquire().await` を呼ぶ直前に、次を記録しました。

- pool size: poolが現在管理する接続数。idleと使用中を両方含む
- idle: 誰も使用しておらず、直ちに貸し出せる接続数
- in use: `size - idle` で求めた接続数

| 状態 | samples | 全1,173件に対する割合 |
|---|---:|---:|
| size 50、idle 0、in use 50 | 916 | 約78.1% |
| idle 0の全状態 | 917 | 約78.2% |
| idleが1以上 | 256 | 約21.8% |

size 38・idle 0が1件あり、それ以外のsampleではsizeは50まで増えていました。
アプリの `MySqlPoolOptions` は `max_connections(50)` です。run終了後のMySQLは
`Max_used_connections=51`、`Threads_connected=51` で、MySQL自体の
`max_connections=151` でした。追加の1接続は計測用mysql clientを含みます。

このsnapshotには限界があります。`size()` と `num_idle()` は1つの原子的snapshotではなく、
値を読んだ直後に別taskが接続を取得・返却できます。idle 0でもsizeが上限未満なら、
待つ代わりに新規接続を開いている可能性があります。

一方、今回の主要状態はsize 50・idle 0です。上限まで作成済みで貸出可能接続もないため、
直後の `acquire().await` が接続返却を待つという解釈と、次の状態別実測が整合します。

| 取得直前の状態 | samples | acquire平均 | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| size 50 / idle 0 | 916 | 54.762ms | 52.445ms | 117.398ms | 146.666ms | 195.700ms |
| idle 1以上 | 256 | 3.968ms | 0.426ms | 16.138ms | 87.450ms | 113.327ms |
| size 38 / idle 0 | 1 | 32.145ms | 32.145ms | 32.145ms | 32.145ms | 32.145ms |

ここで測る `pool_acquire_us` は純粋なqueue時間ではありません。pool状態を読む短い処理、
SQLxの `acquire()` 全体、返却された接続のhealth checkを含みます。sizeが上限未満なら
新規接続の作成時間も入り得ます。したがって名称はacquire phaseとし、
すべての113msをqueueだけの時間とは断定しません。

それでも、上限50・idle 0の群はidleありの群より平均で約13.8倍長く、
p95も約7.3倍です。取得直前のsaturationと長いacquire phaseに強い関連があります。

## はじめに知っておく用語

### database connection

アプリprocessとMySQL serverの間の通信路です。SQLを1回送るたびに新しいTCP接続と
MySQL認証を行うと高コストなので、SQLxは接続を再利用します。

接続は同時に複数transactionを実行できません。あるhandlerがtransactionを開始して
外部HTTPを待っている間、その接続を別handlerへ貸し出せません。

### connection pool

再利用する接続を一定数保持し、taskへ貸し出す仕組みです。今回の上限は50です。

```text
HTTP task
  ↓ acquire
SQLx pool ── idle connectionがあれば貸出
  │
  └─ 50接続すべて使用中なら返却まで待機
```

poolは接続作成コストを減らしますが、DBの処理能力を増やすものではありません。
上限を上げるとアプリ側の待ちが短く見える一方、MySQLの実行thread、lock待ち、
buffer、COMMIT競合が増えることがあります。

### acquire phase

poolから接続を借りる操作を含む計測区間です。idle接続があれば短時間で終わります。idleがなく、
pool sizeが上限なら、別taskが接続を返すまで非同期に待ちます。

SQLxのpoolは取得要求を公平な順序で処理します。公平性はstarvationを防ぎますが、
待ち行列そのものをなくすわけではありません。
計測値にはhealth checkや上限未満での接続生成も含むため、queue待ちだけを表す値では
ありません。

### transaction BEGIN

取得済み接続でMySQL transactionを開始する操作です。今回、SQLxの `Pool::begin()` は
内部で「pool acquire + transaction begin」を続けて行うため、以前の1計測だけでは
どちらが遅いか分かりませんでした。

明示的に接続を取得してから `Acquire::begin` を呼び、2つのawaitを別々に計測しました。
結果は、BEGIN文ではなくacquire前のqueueが支配的でした。

### saturation

資源が上限まで使われ、新しい仕事が待つ状態です。CPU 100%だけを意味しません。
今回のpoolは50接続すべてが使用中で、coordinate requestが接続返却を待っていました。
これはconnectionという有限資源のsaturationです。

### queue

処理可能になるまで仕事が並ぶ待ち行列です。現在はSQLx pool内にacquire待ちができます。
pool上限を増やすだけでは、queueがMySQLのthread・row lock・COMMITへ移る可能性があります。

最適化では「queueが見えなくなった」ではなく、「request全体のp95が短くなり、
throughputとscoreが上がった」ことを確認します。

### connection holding time

接続をpoolから借りてから返すまでの時間です。SQL実行時間だけでなく、
transaction内で行うRust処理、外部HTTP、retry sleep、lock待ちを含みます。

同じ毎秒request数でもholding timeが半分なら、平均して同時使用する接続数も
おおむね半分になります。接続を増やすより、不要な保持区間を短くする方が
DBへ余計な並行負荷を加えずにqueueを減らせます。

### Littleの関係

安定した系では、平均処理中件数はおおよそ
`到着率 × 平均滞在時間` で表せます。

今回の評価APIは60秒に1,795件、約29.9件/秒で、HTTP平均時間は403msでした。
単純に掛けると約12.1件が平均して評価処理中です。

ただし403msにはconnectionを取得する前のpool待ちやmiddlewareも含まれます。
その全時間で接続を保持しているわけではないため、12.1を使用接続数とは断定しません。
次の診断で評価APIをpool acquire、DB準備、決済HTTP、完了write、COMMITへ分け、
実際のholding timeを測る必要があります。

### p50・p95・p99

値を小さい順に並べたpercentileです。p50は中央、p95は95%がその値以下、
p99は99%がその値以下になる境界です。

poolは平均だけでなくtail latencyが重要です。少数の長いtransactionが接続を保持すると、
後続requestがまとめて待ち、p95 / p99へqueueの波が現れます。

## endpoint別の同run比較

nginx timing logの主要endpointは次のとおりでした。

| endpoint | count | avg | p50 | p95 | p99 | max | 累積 |
|---|---:|---:|---:|---:|---:|---:|---:|
| coordinate | 75,062 | 65ms | 59ms | 162ms | 239ms | 430ms | 4,903.958秒 |
| app notification | 108,681 | 35ms | 2ms | 186ms | 271ms | 524ms | 3,796.825秒 |
| chair notification | 83,278 | 45ms | 3ms | 200ms | 284ms | 524ms | 3,778.247秒 |
| evaluation | 1,795 | 403ms | 382ms | 769ms | 846ms | 993ms | 723.855秒 |
| nearby | 14,485 | 36ms | 11ms | 127ms | 181ms | 389ms | 516.727秒 |

通知は高頻度ですが、状態不変cache hitではDB transactionを開始しません。
HTTP時間だけから接続保持を推定できないため、cache missとtransaction区間を分ける必要が
あります。

評価APIはrequest数が通知より少ない一方、handler冒頭でtransactionを開始し、
ride row lock、token / fare / settings読取り、外部決済、完了write、COMMITまで
同じ接続を保持します。外部I/Oをtransaction内で待つ構造が明確なので、
次の詳細計測対象に選びます。

## InnoDB row lockとの比較

同じMySQL process lifetimeでは次の値でした。

| metric | 値 |
|---|---:|
| row lock waits | 3,371回 |
| row lock wait time合計 | 54,578ms |
| row lock wait平均 | 16ms |
| row lock wait最大 | 142ms |
| run終了後のcurrent waits | 0 |

row lock待ちは実在します。しかしcoordinate sampleのcurrent-state write p95は5.007ms、
pool acquire p95は113.156msです。row lockだけを先に最適化しても、
接続を借りる前の待ちは直接なくなりません。

prepared statementのcurrent-state writeは75,063回、累積69.115秒、平均0.921ms、
最大142.58msでした。平均とp95ではpool acquire phaseより小さく、最大値だけを見て
current UPDATEを主原因にしない判断とも一致します。

## instrumentationの実装

`chair_post_coordinate` は従来、次の1行でした。

```rust
let mut tx = pool.begin().await?;
```

変更後は同じ処理を明示的に分けています。

```rust
let mut connection = pool.acquire().await?;
let mut tx = connection.begin().await?;
```

commit後は明示的にconnectionをdropし、handlerから所有権を解放してpoolへの非同期返却を
開始します。SQLx 0.8.2は返却task内で接続をpingし、rollbackなど未処理のprotocol状態を
flushしてからidle queueへ戻します。したがって、drop直後に他taskが借りられるとは
限らず、短時間はcache更新と返却処理が重なる可能性があります。
`Pool::begin()` も内部で接続取得後にtransactionを開始するため、SQLとcommit範囲は
変えていません。

`ISUCON_DIAGNOSTIC=1` のときだけ64 requestに1件をsampleし、次をJSONへ追加します。

- `pool_acquire_us`
- `transaction_begin_us`
- 後方比較用の合計 `pool_begin_us`
- `pool_size_before`
- `pool_idle_before`
- `pool_in_use_before`

診断を付けない通常runではJSON生成とstdout出力を行いません。`Instant` のcheckpointと
pool状態取得もsample対象だけです。接続取得とtransaction開始を明示した処理自体は
通常runでも同じですが、commit直後に返却する境界を維持しています。

集計scriptは新しい2 phaseを表示し、fieldがない過去logはnullをpercentileへ混ぜません。
旧 `pool_begin_us` は合計として残すため、Benchmark 27との比較もできます。

レビュー修正後のDocker imageでも10秒診断を実行し、`pass=true`、4,769点、
coordinate成功sample 93件、全split fieldの集計成功を確認しました。この短走は
`Option`化したpool fieldとreportの疎通確認であり、60秒scoreの比較には使いません。

## 仮説と実際

| 仮説 | 実際 | 判断 |
|---|---|---|
| current-state UPDATEのrow lockが `pool.begin()` p95の主因 | current write p95 5.007ms、acquire p95 113.156ms | 主因ではない |
| SQL `BEGIN` 自体が遅い | BEGIN p95 2.327ms | 棄却 |
| SQLx poolのacquire phaseが遅い | 全体p95 113.156ms。size 50 / idle 0群は平均54.762ms、idleあり群は3.968ms | saturationとの関連を支持 |
| pool上限を即座に増やせばよい | MySQL側の並行実行・lock・CPUへの影響は未計測 | まだ実装しない |
| 外部決済を含む評価transactionが有力な保持元 | evaluation平均403ms・p95 769ms、実装上も決済awaitをtransaction内に含む | 次のphase計測へ進む |

## 検討する選択肢

| 選択肢 | 期待効果 | リスク・確認事項 |
|---|---|---|
| 評価の外部決済をtransaction外へ出す | 決済HTTP中のconnectionとride row lockを解放 | 二重評価、二重決済、同時再送、crash recovery、owner境界 |
| 評価を準備transactionと完了transactionへ分ける | 読取り・claim後に接続を返し、決済後だけ短いwrite | claim状態、失敗時の回収、別requestの状態遷移を設計 |
| 通知cache missのtransactionを短縮 | 高頻度経路の接続保持を減らす | 通知順序、sent cursor、stale payloadを壊さない |
| pool上限を50より増やす | app側acquire queueを短期的に減らす | MySQL CPU・thread・lock・COMMIT待ちが悪化し得る |
| pool上限を減らす | MySQLの過剰並行を抑えtailを短くできる場合がある | app側queueが増えthroughputを落とし得る |
| `min_connections` を設定 | 起動直後のconnection handshakeを減らす | 今回の主要状態は既にsize 50なので定常飽和には効かない |
| transactionを使わないreadへ分解 | 接続保持を短くできる | snapshot整合性、通知cursor、競合不変条件を再設計 |

## 次に行う計測

評価APIを少なくとも次のphaseへ分けます。

1. pool acquire
2. transaction BEGIN
3. ride row lockと事前条件確認
4. token・fare・settingsの準備
5. 決済HTTPとretry sleep
6. `COMPLETED` / chair stats / evaluationのwrite
7. COMMIT
8. cache invalidationとresponse生成

同時に次を記録します。

- payment attempt数とstatus分類
- 決済中のactive evaluation数
- connectionを保持した実時間
- pool size / idle / in-use
- 同じrideの並行評価があるか
- error / cancellationのterminal phase

決済時間がconnection holding timeの大半なら、ride IDの冪等keyを土台に、
短いclaim transaction、transaction外の決済、短い完了transactionへ分割します。
決済失敗やprocess crash後にclaimを回収できない設計は採用しません。
