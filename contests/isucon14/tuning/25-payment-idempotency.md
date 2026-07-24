# Benchmark 25: 決済の冪等化とowner売上の配送境界

[チューニング目次へ戻る](../TUNING.md)

## 結論

`TODO.md` の決済・評価項目から、次の3点を優先して実装しました。

1. すべての決済POSTへride IDを `Idempotency-Key` として付ける
2. 一時エラー時の `GET /payments` とuserの全ride取得を削除し、同じkeyでPOSTを再試行する
3. owner売上では、評価レスポンスの配送と集計処理が重なったride IDだけを除外する

最初の2点は、Benchmark 24で明確になった「決済成功後にDB更新が失敗すると、
同じ評価の再送で二重課金し得る」という問題への対策です。3点目は、
完了時刻を決済後へ移しても残る、DB commitから評価レスポンス処理までの短い境界を
小さくする対策です。

最終レビュー反映後の公式60秒ベンチマークは95,596 / 101,037 / 115,968点、
推定代表値の中央値101,037点でした。3走とも `pass=true`、error mapは空です。直前の
Benchmark 24中央値94,173点より約7.3%高い一方、Benchmark 23中央値103,046点より
約1.9%低いため、最高点更新とは扱いません。決済の正当性を上げながら、エラー時の
確認GETと全ride取得を除去できたことを採用理由とします。

## はじめに知っておく用語

### 冪等性

冪等性は、同じ操作を複数回実行しても、最終的な効果が1回実行した場合と同じになる
性質です。

決済で重要なのは、HTTPリクエストの回数ではなく課金の回数です。通信切断や500応答が
起きると、呼び出し側には「決済されなかった」のか「決済は成功したが応答だけ失われた」
のか分かりません。単純にPOSTを再送すると、後者で2回課金する可能性があります。

```text
client             payment service
  | POST payment          |
  |---------------------->|
  |                 課金は成功
  |<---- 応答が途中で消失 |
  |
  | POSTを単純再送         |
  |---------------------->|
  |                 2回目の課金
```

同じ操作へ同じ `Idempotency-Key` を付けると、決済サービスは2回目を新しい決済として
扱わず、最初の結果へ収束させられます。

### idempotency key

idempotency keyは、呼び出し側が「これはどの論理操作か」を示す識別子です。今回の
論理操作は「1つのrideの利用料金を1回支払うこと」なので、ride IDを使います。

よいkeyには次の性質が必要です。

- 同じ論理操作のretryでは同じ値になる
- 異なる論理操作では衝突しない
- process再起動後も再構築できる
- payloadと対応関係を説明できる

requestごとに新しいULIDを作ると、retryのたびにkeyが変わるため冪等になりません。
user IDだけでは、そのuserの複数rideが同じkeyへ衝突します。ride IDはDBへ既に保存され、
再送・再起動でも同じ値を使えるため、この課題に適しています。

### at-least-onceとexactly-once

retryする通信は、同じrequestを1回以上送るat-least-onceになりやすい設計です。
idempotency keyは、複数回送っても決済の効果を1回へ収束させます。

ただし、これだけでアプリケーション全体がexactly-onceになるわけではありません。
決済サービスとMySQLは別システムで、共通transactionを持たないためです。

```text
決済成功 ─ process crash ─ MySQL未更新
```

この場合も同じride IDで再送すれば二重課金は避けられますが、未完了rideを自動回収して
再試行する仕組みは別途必要です。idempotency keyは「重複を安全にする土台」であり、
crash recoveryそのものではありません。

### response delivery boundary

DB commitと、クライアントがHTTPレスポンスを処理する時刻は一致しません。

```text
MySQL COMMIT
  ↓
Axum / Hyperがresponse bodyを送る
  ↓
ベンチマーカーがJSONをdecodeする
  ↓
既知の完了ride集合へ追加する
```

owner売上はDBの完了rideを読みます。一方、ベンチマーカーの上限は受信済み評価から
作られます。この2つが重なると、DBにはあるがベンチマーカーはまだ知らないrideを
返す可能性があります。

## 公式実装をどう確認したか

決済仕様の説明だけでなく、次のローカルソースを読みました。

- `docs/ISURIDE.md`
- `bench/payment/server.go`
- `bench/payment/handler.go`
- `bench/payment/handler_test.go_`

`bench/payment/handler.go` は、keyがある場合に既知keyの `Payment` を再利用します。
既存keyへ異なるtokenまたはamountを送ると422を返し、同じpayloadなら処理済み結果を
再利用します。処理中の同じkeyには409を返します。

この実装から、Rust側の正しいretry条件を次のように決めました。

- 1回目とretryで同じride IDを送る
- tokenとamountをretry中に変えない
- network error、同じkeyの処理中を示す409、5xxは同じPOSTをretryする
- 400や422など、同じkey・payloadでは回復しない4xxは即時に失敗する
- 履歴件数の一致を使って「成功したはず」と推測しない

## 変更前の問題

変更前はPOSTが204以外を返すと、次の確認をしていました。

```text
POST /payments
  └─ 204以外
       ├─ GET /payments
       ├─ SELECT 全rides WHERE user_id = ? ORDER BY created_at
       └─ payment件数とride件数を比較
```

この方式には3つの問題があります。

### 件数一致は対象rideの成功を直接示さない

全ride数と全payment数が同じでも、今回のrideが支払い済みかを識別していません。
順序やamountも比較せず、件数だけを成功判定に使っています。

### エラー時に外部HTTPとDB全履歴読取りが増える

決済サービスは高負荷ほど一時エラーを返しやすい実装です。負荷が高いときほど
確認GETとuserのride全件取得が増え、評価transactionとride row lockの保持時間を
さらに延ばします。

### 決済成功後のDB失敗をまたぐretryに弱い

決済が204を返した後、MySQL writeまたはcommitが失敗すると、アプリは未完了のままです。
同じ評価APIを再送したとき、以前は新しい決済POSTになり、二重課金し得ました。

## 実装

### ride IDを全POSTへ付ける

`request_payment_gateway_post_payment` はkeyを明示的に受け取ります。

```rust
request_payment_gateway_post_payment(
    &payment_client,
    &payment_gateway_url,
    &payment_token.token,
    &ride_id,
    &PaymentGatewayPostPaymentRequest { amount: fare },
)
.await?;
```

送信側はすべての試行で同じheaderを設定します。

```rust
client
    .post(format!("{payment_gateway_url}/payments"))
    .bearer_auth(token)
    .header("Idempotency-Key", idempotency_key)
    .json(param)
    .send()
    .await?;
```

retry loopの外でkeyを作り直していないため、500、502、504、通信エラーのいずれでも
同じride IDが使われます。

### GET照合を削除する

204以外は `PostPayment(status)` として分類します。network error、409、5xxだけを
retryし、それ以外の4xxは即時に返します。決済履歴GET、callback trait、user ID、
transaction参照、ride全件取得を削除しました。

これにより `payment_gateway.rs` は「決済POSTを冪等に再試行する」という責務へ縮まり、
DBのride一覧と決済履歴の件数を比較する間接的な成功判定を持ちません。

### 完了時刻を最終SQLにする

決済成功後のwrite順序は次です。

```text
COMPLETEDを追加
chair_statsを更新
evaluation + updated_atを更新  ← 最終SQL
COMMIT
同じupdated_atをresponseへ返す
```

`updated_at` をtransaction内の最後に書くことで、完了時刻からcommitまでの区間を
短くします。transaction内の変更はcommitまで外から見えないため、途中で
`COMPLETED` だけが公開されることはありません。

### owner売上は重なったrideだけ除外する

既存の `ActiveRideEvaluationTracker` にride IDも持たせました。owner request開始時に
snapshotを取り、次の集合を売上結果から除外します。

- owner request開始時に評価response bodyが生存していたride
- ownerのSQL実行中に評価response bodyが完了したride
- ownerのSQL完了時にも評価response bodyが生存しているride

開始前に完了済みのrideは除外しません。固定1秒の猶予をowner売上にも使うと、
ベンチマーカーが既に知っている正しい売上まで小さく返す可能性があるためです。

trackerは単調増加するrevisionを使います。snapshot開始後にguardがdropしたrideは、
drop時点でactive集合から消えても `completed_ride_evaluations` のrevisionから復元できます。
live snapshotより新しい完了記録だけを保持し、次のsnapshot開始時に不要な記録を回収します。

## 検証

### Rust unit test

ローカルTCPサーバーを使い、1回目に500、2回目に204を返しました。受信した2 requestが
どちらも次を満たすことを確認します。

- `POST /payments`
- `Idempotency-Key: ride-1`
- GETではない

422を1回だけ返すtestも追加し、永続的なclient errorをretryしないことを確認しました。
owner trackerには、開始前に完了したrideを除外せず、開始時active、実行中active、
実行中完了のrideだけを返すtest、generationとpruneをride IDで直接検証するtest、
ownerの売上filterが指定rideだけを除外するtestを追加しました。

```text
cargo test --all --all-targets
20 passed
```

### HTTP・DB境界テスト

`scripts/test-owner-sales-response-boundary.sh` は、8秒遅延決済を使います。今回、
次の確認を追加しました。

- InnoDBのride行ロックだけでなく、決済mockのTCP accept logを条件pollする
- 名前付きpipeを使い、TCP requestの1行目を受信してから8秒delayを開始する
- 評価response JSONをparseする
- responseの `completed_at` とDBの `updated_at` のミリ秒値を比較する

最終結果は次です。

```text
OK: pending evaluation timestamp is after the known completion
OK: payment request acceptance and evaluation response completed_at were verified
OK: pending_updated_at known_updated_at completed_rows_in_window=
    2026-07-24 20:10:29.355394
    2026-07-24 20:10:21.516752
    0
OK: owner sales stayed at 436200 for the known ride's until boundary
```

行ロックだけでは、handlerが決済awaitまで到達したとは限りません。またlistener起動と
同時にsleepすると、fixture作成中にdelayを消費します。mockはrequestの1行目を受信してから
sleepを始め、acceptも待つことで、known rideの時刻を動かす条件を
「決済requestが実際に始まり、8秒の待機区間に入った後」に固定しました。

### 併用した検証

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
./scripts/test-owner-sales-response-boundary.sh
./scripts/test-chair-stats-transitions.sh
./scripts/test-chair-stats-consistency.sh
./scripts/smoke-test.sh
```

- Rust unit test: 20 passed
- Clippy: `-D warnings` で成功
- chair statsの遷移・再送: 成功
- initialize / 再起動repair: 旧履歴集計との差0
- smoke test: `/` は200、initializeはRustを返した

## 公式60秒ベンチマーク

ホストとColimaのCPU・メモリは変更せず、4 CPU / 4 GiBのままです。

| run | pass | score | error map | 最終eval reqs | matching不満 | pickup不満 | drive不満 |
|---:|:---:|---:|---|---:|---:|---:|---:|
| 1 | true | 95,596 | 空 | 1,370 | 23.5% | 42.0% | 69.8% |
| 2 | true | 101,037 | 空 | 1,402 | 26.8% | 40.3% | 69.5% |
| 3 | true | 115,968 | 空 | 1,639 | 16.1% | 36.6% | 76.1% |

- 観測範囲: 95,596–115,968点
- 推定代表値: 中央値101,037点
- 全run: `pass=true`、error map空
- Benchmark 24中央値94,173点との差: +6,864点、約+7.3%
- Benchmark 23中央値103,046点との差: -2,009点、約-1.9%

3走の幅は20,372点あり、決済変更だけの因果効果を精密に推定できるsample数ではありません。
また、このベンチにはowner ride overlapの強化も含まれます。したがって、+7.3%を
`Idempotency-Key` 単独の改善率とは扱いません。

一方、3走すべてでエラーがなく、3走目は評価request 1,639件と115,968点まで進みました。
エラー時に確認GETとuser履歴SELECTを追加しない設計が、高負荷時の余計な仕事を減らすという
仮説とは整合します。

最終レビュー前の候補版は100,033 / 115,709 / 96,794点、中央値100,033点でした。
その後、永続4xxの不要retry停止、ownerが一時保持する値を全 `Ride` から
`(ride_id, sale)` へ縮小、境界test追加を行いました。最終中央値は候補版より
+1,004点、約+1.0%ですが、run間の分散内なので個別変更の効果とは断定しません。

## レビュー指摘をどう判断したか

### 決済成功後のDB失敗で二重課金する

妥当と判断しました。公式決済サービスにkeyの再利用機能があり、ride IDは論理操作と
1対1なので実装しました。

### 完了時刻を決済後へ移すだけではresponse境界が残る

妥当と判断しました。完了時刻のwriteを最終SQLへ移し、owner requestと実際に重なる
ride IDをtrackerで除外しました。

### owner売上にもbody drop後1秒leaseを使う

採用しませんでした。nearbyは椅子を一時的に返さない保守的判断が可能ですが、owner売上は
下限より小さい値もエラーになります。既知rideまで1秒隠すと過小計上を作るためです。

## 残る制約

### body dropからclient計上まで

serverが観測できるのはresponse bodyの消費・dropまでです。その後、benchmark clientが
JSONをdecodeして既知集合へ追加するまでの時刻をserverは知りません。今回のride overlapは
owner requestとbody lifecycleが重なる区間を覆いますが、body drop直後にowner requestが
始まり、client計上だけが遅れる境界はprotocol ACKなしでは完全に閉じられません。

したがって「CODE=24を理論上完全に解消した」とは記録しません。3走でCODE=24を含む
error mapが空だったことと、残存境界を分けて扱います。

### DB transaction中の外部HTTP

評価handlerは、決済HTTPと100ms retry sleepの間もMySQL connection、transaction、
ride row lockを保持します。idempotency keyにより外へ出すための安全性は上がりましたが、
今回まだtransactionを分割していません。

次の設計では少なくとも次が必要です。

- `PAYMENT_PENDING` などの明示状態を条件付きでclaimする
- DB transactionを閉じてから冪等な決済POSTを行う
- 短いwrite transactionでevaluationとCOMPLETEDを確定する
- process crash後のpending状態を再開する
- 同じrideの並行評価を1つへ収束させる

### 複数process

ride trackerはprocess内メモリです。単一webapp構成では有効ですが、評価とowner requestが
別processへ振り分けられる構成では共有されません。水平分割する前にDBまたはRedisへ
配送中rideを共有し、世代とcrash後の回収を設計します。

## 次に計測すること

1. 評価handlerをpool待ち、最初のread transaction、決済HTTP、write transactionへ分けて
   p50 / p95 / p99を採取する
2. transaction外決済を状態機械と故障注入付きで実装し、row-lock保持時間を比較する
3. 同じrideへ並行評価2本を送り、決済記録1件・COMPLETED 1件へ収束することを確認する
4. owner salesのN+1を1集約SQLへ変え、ride overlap filterを維持したまま計測する
5. `CODE=24` 再発時はbody drop、client decode、owner snapshot、ride ID、revisionを同じ
   timelineへ記録する
