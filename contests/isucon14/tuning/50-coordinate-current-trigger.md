# Benchmark 50: 位置履歴INSERTトリガーによるcurrent更新の統合

[チューニング目次へ戻る](../TUNING.md)

## 結論

`chair_locations`への履歴INSERT後に、Rustから
`chair_current_locations`を更新する構成に対し、MySQLの`AFTER INSERT`
トリガーでアプリ発行DMLを2本から1本、DB往復を2回から1回へ減らしました。

アプリから見た位置書込み区間は平均3.510msから2.563msへ約27.0%短くなりました。
しかし、通常60秒ベンチ3走の中央値は121,185点で、直前のshared pool構成の
中央値135,410点を10.51%下回りました。同じ時間帯に元の2クエリ構成へ戻した
単発対照は132,970点でした。

単発対照だけでは候補固有の回帰を因果推定できません。ただし、候補3走が過去の
shared pool分布をすべて下回り、採用を支持するscore evidenceがなかったため、
保守的に不採用としました。トリガー実装はすべて戻し、現在のソースは履歴INSERTと
current UPDATEをRustから同じtransaction内で実行する構成です。

## 計測条件

| 項目 | 値 |
|---|---|
| 計測日 | 2026-07-25 |
| 2クエリ対照 | `24c7a1bfb792f39df5650b13ad454f53fe237f71` |
| トリガー候補 | `2c46bb2914e152f30c613e025f03d05c9cb32968` |
| CPU / memory | Colima 4 CPU / 4 GiB、全runで変更なし |
| disk | 100 GiB、変更なし |
| architecture | Apple Silicon / aarch64 |
| MySQL | 8.4.10、image digest固定 |
| DB pool | shared 50、general DB phase permit 26、coordinate headroom 24 |
| 診断 | 30秒、候補と2クエリ対照を各1回。通常スコア推定には不使用 |
| 通常 | 候補60秒を3回、候補を戻した同時間帯対照60秒を1回 |

候補の通常3走は同一ソースです。診断ログは
`/tmp/isucon14-b50-trigger.*`、通常ログは
`/tmp/isucon14-b50-normal-{1,2,3}.*`、復元対照は
`/tmp/isucon14-b50-control.*`へ保存しました。`/tmp`の生ログはcommit対象ではなく、
必要な集計値をこの文書へ転記しています。

## 調べた理由

Benchmark 49で、static poolの片側に余った接続を融通できない問題を解消しました。
shared pool 50とgeneral DB phase permit 26を採用した後も、coordinate診断では
次の時間が残っていました。

| phase | 平均 | p95 |
|---|---:|---:|
| 履歴INSERT | 0.926ms | 2.690ms |
| current更新 | 2.584ms | 8.155ms |
| COMMIT | 4.611ms | 12.626ms |
| handler全体 | 14.061ms | 36.534ms |

履歴と最新状態は同じ座標を別の形で保存しています。

```text
chair_locations
  すべての座標を保存する履歴

chair_current_locations
  chairごとに最新1行だけを保存するprojection
```

通常経路は次の2往復でした。

```text
Rust
  -> INSERT chair_locations
  <- 完了
  -> UPDATE chair_current_locations
  <- 完了
  -> COMMIT
```

DBが履歴INSERTと同時にcurrent rowを更新できれば、current-stateを維持しながら
アプリとDBの2回目の往復を消せる、という仮説を立てました。

## INDEXの役割

この施策はINDEXを追加する施策ではありません。既存の2つのINDEXを使って
書込み対象を特定します。

```sql
chair_locations:
  PRIMARY KEY (id)
  INDEX idx_chair_locations_chair_created_at (chair_id, created_at)

chair_current_locations:
  PRIMARY KEY (chair_id)
```

`chair_current_locations`の主キーはB-treeです。`chair_id = ?`の更新では、MySQLは
全chairを順番に調べず、B-treeを根からたどって対象の1行を探します。計算量の目安は
全件走査の`O(N)`ではなく`O(log N)`です。

ただしINDEXは読取りを無料にする仕組みではありません。履歴INSERTでは主キーと
`(chair_id, created_at)`の両方へentryを追加し、current更新では主キーを探して
対象行をlockします。INDEXを追加するほどINSERT時の更新対象も増えるため、
「検索に使えそう」という理由だけで増やしません。

トリガーに変えても、current rowを主キーで探して更新するDBの仕事は残ります。
削減できるのは主にRustとMySQL間の1往復であり、B-tree更新やrow lockではないことが
重要です。

## 実験したSQL

候補は履歴行の`AFTER INSERT`トリガーで、最新状態をguard付きupsertしました。

```sql
CREATE TRIGGER chair_locations_after_insert_current
AFTER INSERT ON chair_locations
FOR EACH ROW
INSERT INTO chair_current_locations (
  chair_id,
  location_id,
  latitude,
  longitude,
  created_at
)
VALUES (
  NEW.chair_id,
  NEW.id,
  NEW.latitude,
  NEW.longitude,
  NEW.created_at
) AS incoming
ON DUPLICATE KEY UPDATE
  latitude = IF(
    incoming.created_at > chair_current_locations.created_at
      OR (
        incoming.created_at = chair_current_locations.created_at
        AND incoming.location_id > chair_current_locations.location_id
      ),
    incoming.latitude,
    chair_current_locations.latitude
  ),
  longitude = IF(
    incoming.created_at > chair_current_locations.created_at
      OR (
        incoming.created_at = chair_current_locations.created_at
        AND incoming.location_id > chair_current_locations.location_id
      ),
    incoming.longitude,
    chair_current_locations.longitude
  ),
  location_id = IF(
    incoming.created_at > chair_current_locations.created_at
      OR (
        incoming.created_at = chair_current_locations.created_at
        AND incoming.location_id > chair_current_locations.location_id
      ),
    incoming.location_id,
    chair_current_locations.location_id
  ),
  created_at = GREATEST(
    incoming.created_at,
    chair_current_locations.created_at
  );
```

`NEW.*`は挿入された履歴行です。時刻だけで比較すると同一時刻の結果が不定になるため、
canonical orderを`(created_at, location_id)`にしました。新しい時刻、または同じ時刻で
大きいlocation IDだけをcurrentへ反映します。

トリガーは元のINSERTを実行したtransactionの一部です。履歴INSERTをrollbackすると
current更新もrollbackされます。この原子性は、Rustから2クエリを同じtransactionで
実行する現在の方式と同じです。

## Dockerと既存volumeで必要だった処理

初期SQLでは21,209件の履歴からcurrent rowを一括構築した後にトリガーを作りました。
最初からトリガーを有効にすると、初期データの全履歴INSERTでcurrent upsertが走り、
初期化時間と書込み量が増えるためです。

既存volumeにはトリガーがないので、Rust起動時にも`CREATE TRIGGER IF NOT EXISTS`を
実行する候補を作りました。`POST /api/initialize`は元tableをdropするため、
そのトリガーも一緒にdropされ、初期SQLで再作成されます。

MySQLはbinary logが有効な状態で、通常の`isucon`ユーザーからトリガーを作ると
次のエラーになりました。

```text
ERROR 1419:
You do not have the SUPER privilege and binary logging is enabled
```

ローカルの単一MySQLで複製を使っていないため、候補runだけ
`--log-bin-trust-function-creators=ON`を追加しました。これはCPU / memory変更では
ありませんが、トリガー作成者を信頼する設定なので、共有DBへ理由なく入れる設定では
ありません。候補棄却時に設定も戻しました。

## Rust / SQLxで確認した点

SQLxの`query()`はprepared statementを使います。MySQLは候補の
`CREATE TRIGGER`をprepared protocolで受け付けず、起動時に次のエラーになりました。

```text
MySQL error 1295:
This command is not supported in the prepared statement protocol yet
```

値を外部入力から組み立てない固定DDLだけを`sqlx::raw_sql()`へ変更すると起動しました。
通常のDMLはbind parameterとprepared statementを維持しています。

```rust
sqlx::raw_sql(
    r#"
CREATE TRIGGER IF NOT EXISTS ...
    "#,
)
.execute(pool)
.await?;
```

`raw_sql()`はparameter bindを提供しないため、リクエスト値を文字列連結する用途には
使いません。今回安全だった理由は、コンパイル時に固定されたDDLだけを渡したためです。

## 正当性の検証

候補専用の回帰スクリプトを一時的に作り、次を確認しました。

- 古い`created_at`を後からINSERTしてもcurrent rowが巻き戻らない
- 同じ`created_at`では大きいlocation IDが選ばれる
- transaction内で新しい履歴をINSERT後にrollbackすると、履歴とcurrentの両方が戻る
- startup repair、2秒reconciliation、owner距離watermarkが既存どおり動く
- `GET /`はHTTP 200、`POST /api/initialize`は`{"language":"rust"}`

最初の手動fixtureでは既存のlocation IDを再利用してMySQL 1062になりました。
これは候補実装の障害ではなくfixtureの主キー重複だったため、異なる26文字IDへ直して
再実行しました。失敗の種類を分けるため、アプリのerror mapには混ぜていません。

候補を棄却したため、専用トリガーtestも最終ソースから削除しました。既存の
`test-latest-location-reconciliation.sh`と`test-owner-distance-watermark.sh`は
現在の2クエリ構成に対して引き続き使用できます。

## 診断ベンチ

Colimaの4 CPU / 4 GiB、DB接続総数50、shared pool、general permit 26を固定しました。
診断あり30秒runなので、スコアは通常runの推定値に混ぜません。

| 項目 | 2クエリ対照 | トリガー候補 | 差 |
|---|---:|---:|---:|
| score | 60,582 | 59,410 | -1.93% |
| coordinate sample | 468 | 446 | - |
| pool acquire平均 | 3.663ms | 3.492ms | -0.171ms |
| 履歴INSERT平均 | 0.926ms | 2.563ms | trigger込み |
| current更新平均 | 2.584ms | 0ms | 2回目の往復なし |
| 履歴 + current平均 | 3.510ms | 2.563ms | -0.947ms、-27.0% |
| COMMIT平均 | 4.611ms | 4.870ms | +0.259ms |
| handler全体平均 | 14.061ms | 13.659ms | -0.402ms、-2.86% |
| handler p95 | 36.534ms | 34.184ms | -2.350ms |
| handler p99 | 60.683ms | 61.241ms | +0.558ms |
| handler最大 | 75.329ms | 93.603ms | +18.274ms |
| InnoDB row-lock waits | 1,525 | 1,477 | process起動後累積 |
| InnoDB row-lock time | 29.799s | 25.876s | process起動後累積 |

アプリ側の2回目の往復は消えましたが、tail latencyと診断スコアは一方向に改善して
いません。

row-lock値にはcurrent更新以外の全DB処理も含まれます。参考としてcurrent write件数で
正規化すると、1,000 writeあたりのwaitは対照51.2回、候補51.8回、lock timeは
対照1.000秒、候補0.908秒です。処理構成が異なるrunのprocess累積値なので、
current更新単体の改善や採否の根拠には使いません。

`performance_schema`でもDB内のcurrent更新は残っていました。

| 実装 | 実行回数 | 平均 | 最大 | 累積 |
|---|---:|---:|---:|---:|
| Rustからのprepared current write | 29,792 | 1.257ms | 114.228ms | 37.438s |
| `AFTER INSERT`トリガー | 28,513 | 1.314ms | 134.384ms | 37.461s |

runごとに処理件数が違うため累積時間を直接優劣に使いません。1回平均を見ると、
トリガーがDB内のcurrent更新を高速化した証拠はなく、往復削減だけが観測できました。
トリガー統計の`SUM_LOCK_TIME`は23.493秒でしたが、これはInnoDBのrow-lock waitだけを
表す値ではないため、別表のrow-lock metricと混同しません。

## 通常60秒ベンチと採否

トリガー候補を同じrevision・同じ資源で3走しました。

| run | score | pass | error map |
|---:|---:|---|---|
| 1 | 121,185 | true | 空 |
| 2 | 117,326 | true | 空 |
| 3 | 121,580 | true | 空 |
| 中央値 | 121,185 | - | - |

最終時点の不満率もstdoutから採取しました。

| run | matching | pickup | drive |
|---:|---:|---:|---:|
| 候補1 | 56.7% | 33.1% | 61.5% |
| 候補2 | 60.7% | 35.9% | 58.9% |
| 候補3 | 67.3% | 32.9% | 58.5% |
| 2クエリ単発対照 | 68.9% | 30.8% | 55.7% |

このrunでは完了ride数、空車移動、乗車中移動のscore内訳を独立保存していません。
したがって、score差を特定の不満率や処理段階へ帰属させません。次の比較では
benchmarkerのscore componentをrunごとに保存する必要があります。

比較値は次のとおりです。

| 比較 | score |
|---|---:|
| Benchmark 49 shared pool 3走中央値 | 135,410 |
| トリガー候補3走中央値 | 121,185 |
| 候補を戻した同時間帯の2クエリ対照 | 132,970 |

- Benchmark 49中央値比: `-14,225`点、`-10.51%`
- 同時間帯対照比: `-11,785`点、`-8.86%`
- 同時間帯対照は候補中央値より9.72%高い

候補3走がすべて`pass=true`でも、採用を支持するスコア分布ではないため採用しません。
同時間帯対照は1走なので、差の因果推定には候補 / 対照の近接ペアと実行順反転がさらに
必要です。今回の結論は「トリガーが10.51%の低下を起こした」という断定ではなく、
「正当性だけでは採用せず、性能evidenceが不足する候補を保守的に棄却する」です。

## なぜ局所改善して全体が悪化したか

確定できた事実は次です。

- RustとMySQL間の1往復は削減できた
- DB内のcurrent row lookup、更新、INDEX保守、lockは残った
- coordinate平均は約0.4ms短くなった
- p99と最大値は改善しなかった
- 候補3走は過去中央値より約10.5%低く、単発対照も候補中央値より約9.7%高かった

トリガーは親の履歴INSERT文の実行区間へcurrent更新を組み込みます。別statementで
更新する方式とlock取得順・statement境界・schedulerへ見える待機形態が変わるため、
高並行時のtailや他queryとの干渉が変わった可能性があります。ただし、今回のmetricだけで
「特定のrow lockが原因」とは断定できません。score componentも保存していないため、
全体scoreが候補3走で低かった具体的な原因は今回の計測では特定不能です。

## 他に考えられる選択肢

### 1. 現在の2クエリ構成を維持する

今回維持した選択です。明示的で、Rustの診断phaseを分けやすく、単発対照は候補3走を
上回りました。DB内の仕事は残りますが、性能evidenceのないトリガー候補を持ち込みません。

### 2. 1つのstored procedureへまとめる

ネットワーク往復は減らせますが、DB内の仕事とlockはトリガー同様に残ります。
deploy、権限、計測の複雑さも増えるため、今回の結果から優先度を下げます。

### 3. current rowを非同期更新する

HTTP handlerからcurrent更新を外せますが、nearbyの3秒可視性、全履歴、同一chair順序、
pickup / destination到着判定、shutdown / initialize時のflushを同時に守る必要があります。
次の実験はper-chair bounded queueとして、depth、最古未flush時間、drop / retry、
status反映遅延を計測します。

### 4. current UPDATEをやめて履歴から毎回読む

書込みは減りますが、Benchmark 18以前の候補chairごとの履歴sortへ戻ります。
nearbyの単発SQLが約26.4msだったため、現状では選びません。

### 5. 履歴を間引く

累積距離とpickup / destination到達判定が変わるため、無条件の間引きは正答性を壊します。
nearby用最新値のcoalesceと、全履歴・状態判定用queueを分離する案だけを検討します。

## 再現コマンド

最終mainには候補実装を残していませんが、候補commitを履歴へ保存しています。
現在のjj workspaceを動かさずに再現する場合は、別cloneでcommitを指定します。

```sh
# 別directoryへcloneし、候補を固定
git clone <this-repository-url> isucon14-b50-reproduction
cd isucon14-b50-reproduction/contests/isucon14
git checkout --detach 2c46bb2914e152f30c613e025f03d05c9cb32968

# 候補の起動と正当性fixture
./scripts/up.sh
./scripts/smoke-test.sh
./scripts/test-current-location-trigger.sh

# 候補の通常60秒
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b50-trigger.log \
  ./scripts/benchmark.sh 60

# 候補の30秒診断
diagnostic_since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
ISUCON_DIAGNOSTIC=1 \
MYSQL_STATUS_OUTPUT_FILE=/tmp/isucon14-b50.mysql.tsv \
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b50.log \
  ./scripts/benchmark.sh 30

./scripts/report-coordinate-phases.sh "$diagnostic_since"
./scripts/report-db-admission.sh \
  "$diagnostic_since" \
  /tmp/isucon14-b50.mysql.tsv

# 対照は同じcloneで比較元commitを指定し、同じコマンドを実行する
git checkout --detach 24c7a1bfb792f39df5650b13ad454f53fe237f71
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b50-control.log \
  ./scripts/benchmark.sh 60
```

候補と対照を厳密に因果比較する次回実験では、`候補→対照`と`対照→候補`を含む
近接ペアを複数回実行します。今回は候補3走と対照1走なので、再現コマンドを残しても
因果の確度自体が増えるわけではありません。

## 参考資料

- [MySQL 8.4: Trigger Syntax and Examples](https://dev.mysql.com/doc/refman/8.4/en/trigger-syntax.html)
- [MySQL 8.4: CREATE TRIGGER Statement](https://dev.mysql.com/doc/refman/8.4/en/create-trigger.html)
- [MySQL 8.4: INSERT ... ON DUPLICATE KEY UPDATE](https://dev.mysql.com/doc/refman/8.4/en/insert-on-duplicate.html)
- [SQLx 0.8.2: `raw_sql`](https://docs.rs/sqlx/0.8.2/sqlx/fn.raw_sql.html)
- [Benchmark 18: 最新位置cache](./18-latest-location-cache.md)
- [Benchmark 49: shared DB pool](./49-db-shared-pool-admission.md)
