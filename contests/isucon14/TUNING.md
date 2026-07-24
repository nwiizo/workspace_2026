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
| 日時 | 2026-07-24 |
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
| 高頻度検索へのINDEX | 主要INDEXと `coupons(code)` を追加済み。`users(access_token)` と `users(invitation_code)` は既存の `UNIQUE` INDEXで充足 | `coupons(used_by)` を単独比較し、未使用INDEXを増やさない |
| nearbyの2N+1解消 | `LATERAL` と `NOT EXISTS` で1 SQL化済み | 未完了判定を `rides.evaluation IS NULL` へ単純化して比較 |
| owner椅子一覧をownerで先に絞る | 実装・単独ベンチ済み | 最新位置と累積距離のcurrent-state化 |
| 最新位置と累積距離をUPSERT管理 | 未実装 | 履歴INSERTと同じtransactionでcurrent-stateを更新 |
| pending rideと空き椅子のbatch matching | 最大64件、近傍優先まで実装済み | 地域間の距離上限、実行間隔、二部マッチングを比較 |
| JSON通知のcache | 未実装 | 同じpayloadの再計算をなくし、long pollingをSSEより先に比較 |
| 座標更新の非同期・bulk INSERT | 通常経路を4 SQLから2 SQLへ削減済み | per-chair順序付きqueueと3秒以内のbulk反映を実験 |
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

### 計測と判断

#### ベンチマーク

決められた操作を一定時間実行し、正しさ、処理量、応答時間を測るプログラムです。
ISUCON14の公式ベンチマーカーは、単にHTTP requestを大量送信するだけでは
ありません。利用者がrideを依頼し、椅子が移動し、状態通知を受け、評価と決済まで
終える一連の世界を進めます。

そのため、単一APIだけを速くしても、状態の順序を壊したり、完了rideが増えなければ
総スコアは上がりません。最終行の `pass`、スコア、error mapに加え、途中の
`eval reqs` とmatching / pickup / drive不満率を読みます。

#### 実測値・推定値・中央値

実測値は、そのrunでログに出た事実です。推定値は、限られた実測から「同じ条件を
繰り返したときの代表的な値」を推し量ったものです。この文書では同一条件を3回以上
測れた場合だけ、中央値を推定代表値として使います。

1走の80,000点を「通常80,000点」とは扱いません。3走が76,761、80,354、
88,638点なら、観測範囲は76,761–88,638点、推定代表値は中央値80,354点です。
観測範囲の外へ次回値が出ない保証はありません。

#### latency・throughput・p95 / p99

latencyは1回の処理が終わるまでの時間、throughputは一定時間に終えられる処理量です。
平均latencyが5msでも、100回に1回だけ500msかかれば、その遅い1回が状態進行を
止めることがあります。

p95は測定値を短い順に並べた95%地点、p99は99%地点です。p99が高いと、少数の
requestだけが大きく遅れる「裾の長い」分布だと分かります。ISUCON14は30msで
1tick進むため、平均だけでなく30ms超過率とp95 / p99が重要です。

#### ボトルネック

全体の速度を最も強く制限している境界です。水道管の一番細い箇所を太くしない限り
全体の流量が増えないのと同じです。CPU使用率が高い箇所だけを意味しません。
DB connection待ち、diskへのflush、外部HTTP、lock、短すぎるpollingも
ボトルネックになります。

1つを解消すると別の待ちが次のボトルネックになります。MySQLのCOMMIT待ちを減らした
後にmatcherの待ちや決済HTTPが目立つのは正常な変化です。変更後は必ず計測し直し、
前回の分析をそのまま使い続けません。

### SQLとMySQL

#### SQL

MySQLへ検索、追加、更新、削除を依頼する言語です。Rust側で1行に見える
`sqlx::query(...).fetch_all(...)` も、network越しにMySQLへrequestを送り、
MySQLが解析・実行し、結果を返し、Rustが型へdecodeする複数段階を含みます。

SQLの性能は文の長さでは決まりません。何行を読み、どのINDEXを使い、途中結果を
何行作り、何回呼ばれるかで決まります。単発0.1msのSQLでも60秒に20万回呼ばれれば
累積負荷になります。

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

#### 全件走査

条件に合う行を探すため、tableまたはINDEXの広い範囲を先頭から確認する処理です。
小さなtableでは問題にならなくても、履歴が増えると読んだ行数に比例して遅くなります。
MySQLの実行計画では `Table scan`、`rows`、`actual rows` などを確認します。

全件走査そのものが常に悪いわけではありません。500行の大半を返す処理では、
INDEXを何度もたどるよりscanが速い場合があります。問題は「返すのは1行なのに
数万行読む」ことや、それが高頻度経路で繰り返されることです。

#### 実行計画と `EXPLAIN ANALYZE`

実行計画は、MySQLがtableをどの順序で読み、どのINDEXとjoin方法を使うかを示します。
見積りだけの `EXPLAIN` に対し、`EXPLAIN ANALYZE` は読み取りSQLを実際に実行し、
各段階のactual time、rows、loopsを表示します。

見積り行数と実測行数が大きく違うと、統計情報が実データ分布を表していない可能性が
あります。`loops` が外側の行数だけ増えていれば、相関subqueryやnested loopが
繰り返されていると判断できます。更新SQLへ無造作に使うと実データを変更するため、
この記録では読み取りSQLに限定しています。

#### materialize

subqueryやCTEの途中結果を一時的な表として作る処理です。同じ途中結果を再利用できる
利点がある一方、行数が多いとmemoryを使い、収まらなければdisk上の一時表へ移る
可能性があります。

実行計画に `Materialize` があっても即座に悪いとは判断しません。作る行数、作る回数、
その後どれだけ再利用されるかを見ます。相関subqueryの内側で何千回もmaterialize
される場合と、request中に一度だけ小さな表を作る場合では意味が異なります。

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
