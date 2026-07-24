# Benchmark 24: owner売上の完了時刻境界

[チューニング目次へ戻る](../TUNING.md)

## 結論

`CODE=24` の候補だった「オーナー売上がベンチマーカーの確定済み売上より大きい」
競合を、外部決済を8秒遅らせるHTTP回帰テストで再現しました。

変更前は、評価APIが外部決済を始める前に `rides.evaluation` を更新していました。
`rides.updated_at` は `ON UPDATE CURRENT_TIMESTAMP(6)` なので、この時点が
オーナー売上の完了時刻にもなります。その後に外部決済で待つと、まだ評価レスポンスを
処理していないrideが、後から完了したrideの `until` より古い時刻を持ち得ます。

最終実装では次の順序にしました。

```text
rideをFOR UPDATE
決済token・運賃・決済先を取得
外部決済が成功
COMPLETEDを追加
chair_statsを差分更新
evaluation + completed_atを最終SQLで保存
COMMIT
同じcompleted_atを評価レスポンスへ返す
```

追加SQLで時刻を上書きする試作も正しさは満たしましたが、最終版では既存の
evaluation更新を決済後へ移しました。さらに、決済前にride全列を読み直していた
冗長な1 SQLを削除したため、変更前よりDB往復は1本少なく、完了時刻だけの追加UPDATEも
ありません。

## はじめに知っておく用語

### commit

transaction内の変更を、ほかのtransactionから見える確定状態にする操作です。
今回の評価APIでは、evaluation、`COMPLETED`、chair statsが同じcommitで公開されます。

commitとHTTPレスポンスの受信は同じイベントではありません。

```text
DB commit
  ↓
サーバーがresponse bodyを生成
  ↓
HTTP stackがsocketへ書く
  ↓
クライアントが受信・JSON decode
  ↓
ベンチマーカーが「確定済み評価」へ追加
```

DBはcommit直後から売上を返せますが、ベンチマーカーがそのrideを集計対象へ追加するのは
HTTPレスポンスを処理した後です。この観測時点の差が競合の土台になります。

### `updated_at` と完了時刻

`rides.updated_at` は一般的な「最後に行を変更した時刻」という名前ですが、この課題では
次の2つのAPI仕様をつなぐ重要な値です。

- 評価APIが返す `completed_at`
- `GET /api/owner/sales?until=...` が売上を含める上限

したがって、単なる監査用時刻ではありません。評価完了後に別用途でride行を更新すると、
履歴の完了時刻と売上期間が同時に変わります。

列型は `DATETIME(6)` なので、マイクロ秒まで保存できます。APIはミリ秒整数を返すため、
owner salesは指定ミリ秒の末尾まで含める目的で、上限へ999マイクロ秒を加えています。

```sql
updated_at BETWEEN :since AND :until + INTERVAL 999 MICROSECOND
```

### watermarkとしての `until`

watermarkは「ここまでのイベントは既知である」という境界です。ベンチマーカーは
受信済み評価の `ServerCompletedAt` の最大値を `until` としてowner salesへ渡します。

時刻が処理順と一致していれば、`updated_at <= until` のrideはベンチマーカーも知っている
はずです。しかし、外部決済の前に時刻を刻むと、処理中のrideがwatermarkより古くなり、
この前提が崩れます。

### snapshot

ベンチマーカーはowner確認の開始時点で、自分が処理済みと認識したride集合を複製します。
サーバーの売上がその集合より大きいと、まだ知らないrideを返したと判断します。

MySQLのtransaction snapshotだけを変更しても解決しません。問題のrideはowner query時には
すでにcommit済みであり、DBにとっては正しく見えるデータだからです。DBとHTTPクライアント
という2つの観測者の進み方をそろえる必要があります。

## ベンチマーカーで確認した判定

`bench/benchmarker/world/owner.go` と `world/user.go` を読み、次の順序を確認しました。

1. 評価APIを呼ぶ
2. responseを受信してdecodeする
3. `ServerCompletedAt` と `Evaluated=true` を設定する
4. ownerの `CompletedRequest` へ追加する
5. owner確認は、既知rideの最大 `ServerCompletedAt` を `until` にする
6. owner APIの値が現在の既知集合より大きければ `CODE=24`

ここから、単にSQLの合計式が間違っている場合だけでなく、完了時刻が処理順と逆転した
場合にも過大値が起きると考えました。

## 変更前の競合順序

ride Aは決済が遅く、ride Bは後から開始して先にレスポンスまで終わる例です。

```text
ride A: evaluation UPDATE (updated_at=t1)
        └─ 外部決済待ち ─────────────────┐

ride B:                 evaluation完了(t2) → clientがBを記録
                                                 t1 < t2

ride A:                                      commit

owner: GET /sales?until=t2
       DBはAとBを返す
       clientの既知集合はBだけ
       → server salesが700円大きい
```

重要なのは、ride Aのcommitがride Bより後でも、Aの `updated_at` は決済前の
`t1` のままだった点です。

## 確認したログと値

### 最初の再現が成立しなかった理由

最初は決済を2秒だけ遅らせ、評価transaction中に既知rideを新規INSERTしました。
結果は次でした。

```text
initial_total=435500
baseline_total=436900
after_total=436900
```

既知rideと評価中rideはどちらも700円なので、`baseline_total` は初期値から1,400円
増えています。つまり、基準売上を取る前に評価がcommitしており、検証したい境界を
作れていませんでした。

決済遅延を長くするだけでは、fixtureの新規INSERTが評価transactionと競合して
基準取得を遅らせる可能性も残ります。そこで既知rideは評価開始前に作成し、
評価rideのInnoDB行ロックを確認した後、既知rideの `updated_at` だけを進めました。

### condition-based waiting

「評価が始まっただろう」と固定sleepで推測せず、次の
`performance_schema.data_locks` をpollしました。

```sql
SELECT COUNT(*)
FROM performance_schema.data_locks
WHERE OBJECT_SCHEMA = 'isuride'
  AND OBJECT_NAME = 'rides'
  AND LOCATE(:pending_ride_id, COALESCE(LOCK_DATA, '')) > 0
```

対象rideの行ロックが実際に見えた後だけ、既知rideの時刻更新へ進みます。
ただし行ロックだけでは、handlerが外部決済まで到達したことを証明できません。
現在のテストは遅延決済mockを `nc -v` で起動し、container logの `connect to` も
条件pollします。名前付きpipeを使い、listener起動時ではなくrequestの1行目を受信した
時点から8秒delayを開始します。行ロックと決済request受理の両方を確認してから
既知rideの時刻を進めるため、端末やDockerの速度差を「何秒待つか」へ埋め込みません。

### 修正前の赤い結果

```text
pending.updated_at = 2026-07-24 19:21:06.364096
known.updated_at   = 2026-07-24 19:21:06.515276

initial_total  = 435500
baseline_total = 436200
after_total    = 436900
```

- 基準時点は既知rideの700円だけを含む
- 評価commit後は、クライアントが評価を計上する前提にしていないのにさらに700円増える
- pendingの時刻はknownより約151ms古い
- pendingには `COMPLETED` があり、`until` の範囲内に1行入る

この4点が同時に成立したため、時刻逆転の仮説を支持する証拠としました。

## 回帰テスト

`scripts/test-owner-sales-response-boundary.sh` は次を自動化します。

1. 8秒遅延する決済HTTPサーバーを一時コンテナで起動する
2. pending rideと既知のcompleted rideを作る
3. 評価APIを開始する
4. InnoDBのpending ride行ロックと、決済mockのTCP accept logを条件pollで待つ
5. known rideの時刻をpendingより後へ進める
6. pendingが未commitの基準売上を確認する
7. pendingのcommitを条件pollで待つ
8. 同じ `until` でowner salesを再取得する
9. pendingの時刻がknownより後で、売上が増えていないことを確認する

実行方法は次です。

```sh
./scripts/test-owner-sales-response-boundary.sh
```

最終実装では次を確認しました。

```text
OK: pending evaluation timestamp is after the known completion
OK: payment request acceptance and evaluation response completed_at were verified
OK: pending_updated_at known_updated_at completed_rows_in_window=
    2026-07-24 20:10:29.355394
    2026-07-24 20:10:21.516752
    0
OK: owner sales stayed at 436200 for the known ride's until boundary
```

決済成功後にpendingの完了時刻がknownより約7.84秒後になり、knownの `until` に入る
`COMPLETED` 行は0件です。

このテストは、ネットワーク上の未受信byteそのものを証明するものではありません。
時刻逆転を作った長い決済待ちを解消し、responseの `completed_at` とDBの
`updated_at` が一致することを検証します。DB commitからclient計上までの短い境界は、
Benchmark 25でride IDのresponse overlap trackerを追加して別に狭めています。

## 実装の選択

### 試作1: 決済成功後に時刻だけ再UPDATE

最初の修正は、既存処理順を維持し、決済成功後に次を追加しました。

```sql
UPDATE rides SET updated_at = :completed_at WHERE id = :ride_id
```

同じ `completed_at` をAPIレスポンスにも返すため、境界テストは通りました。ただし、
評価ごとにSQLが1本増えます。

公式60秒ベンチは次でした。

| run | pass | score | error map |
|---:|:---:|---:|---|
| 1 | true | 93,714 | 空 |
| 2 | true | 102,206 | 空 |
| 3 | true | 86,889 | 空 |

- 観測範囲: 86,889–102,206点
- 推定代表値: 中央値93,714点

正しさを得るために必須の追加SQLではなかったため、この形は採用しませんでした。

### 最終版: 既存の完了writeを決済後へ移動

最終版は、既存のevaluation UPDATE、`COMPLETED` INSERT、chair stats UPSERTを
決済成功後へ移しました。レビュー後はevaluation UPDATEをtransaction内の最終SQLにし、
時刻からcommitまでの区間も短くしています。

```rust
let completed_at = chrono::Utc::now();
sqlx::query("UPDATE rides SET evaluation = ?, updated_at = ? WHERE id = ?")
    .bind(req.evaluation)
    .bind(completed_at)
    .bind(&ride_id)
    .execute(&mut *tx)
    .await?;
```

Rustで作った1つの `DateTime<Utc>` をDBとレスポンスの両方へ使います。
DBはマイクロ秒、レスポンスは `.timestamp_millis()` でミリ秒になりますが、
どちらも同じ時点から導出されます。

決済前にride全列を再SELECTしていた処理は削除しました。最初の
`SELECT ... FOR UPDATE` で得たrideは同じtransaction中に保持され、必要な
user IDと座標をすでに持つためです。

## 正しさと残る境界

### 改善したこと

- 決済待ち時間を完了時刻へ含めない
- 長い決済待ちによるowner salesのwatermarkと評価処理順の逆転をなくす
- evaluation、`COMPLETED`、chair statsは同一transactionのまま
- 決済失敗時は完了writeへ進まない
- responseの `completed_at` とDBの期間境界を同じ値から作る

### この段階で解決していなかったこと

外部決済とMySQL commitは、1つの原子的transactionではありません。決済成功後に
MySQL errorやcommit失敗が起きれば、課金済みなのにAPIは失敗する可能性があります。
この可能性は従来も「決済成功後のcommit失敗」として存在しましたが、完了writeを
決済後へ移したことで、決済後に実行するSQLは増えています。

根本対策として列挙したうち、ride IDのidempotency keyはBenchmark 25で実装しました。
payment intent / outboxとcrash recoveryは引き続き未実装です。

- 決済requestへride ID由来のidempotency keyを付ける
- payment intent / outboxの状態をDBへ保存する
- retryしても同じ決済結果へ収束する
- process crash後に未確定決済を回収する

今回の変更だけを「決済のexactly-once保証」とは扱いません。

## 公式60秒ベンチマーク

固定したローカル条件は4 CPU / 4 GiBで、ホストとColimaのCPU・メモリ設定は
変更していません。

| run | pass | score | error map | 最終eval reqs | matching不満 | pickup不満 | drive不満 |
|---:|:---:|---:|---|---:|---:|---:|---:|
| 1 | true | 94,173 | 空 | 1,339 | 28.6% | 41.6% | 70.0% |
| 2 | true | 104,048 | 空 | 1,459 | 24.4% | 39.6% | 71.2% |
| 3 | true | 93,408 | 空 | 1,270 | 31.2% | 43.2% | 69.3% |

- 観測範囲: 93,408–104,048点
- 推定代表値: 中央値94,173点
- `CODE=24`: 3走合計0件
- 全run: `pass=true`、error map空
- 直前のBenchmark 23中央値103,046点との差: -8,873点、約-8.6%
- 追加UPDATE試作の中央値93,714点との差: +459点、約+0.5%

run 2は直前中央値を上回る一方、3走中央値は下回りました。処理内容と地域・ride生成には
走行ごとの分散があるため、今回のスコアから性能改善とは判断しません。
採用理由は、決定的な赤/緑テストで `CODE=24` を生んだ長い時刻逆転を除去でき、
追加SQLを残さず、冗長なSELECTも1本削減できたことです。commitからclient計上までの
境界は残るため、この段階だけで `CODE=24` を理論上完全に解消したとは扱いません。

## 併用した検証

```sh
cargo fmt --manifest-path webapp/rust/Cargo.toml -- --check
cargo clippy --manifest-path webapp/rust/Cargo.toml \
  --all-targets --all-features -- -D warnings
cargo test --manifest-path webapp/rust/Cargo.toml --all --all-targets
./scripts/test-owner-sales-response-boundary.sh
./scripts/test-chair-stats-transitions.sh
./scripts/test-chair-stats-consistency.sh
./scripts/smoke-test.sh
./scripts/benchmark.sh 60
```

- Rust unit test: Benchmark 24時点は14件、Benchmark 25の境界強化後は20件passed
- evaluation authorization / 決済成功 / 決済失敗 / 再送: 成功
- chair statsの初期化・再起動修復: 旧履歴集計との差0
- smoke test: `/` 200、`/api/initialize` は `{"language":"rust"}`

## 他に考えられる選択肢

### owner salesから評価配送中のride IDを除外する

process trackerへchair IDだけでなくride IDも持たせ、owner queryから評価配送中のrideを
除外する案です。これはBenchmark 25で実装しました。ただし固定leaseは使わず、
owner request開始時にactiveだったrideと、SQL中に完了したrideだけをrevisionで追います。
chair単位で除外すると過去の正しい売上まで消すため、ride単位にしました。
複数processでは共有状態が必要で、body dropからclient計上までの境界にはprotocol ACKが
必要という制約は残ります。

### owner salesを差分集計表へ移す

chair別・model別のcurrent-state表を評価transactionで更新すれば、N+1は減らせます。
しかし、評価responseより先に集計表を公開すれば同じ `CODE=24` が起きます。
保存先を変えるだけでは観測境界は解決しません。

### transaction isolation levelを変える

`READ COMMITTED` や `SERIALIZABLE` はDB transaction同士の見え方を変えます。
今回のowner queryはpending評価のcommit後に始まるため、isolation levelを上げても
確定済みrideは見えます。HTTPクライアントの既知集合とは同期しないため不採用です。

### 固定時間の除外

完了から一定時間だけowner salesへ入れない方法は簡単ですが、正しい売上を小さく返す
別の不整合を作り得ます。配送時間の上限もprotocolでは保証されません。
固定時間だけを根拠に正しさを主張しません。

### 外部決済をDB transactionの外へ出す

DB接続とride row lockを外部HTTP中に保持しないため、性能上の価値が大きい案です。
ただし、idempotency key、状態機械、crash recoveryを先に設計しないと、二重決済または
決済欠落へ変わります。次の独立したP0として扱います。

## 次に計測すること

1. `CODE=24` が再発した場合は、pending / knownのride ID、DB時刻、評価response処理時刻、
   owner snapshotの `until` を同じログへ出す
2. 評価transactionのp50 / p95 / p99を、決済HTTP、pool待ち、row-lock待ち、DB writeへ分解する
3. 実装済みidempotency keyを土台に、payment intentで外部HTTPをtransaction外へ出す設計を検証する
4. owner salesのN+1を1集約SQLへ変える場合も、この境界テストを必須回帰にする
