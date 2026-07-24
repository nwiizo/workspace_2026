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

| 用語 | この文書での意味 |
|---|---|
| ベンチマーク | 決められた操作を一定時間実行し、正しさと処理量を測るプログラム |
| ボトルネック | 全体の速さを制限している、最も詰まっている部分 |
| SQL | MySQLへデータの検索・追加・更新を依頼する文 |
| INDEX | 検索対象を速く見つけるための、テーブルとは別の索引 |
| 全件走査 | 条件に合う行を探すため、テーブルの行を先頭からすべて確認すること |
| transaction | 複数のDB操作を「全部成功」または「全部取り消し」にまとめる仕組み |
| autocommit | SQL文を1つ実行するごとに、自動で確定するMySQLの通常動作 |
| connection | RustアプリとMySQLの間にある1本の通信路 |
| connection pool | connectionを毎回作らず、一定本数を複数リクエストで貸し借りする仕組み |
| polling | 新しい情報がないか、クライアントから一定間隔で繰り返し問い合わせること |
| N+1 | 最初の1回で一覧を取り、その各要素ごとに追加SQLを発行して回数が増える問題 |
| 実行計画 | MySQLがSQLをどの順序・方法で処理するかを示したもの |
| materialize | 途中結果を一時表として実際に作ること |
| window関数 | 行をグループ内の順序に並べ、前後の行を参照しながら計算するSQL機能 |

難しい用語が必要な箇所では、先に日常的な例を示し、その後に正確な仕組みを説明します。

## ベンチマーク記録

| 記録 | 変更 | 60秒結果 |
|---|---|---|
| [00-baseline.md](./tuning/00-baseline.md) | 公式Rust初期実装 | 共有負荷時は失敗・0点、静穏時再計測は`pass=true`・5,906点 |
| [01-indexes.md](./tuning/01-indexes.md) | 高頻度SQLへB-tree INDEX追加 | `pass=false`、スコア364 |
| [02-notification-transactions.md](./tuning/02-notification-transactions.md) | 空通知pollingのtransaction削減 | `pass=true`、スコア2,357 |
| [03-owner-chairs.md](./tuning/03-owner-chairs.md) | owner対象へ絞ってから距離集計 | `pass=true`、スコア5,601、エラー0 |
| [04-nearby-chairs.md](./tuning/04-nearby-chairs.md) | nearby N+1を1 SQLへ集約 | `pass=true`、スコア4,116、`CODE=26` 1件 |
| [05-chair-stats.md](./tuning/05-chair-stats.md) | 通知内の椅子統計を1 SQLへ集約 | `pass=false`、スコア4,460、`CODE=32` 2件 |
| [06-matcher-batch.md](./tuning/06-matcher-batch.md) | matcherを最大64件のバッチ処理へ変更 | `pass=true`、スコア2,393、エラー0 |
| [07-matcher-nearest.md](./tuning/07-matcher-nearest.md) | 乗車地点に近い空き椅子を優先 | `pass=true`、スコア16,909、エラー0 |
| [08-coordinate-hot-path.md](./tuning/08-coordinate-hot-path.md) | 座標更新の通常経路を4 SQLから2 SQLへ削減 | `pass=true`、スコア11,599、`CODE=17` 2件 |
| [09-coupon-code-index.md](./tuning/09-coupon-code-index.md) | 招待coupon検索の全走査とlock範囲を削減 | `pass=true`、スコア15,415、エラー0 |
| [10-notification-retry-interval.md](./tuning/10-notification-retry-interval.md) | 通知pollingを30 / 50 / 100msで比較 | 30msを維持、50 / 100msは不採用 |
| [80-rust-implementation.md](./tuning/80-rust-implementation.md) | Rust / sqlxとrelease buildの知識 | 再build 30分52秒→11.02秒 |
| [90-local-environment.md](./tuning/90-local-environment.md) | build context、BuildKit、固定Colima資源 | context 467MB→32.5KB |

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
