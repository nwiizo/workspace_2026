# Benchmark 40: drive区間を同一ride IDでphase分解

## 結論

drive評価を落としていた主因は、matcherの距離選択ではなく、走行中の
`POST /api/chair/coordinate` がDB connection poolの空きを待つ間に椅子の次の移動tickが
止まることでした。

最終診断1走は `pass=true`、146,727点、error map空でした。
計測コードを有効にした1走なので推定代表値には使いません。ベンチマーカーが直接記録した
2,310完了rideでは、drive評価だけの不満率は77.3%でした。ride IDを1/32で選んだ
相関可能な74件では、drive tickを増やし得る1,191回のcoordinate POSTのうち1,031回、
86.6%がclientから見て30ms以上でした。失敗POSTは0件です。

```text
coordinate client平均        106.873ms
coordinate server平均         76.515ms
server内 pool.acquire()平均    64.349ms
server内 current UPDATE平均     2.681ms
```

client / server平均はrequest単位同士です。server時間の約84.1%がSQL実行前のpool待ちです。
抽出74 rideの実際の余分tickは合計3,718、coordinate応答時間から見積もったblocked tickは
3,678で、差は40tick、約1.1%でした。
上位失敗rideでも、余分tickとblocked tick見積りがほぼ同じです。

この結果から、次はDB接続数を無条件に増やさず、総接続上限50の中でcoordinate用接続を
予約し、通知・評価・matcherのburstから走行を隔離する仮説を検証します。

## 今回確認した問い

Benchmark 39では、chair modelのspeedをmatcherへ入れてpickup予測tickを最小化しましたが、
通常3走中央値は0.9%下がり、pickup不満率もほぼ変わりませんでした。一方、drive不満は
毎走約70%残りました。

次の問いを分けて確認しました。

1. driveの理想tickと実tickは同じrideで何tickずれるか
2. `PICKUP` 座標のcommitからapp / chair通知handlerがresponseを作るまで何msか
3. app通知handlerのresponse構築からchairが `CARRYING` をcommitするまで何msか
4. 走行中のcoordinate POSTはclientから何msに見えるか
5. coordinate handler内のpool、SQL、COMMITのどこが長いか
6. `ARRIVED` commit後にapp / chair通知handlerがresponseを作るまで何msか

通知の時刻はserver内で観測した境界です。response bodyがsocketへ書かれた時刻でも、
clientが受信・decodeした時刻でもありません。「配送」「受信」とは呼ばず、
`response built` として扱います。

スコアだけでは、椅子が遠いのか、通知待ちなのか、DB接続待ちなのかを区別できません。
今回の目的は高速化を入れることではなく、次の実装対象を一つに絞ることです。

## 公式ベンチマーカーのdrive判定

`bench/benchmarker/world/request.go` は次の値を使います。

```text
ideal = ceil(manhattan_distance(pickup, destination) / chair_speed)
actual = ArrivedAt - PickedUpAt
pass   = actual - ideal < 5
```

余分な時間が4tickまでなら合格、5tick以上なら不満です。1tickは30msなので、単純換算では
150msの余分な待ちが不合格境界です。ただし、HTTP応答時間とtick差をそのまま
`wall time / 30ms`だけで決められない点に注意が必要です。複数chairが同時に動き、
world全体は30msごとに進みます。個々のchairがHTTP待ち中なら、そのchairだけ次の
`Tick`を実行できません。

最終ログの「椅子の実移動時間に不満」という表示は、実装上、pickupとdriveの2評価を
合算した割合です。

```go
1 - (pickup_pass + drive_pass) / (completed * 2)
```

今回の最終ログは72.7%でしたが、診断JSONからdriveだけを数えると77.3%です。
計算対象が違うため、矛盾ではありません。今後drive施策を比較するときは、最終表示だけで
なく `drive_pass` を独立集計します。

## なぜcoordinate POSTが移動tickを止めるのか

ベンチマーカーのchairは、1tickで次の順に処理します。

```text
現在地からspeed分だけ移動する
  -> Locationをdirtyにする
  -> POST /api/chair/coordinate
  -> responseを受け取る
  -> dirtyを解除する
```

`Chair.Tick` は処理中に `tickDone` を保持します。coordinate POSTが返らないまま次の
world tickになっても、同じchairはそのtickの移動をskipします。そのため、距離とspeedから
必要な移動回数が15回でも、各POSTで1–3tick止まれば、`ArrivedAt - PickedUpAt` は
15を大きく超えます。

この関係を直接確認するため、client側の各coordinate POSTへ次を記録しました。

- ride IDとchair ID
- POST開始時のworld tick
- `CARRYING` / `ARRIVED` などのchair状態
- 座標
- clientから見たrequest時間
- 成否

走行中のPOST attemptは成否にかかわらず
`picked_up_tick < world_tick < arrived_tick` で同じrideだけに絞りました。
ベンチマーカーは目的地へ移動したtickで `ArrivedAt` を先に確定し、その後で最終座標を
POSTします。したがって `world_tick == arrived_tick` の最終POSTはserverのARRIVED遷移には
必要ですが、その待ち時間は確定済みのdrive tickを増やしません。旧集計の `<=` はこの
POSTまでblocked tickへ足していたため修正しました。

## 計測設計

### 全rideのベンチマーカー評価

完了した全rideへ `DRIVE_BENCHMARK_DIAGNOSTIC` を出します。

```json
{
  "ride_id": "...",
  "chair_speed": 5,
  "distance": 60,
  "ideal_drive_ticks": 12,
  "actual_drive_ticks": 99,
  "excess_drive_ticks": 87,
  "drive_pass": false,
  "picked_up_tick": 1770,
  "arrived_tick": 1869
}
```

理想値をDB時刻から推測せず、採点に使うベンチマーカー自身のtickを正本にしています。

### 1/32 rideの全イベント追跡

全coordinateを常時出力するとログI/O自体が負荷になります。ride IDをFNV-1aで32 bucketへ
分け、bucket 0だけを追跡します。Rust serverとGo benchmarkerへ同じhashのテストを置き、
同じrideを選ぶことを確認しました。

選択したrideでは次を出力します。

| prefix | 内容 |
|---|---|
| `COORDINATE_CLIENT_DIAGNOSTIC` | clientから見たcoordinate POST時間とworld tick |
| `COORDINATE_DIAGNOSTIC` | pool取得、BEGIN、履歴INSERT、current UPDATE、ride検索、遷移、COMMIT |
| `RIDE_STATUS_DIAGNOSTIC` | `CARRYING` POSTのpool取得、ride lock、status INSERT、COMMIT |
| `NOTIFICATION_DIAGNOSTIC` | app / chairのpoll responseへPICKUP、CARRYING、ARRIVEDを載せたserver request |

既存のcoordinate / notificationレポートは1/64周期サンプルを母集団としていました。
ride追跡を同じ集計へ混ぜると、走行中のrequestへ偏ります。そこで各JSONへ
`periodic_sample` と `trace_ride` を分け、従来レポートは周期サンプルだけを使います。
`jq` の `false // true` は `true` になるため、field欠落との後方互換を
`// true` で書いてはいけません。現在は `has("periodic_sample")` でfield有無を先に判定し、
true / false / fieldなしのfixtureを `scripts/test-diagnostic-filters.sh` で固定しています。
同じtestでpickup tick、走行中tick、arrived tick、失敗POSTを並べ、走行中なら失敗attemptも
残す境界を固定しました。失敗attemptとretry backoff中もchairのtickは止まるためです。
server側は相関ride、成功handler phase、CARRYING / ARRIVEDの厳密なcommit区間をfixtureで
固定し、milestoneはcommit後のhandler cancellationでも失わないことを別fixtureで確認します。

### 診断専用であること

`ISUCON_DIAGNOSTIC=1` のときだけ計測します。通常構成では、FNV選択、全ride JSON、
全coordinate client時間、強制traceは実行しません。診断overlayはbenchmark containerにも
同じ環境変数を渡します。

高頻度JSONはrequest / tick goroutineからstdoutへ同期書き込みせず、channelへ渡して
専用writerが出力します。Rust側writerは1行ごとにstdout lockを解放します。最初の実装は
channel待機中もlockを保持したため通常のtracing出力と競合し、rideが進まず失敗しました。
この失敗は後述し、性能値には混ぜていません。

Rustのchannelは16,384行のbounded queueで、request側は `try_send` します。満杯なら
欠落数を増やし、report開始時のflush endpointがbarrier以前の書き込み完了と欠落数を返します。
欠落が非0ならreportは停止します。Go側もValidation終了前にFIFO barrierを待つため、
process終了時の末尾ログを未flushのまま扱いません。

## 実行方法

ベンチ出力にはベンチマーカー側のtickが含まれるため、再利用できるファイルへ保存します。

```sh
diagnostic_since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
benchmark_log=$(mktemp /tmp/isucon14-drive.XXXXXX)

ISUCON_DIAGNOSTIC=1 \
BENCHMARK_OUTPUT_FILE="$benchmark_log" \
./scripts/benchmark.sh 60

./scripts/report-drive-phases.sh \
  "$diagnostic_since" \
  "$benchmark_log"
```

今回の境界は次のとおりです。

```text
診断開始:         2026-07-25T03:23:50Z
MySQL起動:        2026-07-25T03:24:21Z
最初の座標sample: 2026-07-25T03:24:54.485524745Z
```

MySQLは診断run用に再起動されており、InnoDB累積値に前回runは混ざっていません。

## 計測器が原因で失敗したrun

非同期writerの初版は `std::io::StdoutLock` を取得してからchannelを待ち続けました。
Rustの通常ログも同じstdout lockを必要とするため、Tokio worker上のloggingが停止し、
API全体が進まなくなりました。

```text
診断開始: 2026-07-25T03:06:53Z
結果:     pass=false、0点
error:    CODE=25 15件、CODE=32 10件
症状:     完了ride 0、最初の診断JSON直後からwebapp logが停止
```

writerを1行ごとに `lock -> write -> flush -> unlock` へ直すと、同じホスト設定で
`pass=true`、最終runは2,310完了rideへ戻りました。したがって0点runはアプリの性能比較ではなく、
計測器の故障として扱います。非同期化は「待たない」だけでなく、共有出力lockを
`.await`相当のchannel待機区間へ持ち越さないことが必要です。

## ベンチマーカーの評価結果

| metric | samples | avg tick | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| 理想drive | 2,310 | 15 | 12 | 42 | 50 | 52 |
| 実drive | 2,310 | 55 | 38 | 176 | 264 | 389 |
| 余分なdrive | 2,310 | 39 | 22 | 144 | 225 | 337 |

| 完了ride | drive合格 | drive不合格 | drive不満率 |
|---:|---:|---:|---:|
| 2,310 | 523 | 1,787 | 77.3% |

理想の中央値12tickに対し、実値は38tick、余分な時間は中央値22tickです。5tick境界を
少し超えているのではなく、通常のrideでも大きく超えています。

## 同一rideを結合した結果

| ride | drive不合格 | coordinate POST | 失敗POST | 30ms以上 | 余分tick | blocked tick見積り |
|---:|---:|---:|---:|---:|---:|---:|
| 74 | 65 | 1,191 | 0 | 1,031 | 3,718 | 3,678 |

blocked tickは各POSTについて次を合計しました。

```text
max(0, ceil(client_duration_us / 30,000) - 1)
```

POSTの完了までに30msを超えるたび、次の移動機会を何回失うかの近似です。実際の
余分tickより40tick、約1.1%少ない値でした。world内の並行実行、30ms境界の位相、
networkとschedulerの揺れがあるため完全一致は期待しません。それでも、3,718に対して
3,678まで一致することは、coordinate応答待ちが主要因という仮説を強く支持します。
失敗したPOST自体の時間は含めますが、attempt間のretry backoffはこのdurationに入りません。
今回は失敗POSTが0件なので影響しませんが、失敗があるrunではblocked見積りを下限値として
扱い、失敗件数と併読します。

### clientから見たcoordinate

| metric | sample | avg µs | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| request | 1,191 | 106,873 | 95,510 | 240,737 | 324,078 | 611,029 |
| ride内平均 | 74 | 117,069 | 117,191 | 230,824 | 284,562 | 346,467 |
| ride内最大 | 74 | 186,831 | 166,349 | 372,024 | 573,528 | 611,029 |

上位失敗例:

| ideal | actual | excess | POST数 | 30ms以上 | blocked見積り | 最大POST |
|---:|---:|---:|---:|---:|---:|---:|
| 50 | 320 | 270 | 49 | 49 | 273 | 275.511ms |
| 52 | 272 | 220 | 50 | 50 | 216 | 350.020ms |
| 35 | 251 | 216 | 34 | 34 | 217 | 573.528ms |
| 38 | 166 | 128 | 37 | 37 | 127 | 198.218ms |

1行目は49回すべてが30msを超え、余分270tickに対してblocked見積り273tickでした。
距離やspeedをmatcherで選び直しても、移動の各POSTがこの時間止まる限りdrive評価は
改善しません。

## server内coordinate phase

相関rideだけを `CARRYING` commit後から `ARRIVED` commitより前までのwall-clock範囲で
抽出しました。成功したDB commitだけを対象にし、最終ARRIVED POSTを含みません。
client側1,191件に対してserver側は1,195件です。Go側はValidationのbarrier完了後に
processを終了し、Rust側はreport開始時に `dropped_lines=0` とflush完了を確認済みです。
したがって末尾未flushではありません。両者には共通request IDがないため
rideと区間で相関しており、完全な1対1 joinとは扱いません。

| phase | avg µs | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|
| pool acquire | 64,349 | 64,521 | 133,390 | 161,403 | 302,883 |
| transaction BEGIN | 699 | 353 | 2,761 | 4,560 | 11,787 |
| 履歴INSERT | 1,104 | 462 | 3,939 | 10,776 | 36,442 |
| current UPDATE | 2,681 | 845 | 8,215 | 43,566 | 116,066 |
| current ride検索 | 1,276 | 833 | 3,718 | 7,387 | 24,896 |
| COMMIT | 6,392 | 5,228 | 15,219 | 21,540 | 40,727 |
| handler全体 | 76,515 | 76,122 | 152,194 | 184,452 | 317,593 |

pool待ちはserver全体平均の84.1%です。current UPDATEを2.681msから半分にしても、
pool待ち64.349msが残れば30msを切れません。

周期サンプル985件でも、pool size 50 / idle 0が770件、78.2%でした。その状態の
pool acquireは平均61.395msです。ride追跡だけの偏りではなく、coordinate全体で
接続枯渇が起きています。

fresh MySQL process lifetimeでは行lock待ち4,243回、累積91.976秒、平均約21msでした。
これは全endpointの累積であり、coordinateや通知へ帰属させる時刻相関は取っていません。
lock待ちがconnection保持を延ばしpool待ちへ寄与する説明とは整合しますが、因果の証明では
ありません。次の比較では処理件数で正規化したrow-lock、`Threads_running`、
pool別connection所有時間を併記します。

## CARRYING statusと通知

### `POST .../status` のCARRYING

| phase | samples | avg µs | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| pool acquire | 84 | 65,725 | 56,672 | 132,460 | 270,026 | 290,309 |
| transaction BEGIN | 84 | 888 | 381 | 1,877 | 4,921 | 24,349 |
| ride lock | 84 | 691 | 482 | 2,111 | 4,160 | 6,115 |
| status write | 84 | 2,850 | 1,937 | 6,739 | 12,419 | 18,487 |
| COMMIT | 84 | 5,486 | 4,538 | 12,275 | 14,433 | 14,978 |
| handler全体 | 84 | 75,712 | 66,733 | 143,722 | 293,587 | 314,117 |

こちらもpool待ちが支配的です。phase表は成功してcommitしたCARRYINGだけを使います。
pool取得失敗、SQL error、cancellationは `outcome` と `terminal_phase` の別表へ出します。
今回の追跡範囲では失敗・cancellationは0件でした。

### 同じrideのmilestone間隔

| 区間 | ride | avg ms | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|---:|
| PICKUP commit → app response built | 74 | 186 | 187 | 328 | 335 | 351 |
| PICKUP commit → chair response built | 74 | 198 | 194 | 351 | 362 | 453 |
| app response built → CARRYING commit | 74 | 153 | 133 | 307 | 394 | 400 |
| CARRYING commit → app response built | 74 | 86 | 70 | 220 | 351 | 422 |
| CARRYING commit → chair response built | 74 | 143 | 133 | 331 | 380 | 380 |
| CARRYING commit → ARRIVED commit | 74 | 2,134 | 1,729 | 4,545 | 8,198 | 9,819 |
| ARRIVED commit → app response built | 74 | 216 | 222 | 378 | 420 | 463 |
| ARRIVED commit → chair response built | 74 | 220 | 225 | 380 | 385 | 508 |

pickup前後と到着後の通知にも100–200ms級の遅延があります。ただしdrive評価の
`PickedUpAt` はCARRYING POST成功後、`ArrivedAt` はbench内で目的地へ移動したtickに
設定されます。driveの余分tickへ直接効く区間は、主にCARRYING後の各coordinate POSTです。
通知改善もthroughputと次状態への移行には重要ですが、drive評価の最初の実装対象は
coordinate pool待ちです。通知の境界はserver response構築なので、network配送時間を
含まず、client受信までの遅延を過小評価し得ます。

通知周期サンプルでは、rideありのconnection所有はapp平均10.993ms、chair平均12.706ms
でした。一方、各通知requestは存在確認とtransaction開始でpoolを2回取得し、その平均は
app 47.720 + 49.757ms、chair 49.868 + 51.591msです。通知自身もpool待ちの被害を受け、
同時に2回の取得要求でpool待ち行列へ戻っています。

## 仮説はどう変わったか

### 計測前

- speedが遅いchairを選んでいるためdriveも遅い可能性
- `CARRYING` や `ARRIVED` の通知 pollingが主な5tick超過かもしれない
- current-state UPDATEのrow lockがcoordinateを止めている可能性

### 計測後

- driveのidealはspeedを含むため、speedだけでは実値との差を説明できない
- drive区間のclient coordinate request平均106.873msが30msを大きく超える
- server時間の84.1%はpool取得で、current UPDATE平均2.681msは第一の律速ではない
- blocked tick見積りが実際の余分tickへ近く、coordinate応答待ちが直接原因
- 通知も遅いが、driveの `PickedUpAt -> ArrivedAt` へ直接積み上がるのは走行中POST

## 次に比較する実装

### 1. 総接続数50を用途別に分ける

最初の候補はgeneral 34 + coordinate 16です。総接続数は50のままなので、
Benchmark 33で不採用にした「全endpointが同時に75 / 100接続までDBへ入る」構成とは
異なります。

16の根拠は、nginxでcoordinate POSTが62,983回、負荷区間で概ね毎秒1,050回あり、
pool待ちを除いた追跡coordinateのserver滞在が平均約12.2msだったことです。
Littleの法則による平均同時実行は概算12.8 connectionです。ただし、これは平均到着率と
平均滞在時間の積であり、burst、tail、隔離後に増える到着率を保証しません。16は採用値では
なく、平均へ小さい余白を置いた最初の検証値です。

採否では次を同時に見ます。

- coordinate client p95 / 30ms超過率
- coordinate pool acquire p50 / p95
- drive不満率と完了ride数
- general poolへ移した通知、matching、評価のp95
- coordinate / general各poolのsize、idle、acquire、connection所有
- MySQL総接続数、`Threads_running`、処理件数で正規化したInnoDB row-lock wait
- 通常3走のスコア中央値とerror map

固定partitionではgeneralがidleでもcoordinateは借りられず、その逆も起きます。
general側が34でstarvationするなら、単に総上限を増やす前に次を比較します。

- 共有pool 50のままgeneral取得だけを34 permitへ制限し、coordinateへheadroomを残す
  admission control
- 通知の二重pool取得を減らしてgeneral側の需要を下げる
- static 2-poolの予約数を測定値から調整する

2-poolを試す場合は通常handlerだけでなく、initialize、reconciliation、matcherを含む
全接続先を一覧化し、どちらへ属するかを明示します。

### 2. 通知の二重pool取得を安全に除く

通知は最初の存在確認connectionを捨て、もう一度poolを取得してtransactionを開始します。
Benchmark 35ではconnectionをそのまま引き継いだ版が配送状態の問題と同時に入り、
`CODE=26/29`で失格しました。chair ride選択はその後Benchmark 36で修正済みです。

現在の配送状態機械を維持したまま接続再利用だけを再検証する余地があります。ただし、
connectionを長く保持して別endpointの待ちを増やす可能性があるため、drive専用poolと
一度に混ぜません。

### 3. coordinateをprocess queueへ入れる

HTTPをDB commit前に返せば、clientの移動停止を最も短くできます。一方で次をすべて
維持する必要があります。

- chairごとの座標順序
- 全中間座標を使う累積距離
- pickup / destination一致時の状態遷移
- responseで返した `recorded_at` とDB可視性
- owner / nearbyの3秒以内反映
- process crash時の未flush座標
- initialize中のqueue世代分離

効果は大きい可能性がありますが、正当性riskも最大です。まず接続隔離という小さい変更で
30ms超過率が下がるか確認し、残るserver内部時間が11ms程度ならqueue化を後順位にします。

### 4. current UPDATEだけを速くする

平均2.681msなので、INDEXやSQL書換えだけでは64.349msのpool待ちを消せません。
row-lock p99は無視できませんが、第一施策にはしません。専用pool後にpool待ちが下がり、
current writeが新しいp95律速として現れた場合に再検討します。

### 5. pool上限を増やす

50 / 75 / 100の通常3走では、中央値が107,234 / 105,867 / 103,720点でした。
上限を増やすほどDB内競合が悪化したため、今回の結果だけで75へ戻しません。
「待っているから接続を増やす」ではなく、厳しい5tick制約のcoordinateへ既存予算を
予約する比較にします。

## 実装上の学び

- benchmarker内部のtickを出さないままwall-clockだけで評価原因を推定しない
- high-cardinalityな全イベントを出さず、ride単位で決定的にsamplingする
- 周期サンプルと原因追跡サンプルを同じpercentileへ混ぜない
- `jq` の `//` はnullだけでなくfalseもfallbackするため、boolean fieldの既定値には
  `has(...)` を使う
- server時間だけでなくclient observed timeを取る
- DB commit、server response構築、client受信を別の時刻境界として命名する
- pool待ちとDB内部のrow-lock待ちは別の待ちであり、同時刻相関なしに因果を断定しない
- 非同期log writerはchannelだけでなくstdout lockの保持範囲も確認する
- handler内の最長SQLを削る前に、request時間のどの割合を占めるか確認する

Rust側の診断実装、`OnceLock`、決定的sampling、Drop時のログ扱いは
[`80-rust-implementation.md`](./80-rust-implementation.md)にも分離して記載します。
