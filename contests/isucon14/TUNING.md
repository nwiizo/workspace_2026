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
| [00-baseline.md](./tuning/00-baseline.md) | 公式Rust初期実装 | `pass=false`、スコア0 |
| [01-indexes.md](./tuning/01-indexes.md) | 高頻度SQLへB-tree INDEX追加 | `pass=false`、スコア364 |
| [02-notification-transactions.md](./tuning/02-notification-transactions.md) | 空通知pollingのtransaction削減 | `pass=true`、スコア2,357 |
| [03-owner-chairs.md](./tuning/03-owner-chairs.md) | owner対象へ絞ってから距離集計 | `pass=true`、スコア5,601、エラー0 |
| [04-nearby-chairs.md](./tuning/04-nearby-chairs.md) | nearby N+1を1 SQLへ集約 | `pass=true`、スコア4,116、`CODE=26` 1件 |
| [05-chair-stats.md](./tuning/05-chair-stats.md) | 通知内の椅子統計を1 SQLへ集約 | `pass=false`、スコア4,460、`CODE=32` 2件 |
| [06-matcher-batch.md](./tuning/06-matcher-batch.md) | matcherを最大64件のバッチ処理へ変更 | `pass=true`、スコア2,393、エラー0 |
| [07-matcher-nearest.md](./tuning/07-matcher-nearest.md) | 乗車地点に近い空き椅子を優先 | `pass=true`、スコア16,909、エラー0 |
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
