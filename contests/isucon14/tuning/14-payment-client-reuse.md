# Benchmark 14: 決済HTTP clientの再利用

[チューニング目次へ戻る](../TUNING.md)

## 結論

`reqwest::Client` を評価リクエストごとに作るのをやめ、`AppState` に1個だけ保持して
再利用しました。60秒ベンチ3走はすべて `pass=true`・エラー0で、
76,761–88,638点、中央値80,354点でした。

直前の同一構成であるMySQL `2 / 0` の3走中央値60,102点に対して、
中央値は20,252点、約33.7%増えました。3走の範囲をそのまま将来の保証値には
できませんが、変更後の最小値76,761点も変更前の最大値66,167点を上回りました。
正当性エラーも増えていないため、この変更を採用します。

## TODOからこの項目を選んだ理由

ソース監査では、`app_post_ride_evaluation` が次の処理を行っていました。

1. 決済の `POST /payments` を送る
2. 成否が不明な場合は `GET /payments` で決済済みか確認する
3. 一時エラーなら100ms待って最大5回繰り返す

変更前はPOSTと確認GETのたびに `reqwest::Client::new()` を呼んでいました。
`reqwest::Client` は内部にHTTP connection poolを持つため、requestごとに捨てると、
同じ決済サービスへ次のrequestを送るときにpoolを再利用できません。

評価処理にはDB transaction中の外部HTTPという、さらに大きな課題もあります。
しかしtransaction境界を変えるには、決済成功と `COMPLETED` 公開の順序、
重複決済、process停止時の復旧を同時に設計する必要があります。今回はまず、
決済の意味を変えずに通信資源だけを再利用できる小さな変更を選びました。

## connection再利用の仕組み

HTTP requestを送る前には、一般に名前解決、TCP connection確立、HTTPSならTLS
handshakeが必要です。今回のローカル決済URLはHTTPですが、TCP connection確立と
socket管理のコストは残ります。

`reqwest::Client` は、送信先ごとのidle connectionを内部poolへ保持します。同じ
clientから同じhostへrequestを送ると、相手がconnectionを閉じておらず利用可能なら、
既存connectionを再利用できます。

```text
変更前:
評価A -> Client生成 -> POST -> Client破棄
評価B -> Client生成 -> POST -> Client破棄

変更後:
AppStateのClient -> 評価AのPOST
                  -> 評価BのPOST
                  -> retry時のGET
```

再利用は「必ず同じTCP connectionを使う」という保証ではありません。相手側の
keep-alive設定、idle timeout、同時実行数、通信エラーによって新しいconnectionが
必要になる場合があります。重要なのは、再利用可能なときにpoolを捨てないことです。

`AppState` はAxumのhandlerへcloneされますが、`reqwest::Client::clone()` は内部状態を
共有するための軽量なhandleです。handlerごとに独立したpoolを複製するわけでは
ありません。

参考:

- [reqwest 0.12.9: `Client`](https://docs.rs/reqwest/0.12.9/reqwest/struct.Client.html)

## 仮説と反証条件

### 仮説

- POSTと確認GETで同じclientを使えば、決済serviceへのconnection poolを再利用できる
- connection確立とsocketの作成・破棄が減り、評価handlerの待ち時間が短くなる
- 評価が早く確定すると、椅子が次のrideへ再割当可能になるまでの時間も短くなる
- その結果、最終評価数と総スコアが増える

### 反証条件

- 3走中央値が直前の60,102点を上回らない
- 決済・評価に関するエラーが増える
- `pass=false` になる
- client再利用によって認証tokenやrequest bodyが別requestへ混ざる

`reqwest::Client` はrequest builderごとにheaderとbodyを持つため、clientを共有しても
Bearer tokenやJSON bodyは共有されません。

## 実装

`AppState` に決済用clientを追加し、process起動時に一度だけ生成します。

```rust
pub struct AppState {
    pub pool: sqlx::MySqlPool,
    pub payment_client: reqwest::Client,
}
```

評価handlerはclientへの参照を決済関数へ渡し、POSTと確認GETの両方で使います。

```rust
request_payment_gateway_post_payment(
    &payment_client,
    &payment_gateway_url,
    &payment_token.token,
    &PaymentGatewayPostPaymentRequest { amount: fare },
    |payment| payment.ride_id == ride_id,
)
.await?;
```

変更していないものは次のとおりです。

- 最大5回、100ms間隔というretry policy
- POSTが204以外のときにGETで結果を確認する処理
- Bearer tokenと決済額
- DB transactionの開始・commit位置
- 決済成功後に評価と `COMPLETED` を確定する順序

今回の唯一のsource変更はclient再利用です。観測範囲の分離はこの変更を採用する
強い根拠ですが、変更前後を交互に測るrevert runとTCP接続数の直接計測は行って
いません。そのため、connection poolが何回再利用され、何msを削減したかまでは
この比較だけでは断定しません。

## 計測条件

- 2026-07-24
- Apple Silicon / Colima 4 CPU・4 GiB
- ホストとColimaのCPU / memoryは変更なし
- Rust、SQL、INDEX、matcher 500msは同じ
- MySQL 8.4.10、`innodb_flush_log_at_trx_commit=2`、`sync_binlog=0`
- 公式ベンチマーカー60秒、静的ファイル検証あり
- 各run前にstackを再起動し、`POST /api/initialize` から開始

## ベンチ結果

### 変更前

直前の採用構成で記録済みの3走です。

| run | pass | スコア | 最終tick評価数 | エラー |
|---:|---:|---:|---:|---:|
| 1 | true | 66,167 | 926 | 0 |
| 2 | true | 60,102 | 877 | 0 |
| 3 | true | 58,220 | 802 | `CODE=31` 1件 |

- 観測範囲: 58,220–66,167点
- 代表値: 中央値60,102点

### 変更後

| run | pass | スコア | 最終tick評価数 | matching不満 | pickup不満 | drive不満 | エラー |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | true | 76,761 | 1,038 | 31.2% | 41.5% | 69.7% | 0 |
| 2 | true | 88,638 | 1,224 | 25.8% | 40.4% | 74.7% | 0 |
| 3 | true | 80,354 | 1,112 | 32.3% | 41.4% | 74.2% | 0 |

- 観測範囲: 76,761–88,638点
- 代表値: 中央値80,354点
- 変更前中央値との差: +20,252点、約+33.7%
- 全run: `pass=true`、error map空

中央値は、3個のスコアを小さい順へ並べた中央の値です。平均値より極端な1走の
影響を受けにくいため、少数回の採用判断では中央値を代表値にしました。
ただし3走だけでは信頼区間を安定して推定できません。76,761–88,638点は今回実際に
観測した範囲であり、将来のスコアが必ずこの範囲に入るという予測区間ではありません。

## どのログを見て、どう判断したか

ベンチマーカーの最終行で次を確認しました。

```text
結果 pass=true スコア=76761 種別エラー数=map[]
結果 pass=true スコア=88638 種別エラー数=map[]
結果 pass=true スコア=80354 種別エラー数=map[]
```

さらに途中の `eval reqs` と最終不満率を読み、単に1件の長距離rideで点が増えたの
ではなく、最終評価数が変更前の802–926件から1,038–1,224件へ増えていることを
確認しました。

変更後run 3の終了直後には、MySQL `performance_schema` で次を確認しました。

| statement | 回数 | 累積時間 | 平均 |
|---|---:|---:|---:|
| `BEGIN` | 203,592 | 4.034秒 | 0.020ms |
| `COMMIT` | 203,364 | 209.556秒 | 1.030ms |
| `ROLLBACK` | 213 | 0.008秒 | 0.035ms |

この変更はDB transaction数を減らすものではありません。処理量が増えた結果、
transaction回数は増えています。したがって「COMMIT回数が減ったから速くなった」
とは判断しません。

nginx access logでは変更後run 3に評価POSTが1,151件ありました。webapp logには
決済・reqwest・gatewayを含むエラー行がありませんでした。nginxはwebappから
決済serviceへの内部通信を中継しないため、TCP再利用回数そのものはこのログからは
分かりません。採用根拠は、同じ実装条件での3走中央値、評価数、全runの正当性です。

## 実際にはどうだったか

仮説どおり、変更後は3走すべてが変更前の最大値を上回り、最終評価数も増えました。
これは評価確定の待ちが減り、椅子を次のrideへ戻す循環が速くなったという説明と
整合します。

一方、今回TCP `connect()` 回数やconnection IDを直接採取していません。そのため
「connection確立が何回から何回へ減ったか」は未確認です。スコア差と実装の仕組みは
仮説を支持しますが、因果の内部指標を埋めるには診断runが必要です。

## 次に考えられる選択肢

### 1. connection再利用を直接測る

診断runだけで `ss`、`tcpdump`、または `strace -e connect` を使い、決済先への
新規connection数、TIME_WAIT、再送を数えます。計測自体が負荷になるため、最終
スコアrunとは分けます。

### 2. poolのidle timeoutと最大idle数を調整する

決済serviceのkeep-aliveと実際の並行数を観測してから調整します。大きすぎるpoolは
socketを余分に保持し、小さすぎるpoolは再接続を増やします。既定値を根拠なく
変えるより、connection数と待ち時間を先に測ります。

### 3. `Idempotency-Key` で確認GETをなくす

ride IDを冪等keyにできれば、POSTの応答が不明な場合も同じkeyで安全にretryでき、
一覧GETと全件JSON decodeを除去できる可能性があります。ただし決済serviceがkeyを
どのように扱うか、同じkeyで異なる金額を拒否できるかを確認する必要があります。

### 4. 外部HTTPをDB transaction外へ出す

現在も決済待ちと100ms sleepの間、DB connectionとtransactionを保持します。
先に決済し、成功後に短いwrite transactionで評価を確定すればpool待ちとlock保持を
減らせます。ただし次の障害境界を設計せずに順序だけ変えてはいけません。

- 決済成功後、DB確定前にprocessが停止する
- client retryにより同じrideを二重決済する
- DBだけ完了し、決済が欠落する
- 別requestが決済中のrideを同時評価する

outbox、決済状態表、一意な冪等key、reconciliation処理を含めて別の性能実験として
扱います。

## 残すTODO

- 診断runで決済先へのTCP connect回数とconnection再利用率を採取する
- 評価handlerを「pool取得」「SQL」「外部HTTP」「retry sleep」に分けてp95 / p99を測る
- `Idempotency-Key` と確認GET削除の正当性を検証する
- 外部HTTPをtransaction外へ出すための障害回復設計を作る
