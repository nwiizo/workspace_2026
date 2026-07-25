# Benchmark 51: 座標永続化をper-chair順序付きqueueへ移す

## 結論

`POST /api/chair/coordinate` のHTTP応答からMySQL transactionを外す非同期queueを実装し、
正当性fixture、30秒診断、60秒の近接A/Bを行いました。

候補の通常3走は110,353–126,964点、中央値122,125点でした。同期対照は
124,230–128,061点、中央値127,499点です。候補は対照より中央値で5,374点、
4.22%低下しました。

候補はcoordinate endpointを平均2ms、p95 7msまで短縮し、drive不満率の中央値を
58.7%から51.4%へ7.3ポイント改善しました。一方でmatching不満率は64.2%から
73.0%へ8.8ポイント悪化しました。HTTP応答を早めたことで次のcoordinateや他の処理が
早く投入され、共有DB poolとMySQLの仕事量を前倒ししたためです。

さらにDB同時実行数を24 / 40へ制限した構成やstatic pool分離も比較しましたが、
一般APIの待ちまたは座標の可視性待ちへ詰まりが移りました。得点、エラー0、3秒以内の
可視性、永続化保証を同時に改善できなかったため、queue実装は棄却して同期経路へ戻しました。

## 対象にしたTODO

Benchmark 49でshared pool 50 + general DB phase permit 26を採用した後も、
coordinateは接続取得、履歴INSERT、current投影、必要時のstatus遷移、COMMITを
HTTP request内で待っていました。

今回検証したTODOは次です。

> 接続隔離後もcoordinateが30msを超える場合は、per-chair順序、全履歴、
> status遷移、3秒可視性を維持する非同期queueを別施策として比較する

ここで「HTTPを早く返す」は実装手段であり、採用条件ではありません。採用条件は、
正当性を維持しながら最終スコアの推定代表値を改善することです。

## 変更前の同期経路

変更前は1 requestが次を順に行います。

```text
認証
  ↓
chairごとのrecorded_atを予約
  ↓
DB connection取得
  ↓
BEGIN
  ↓
chair_locationsへ履歴INSERT
  ↓
chair_current_locationsへ最新値を投影
  ↓
現在rideを確認
  ↓
pickup / destination候補だけride row lockとstatus current read
  ↓
必要ならPICKUP / ARRIVEDを1回だけINSERT
  ↓
COMMIT
  ↓
process cache更新
  ↓
HTTP 200
```

この構成はHTTP応答が遅くなる一方、clientからDBへの自然なbackpressureにもなっています。
1台の椅子は前のcoordinate応答を待ってから次の進行へ移るため、DBが遅くなると投入側も
遅くなります。

## 仮説

HTTP request taskがDB待ちを所有しなくても、同じ仕事をbounded queueのworkerが行えば、
次を期待できると考えました。

1. coordinate HTTP応答を短縮できる
2. clientの移動tickがDB transaction待ちで止まりにくくなる
3. chairごとのworker順序で位置履歴とstatus遷移を維持できる
4. bounded channelとDB in-flight上限でmemoryと接続数を制御できる
5. DB処理は消えないが、request処理と永続化の待ちを分離できる

反証条件は先に次のように置きました。

- 位置履歴の欠落または順序違反が1件でもある
- `PICKUP` / `ARRIVED` が欠落または重複する
- initialize後に前世代の座標が再出現する
- queue full、worker停止、post-ACK永続化失敗が発生する
- `recorded_at`決定からcommitまでが3秒以上になる
- ベンチマークのerror mapが空でない
- 通常3走中央値が同期対照を改善しない

## 守る必要がある不変条件

### 全座標を履歴へ残す

nearby用の最新座標だけなら、途中値を上書きして最後の1件だけ保存するcoalescingが
考えられます。しかし `chair_locations` は累積距離の正本でもあります。

```text
(0, 0) -> (10, 0) -> (0, 0)
```

この3点を最初と最後だけへ縮約すると、実際の距離20が0になります。pickupまたは
destinationと一致した中間点を捨てるとstatus遷移も失います。そのためqueue満杯時にも
古い座標を上書きせず、HTTP 500で未受理を明示する設計にしました。

### 同じchairの順序を維持する

worker poolへround-robinで投げるだけでは、同じchairのjob A / Bが別workerで並行し、
Bが先にcommitする可能性があります。これを避けるため、chair IDを安定したFNV-1aで
hashし、同じchairを必ず同じshardへ送ります。

```text
shard = fnv1a(chair_id) % shard_count
```

各shardは1つのreceiverがFIFOで処理します。異なるchairは別shardで並行でき、
同じchairだけはenqueue順に直列化されます。

Rust標準の `DefaultHasher` はprocess間で同じ配置を保証する用途には向きません。
FNV-1aは暗号学的hashではありませんが、外部入力から防御するhash tableではなく、
再現可能なqueue配置に使うため十分です。

### `recorded_at`をHTTP応答前に予約する

owner距離の順序は `(created_at, id)` です。DB workerが処理を始めた時刻を
`recorded_at`にすると、queue待ちによってclientが送った順序とDB時刻の関係が変わります。

handlerで既存のchair別high-water markを使い、

```text
max(DBと同じµs精度の現在時刻, chairの直前予約値 + 1µs)
```

をenqueue前に予約しました。workerはこの値をそのままINSERTします。

### status遷移を同じtransactionへ残す

履歴だけをqueue化し、`PICKUP` / `ARRIVED`をHTTP側で先に判定すると、
履歴と状態のcommit境界が分かれます。候補workerは同期経路と同じtransaction内で、

1. 履歴INSERT
2. current投影
3. 現在ride確認
4. 候補座標だけride row lock
5. lock後にevaluationとstatusをcurrent read
6. 条件を満たすstatusを1行だけ追加

を行いました。cacheと通知revisionはcommit後だけ更新します。

### initializeを世代境界にする

queueに前runのjobが残ったまま `init.sh` がtableを作り直すと、初期化後のDBへ古い座標が
書き戻されます。

候補では既存のmaintenance write lockを取得してからgenerationを進めました。workerは
maintenance read lock取得後にjobのgenerationを再確認し、古いjobを永続化せず捨てます。

```text
handler: generation=7でenqueue
initialize: maintenance write lock → generation=8 → DB再作成
worker: maintenance read lock → job=7 / current=8 → staleとして破棄
```

initialize開始前にcommit済みのjobは `init.sh` で消え、未処理jobはgenerationで消えるため、
初期化後へ再出現しません。

### queue満杯で待たない

初版はbounded channelの空きを `reserve().await` で待ちました。しかしworkerは
maintenance read lock、initializeはwrite lockを使います。

```text
handlerがqueue空き待ち
  ↓
initializeがhandlerのread lock解放待ち
  ↓
workerがinitializeのwrite lock後ろで待つ
  ↓
queueが減らずhandlerも進めない
```

この循環待ちを回帰で確認したため、enqueueは同期的な `try_send` へ変更しました。
満杯ならrequestを成功扱いせず500を返します。受理したjobを失うより、未受理をclientへ
明示する方が不変条件を保ちやすいためです。

## 実験実装

実験revision `43f3bf3238d4` には次を実装しました。最終revisionでは
`8e2ec1278e65` でrevertしています。

- `tokio::sync::mpsc` のbounded channel
- 128 shard、shardあたり既定64 job
- chair IDの安定hashによる同一chair FIFO
- enqueue前のULIDと単調 `recorded_at` 予約
- queue generationによるinitialize分離
- accepted / completed / failed / stale / full counter
- queue待ち、DB admission待ち、ACKからcommitまでの診断field
- optionalな全worker共通Semaphore
- queue full / worker closed / post-ACK失敗の明示log
- 位置履歴、current投影、status遷移を行うworker transaction

queueは環境変数があるときだけ有効にし、同じbinaryで同期対照を実行できるようにしました。

```sh
ISUCON_COORDINATE_QUEUE_SHARDS=128
ISUCON_COORDINATE_QUEUE_CAPACITY=64
ISUCON_COORDINATE_QUEUE_MAX_IN_FLIGHT=24
```

`MAX_IN_FLIGHT` はshard数とは別です。shard数は同じchairの順序とhead-of-line blockingを
制御し、in-flight数は同時にDB transactionへ入れるworker数を制御します。

## 正当性テスト

実験revisionで次を実行しました。

```sh
cargo fmt --manifest-path webapp/rust/Cargo.toml --all -- --check
cargo clippy \
  --manifest-path webapp/rust/Cargo.toml \
  --all-targets \
  --all-features \
  -- -D warnings
cargo test \
  --manifest-path webapp/rust/Cargo.toml \
  --all \
  --all-targets
shellcheck \
  scripts/report-coordinate-phases.sh \
  scripts/test-coordinate-write-queue.sh
./scripts/test-coordinate-write-queue.sh
```

結果はlib 47件、main 8件が成功しました。HTTP/DB統合fixtureでは次を確認しました。

| fixture | 確認内容 | 結果 |
| --- | --- | --- |
| 24座標burst | 全履歴、緯度順、currentの最終値、3秒以内の収束 | 成功 |
| pickup同一座標を2回 | 履歴2行、`PICKUP` 1行 | 成功 |
| destination同一座標を2回 | 履歴2行、`ARRIVED` 1行 | 成功 |
| 48座標burst直後にinitialize | 初期化後の旧座標0行 | 成功 |
| queue error log | full、closed、post-ACK失敗 | 0件 |

これらは固定fixture上の順序と状態機械を検証します。process crash直前にHTTP 200済みjobが
memory queueだけにある場合のdurabilityは保証しません。この残余riskも採否へ含めます。

## 30秒診断: 128 shard、DB上限なし

最初の候補は128 workerがそのままshared pool 50へ進む構成です。

```sh
ISUCON_DIAGNOSTIC=1 \
ISUCON_DB_GENERAL_PERMITS=26 \
ISUCON_COORDINATE_QUEUE_SHARDS=128 \
ISUCON_COORDINATE_QUEUE_CAPACITY=64 \
./scripts/benchmark.sh 30
```

結果は `pass=true`、62,147点、error map空でした。診断runはlog生成の負荷を含むため、
この点数を通常runの推定代表値へ混ぜません。

### coordinate HTTP

| 指標 | 値 |
| --- | ---: |
| request | 29,798 |
| 2xx | 29,798 |
| 平均 | 2ms |
| p95 | 7ms |
| p99 | 10ms |
| 最大 | 17ms |

HTTPだけを見ると明確に短縮しています。

### queueからcommit

診断sampleは466件です。

| 指標 | 平均 | p50 | p95 | p99 | 最大 |
| --- | ---: | ---: | ---: | ---: | ---: |
| queue待ち | 16.110ms | 0.032ms | 94.814ms | 239.824ms | 347.527ms |
| `recorded_at`→commit | 29.355ms | 14.540ms | 114.549ms | 259.850ms | 354.270ms |
| pool acquire | 2.828ms | 1.459ms | 8.663ms | 15.369ms | 20.078ms |

3秒超、queue full、worker停止、post-ACK失敗は0件でした。sampled shardの最大depthは21 / 64です。

### general側

一般DB phaseの周期824 sampleはpermit待ちp95 219.644ms、p99 295.673msでした。
coordinate sampleではshared pool 50 / idle 0が217 / 466、general admission sampleでは
50 / idle 0が273 / 824です。

coordinateのHTTP待ちは消えましたが、DBの仕事自体は残っています。さらにHTTP 200が
早くなったことでclientが次の仕事を早く投入し、shared pool全体は飽和していました。

## 通常60秒の近接A/B

診断を無効にし、候補と同期対照を次の順で実行しました。

```text
候補1 → 対照1 → 対照2 → 候補2 → 候補3 → 対照3
```

実行順を反転するのは、時間経過によるDocker cache、温度、ホスト上の外乱を片側だけへ
寄せないためです。Colimaは全runで4 CPU / 4 GiBのまま変更していません。

| 組 | 候補 | 同期対照 | 候補 - 対照 |
| ---: | ---: | ---: | ---: |
| 1 | 126,964 | 124,230 | +2,734 |
| 2 | 110,353 | 128,061 | -17,708 |
| 3 | 122,125 | 127,499 | -5,374 |
| 中央値 | 122,125 | 127,499 | -5,374 |

全6走は `pass=true`、error map空です。候補は3組中2組で下回り、中央値は4.22%低下しました。

### revert後の最終ソース確認

queue実装をrevertしたソースを改めてDocker imageへbuildし、環境変数なしで60秒runを
1回実行しました。結果は `pass=true`、120,343点、error map空です。最終不満率は
matching 69.1%、pickup 29.9%、drive 59.2%でした。

これは近接A/Bの6走より後に行った単発の配布状態確認なので、同期対照の中央値127,499点へ
混ぜません。実験用binaryを使い続けず、queue関連コードを除去した最終ソースがbuildされ、
公式ベンチを完走したことの確認に使います。

### 不満率の中央値

| 不満率 | 候補 | 同期対照 | 差 |
| --- | ---: | ---: | ---: |
| matching待ち | 73.0% | 64.2% | +8.8pt |
| pickupまで | 33.4% | 32.4% | +1.0pt |
| drive実移動 | 51.4% | 58.7% | -7.3pt |

仮説どおり、coordinateのHTTP短縮は実移動を改善しました。しかし増えた負荷でmatcherや
一般APIが相対的に遅くなり、乗車中距離と完了件数から得る価値を相殺しました。

これは「endpoint latencyが改善した」という局所目的と、「ISUCONの総得点が上がる」という
全体目的が一致しなかった例です。

## DB同時実行数を制限した追加診断

128 workerがpool取得待ちへ一度に入ることを避けるため、全worker共通Semaphoreを追加し、
24 / 40 in-flightを比較しました。さらに既存のstatic 26 / 24 pool分離も試しました。

| 構成 | score | error map | coordinate visibility | general admission p95 | 判断 |
| --- | ---: | --- | --- | ---: | --- |
| shared、128相当 | 62,147 | 空 | p99 259.850ms、最大354.270ms | 219.644ms | 通常中央値-4.22% |
| shared、24 | 54,947 | `26:1` | p99 621.757ms、最大919.802ms | 118.110ms | 正当性errorで棄却 |
| shared、40 | 39,558 | 空 | p99 283.614ms、最大1.383秒、1秒超1件 | 199.006ms | 可視性とgeneral待ちで棄却 |
| static 26 / 24、24 | 42,518 | 空 | p99 1.240秒、最大2.059秒、1秒超7件 | separate pool | coordinate側飽和で棄却 |

24 shardだけに減らした初期案も53,176点、`CODE=26` 6件でした。queue待ちp99は
1.474秒、最大2.401秒です。同じchairだけでなくhash衝突した別chairも1 receiverへ並ぶため、
shardを減らすとhead-of-line blockingが増えました。

### 24 in-flightで何が起きたか

24本へ制限すると、queue admission待ちp95は18.286ms、queue待ちp95は299.688msでした。
general admission待ちp95は128相当の219.644msから118.110msへ短縮しましたが、
shared pool 50 / idle 0は周期sample 824件中283件でした。

coordinateを抑えただけでは、HTTP 200で解放されたclientが生む通知、nearby、ride履歴、
評価などの一般処理も増えます。`coordinate 24 + general permit 26 = 50` は同時実行の
上限を説明しますが、burst時にpoolを空ける保証ではありません。

### 40 in-flightで何が起きたか

24と128の中間なら両側の待ちを均衡できると考えました。しかしgeneral admission待ちp95は
199.006msへ戻り、座標の最大可視性も1秒を超えました。中間値だから中間の総合結果になる
わけではありません。closed-loop workloadではresponse時間が次の到着率を変えるためです。

### static poolで何が起きたか

general 26とcoordinate 24を別poolへ分けると一般処理はcoordinateから隔離されます。
一方、coordinate poolは24 / 24使用が150 / 344 sampleとなり、`recorded_at`からcommitまで
1秒超が7 sample、最大2.059秒になりました。

接続を隔離すると、一方のidle connectionを他方へ融通できません。Benchmark 49で
static partitionをshared poolへ戻した理由が、非同期queueでも再現しました。

## どのログを見て、どう判断したか

| ログ・集計 | 見た値 | 判断 |
| --- | --- | --- |
| benchmark最終行 | pass、score、error map | 採用判断の最上位 |
| benchmark不満率 | matching / pickup / drive | scoreが動いた方向を分解 |
| nginx endpoint集計 | coordinate count、平均、p95 / p99、HTTP status | request経路の局所改善 |
| `COORDINATE_DIAGNOSTIC` | queue depth、queue待ち、admission待ち、各SQL、commit | 遅延がどの境界へ移ったか |
| `DB_ADMISSION_DIAGNOSTIC` | general permit待ちとpool状態 | coordinate短縮の一般API副作用 |
| MySQL 1秒status | Threads connected / running、row lock、Questions | app pool外のDB実行競合 |
| queue error log | full、closed、post-ACK persistence failure | 成功応答済みデータ損失の兆候 |
| HTTP/DB fixture | 履歴順、current、PICKUP / ARRIVED、世代分離 | benchmarkだけでは固定できない競合 |

ログは個別に結論を出すためではなく、因果の鎖を確認するために使いました。

```text
coordinate HTTPが短縮
  ↓
clientが次の処理を早く投入
  ↓
shared poolが50 / 50になりgeneral permit待ちが増える
  ↓
driveは改善するがmatching待ちが悪化
  ↓
通常3走中央値が低下
```

この鎖はendpoint分布、pool状態、不満率、通常A/Bが同じ方向を示しています。

## なぜpost-ACK queueはdurabilityが弱いか

HTTP 200はclientにとって「requestが受理された」境界です。しかしmemory queue版は、

```text
enqueue成功 → HTTP 200 → process停止 → DB未commit
```

の順になると座標を失います。通常runでpost-ACK persistence errorが0件でも、
process crashを起こしていないだけで、この順序自体は消えません。

at-most-once / at-least-onceという言葉だけで考えると曖昧になります。今回必要なのは
次の対応です。

| 境界 | memory queue候補 |
| --- | --- |
| HTTP 200前にDBへ永続化 | しない |
| process crash後に未処理jobを復元 | できない |
| 同じjobをretryしても履歴重複しない | location IDをclientが再送しないため保証しない |
| worker errorをclientへ返す | HTTP応答後なので返せない |

競技中にprocessを再起動しない前提を置くことはできますが、今回は得点も改善していないため、
durability riskを受け入れる理由がありません。

## 他に考えられる選択肢

### 同期経路の仕事量を減らす

今回採用した判断です。HTTPから仕事を隠すのではなく、履歴INSERT、current投影、
ride lookup、COMMIT、row lockの実行回数と保持時間を減らします。正本のcommit後に200を返す
境界を維持できます。

### durable outbox

HTTP transactionでjobをoutbox tableへINSERTし、別workerが本処理します。process crash後も
復元できますが、coordinate履歴自体をINSERTするのとDB往復が近く、outboxのclaim、
retry、重複排除、掃除が増えます。今回の局所目的には重すぎます。

### handlerがworker commitを待つ

oneshot channelでworker結果を待てばpost-ACK lossを防げます。ただしHTTP latencyは
DB commitに再び依存します。per-chair順序制御によってlock競合を減らせる可能性はありますが、
今回の同期handlerも同じchairのclient順序でかなり直列化されています。

### overload時だけ同期で待つ

queue depthが閾値を超えたとき、そのjobのcommitまでHTTP応答を待つadaptive backpressureです。
低負荷では速く、高負荷では投入を抑えられます。ただし閾値、hysteresis、failure通知、
process crash境界が増えます。今回の通常A/Bが負であるため、パラメータだけを増やして
追跡しません。

### 履歴のbulk INSERT

複数chairの履歴をまとめればSQL往復やcommit回数を減らせます。しかしbatch内で
chairごとのcurrent投影、pickup / destination一致、ride lock、status遷移が必要です。
最古jobのflush期限と部分失敗も設計する必要があります。履歴だけをbulk化して状態更新と
transactionを分けると正当性境界が増えます。

### 最新値だけcoalesce

nearby用cacheには有効ですが、正本履歴には使えません。累積距離とstatus一致を壊すため、
`chair_locations`の代替にはしません。

### DB側の差分集約

owner距離の読み取りを軽くする目的なら、全履歴は保存したままcommit時にchair別累積距離を
差分更新する選択肢があります。queueのHTTP境界とは別施策として、安定化watermark、
同時刻tie、rollback、initialize backfillを検証する必要があります。

## 採否

非同期queueは不採用です。

理由は次の4点です。

1. 通常3走中央値が同期対照より4.22%低い
2. 3組中2組で候補が下回る
3. drive改善よりmatching悪化が大きい
4. post-ACK memory queueのdurability riskを受け入れる得点根拠がない

実験実装を `43f3bf3238d4`、revertを `8e2ec1278e65` と分けたため、将来DB仕事量を
大きく減らした後に同じ仮説を再検証できます。ただし次回はこの結果を無視して
shard数だけを再探索せず、到着率、一般API待ち、永続化境界を一緒に計測します。

## 次のTODO

queueで待ちを移すのではなく、現在の同期経路と共有DBの仕事量を直接減らします。
優先候補は次です。

1. `app_get_rides` のrideごとのstatus / coupon / chair / owner N+1
2. `app_post_rides` のuser全履歴とride別最新status走査
3. `app_get_nearby_chairs` のride antijoinとtracker処理のphase分解
4. matcherの64件batchを、割当件数・期限・pickup予測の辞書順目的で比較

今回の結果から、単一endpointを速くする施策でも、その応答が次のrequestを開始させる場合は、
全endpointの到着率と得点構造まで測る必要があります。
