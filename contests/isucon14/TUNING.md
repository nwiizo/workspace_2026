# ISUCON14 Rust チューニング

公式 Rust 実装を、公式ベンチマーカーで計測しながら改善した記録です。ベンチマークごとに、観測・仮説・変更・効果・代替案を独立したファイルへ分けています。

## 読み方

各記録は次の順序で記載します。

1. 同じ条件でベンチマークを実行する
2. エラーコード、HTTP経路、SQL、資源使用量から遅い境界を特定する
3. 反証可能な仮説を1つ立てる
4. 仮説を検証できる最小変更を入れる
5. 同じ条件で再計測する
6. 効果がなければ変更を重ねず、計測へ戻る

コンテナのhealthcheck成功は、ベンチマーカーの制限時間内に応答できることを意味しません。スコアだけでなく、走査行数、SQL回数、transaction回数、タイムアウトしたAPIを併せて判断します。

## 共通計測条件

| 項目 | 内容 |
|---|---|
| 日時 | 2026-07-25 |
| ホスト | Apple Silicon macOS / Colima |
| Colima | 4 CPU / 4 GiB |
| 構成 | Rust、MySQL、nginx、matcher、benchmarkを同一Dockerホストで実行 |
| 初期データ | chairs 500、chair_locations 21,209、rides 750、ride_statuses 4,496 |
| ベンチマーカー | 公式Go実装、静的ファイル検証あり |

公式競技環境とはマシン構成が異なるため、スコアの絶対値ではなく、同一ホスト・同一走行時間で変更前後を比較します。

## スコアの値種別と推定ルール

この記録では、ログから得た値と、少数の観測から推定した代表値を分けます。

| 表記 | 意味 | 使い方 |
|---|---|---|
| 実測値 | ベンチマーカーの最終行に実際に出たスコア | run単位の事実としてそのまま記録する |
| 観測範囲 | 同一条件の実測値の最小値–最大値 | 今回見えたばらつき。将来を保証する予測区間ではない |
| 推定代表値 | 同一条件を3回以上測ったときの中央値 | その構成の典型的なスコアを推定する採否用の値 |
| 推定改善率 | 変更前後の推定代表値の差を変更前で割った値 | `（変更後中央値 - 変更前中央値）÷ 変更前中央値` |
| 未推定 | 同一条件が1走だけ、または測定条件が混在 | 実測値は残すが、典型値や幅を作らない |

中央値は、値を小さい順へ並べた中央の値です。3走が
58,220、60,102、66,167点なら中央値は60,102点です。1回だけ非常に高い、または
低いrunがあっても平均値ほど引っ張られないため、現在は中央値を代表値にしています。

ただし `n=3` は小さなsampleです。最小値–最大値は「今回実際に見た範囲」であり、
95%信頼区間でも、次回スコアの予測区間でもありません。3走未満の施策には、別の
施策で観測した揺れ幅を機械的に当てはめません。コード、DB設定、負荷展開によって
分散の大きさも変わるためです。

共有負荷時の0点と静穏時の5,906点のように条件が異なる値も、平均して1つの推定値に
しません。単発の過去スコアは誤った値ではなく実測値ですが、「この施策なら通常この
点数になる」という推定には使えないため、ベンチマーク一覧で `実測 n=1・未推定`
と明記します。

## スコア構造と評価軸

ベンチマーカー内の世界は30msを1tickとして進みます。1つのAPIが30msを超えると、椅子や利用者が次の行動へ進めず、単発のレスポンス遅延がmatching、pickup、driveの各評価へ連鎖します。このため、平均値だけでなく各APIの30ms超過率とp95 / p99を記録します。

スコアは次の3要素の合計です。

| 要素 | スコア寄与 | チューニング上の意味 |
|---|---:|---|
| 椅子がmatching位置から乗車地点へ移動した距離 | 距離 × 0.1 | 遠い椅子の割当は完了までを遅らせ、単位距離の価値も低い |
| 乗車地点から目的地までの移動距離 | 距離 × 1 | 空車移動の10倍の価値があるため、椅子を早く乗車状態へ移す |
| 完了ライド | 件数 × 5 | API全体のthroughputと通知遅延を改善して完了数を増やす |

したがって、HTTPリクエスト数や単体SQL時間だけでは採否を決めません。各runで完了ライド数、空車移動距離、乗車中移動距離、matching / pickup / driveの不満率を併記し、スコアが変化した理由を分解します。近傍優先matcherがID順batchより大きく伸びた記録は、この評価構造と整合します。

### 評価軸から見た現在の実装

| 改善対象 | 現在の状態 | 次の検証 |
|---|---|---|
| 高頻度検索へのINDEX | 主要INDEX、`coupons(code)`、`coupons(used_by)` を追加済み。`users(access_token)` と `users(invitation_code)` は既存の `UNIQUE` INDEXで充足 | prepared statement統計で次の全件走査を探し、未使用INDEXを増やさない |
| nearbyの2N+1解消 | 最新座標はcurrent-state表 + process cache、active / 割当可否はDBで合成。評価response bodyの終了までtrackerで除外 | ride antijoinとtracker確認の内訳を計測 |
| owner椅子一覧をownerで先に絞る | 実装・単独ベンチ済み | 最新位置と累積距離のcurrent-state化 |
| 最新位置をcurrent-state表で管理 | 履歴INSERTと同じtransactionで更新し、cacheを2秒ごとに再同期 | current UPDATEのrow-lock待ちとwrite amplificationを削減 |
| pending rideと空き椅子のbatch matching | 最大64件、近傍優先まで実装済み | 地域間の距離上限、実行間隔、二部マッチングを比較 |
| JSON通知のcache | 未実装 | 同じpayloadの再計算をなくし、long pollingをSSEより先に比較 |
| 座標更新の非同期・bulk INSERT | 通常経路を4 SQLから2 SQLへ削減。pickup / destination候補だけlockし、statusをcurrent readする | per-chair順序付きqueueと3秒以内のbulk反映を実験 |
| 決済HTTP client | process内で1個を共有し、POST / GET / retryでconnection poolを再利用済み | TCP connect回数を診断runで採取 |
| 決済の `Idempotency-Key` | 未実装 | ride IDをkeyにして遅い確認GETを除去 |

SSEは形式だけ変更しても、DB query数とpayload生成量が同じなら効果が薄いと考えます。JSON payload cache、`retry_after_ms`、DB connectionを保持しないlong pollingを先に計測し、それでも通知経路が律速の場合にstatus変更時の即時pushと接続単位cacheを含めて実装します。

### キャッシュ・非同期化の正当性上の注意

- nearbyで最大3秒の遅れが許されるのは座標だけであり、`is_active` と割当可否は即時反映する
- nearbyレスポンス全体を3秒cacheすると、割当済みの椅子を空きとして返す可能性がある
- 2地域内だけで利用者が移動するため、地域をまたぐ遠い椅子を無理に割り当てず、次batchへ保留する方が全体効率を上げられる
- matcherを複数processで動かす場合は、leader選出またはrideとchairの条件付きclaimがないと二重割当が起きる
- 座標更新を非同期化しても、椅子ごとの順序、累積距離、`PICKUP` / `ARRIVED` の一度だけの遷移を維持する

### 実装案を採用するための判定基準

| 対象 | 必ず記録する値 | 採用条件 |
|---|---|---|
| matcher | 地域別pending数、最古ride待ち時間、pickup予測tick、完了数、空車移動距離 | starvationとエラーを増やさず、完了数または総スコアの中央値が改善する |
| 通知cache / long polling | cache hit率、recipientあたりSQL数、wake latency、再接続replay件数 | 全遷移の順序とat least onceを維持し、30ms超過率とSQL数が減る |
| 座標queue / batch | API p99、queue depth、最古未flush時間、batch件数、retry数 | 座標を3秒以内に反映し、status遷移・累積距離を壊さずAPI p99が下がる |
| current-state表 | 履歴との不一致件数、initialize再構築時間、hot path SQL数 | 初期化・再起動後も不一致0で、履歴subqueryを削減できる |

matcherは単純なマンハッタン距離だけでなく、椅子モデルのspeedを含むpickup予測tickで比較します。batch内の目的関数は、まず割当可能件数を最大化し、次に期限へ近いrideを救い、その範囲でpickup時間を最小化します。これにより、近い新規rideだけを選び続けて古いrideが残る問題を避けます。

通知cacheはDB上の配信cursorの代替にはしません。recipientごとに `last_status_id` とpayloadを保持し、ride割当・status追加・評価確定でinvalidateします。long pollingではversion確認後にwaiterを登録し、待機前にもう一度versionを確認して、確認と待機開始の間のイベントを取りこぼさないようにします。

座標batchでは、latest-coordinate cacheと永続化待ちの座標列を分けます。中間座標を捨てると累積距離が短くなり、pickupやdestinationとの一致も失うため、nearby用の最新値だけを上書きし、履歴・距離・status判定に必要な全座標は順番どおり処理します。

## はじめに知っておく用語

### この章の使い方

性能用語は、名前だけ覚えても施策の判断には使えません。この章では各用語を、
「何を表すか」「どのlogや数値で観測するか」「性能へどう影響するか」「何を
誤解しやすいか」の順で説明します。

たとえば「INDEXを追加した」という事実だけでは改善とは判断しません。対象SQLの
実行頻度、変更前後の実行計画、走査行数、60秒ベンチの正当性とスコア、追加された
書込みコストまでつないで初めて採否を決められます。この記録での基本単位は、次の
因果の鎖です。

```text
変更
  -> 内部の処理経路が変わったか
  -> SQL時間・走査行数・待ち時間が変わったか
  -> endpointの応答と状態進行が変わったか
  -> 完了ride数・正当性・スコアが変わったか
```

途中の矢印を計測していない場合は、「改善した理由」ではなく「考えられる理由」と
区別します。

### 計測と判断

#### ベンチマーク

決められた操作を一定時間実行し、正しさ、処理量、応答時間を測るプログラムです。
ISUCON14の公式ベンチマーカーは、単にHTTP requestを大量送信するだけでは
ありません。利用者がrideを依頼し、椅子が移動し、状態通知を受け、評価と決済まで
終える一連の世界を進めます。

そのため、単一APIだけを速くしても、状態の順序を壊したり、完了rideが増えなければ
総スコアは上がりません。最終行の `pass`、スコア、error mapに加え、途中の
`eval reqs` とmatching / pickup / drive不満率を読みます。

#### score・`pass`・error map・最終評価数

scoreは、60秒間に世界をどこまで正しく進められたかをまとめた結果です。単純な
HTTP request数ではありません。benchmarker実装上の `pass=true` は、worldのtick処理を
止めるcritical errorと決済errorによってscenarioの `failed` flagが立たなかったことを
表します。non-critical errorはerror mapへ加算されても `pass=true` のままになり得るため、
`pass` とerror mapを必ず別々に確認します。scoreが高くても `pass=false` なら、そのrunは
採用できません。

error mapは、どの種類の失敗が何回起きたかを示します。timeoutが多い場合も、
必ずしもそのendpoint自身が遅いとは限りません。先に重いSQLがDB connectionを
占有し、別endpointを巻き込んでいる場合があるため、発生順とserver logを合わせます。

最終評価数は、依頼から決済・評価まで完了したride数に近い、状態進行の重要な指標です。
scoreと同じ方向へ増えたかを見ると、「軽いAPIだけ大量に返した」のか、「完了する仕事が
実際に増えた」のかを区別しやすくなります。

#### tick・状態進行・不満率

ISUCON14のbenchmarkerは約30msを1 tickとして利用者と椅子の行動を進めます。
通知や座標反映が1 tick遅れると、次の操作開始も遅れます。個々の遅延は小さくても、
matching、pickup、drive、決済まで連鎖すると60秒間の完了数へ影響します。

benchmark logの3つの不満率は、次の判定を集計しています。

1. matching: 依頼からmatchまでが100 tick未満だったrideの割合を反転
2. dispatch: 割当時の椅子から乗車地点までの距離が `10 × 椅子のspeed` 未満だった
   rideの割合を反転
3. 実移動: pickupの実時間と理想時間の差が15 tick未満、driveの差が5 tick未満という
   2判定を合算し、`完了ride数 × 2` で割った成功率を反転

第2値は時間の実測ではなく、距離とspeedによる割当品質の判定です。第3値もpickupと
driveを分離した値ではなく、2判定の平均です。原因を直接示すCPU profilerではないため、
個別interval、endpoint latency、SQL logと合わせてボトルネックを絞ります。

#### 実測値・推定値・中央値

実測値は、そのrunでログに出た事実です。推定値は、限られた実測から「同じ条件を
繰り返したときの代表的な値」を推し量ったものです。この文書では同一条件を3回以上
測れた場合だけ、中央値を推定代表値として使います。

1走の80,000点を「通常80,000点」とは扱いません。3走が76,761、80,354、
88,638点なら、観測範囲は76,761–88,638点、推定代表値は中央値80,354点です。
観測範囲の外へ次回値が出ない保証はありません。

#### run・標本数 `n`・観測範囲

runは、初期化から60秒ベンチ終了までの1回です。`n=3` は同じ条件を3回測ったという
意味で、3倍の時間を測った意味ではありません。観測範囲は最小値から最大値までです。
標本数が少ないため、範囲は将来の上下限を保証しません。

中央値は値を小さい順に並べた中央です。極端に遅い1走の影響を平均より受けにくいため、
この記録では代表値に使います。ただし中央値だけでは揺れ幅が消えるため、必ず各runと
観測範囲も残します。

#### 対照・変更変数・noise

対照は変更前の比較対象、変更変数はその実験で意図的に変えたものです。1つのrunで
SQL、poll間隔、DB設定を同時に変えると、scoreが上がってもどれが効いたか分かりません。
そこで原則として1施策ずつ比較します。

noiseは、施策以外の揺らぎです。初期データの乱数、OS scheduler、cacheの温まり方、
同時実行の順序、ホスト上の別processなどが含まれます。CPU / memory条件を固定し、
複数runを残すのはnoiseと施策効果を混同しにくくするためです。

#### 相関・因果・仮説・反証条件

score上昇とSQL時間短縮が同時に起きたことは相関です。変更したINDEXによって
実行計画がtable scanからindex lookupへ変わり、走査行数が減り、同じ正当性で
複数runの処理量が増えたところまで確認すると、因果の説明が強くなります。

仮説は変更前に予想した処理経路、反証条件は「何が起きたらその予想を捨てるか」です。
たとえばcovering INDEXの仮説なら、実行計画がcoveringにならない、走査は減っても
scoreが継続的に下がる、書込み待ちが増える、のいずれかを反証条件にできます。

#### latency・throughput・p95 / p99

latencyは1回の処理が終わるまでの時間、throughputは一定時間に終えられる処理量です。
平均latencyが5msでも、100回に1回だけ500msかかれば、その遅い1回が状態進行を
止めることがあります。

p95は測定値を短い順に並べた95%地点、p99は99%地点です。p99が高いと、少数の
requestだけが大きく遅れる「裾の長い」分布だと分かります。ISUCON14は30msで
1tick進むため、平均だけでなく30ms超過率とp95 / p99が重要です。

平均、中央値、p95、p99は互いに置き換えられません。100件を短い順に並べた場合、
p95はおおむね95番目、p99は99番目の値です。平均だけが悪化した場合も、全体が少しずつ
遅くなった場合と、少数の極端値に引っ張られた場合の両方があります。分布全体、
histogram、閾値超過率を見て仮説を分けます。p99が悪化した場合は、lockやqueueによる
一部の長い待ちを候補にし、同時刻のserver側の待ちで確認します。

#### 壁時計時間・CPU時間・累積SQL時間

壁時計時間は、人が時計で測る開始から終了までの時間です。CPU時間はCPU coreが
実際に命令を実行した時間です。SQLの累積時間は、同じSQLを並行実行した各回の時間を
足した値です。

60秒ベンチ中に、SQL累積時間が120秒になることは矛盾ではありません。2本以上が
並行して待機・実行されれば、個々の経過時間の和は60秒を超えます。累積時間は
「DBがそのSQLへどれだけ多くの時間を費やしたか」の順位付けに使い、endpointの
壁時計latencyとは区別します。

#### concurrency・queue・Littleの関係

concurrencyは同時に処理途中になっている仕事の数です。throughputを毎秒100件、
平均latencyを50msとすると、安定状態では平均して約5件が処理途中になります
（`100 × 0.05 = 5`）。latencyが伸びたまま到着数が同じなら、処理途中の仕事が増え、
connection poolやqueueの上限へ近づきます。

これは平均値の関係であり、個々のrequestの完了時間を予測する式ではありません。
到着がburstする場合や上限でrejectする場合は、queue depthとp95 / p99も必要です。

#### ボトルネック

全体の速度を最も強く制限している境界です。水道管の一番細い箇所を太くしない限り
全体の流量が増えないのと同じです。CPU使用率が高い箇所だけを意味しません。
DB connection待ち、diskへのflush、外部HTTP、lock、短すぎるpollingも
ボトルネックになります。

1つを解消すると別の待ちが次のボトルネックになります。MySQLのCOMMIT待ちを減らした
後にmatcherの待ちや決済HTTPが目立つのは正常な変化です。変更後は必ず計測し直し、
前回の分析をそのまま使い続けません。

#### utilization・saturation・backpressure

utilizationは資源が仕事をしていた割合です。DockerのCPU 100%は原則1 core相当なので、
MySQL 240%は約2.4 coreを使用している読み方になります。ただしCPUが高いだけでは、
有益な処理か、無駄なscanかは分かりません。

saturationは、資源が処理しきれず仕事が待ち行列へ溜まった状態です。DB poolの
取得待ち、MySQLの `Threads_running`、disk I/O待ち、queue depthなどで観測します。
CPU 60%でも、単一lockやconnection上限でsaturationすることがあります。

backpressureは、下流が処理できないときに上流の投入速度を抑える仕組みです。無制限に
taskを生成するより安全ですが、待ちがどこへ移ったかを隠さないよう、queue長、最古待ち
時間、timeout、drop数を記録します。

### SQLとMySQL

#### SQL

MySQLへ検索、追加、更新、削除を依頼する言語です。Rust側で1行に見える
`sqlx::query(...).fetch_all(...)` も、network越しにMySQLへrequestを送り、
MySQLが解析・実行し、結果を返し、Rustが型へdecodeする複数段階を含みます。

SQLの性能は文の長さでは決まりません。何行を読み、どのINDEXを使い、途中結果を
何行作り、何回呼ばれるかで決まります。単発0.1msのSQLでも60秒に20万回呼ばれれば
累積負荷になります。

#### table・row・column・schema

tableは同じ形のデータを持つ集合、rowは1件の記録、columnは各項目です。schemaは
table、column、型、制約、INDEXを含むデータ構造の定義です。アプリコードだけを
変えても、`POST /api/initialize` がschemaを作り直すなら、初期化SQLにも変更を
反映しないと次のrunで消えます。

row数だけで重さは決まりません。1 rowの幅、返すcolumn数、同じrowを読む回数、
buffer poolへ収まるか、条件で何件へ絞れるかが効きます。`SELECT *` は不要なcolumnも
読み、covering INDEXの選択肢を狭めるため、hot pathでは必要列を確認します。

#### PRIMARY KEY・secondary INDEX・clustered構造

PRIMARY KEYはrowを一意に識別するキーです。InnoDBではtable本体がPRIMARY KEY順の
B-treeとして保持され、leafにrow全体があります。これをclustered INDEXと呼びます。

PRIMARY KEY以外のINDEXはsecondary INDEXです。そのleafにはINDEX列とPRIMARY KEYが
入ります。secondary INDEXで候補を見つけた後、必要なcolumnがINDEX内にない場合は
PRIMARY KEYを使ってtable本体をもう一度読みます。したがって、候補が大量にある
低選択性INDEXは、全件走査より遅いこともあります。

#### INDEX

検索対象を速く見つけるために、table本体とは別に管理する索引です。MySQLの一般的な
B-tree INDEXは、値を順序付きで持ちます。書籍の索引で単語からページを探すように、
条件に合う位置へ木構造をたどり、必要な範囲だけを読めます。

複合INDEX `(chair_id, created_at)` は左端の `chair_id` で対象椅子を絞り、その中を
`created_at` 順に探せます。一方、`created_at` だけを条件にすると左端列を飛ばす
ため、同じ形では効きにくくなります。これを左端prefixの性質と呼びます。

INDEXは無料ではありません。INSERT / UPDATEごとに索引も更新し、diskとmemoryを
使います。候補を思いつくたび追加せず、対象SQL、実行頻度、`EXPLAIN ANALYZE`、
書き込み増加を対応付けます。詳細は [INDEXの記録](./tuning/01-indexes.md) を参照して
ください。

#### B-tree・page・root・leaf

B-treeは、並んだkeyを一定個数ずつpageへ格納し、rootからbranchをたどってleafへ
到達する木構造です。1 rowずつ先頭から比べるのではなく、各段で候補範囲を狭めます。
そのため位置探索は概念上 `O(log N)`、見つけた後の範囲読みは一致件数に比例します。

実際の速度は計算量だけでなくpage I/Oで決まります。必要pageがbuffer poolにあれば
memoryから、なければstorageから読みます。INDEXを増やしすぎると全INDEXのpageが
memoryを競合し、書込み時のpage分割やdirty page flushも増えます。

#### cardinality・selectivity・一意性

cardinalityは、あるcolumnに異なる値がおおよそ何種類あるかです。selectivityは、
条件によってtable全体からどれだけ少数へ絞れるかです。厳密な表現は用途で異なりますが、
この記録では「一致行数 ÷ 全行数」が小さいほど選択性が高い、と読みます。

100万rowに一意なride IDが100万種類あるcolumnの等価検索は、通常1 rowへ絞れるため
高選択性です。true / falseしかないcolumnが半々なら50万row残るため低選択性です。
`UNIQUE` は重複を禁止する制約であり、高選択性になりやすいものの、INDEXを速くする
ためだけに既存データの意味を変えて追加してはいけません。

MySQLのoptimizerはtable統計からcardinalityとcostを見積もります。偏ったデータや
古い統計では見積りを外すため、`EXPLAIN` の推定 `rows` と `EXPLAIN ANALYZE` の
`actual rows` を比較します。

#### 複合INDEX・左端prefix・range条件

複合INDEX `(a, b, c)` は、まず `a`、同じ `a` の中で `b`、さらに同じ組の中で `c`
という辞書順で並びます。`WHERE a = ? AND b = ?` は連続範囲を直接探せますが、
`WHERE b = ?` だけでは異なる `a` の範囲に同じ `b` が散らばるため、原則として
効率よく絞れません。これが左端prefixです。

`a = ? AND b > ?` のようにrange条件へ入った後は、`c` を追加しても探索範囲を
さらに狭められない場合があります。ただしIndex Condition Pushdown、covering、
`ORDER BY` 回避には役立つ場合があるため、列順は「WHEREに出た順」ではなく、
等価条件、range、並び順、返す列を合わせて実行計画で決めます。

#### covering INDEX

検索条件、並び順、返すcolumnがすべて1本のINDEXに含まれ、table本体を読まずに
結果を返せる状態です。実行計画では `Using index` や `Covering index lookup` が
手掛かりになります。

読み取り経路は短くなりますが、返すcolumnをすべてINDEXへ加えるとleafが太くなり、
1 pageに入るentryが減ります。その結果、memory、storage、INSERT / UPDATEの負荷が
増えます。「coveringになった」ことと「総合ベンチが速くなった」ことは別に検証します。

#### `ORDER BY`・sort・backward index scan

INDEXの並びが `ORDER BY` と合えば、MySQLは別のsortを作らず順番どおり読めます。
昇順INDEX `(chair_id, created_at)` でも、`chair_id = ?` の範囲を末尾から逆向きに
読むbackward index scanによって `created_at DESC` を処理できる場合があります。
したがって、降順が欲しいという理由だけで `DESC` INDEXを重複追加しません。

実行計画にsortが残る場合は、joinやsubqueryで順序を維持できない、複数chairをまとめて
並べ直す、projectionが広い、optimizerが別INDEXを安いと判断した、などの理由を先に
調べます。降順INDEXは、複合列にASC / DESCが混在し既存INDEXの逆向き走査では
順序を作れない場合などに比較対象になります。

#### wall-clock・状態version・ENUM順

`created_at` は「いつ観測したか」を表すwall-clockで、状態の前後関係を保証するversion
とは限りません。並行transactionとlock待ちがあると、処理開始、時刻の評価、lock取得、
commitの順番は一致しない場合があります。同率時刻へIDを足せば全順序は作れますが、
異なる時刻自体が状態順と逆なら解決しません。

ISURIDEの `ride_statuses.status` は
`MATCHING -> ENROUTE -> PICKUP -> CARRYING -> ARRIVED -> COMPLETED` の順に宣言した
MySQL ENUMです。MySQLはENUMを通常、文字列の辞書順ではなく宣言時の内部index順で
並べます。このため現在の直線的な状態機械では `ORDER BY status` を状態versionとして
使えます。分岐、取消、再開を追加して単純な大小関係で表せなくなった場合は、明示的な
sequenceまたはcurrent-state表へ移行します。

複合INDEX `(ride_id, app_sent_at, status)` では、先頭2列を等価条件で固定すると、その
範囲の末尾列はstatus順に並んでいます。未送信通知を状態順で1件取るときもfilesortを
避けられます。詳細と実際に再現した時刻逆転は
[Benchmark 19](./tuning/19-status-semantic-order.md) に記録しています。

#### 全件走査

条件に合う行を探すため、tableまたはINDEXの広い範囲を先頭から確認する処理です。
小さなtableでは問題にならなくても、履歴が増えると読んだ行数に比例して遅くなります。
MySQLの実行計画では `Table scan`、`rows`、`actual rows` などを確認します。

全件走査そのものが常に悪いわけではありません。500行の大半を返す処理では、
INDEXを何度もたどるよりscanが速い場合があります。問題は「返すのは1行なのに
数万行読む」ことや、それが高頻度経路で繰り返されることです。

#### index lookup・index scan・table scan

index lookupは、等価条件などでB-tree上の狭い位置へ直接移動する処理です。index scanは
INDEXを順に広く読む処理で、table本体よりcolumnが少ない、`ORDER BY`を省ける、といった
利点はありますが、読むentryが多ければ高コストです。table scanはtable本体を広く
読む処理です。

「INDEXを使った」という一言ではlookupとscanを区別できません。返す1 rowのために
何entry読んだか、実行計画のoperation、`rows examined`を合わせて確認します。

#### rows examined・rows sent・loops

rows examinedは条件判定やjoinのために調べたrow数、rows sentはclientへ返したrow数です。
1 row返すSQLで毎回1,000 row調べていれば、差分999 row分が探索コストです。ただし
aggregationは多くのrowから1 rowを作るため、この差だけで無駄とは判断しません。

`loops` は実行計画のその段階が何回繰り返されたかです。内側が0.1msでも外側1万rowに
対して1万回動けば累積1秒以上になります。`actual time=a..b rows=r loops=l` の時間は
versionやoperationによって1 loopあたりの平均として表示されるため、親子operationと
総実行時間を合わせて読みます。

#### prepared statement・placeholder・digest

prepared statementは、`?` の位置へ値を後からbindして同じSQL形を繰り返し実行する
仕組みです。値をSQL文字列へ連結しないため安全性を高め、protocol上でSQL形と値を
分けられます。SQLxのMySQL driverもこの経路を使います。

digestは、定数を正規化して同じ形のSQLを集約する識別です。ただし今回の
`events_statements_summary_by_digest` ではprepared statement本文が
`statement/com/Execute` へまとまり、本文別の順位を直接得られませんでした。
そこで `prepared_statements_instances` を補助的に使いました。

このtableのtimerはpicosecond単位なので、秒は `1e12`、millisecondは `1e9` で割ります。
複数connectionの平均は各行の `AVG_TIMER_EXECUTE` を単純平均せず、
`SUM(SUM_TIMER_EXECUTE) / SUM(COUNT_EXECUTE)` で重み付き平均を作ります。

ただし、行は現在存在するprepared statement instanceに対応し、deallocateまたは
connection終了で消えます。そのためrun中に閉じたconnectionの実行は終了時集計から
欠ける可能性があります。`Performance_schema_prepared_statements_lost=0` は
instrument枠不足がなかったことを示しますが、connection終了による欠落がないことまでは
証明しません。完全な全期間traceではなく、同一条件でhot SQLを順位付けする観測として
扱います。

#### 実行計画と `EXPLAIN ANALYZE`

実行計画は、MySQLがtableをどの順序で読み、どのINDEXとjoin方法を使うかを示します。
見積りだけの `EXPLAIN` に対し、`EXPLAIN ANALYZE` は読み取りSQLを実際に実行し、
各段階のactual time、rows、loopsを表示します。

見積り行数と実測行数が大きく違うと、統計情報が実データ分布を表していない可能性が
あります。`loops` が外側の行数だけ増えていれば、相関subqueryやnested loopが
繰り返されていると判断できます。更新SQLへ無造作に使うと実データを変更するため、
この記録では読み取りSQLに限定しています。

`EXPLAIN` の `key` は選ばれたINDEX、`possible_keys` は候補、`rows` は推定走査行数、
`filtered` はその後の条件を通る割合の推定です。`key` が表示されても広いindex scan
なら十分に速いとは限りません。

`EXPLAIN ANALYZE` は実データを読み、cache状態や同時負荷の影響も受けます。1回の
0.025msを将来の保証にせず、処理経路の確認と単発差の参考に使い、採否は高頻度時の
集計と60秒ベンチで決めます。

#### join・nested loop・相関subquery

joinは複数tableのrowを条件で結びます。nested loopは、外側のrowごとに内側を探す
実行方法です。外側が少なく、内側がINDEX lookupなら有効ですが、外側1万rowごとに
内側をscanすると走査量が掛け算で増えます。

相関subqueryは、内側のqueryが外側rowの値を参照します。読みやすく正しい表現でも、
外側rowごとに実行される可能性があります。`loops`、走査行数、同じ意味を現在状態の
columnやjoinで表せるかを確認します。書換え時は `NULL`、履歴欠損、同率の最新時刻など、
境界条件で結果集合が同じかを先に検証します。

#### materialize

subqueryやCTEの途中結果を一時的な表として作る処理です。同じ途中結果を再利用できる
利点がある一方、行数が多いとmemoryを使い、収まらなければdisk上の一時表へ移る
可能性があります。

実行計画に `Materialize` があっても即座に悪いとは判断しません。作る行数、作る回数、
その後どれだけ再利用されるかを見ます。相関subqueryの内側で何千回もmaterialize
される場合と、request中に一度だけ小さな表を作る場合では意味が異なります。

#### 履歴table・current state・cache

履歴tableは過去の変更をすべて残し、current stateは現在必要な1件だけを表します。
ISUCON14の `chair_locations` はownerの累積距離に全履歴が必要ですが、nearbyとmatcherは
最新座標1件だけを使います。

```text
履歴: chair_locations             -> 全移動、永続化、初期backfill
共有current: chair_current_locations -> matcher、cache再同期元
process cache                      -> nearbyの最新座標
即時判定: chairs / rides           -> active、割当可否
処理中状態: ActiveRideEvaluationTracker -> 評価response body lifecycleまで再掲載を抑制
```

同じ履歴から高頻度に現在状態を再構成すると、INDEXがあってもloop、sort、decodeが
繰り返されます。current stateを別に持つとreadは短くなりますが、更新漏れ、順序逆転、
初期化、process再起動、複数process間の共有を設計する必要があります。

cacheは「DBより速いHashMap」だけではありません。何を古くしてよいか、正解データは
どこか、cache miss時にどうするか、いつinvalidateまたは更新するかを定義した仕組みです。
Benchmark 18では座標だけをcacheし、即時性が必要なactive状態と割当可否は毎回DBから
読みます。DB commit後にrequest taskが止まっても、共有current-state表から2秒ごとに
全置換して3秒以内の自己修復経路を持ちます。

#### cacheの復元・更新順序・process境界

process内cacheはprocess終了で消えます。履歴とcurrent-state表を同じtransactionで更新し、
起動時と `POST /api/initialize` 後にcurrent-state表から復元します。DB commit成功後だけ
cacheを即時更新し、rollbackした座標を公開しません。

並行requestは記録時刻順にcommitするとは限りません。新しい座標Bが先にcacheへ入り、
古い座標Aが後から到着しても、`recorded_at` とlocation IDを比較してBを維持します。
これはlast writer winsを「lock取得順」ではなく、明示したversion順で決める考え方です。

`Arc<RwLock<HashMap<...>>>` の `Arc` は同じcacheの所有権をhandler間で共有し、
`RwLock` は複数readまたは1 writeを許します。lock guardを保持したままDBやHTTPを
awaitすると待ち行列を広げるため、通常経路ではHashMap操作だけに限定します。

process内cacheは別processへ即時伝播しません。今回追加したDB current-state表から
2秒ごとに収束しますが、即時共有が必要ならRedisや更新eventを比較します。単一processの
即時更新だけを、水平分割後の正しさへ一般化しません。

#### window関数

行をgroup内で並べ、前後の行を参照しながら順位や累積値を計算する機能です。
`ROW_NUMBER() OVER (PARTITION BY chair_id ORDER BY created_at DESC)` なら、
椅子ごとに新しい位置から番号を振り、1番だけを選べます。

N+1を集合SQLへまとめるのに役立ちますが、対象履歴全体のsortやmaterializeを伴う
ことがあります。「SQLが1回になった」だけで採用せず、読んだ行数、sort、一時表、
全体ベンチの変化を確認します。

#### transaction・COMMIT・ROLLBACK

複数のDB操作を、全部成功させるか全部取り消すかの単位へまとめる仕組みです。
rideの評価、`COMPLETED` status追加、coupon更新のように途中状態を見せたくない処理で
必要です。`COMMIT` は変更を確定し、`ROLLBACK` はtransaction開始後の変更を戻します。

transactionが長いと、その間DB connection、snapshot、行lockを保持します。
特にtransaction内で外部HTTPやsleepを待つと、MySQLが仕事をしていない時間にも
資源を占有します。短くする価値はありますが、正当性を保つ更新まで別々にすると
中間状態や欠落が見えるため、境界設計が先です。

#### 原子性と「後から来るtransaction」

原子性は、1つのtransactionに含めた変更を全部見せるか、全部見せないかを保証する
性質です。たとえば `evaluation` 更新と `COMPLETED` 追加を同時にcommitすれば、
片方だけが見える途中状態をなくせます。

ただし原子性は、そのcommit後に別transactionが `ENROUTE` を追加することまでは
禁止しません。Benchmark 16で最初に見落としたのがこの境界です。

```text
T1: evaluation更新 + COMPLETED追加 -> COMMIT
T2: 待機解除後、古い実装のままENROUTE追加 -> COMMIT
```

この結果、現在状態の `evaluation` は完了を示す一方、履歴の最新statusは
`ENROUTE` になります。「同じtransactionに入れたから安全」という説明は、
その不変条件を変更するwriterをすべて列挙して初めて成立します。

確認するlogとqueryは次です。

- server logの400、deadlock、lock wait
- `SHOW ENGINE INNODB STATUS`
- Performance Schemaの最大実行時間とlock時間
- `evaluation` と最新statusのXORを数える差分query
- 並行順序を意図的に作る競合test

#### `SELECT ... FOR UPDATE` と行lock

`SELECT ... FOR UPDATE` は、読み取ったInnoDB rowをtransaction終了まで他の更新者と
排他的に調整するlocking readです。通常の `SELECT` は値を読むだけなので、
読んだ直後に別transactionが変更できます。

ISUCON14ではride IDを主キーで検索するため、対象は基本的に1 ride rowです。同じrideの
status writerがすべてこのrowを先にlockすれば、writerは一列に並びます。

```text
評価       ┐
ENROUTE    ├─ rides.id のrow lock ─ lock取得後に最新状態を確認 ─ 書込み
CARRYING   │
PICKUP     │
ARRIVED    ┘
```

lockは「取れば正しい」のではありません。次も揃える必要があります。

1. 同じ不変条件へ関与する全writerが同じlockを使う
2. 複数rowを取る場合は取得順を統一する
3. lockを取った後の値で条件を再確認する
4. 外部HTTPやsleepをlock保持中に入れない
5. 状態を書かないreadまで不必要に直列化しない

Benchmark 16では全座標をlockすると中央値が93,606点から90,523点へ下がりました。
pickup / destination候補だけに絞り、通常座標を待ち行列から外した最終版は98,580点です。
正しさに必要な境界と、性能上避けたい過剰lockを実測で分けました。

#### TOCTOU・lock後の再読

TOCTOU（Time Of Check to Time Of Use）は、条件を確認してから結果を使うまでに、
別処理が条件を変える競合です。

```text
座標A: 最新status=CARRYINGを確認
座標B: ARRIVEDを追加
評価 : COMPLETEDを追加してcommit
座標A: 古いCARRYING判断でARRIVEDを追加
```

座標Aが最初のstatusを信じ続けると、完了後の履歴末尾が `ARRIVED` になります。
対策は「先に読んだ値をlock取得後も使う」ことではありません。共通rowをlockし、
待機中に状態が変わった可能性を考えて最新statusを読み直します。

MySQLの既定 `REPEATABLE READ` では、通常の `SELECT` はtransaction内の最初の
consistent readで作ったsnapshotを使い続けます。row lock待ちが終わった後に同じ通常
SELECTを実行しても、待機中のcommitが見えるとは限りません。Benchmark 17ではstatus側も
`FOR UPDATE` にし、最新commitを見るcurrent readへ変更しました。2本の座標requestを
同じride lockの後ろへ待たせる再現で、両方200でも `PICKUP` が1行だけになることを
確認しています。

観測上は不整合0件でも、たまたまその順序が発生しなかっただけかもしれません。
結果集合の比較は必要ですが、writerとlock順序のコード監査を置き換えません。

#### 状態機械・期待する直前状態・compare-and-swap

状態機械は、許される状態と遷移を明示したものです。この実装の主な流れは次です。

```text
MATCHING -> ENROUTE -> PICKUP -> CARRYING -> ARRIVED -> COMPLETED
```

「次statusを追加してよいか」は、現在statusが期待する直前状態かで判断します。
たとえばpickup座標へ到達しただけでは `PICKUP` にせず、lock取得後の最新statusが
`ENROUTE` の場合だけ追加します。

compare-and-swapは「現在値が期待値なら次の値へ変える」操作です。現在は
履歴tableなのでlock後にSELECTとINSERTを行っています。将来current-state表を作るなら、
`UPDATE ... WHERE current_status = 'ENROUTE'` の影響行数で同じ考えを1 SQLにできます。

状態遷移を明示すると、次を判断しやすくなります。

- 重複requestを成功扱いにしてよいか
- 順序を飛ばしたrequestを400にするか
- retryしても履歴が重複しないか
- 完了後にどのwriterも状態を戻せないか

#### 冪等な状態更新

同じrequestを2回送っても、最終状態が1回分と同じなら冪等です。Benchmark 16では
`ENROUTE` がすでに最新なら、もう1行追加せず204を返します。network timeout後に
clientが再送しても、status履歴を重複させません。

「2回目も204を返す」だけでは冪等とは限りません。内部で2行INSERTしてから204なら、
通知順序と集計結果は変わります。HTTP statusとDBの最終状態を両方確認します。

#### lock・deadlock・isolation

lockは、同じrowや範囲を並行更新したときに矛盾を起こさないための待ちです。INDEXが
ない更新は広い範囲を調べ、意図より広いlock範囲や長い保持時間につながることがあります。
待ちの原因はslow query logだけでなく、InnoDB status、deadlock記録、transaction時間で
確認します。

deadlockは、transaction AがBのlockを、BがAのlockを待つ循環です。MySQLは片方を
ROLLBACKして循環を解きます。単にretryを増やす前に、更新順序を統一する、検索範囲を
INDEXで狭める、transactionを短くする、の順で原因を減らします。

isolation levelは、並行transactionからどの時点のデータを見せるかを決めます。
弱めれば必ず安全に速くなるわけではありません。rideとcouponの二重割当など、守るべき
不変条件を列挙してから変更します。

#### autocommit

明示的なtransactionを開始していないとき、SQL文1つを1transactionとして自動で
確定するMySQLの動作です。readだけでもserver内部ではtransactionとして扱われる
場合があります。

autocommit化すれば常に速いわけではありません。複数更新を1つずつcommitすると
flush回数が増え、途中失敗時に一部だけ残ります。空通知のように整合性を束ねる更新が
ない経路では有効ですが、評価確定のような複数更新には適しません。

#### redo log・binary log・fsync

redo logはInnoDBのクラッシュ復旧、binary logはreplicationや時点復旧に使う変更履歴
です。`fsync` はOSのcacheだけでなく永続deviceへ書き出すよう要求する操作で、
commitごとに待つと耐久性が高い代わりにlatencyが増えます。

このローカル競技環境では再初期化可能という前提で同期頻度を緩和しました。これは
SQLの意味を変えず待ちを減らす一方、電源・OS障害時にcommit済みデータを失うriskを
受け入れる設定です。業務DBへそのまま適用できる一般解ではありません。

### アプリケーションと通信

#### process・container・CPU core

processは実行中のプログラム、containerはprocess群へfilesystem、network、資源上限などの
境界を与える仕組みです。containerは仮想machineそのものではなく、CPUはホストやColimaの
割当を共有します。この環境では4 CPU・4 GiBを固定しているため、MySQL、webapp、nginx、
benchmarkerが同じ4 coreを取り合います。

CPU使用率100%は通常1 coreを使い切った状態です。webappが90%、MySQLが240%なら、
合計約3.3 core相当ですが、測定時刻が違う値や短時間sampleは単純加算できません。
CPUだけでなく、block I/O、memory、connection待ちと同じ時刻で記録します。

#### async・concurrency・parallelism

asyncは、networkやDB待ちの間に同じthreadで別taskを進めやすくする実装方式です。
concurrencyは複数の仕事が途中状態にあること、parallelismは複数CPU coreで同時に
命令を実行することです。asyncにしただけでCPU計算が複数coreへ自動分散されるとは
限りません。

async taskがDB queryを待つ間はthreadを占有しなくても、DB connectionは借りたままの
場合があります。task数、thread数、pool上限は別の資源なので、どこで待っているかを
区別します。

#### connection

2つのprocessが通信するための経路です。DBならRustとMySQLのTCP connection、
HTTPならwebappと決済serviceのTCP connectionです。新規作成にはsocket確保、
TCP handshake、HTTPSならTLS handshakeが必要です。

「request」と「connection」は同じではありません。1本のconnection上で複数requestを
順番に送れる場合があります。毎requestでconnectionを作り直すと、アプリ処理以外の
準備コストとTIME_WAITが増えます。

#### DB connection pool

MySQL connectionを毎回作らず、一定本数を複数requestで貸し借りする仕組みです。
handlerはpoolから1本借り、SQLやtransactionを終えたら返します。上限50なら、
51個目の同時処理は空きが返るまで待ちます。

上限を増やせば必ず速くなるわけではありません。MySQLが処理できる量を超えると
query同士の競合、memory、context switchが増えます。`size`、`idle`、`in_use`、
取得待ち時間、MySQLの `Threads_running` を一緒に測って調整します。

#### cache・hit / miss・invalidation

cacheは、計算結果やDBから読んだ値を近い場所へ一時保存し、同じ仕事を省く仕組みです。
hitはcacheから返せた回、missは元のDBやserviceを読む必要があった回です。hit率だけで
なく、miss時latency、memory量、再起動後の再構築時間を測ります。

invalidationは、元データが変わったとき古いcacheを捨てる処理です。cache導入で難しい
のは保存より、「いつ無効にするか」です。ride status、chair位置、fareのように更新頻度と
正当性要件が異なる値を1つのsnapshotへまとめると、一部だけ古い状態を返す危険があります。
まず不変条件と更新eventを列挙し、miss時に正しいDBへfallbackできるようにします。

#### HTTP connection poolと `reqwest::Client`

`reqwest::Client` は送信先ごとのHTTP connectionを内部poolへ保持します。同じclientを
再利用すると、相手がkeep-aliveを許しconnectionが健全なら、TCP connectionを
次のrequestへ使えます。

`Client::new()` をrequestごとに呼び、そのrequest後に捨てると、再利用候補のpoolも
捨てます。今回の決済改善ではclientを `AppState` に保持し、POST、確認GET、retryで
共有しました。cloneは内部poolを共有する軽量handleで、requestのBearer tokenやbodyは
各request builderが別に持ちます。

#### polling

新しい情報がないか、clientから一定間隔で繰り返し問い合わせる方式です。実装が単純で、
再接続にも強い一方、変化がなくても認証、SQL、JSON生成、HTTP responseが発生します。

間隔を長くすると負荷は下がりますが、状態発見が遅れます。ISUCON14では通知が遅れると
次の行動も遅れるため、DB query数の減少だけで採用できません。30 / 50 / 100ms比較では
通知GETが減っても総スコアが上がらず、30msを維持しました。

#### N+1

最初の1回でN件の一覧を取り、各要素ごとに追加SQLを1回ずつ発行して、合計N+1回に
増える問題です。100台の椅子なら、一覧1回と位置取得100回になります。network往復、
pool貸借、query解析が件数に比例して増えます。

join、集約、window関数、`LATERAL` で1つの集合SQLへまとめられます。ただし巨大な
集合SQLが全履歴をsortする場合もあるため、SQL回数の削減だけでなく実行計画と
ベンチ結果を確認します。

#### retry・backoff・冪等性

retryは一時的な通信失敗時に同じ処理を再試行すること、backoffは再試行まで待つ時間
です。すぐ無制限に再試行すると障害中のserviceへさらに負荷を掛けます。

書き込みをretryするには冪等性が重要です。冪等とは、同じ操作を複数回送っても結果が
1回分と同じになる性質です。決済POSTのresponseを受け取れなくても、決済自体は成功
している場合があります。ride IDなどの一意な `Idempotency-Key` がservice側で保証
されれば、同じkeyのretryを二重決済にせず扱えます。

難しい用語が必要な箇所では、定義だけでなく、観測するlog、性能への影響、誤った
判断になりやすい点まで対応付けます。

### 用語を実装へ結び付けるための公式仕様

- [MySQL 8.4: Clustered and Secondary Indexes](https://dev.mysql.com/doc/refman/8.4/en/innodb-index-types.html)
- [MySQL 8.4: ORDER BY Optimization](https://dev.mysql.com/doc/refman/8.4/en/order-by-optimization.html)
- [MySQL 8.4: EXPLAIN Statement](https://dev.mysql.com/doc/refman/8.4/en/explain.html)
- [MySQL 8.4: Performance Schema Event Timing](https://dev.mysql.com/doc/refman/8.4/en/performance-schema-timing.html)
- [MySQL 8.4: prepared_statements_instances](https://dev.mysql.com/doc/refman/8.4/en/performance-schema-prepared-statements-instances-table.html)
- [MySQL 8.0: The ENUM Type](https://dev.mysql.com/doc/refman/8.0/en/enum.html)

## ベンチマーク記録

| 記録 | 変更 | 60秒結果 | 値の扱い |
|---|---|---|---|
| [00-baseline.md](./tuning/00-baseline.md) | 公式Rust初期実装 | 共有負荷時は失敗・0点、静穏時再計測は`pass=true`・5,906点 | 条件別の実測各n=1。条件混在のため未推定 |
| [01-indexes.md](./tuning/01-indexes.md) | 高頻度SQLへB-tree INDEX追加 | `pass=false`、スコア364 | 実測n=1・未推定 |
| [02-notification-transactions.md](./tuning/02-notification-transactions.md) | 空通知pollingのtransaction削減 | `pass=true`、スコア2,357 | 実測n=1・未推定 |
| [03-owner-chairs.md](./tuning/03-owner-chairs.md) | owner対象へ絞ってから距離集計 | `pass=true`、スコア5,601、エラー0 | 実測n=1・未推定 |
| [04-nearby-chairs.md](./tuning/04-nearby-chairs.md) | nearby N+1を1 SQLへ集約 | `pass=true`、スコア4,116、`CODE=26` 1件 | 実測n=1・未推定 |
| [05-chair-stats.md](./tuning/05-chair-stats.md) | 通知内の椅子統計を1 SQLへ集約 | `pass=false`、スコア4,460、`CODE=32` 2件 | 実測n=1・未推定 |
| [06-matcher-batch.md](./tuning/06-matcher-batch.md) | matcherを最大64件のバッチ処理へ変更 | `pass=true`、スコア2,393、エラー0 | 実測n=1・未推定 |
| [07-matcher-nearest.md](./tuning/07-matcher-nearest.md) | 乗車地点に近い空き椅子を優先 | `pass=true`、スコア16,909、エラー0 | 実測n=1・未推定 |
| [08-coordinate-hot-path.md](./tuning/08-coordinate-hot-path.md) | 座標更新の通常経路を4 SQLから2 SQLへ削減 | `pass=true`、スコア11,599、`CODE=17` 2件 | 実測n=1・未推定 |
| [09-coupon-code-index.md](./tuning/09-coupon-code-index.md) | 招待coupon検索の全走査とlock範囲を削減 | `pass=true`、スコア15,415、エラー0 | 実測n=1・未推定 |
| [10-notification-retry-interval.md](./tuning/10-notification-retry-interval.md) | 通知pollingを30 / 50 / 100msで比較 | 30msを維持、50 / 100msは不採用 | 各条件実測n=1・未推定 |
| [11-matcher-interval.md](./tuning/11-matcher-interval.md) | matcherを500 / 100 / 30msで比較 | 500msを維持、30msは41,016点へ悪化 | 条件ごとのnは詳細に記載 |
| [12-status-covering-index.md](./tuning/12-status-covering-index.md) | 最新status検索をcovering INDEX化 | 実行計画は改善、45,075点のため不採用 | 実測n=1・未推定 |
| [13-mysql-commit-durability.md](./tuning/13-mysql-commit-durability.md) | redo / binary logのcommit同期を緩和 | 3走中央値53,198→60,102点、`COMMIT`平均中央値48.6%減 | 各条件実測n=3、中央値を推定代表値に使用 |
| [14-payment-client-reuse.md](./tuning/14-payment-client-reuse.md) | 決済HTTP clientとconnection poolを再利用 | 3走76,761–88,638点、中央値80,354点、エラー0 | 実測n=3、中央値を推定代表値に使用 |
| [15-coupon-used-by-index.md](./tuning/15-coupon-used-by-index.md) | rideに適用済みのcoupon検索をB-tree lookup化 | 3走88,805–100,606点、中央値93,606点、エラー0 | 実測n=3、中央値を推定代表値に使用 |
| [16-nearby-evaluation-filter.md](./tuning/16-nearby-evaluation-filter.md) | nearbyのstatus相関subqueryを除去し、全status writerをride row lockで直列化 | エラー0の3走98,311–98,628点、中央値98,580点 | 実測n=3、中央値を推定代表値に使用。queryだけの100,310点は競合反例により不採用 |
| [17-coordinate-transition-query.md](./tuning/17-coordinate-transition-query.md) | 通常座標のstatus取得を除き、遷移候補だけlock後にcurrent read | 3走98,311–98,628点、中央値98,580点。直前版比+6.6% | 実測n=3、中央値を推定代表値に使用 |
| [18-latest-location-cache.md](./tuning/18-latest-location-cache.md) | 最新座標をcurrent-state表 + process cacheへ分離し、2秒再同期・評価response body tracker・initialize gateを追加 | 最終3走96,888–98,483点、中央値96,926点。nearby SQLは最終run例8.079ms | エラー0の実測n=3。時間依存cooldown、handler-scope guard、暫定cache中央値103,683点は正当性反例により不採用 |
| [19-status-semantic-order.md](./tuning/19-status-semantic-order.md) | 通知と最新statusをwall-clock順からENUMの状態遷移順へ変更 | 3走89,539–99,895点、中央値98,338点。CODE=11は0件 | 実測n=3。診断runのCODE=11を回帰テストで再現し、app / chair両経路を検証 |
| [80-rust-implementation.md](./tuning/80-rust-implementation.md) | Rust / sqlxとrelease buildの知識 | 再build 30分52秒→11.02秒 | build時間の実測。スコア推定対象外 |
| [90-local-environment.md](./tuning/90-local-environment.md) | build context、BuildKit、固定Colima資源 | context 467MB→32.5KB | sizeの実測。スコア推定対象外 |

## 計測コマンド

```sh
# 60秒ベンチ
./scripts/benchmark.sh 60

# 実行中SQL
./scripts/compose.sh exec -T db \
  mysql -uroot -pisucon -e 'SHOW FULL PROCESSLIST'

# statement種類ごとの累積時間
./scripts/compose.sh exec -T db \
  mysql -uroot -pisucon performance_schema -e "
    SELECT DIGEST_TEXT,
           COUNT_STAR,
           ROUND(SUM_TIMER_WAIT / 1000000000000, 3) AS total_seconds,
           ROUND(AVG_TIMER_WAIT / 1000000000, 3) AS avg_ms
    FROM events_statements_summary_by_digest
    WHERE SCHEMA_NAME = 'isuride'
    ORDER BY SUM_TIMER_WAIT DESC
    LIMIT 20"

# コンテナ資源
docker stats --no-stream
```

`EXPLAIN ANALYZE` は候補SQLを実際に実行します。更新SQLへ無造作に使用せず、この記録では読み取りSQLだけに使用しています。
