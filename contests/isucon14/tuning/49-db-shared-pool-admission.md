# Benchmark 49: shared DB poolとgeneral admission control

[チューニング目次へ戻る](../TUNING.md)

## 目的

Benchmark 44では、SQLxの総接続予算50をgeneral 26本とcoordinate 24本へ静的に
分割しました。`POST /api/chair/coordinate`が通知、評価、matcherのburstに
50本すべてを奪われないための隔離です。一方、片方のpoolに空きがあっても、
もう片方はその接続を借りられません。

そこで次の2構成を比較します。

```text
static partition
  general pool:    最大26
  coordinate pool: 最大24
  合計:            最大50

shared + admission
  shared pool:     最大50
  general permit:  最大26
  coordinate:      permitなし
```

shared構成ではgeneral処理が同時に占有できるDB phaseを26以下へ制限します。
そのため、generalだけで50接続を使い切ることはありません。coordinateが少ない瞬間は、
余っている接続を共有できる可能性があります。

## 結論

shared 50 + general permit 26を採用し、環境変数を省略した既定構成にしました。
診断なし60秒の近接A/Bを実行順`static → shared`、`shared → static`、
`static → shared`で3組取り、すべての組でsharedが上回りました。

| 構成 | scores | 中央値 | 範囲 | pass / error |
| --- | --- | ---: | ---: | --- |
| static 26 / 24 | 112,512 / 126,104 / 129,655 | 126,104 | 112,512–129,655 | 全走true / 空 |
| shared 50 + permit 26 | 130,167 / 141,586 / 135,410 | 135,410 | 130,167–141,586 | 全走true / 空 |

中央値差は+9,306点、`9,306 / 126,104 = +7.38%`です。組ごとの差も
+17,655、+15,482、+5,755点で、実行順を反転しても同じ方向でした。

Benchmark 48の過去中央値141,228点より今回のshared中央値は低いものの、計測中の
ホスト負荷条件が異なる値を直接引き算しません。今回の採否は同じsource、同じ時間帯で
交互に実行したstatic / sharedを主根拠にしています。

既定値へ切り替えた最終sourceを環境変数なしで追加確認し、131,963点、
`pass=true`、error map空でした。起動logは
`total=50 general=26 coordinate=24 shared=true`で、比較3走のshared範囲内です。
この追加runは設定配線の確認であり、事前に決めた3走中央値へ後付けで混ぜません。

## 仮説

### 期待した効果

- coordinateのための24接続分の余力をgeneral burstから守る
- coordinateが少ない瞬間はstatic general poolの上限26を超えて接続を融通する
- 総接続数50を維持し、過去の75 / 100接続で見たMySQL競合の悪化を避ける

### 採用前に確認すべき危険

- permitを通らないgeneral DB取得が1か所でもあれば、24本分の余力保証に穴が開く
- permitをHTTP request全体で持つと、DBを使わない時間まで26枠を塞ぐ
- shared SQLx poolの取得待ちはFIFOであり、用途別priority queueではない
- coordinateはgeneral permitを使わないため、coordinate burstがgeneralの余力まで
  使う方向の逆向き隔離はない
- `Pool::clone()`は同じpoolへのhandleを増やすだけで、別のpoolや待ち行列を作らない

## はじめに知っておく用語

### connection pool

DB connectionをあらかじめ保持し、queryごとに借りて返す仕組みです。
SQLxの`max_connections(50)`は「常に50本実行する」という意味ではなく、
そのpoolが同時に保持できる上限です。全connectionが貸出中なら、
次の`acquire()`は返却を待ちます。

### admission control

高価な処理へ入る前に、同時実行数を制限する入口です。今回はTokioの
`Semaphore`を使い、general DB phaseへ入れるtaskを26個までにします。

```text
general task
  -> permitを待つ
  -> DB connectionを待つ
  -> SQL / transaction
  -> connectionを返す
  -> permitを返す
```

permitはDB connectionではありません。permitを取得しても、その後にSQLx poolの
connection待ちが発生し得ます。この2段階を別々に測らないと、待ち場所を誤認します。

### semaphore permit

Semaphoreが発行する同時実行枠です。上限26なら27番目のtaskは、先行taskがguardを
dropするまで待ちます。Rustでは`OwnedSemaphorePermit`を変数に保持し、
DB phase終了時にdropすることで返却します。

### headroom

急な負荷へ対応するため、意図的に使い切らず残す余力です。generalがpermitを厳守して
DB phaseを最大26個に抑えれば、総数50との差24はgeneralだけでは消費できません。
ただし、これは「coordinateが常に待たない」という保証ではありません。24個以上の
coordinate自身が同時実行中なら、coordinate同士の待ちは残ります。

### request scopeとDB phase scope

request scopeは認証開始からresponse完了まで、DB phase scopeは実際にconnectionが
必要な区間だけです。

評価APIは次のようにDB、外部HTTP、DBの3区間を持ちます。

```text
準備DB transaction
  -> 決済HTTP
  -> 完了DB transaction
```

request scopeでpermitを持つと、決済HTTPを待つ間もgeneral枠を塞ぎます。
phase scopeなら準備commit後にpermitを返し、決済後にもう一度取得します。

### FIFOとpriority

SQLx 0.8のpoolは公平なFIFO取得を行います。shared poolでgeneralの同時DB phaseを
減らしても、pool内部に「coordinateをgeneralより先にする」という用途別priorityは
追加されません。厳密な用途別待ち行列が必要なら、static partitionまたはpool取得前の
別queue設計が必要です。

### `before_acquire`で実装しなかった理由

SQLx 0.8の`before_acquire` callbackは、idle connectionを再利用する前に呼ばれます。
新しいconnectionを開く経路では呼ばれず、取得したconnectionを返すまでpermit guardを
所有する仕組みでもありません。そのため、全取得経路へ同じ上限を掛ける用途には
使いませんでした。

## 実装した比較用構成

### 設定

環境変数を省略した既定値がshared 50 + general permit 26です。

```sh
# 既定: shared 50 + general permit 26
./scripts/benchmark.sh 60

# permit数を明示して同じ構成を再現
ISUCON_DB_GENERAL_PERMITS=26 ./scripts/benchmark.sh 60

# static 26 / 24対照を再現
ISUCON_DB_COORDINATE_CONNECTIONS=24 ./scripts/benchmark.sh 60
```

shared時は1つの`MySqlPool`を最大50で作り、coordinate側にはそのcloneを渡します。
`ISUCON_DB_COORDINATE_CONNECTIONS`と`ISUCON_DB_GENERAL_PERMITS`の同時指定は、
意味が競合するため起動時に拒否します。permitは正整数かつ総接続数未満でなければ
なりません。coordinate接続数を明示した場合だけ、比較用static partitionになります。
総接続数だけを小さくした場合は、従来と同じ
`coordinate headroom = min(24, total / 2)`を使い、残りをgeneral permitにします。

### permitを取得する場所

general request全体ではなく、実際のDB phase直前で取得します。

- app / chair / ownerの各handler
- 認証cache miss時のDB検索
- matcher
- latest-location reconciliation
- initialize後のDB / cache refresh

coordinateの主transactionはpermit対象外です。評価APIは準備transactionと
完了transactionで別々に取得し、決済HTTP中はpermitを持ちません。通知はpayload cache
miss後にだけ取得し、DB connection解放後に返します。cache hitはpermitを消費しません。

起動・initializeの`AuthCache::refresh`は3種類の主体を並列に読み、最大3接続を使います。
この処理はmaintenance write lockで通常APIとreconciliationを止めた区間なので、
定常負荷の「general DB phase最大26」という比較対象には混ぜません。

### 診断

`DB_ADMISSION_DIAGNOSTIC`は64回に1回の周期sampleと、permit待ち30ms以上の全件を
非同期診断queueへ出します。採用判断のpercentileには周期sampleだけを使います。
30ms以上だけを追加したsampleを混ぜると、遅い値を意図的に多く含む偏った分布に
なるためです。

各sampleには次を記録します。

- DB phase名
- permit待ち時間
- 取得前後の利用可能permit数
- shared poolのsize / idle / in-use
- 時刻とsequence

MySQLは`scripts/sample-mysql-status.sh`で1秒ごとに次をTSVへ保存します。

- `Threads_connected`
- `Threads_running`
- `Innodb_row_lock_waits`
- `Innodb_row_lock_time`
- `Questions`

status変数は`performance_schema.global_status`を読むため、診断用ローカルDBのrootで
取得します。アプリ用`isucon` userでは権限不足になったため、最初の失敗を受けて
採取経路を修正しました。パスワードやtokenを出力TSVへ書きません。

```sh
since_file=/tmp/isucon14-b49.since
mysql_file=/tmp/isucon14-b49.mysql.tsv
date -u +%Y-%m-%dT%H:%M:%SZ >"$since_file"

ISUCON_DIAGNOSTIC=1 \
ISUCON_DB_GENERAL_PERMITS=26 \
MYSQL_STATUS_OUTPUT_FILE="$mysql_file" \
./scripts/benchmark.sh 30

./scripts/report-db-admission.sh "$(cat "$since_file")" "$mysql_file"
```

## 途中で棄却したrequest-wide permit

最初はcoordinate以外のHTTP route全体へpermitを掛けました。30秒診断runは
62,839点、`pass=true`、error map空でしたが、次の問題がありました。

- cache hit通知もpermitを取る
- validationでDBへ到達しないrequestもpermitを取る
- 評価APIが外部決済HTTP中もpermitを持つ
- response構築や診断flushまでpermit所有時間へ入る

観測上限104,320 acquisition、周期1,631 sampleのうち、周期または強制記録された
30ms以上の待ちは40,615件でした。周期sampleのp95は275.731ms、最大494.637msです。
これはDB接続の同時数を制御したい設計に対して、制御区間が広すぎる証拠です。
この版は採用せず、DB phase scopeへ変更しました。

## DB phase scopeの診断結果

### 共有pool + general permit 26

30秒runは43,524点、`pass=true`、error map空でした。

| 項目 | 値 |
| --- | ---: |
| 観測sequence上限 | 36,481 |
| 周期sample | 571 |
| 周期または強制記録された30ms以上 | 17,594 |
| permit待ち平均 / p50 | 70.192 / 21.371ms |
| permit待ちp95 / p99 / 最大 | 255.870 / 393.539 / 567.241ms |
| pool size 50 / idle 0の周期sample | 194 / 571 |
| MySQL 1秒sample | 32 |
| `Threads_connected`最大 | 51 |
| `Threads_running`平均 / 最大 | 9.75 / 50 |
| row-lock waits増分 | 1,410 |
| row-lock time増分 | 33,792ms |
| Questions増分 | 341,079 |

`Threads_connected`の51にはsampler自身の1接続が含まれます。アプリの上限50を
超えて設定した証拠ではありません。

phase別ではmatcher 453.858ms、fare 290.580ms、chair通知288.429ms、
app通知259.128msがp95の大きい箇所でした。sample数1件のmatcher p95を一般化せず、
高頻度phaseと通常runを合わせて判断します。

### static 26 / 24対照

同じ30秒診断条件のstatic対照は44,229点、`pass=true`、error map空でした。

| 項目 | 値 |
| --- | ---: |
| MySQL 1秒sample | 32 |
| `Threads_connected`最大 | 51 |
| `Threads_running`平均 / 最大 | 10.56 / 41 |
| row-lock waits増分 | 1,224 |
| row-lock time増分 | 41,623ms |
| Questions増分 | 350,260 |
| coordinate sample | 347 |
| coordinate pool acquire p50 / p95 / p99 | 5.807 / 50.078 / 132.624ms |
| coordinate pool acquire最大 | 198.829ms |
| coordinate pool size 24 / idle 0 | 180 / 347 |

共有版はrow-lock累積時間が短い一方、wait回数は多く、30秒scoreはほぼ同じです。
累積statusは処理件数にも依存するため、単独の大小だけで採用しません。

## 通常run

通常runは診断logとMySQL samplerを無効にし、60秒、4 CPU / 4 GiBの同じColima設定で
実行します。ホスト上の別処理が走ったrunは、値を消さず除外理由を併記します。

### 近接A/B

```sh
# static
ISUCON_DB_COORDINATE_CONNECTIONS=24 ./scripts/benchmark.sh 60

# shared
ISUCON_DB_GENERAL_PERMITS=26 ./scripts/benchmark.sh 60
```

| 組 | 先に実行 | static | shared | shared - static |
| ---: | --- | ---: | ---: | ---: |
| 1 | static | 112,512 | 130,167 | +17,655 |
| 2 | shared | 126,104 | 141,586 | +15,482 |
| 3 | static | 129,655 | 135,410 | +5,755 |
| 中央値 | - | 126,104 | 135,410 | +9,306 |

全6 runは`pass=true`、error map空でした。組1のstatic直前にはArc rendererの
CPU使用率が高い時刻があり、ローカル外乱を完全には除去できていません。そのため
1組の差ではなく、順序を反転した3組すべての方向と中央値を使います。

### 既定値の最終確認

```sh
./scripts/benchmark.sh 60
./scripts/compose.sh logs --no-color webapp |
  rg "configured database connection pools"
```

```text
configured database connection pools total=50 general=26 coordinate=24 shared=true
結果 pass=true スコア=131963 種別エラー数=map[]
```

### 比較から除外したrun

| 構成 | score | 除外理由 |
| --- | ---: | --- |
| shared | 113,194 | tuning SVG生成がベンチ中に継続していた |
| static | 122,043 | ホストの`rustup update`がrun途中で開始した |
| static | 94,263 | ホストで別の`cargo install`が複数rustcを起動し、最大約40% CPUを使用 |

除外値も実行記録として残しますが、推定代表値へ混ぜません。特に94,263点を
コード退行と扱わず、process一覧で同時buildを確認してから取り直しました。

## どのログを見て、どう判断するか

| 証拠 | 見る値 | 判断 |
| --- | --- | --- |
| benchmark最終行 | score、pass、error map | 正当性と最終的な処理成果 |
| admission診断 | phase別permit待ち、30ms超、利用可能permit | general入口が新しい律速か |
| SQLx pool状態 | size、idle、in-use | permit待ちとconnection待ちのどちらが先か |
| coordinate診断 | acquire、BEGIN、query、COMMIT、total | 共有によってcoordinate待ちが減ったか |
| notification / evaluation診断 | phase別pool取得とconnection所有 | general側の副作用 |
| MySQL status | connected、running、row-lock、Questions | 接続数ではなくDB実行競合が増えていないか |
| benchmark中のホストprocess | 別build、browser、indexerのCPU | ローカル外乱によるrun除外根拠 |
| webapp起動log | total、general、coordinate、shared | 意図した構成が実際に起動したか |

scoreだけが動き、permit待ち、coordinate待ち、MySQL競合が同じ方向へ動かなければ、
因果を断定しません。逆に診断値が改善しても、通常runの中央値や完了数が悪化するなら
既定値には採用しません。

## 限界と他の選択肢

### static partitionを維持する

接続の融通はできませんが、用途ごとの容量と待ち行列が明確です。shared版が通常runで
安定して上回らない場合は、既定の26 / 24を維持します。

### permit数を変える

24や28などを比較できます。ただし値を増やすほどgeneralがcoordinate余力を使い、
減らすほど通知・評価・matcherがpermit前で待ちます。26で設計と計測経路を検証してから、
待ちの内訳に根拠がある場合だけ次の値を比較します。

### 用途別priority queue

shared poolのFIFOより明示的にcoordinateを優先できますが、starvation回避、cancel、
timeout、maintenanceとの統合が必要です。Semaphoreを1つ置く施策より大きいため、
今回の比較結果なしに先行実装しません。

### connection保持時間をさらに減らす

permitやpoolは待ち場所を制御する仕組みで、SQL量やrow lockそのものは減らしません。
matcher、通知、評価、coordinateのqueryとtransactionを短くできれば、static / shared
の両方に効きます。

### per-chair coordinate queue

HTTP pathからDB書込みを外せますが、chair内順序、全履歴、累積距離、status遷移、
3秒可視性、再起動復旧を同時に守る必要があります。connection設計より正当性riskが
大きいため、Benchmark 49の後に独立した施策として扱います。
