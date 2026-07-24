# Benchmark 26: 通知payload cacheと状態不変時のpoll間隔

[チューニング目次へ戻る](../TUNING.md)

## 結論

`GET /api/app/notification` と `GET /api/chair/notification` に、recipient ID単位の
process内payload cacheを追加しました。app payloadが含むchair statsには別のdependency
revisionを持たせ、別userの評価後に古いstatsを返さないようにします。DBに未送信statusが
残る間は従来どおり30msでpollし、全status送信後またはride未作成時だけ、cache済みresponseの
`retry_after_ms` を100msにします。

レビューでcross-user dependencyを補った後の公式60秒ベンチマークは111,798 /
103,727 / 109,443点でした。小さい順は103,727 / 109,443 / 111,798なので、
推定代表値の中央値は109,443点、観測範囲は103,727–111,798点です。
3走とも `pass=true`、error mapは空でした。

直前のBenchmark 25中央値101,037点との差は次のとおりです。

```text
109,443 - 101,037 = 8,406
8,406 / 101,037 ≒ 8.3%
```

最初に試した「cacheしても常に30ms」の3走は77,341 / 88,757 / 90,850点、
中央値88,757点まで悪化しました。DB処理は減りましたが、応答が速いほど次のpollが
早く始まり、HTTP request数が増えるclosed-loop loadになったためです。この失敗を
隠さず、状態不変時だけ100msへ伸ばす根拠として残します。

## はじめに知っておく用語

### polling

pollingは、新しい状態があるかclientから一定間隔で繰り返し問い合わせる方式です。
ISURIDEではnotification responseの `retry_after_ms` が、次回requestまでの待ち時間を
clientへ伝えます。

変化がないpollにも、次の仕事が発生します。

1. HTTP requestをnginxが受ける
2. Axum middlewareが認証する
3. 最新rideと未送信statusを読む
4. fare、chair、stats、userを読む
5. JSONを生成する
6. HTTP responseを返す

1回が小さくても、60秒で10万回近く繰り返すと累積時間とtransaction数が大きくなります。

### closed-loop load

前のresponseを受け取ってから一定時間待ち、次のrequestを送る負荷をclosed-loopと
呼びます。この形では、serverが速く返すほど同じclientが次のrequestを早く送れます。

```text
遅いresponse
  request ───── 120ms ───── response ─ 30ms待機 ─ next request

速いcache hit
  request ─ 2ms ─ response ─ 30ms待機 ─ next request
```

cache hitを2msへ短縮しても待機を30msのままにすると、1 client当たりのrequest数が
増えます。DB query数が減っても、nginx、socket、認証middleware、JSON bytesの送信、
benchmark clientの処理は残ります。このため、cache hit率だけでは総合性能を判断できません。

### payload cache

payloadはresponse bodyに入るJSONの内容です。今回のcacheは、appまたはchairへ最後に返した
定常状態のJSON bytesをrecipient IDごとに保存します。

```text
app key   = user ID
chair key = chair ID
app value = JSON bytes + 参照したchair stats revision
chair value = JSON bytes
```

access tokenをkeyにしないため、cacheのdebug情報やkey一覧がcredentialそのものになりません。
同じrecipientが状態不変の間にpollした場合、MySQLとJSON再生成を通らず同じbytesを返します。

### cache hitとcache miss

cache hitは保存済みpayloadを利用できる場合、cache missはDBを正本として読み直す場合です。
cache missは失敗ではありません。process起動直後、initialize直後、ride作成、chair割当、
status追加、評価確定の後は意図的にmissへ戻します。

変更後診断runのHTTP件数と最新ride存在確認SQLの終了時snapshotから単純比を取ると、
appは約82%、chairは約75%がcache hit相当です。ただし
`prepared_statements_instances` は終了したconnectionの実行を失うため、厳密なhit率では
なく、おおよその内部整合確認として扱います。

### invalidation

invalidationは、正本が変わったとき古いcacheを削除することです。cache実装で難しいのは
保存処理より、更新点を漏れなく列挙することです。

今回のapp通知payloadが変わるeventは次です。

- 新しいrideの作成
- matcherによるchair割当
- `ENROUTE` / `PICKUP` / `CARRYING` / `ARRIVED` / `COMPLETED` の追加
- 評価確定に伴う `updated_at` とchair statsの変更
  - statsは同じchairを過去に利用した別userのpayloadにも含まれる
- initializeによるDB世代の入れ替え

chair通知payloadが変わるeventは次です。

- matcherによる新しいrideの割当
- statusの追加
- initialize

TTLだけに頼ると、期限内は古いpayloadを返します。今回はwriterのcommit後に対象IDを
明示的にinvalidateし、miss時はDB履歴へ戻します。

### revisionとgeneration

単に `HashMap::remove` するだけでは、読み取りと更新が競合したとき古いpayloadが後から
cacheへ戻るraceが残ります。

```text
notification A: revision=4を読む
writer B:       DBをcommit、revision=5へ進めてcacheを削除
notification A: 古いDB snapshotのpayloadをcacheへ保存
```

そこでrecipientごとのrevisionを持ち、miss開始時のrevisionと保存時のrevisionが同じ場合
だけinsertします。上の例では4と5が違うため、Aの古いpayloadは保存されません。

app payloadにはrecipient revisionとは別に、参照したchair statsのrevisionも保存します。
あるuserの評価でchair statsが増えると、そのchairを参照する別userのentryもlookup時に
missになります。評価前に始まったreaderが古いstatsを後から保存しようとしても、
chair stats revisionが一致しないためinsertを拒否します。

generationはinitializeでDB全体が入れ替わる世代です。initialize前のsnapshotは、
同じuser IDやchair IDが偶然存在しても、新世代cacheへ保存できません。

### 配信cursorとresponse loss

通知は全statusを状態遷移順に返す必要があります。cacheは
`ride_statuses.app_sent_at` と `chair_sent_at` の代わりではありません。

未送信statusが1件でもあるrequestはcacheしません。DB transaction内で最も早い未送信statusを
1件返し、sent時刻を更新します。全未送信statusを送り終えた次のpollで初めて、最新状態の
payloadをcacheします。

```text
未送信 MATCHING  -> DBから返す、cacheしない
未送信 ENROUTE   -> DBから返す、cacheしない
未送信 PICKUP    -> DBから返す、cacheしない
未送信なし       -> 最新PICKUPをDBから返し、ここでcache
状態不変poll     -> cache hit
```

これにより、最新1件のcacheが途中のstatusを飛ばすことを防ぎます。

ただし既存APIはHTTP responseのACKを受け取れません。`*_sent_at` はJSON生成・socketへの
配送より前にcommitされるため、その後にserialization failure、handler cancellation、
または接続切断が起きると、clientが受け取っていないstatusを次回pollで再送できません。
したがって現状をat-least-onceとは呼びません。今回のcacheはこの既存cursorを変更せず、
未送信statusをcacheで飛ばさない範囲の改善です。厳密な再送保証にはclient ACKを伴う
protocolか、次回pollで前回statusをACKしてからcursorを進める設計が必要です。

### p50 / p95 / p99と累積時間

p50は半数のrequestがその時間以下、p95は95%、p99は99%がその時間以下だった境界です。
平均だけでは、一部の長いrequestがbenchmarkerのtickを止める現象を見落とします。

累積時間は、各requestの処理時間を合計した値です。並列処理のためwall-clock 60秒を
超えても異常ではありません。高頻度endpointでは、単発が中程度でも回数により最大の
累積負荷になります。

## なぜ通知を最優先にしたか

Benchmark 25の次候補には、決済HTTPをDB transaction外へ出す案もありました。決済APIは
1回平均約403msと単発では最も重い一方、診断runの累積は約691秒でした。

変更前診断runでは、通知2経路が次の値でした。

| endpoint | count | average | p50 | p95 | p99 | 累積 |
|---|---:|---:|---:|---:|---:|---:|
| app notification | 94,763 | 113ms | 96ms | 274ms | 344ms | 10,726.603秒 |
| chair notification | 71,941 | 130ms | 119ms | 289ms | 352ms | 9,357.486秒 |
| evaluation POST | 1,717 | 403ms | 391ms | 739ms | 816ms | 691.260秒 |

通知2経路の累積は約20,084秒で、evaluation POSTの約29倍でした。さらに
app notificationは最大8 SQL、chair notificationは最大6 SQLとcommitを、同じpayloadでも
繰り返します。

決済transaction分割には、永続的なpending状態、同時評価のclaim、process crash後の回収、
同じtokenとamountの再構築が必要です。大きな状態機械を先に追加せず、回数と累積寄与が
最大の通知へ、正当性を限定できるcacheを試しました。

## 仮説と反証条件

最初の仮説は次でした。

> 未送信statusがない状態不変pollをprocess内payload cacheから返せば、通知SQLと
> transactionが減り、30ms超過と総スコアが改善する。

反証条件を次のように置きました。

- 状態遷移の順序を壊す、または既存cursorよりresponse lossを増やす
- appまたはchairへstale payloadを返す
- `pass=false` またはerror mapを悪化させる
- 3走中央値が直前の101,037点を下回る
- SQLは減ってもHTTP request増加により総スコアが下がる

30ms固定cacheは最後の2条件に該当し、最初の仮説をそのままでは棄却しました。

次の仮説は、観測したclosed-loop loadだけを変更しました。

> 未送信status中は30msを維持し、状態不変cacheだけ100msにすれば、遷移配信を遅らせる
> 区間を限定しながら、cache hitによるHTTP request増加を抑えられる。

## 実装

### cacheを `AppState` で共有する

`NotificationCache` は `Arc<StdMutex<NotificationCacheState>>` を持ちます。Axumが
`AppState` をcloneしても、全requestが同じcacheとrevisionを参照します。

lock内では `HashMap` のlookup、insert、remove、revision加算だけを行い、DB queryや
`.await` を実行しません。同期mutexのguardをI/O待ち中に保持しないため、Tokio workerを
長時間止めません。

payloadは `Bytes` で保持し、cache hitでは参照countを増やしてresponse bodyへ渡します。
同じJSONを毎回serializeして新しい `String` / `Vec<u8>` を作る処理も省きます。

### cacheする条件

cacheするのは次の2条件だけです。

1. recipientにrideが1件もない
2. 最新rideに未送信statusがない

未送信statusを返したrequestは、sent時刻をcommitしてもcacheしません。次のpollで
もう一度DBを確認し、未送信がないことを確認してからcacheへ移ります。この1回の追加missで、
status列を途中までしか送っていない状態をcacheする危険を避けます。

### writer別のinvalidation

| writer | app cache | chair cache | 理由 |
|---|---|---|---|
| `app_post_rides` | user ID | - | 最新rideとMATCHINGが変わる |
| `internal_get_matching` | rideのuser ID | 割当chair ID | 同じMATCHINGでもchair情報が追加される |
| `chair_post_ride_status` | rideのuser ID | chair ID | ENROUTEまたはCARRYINGが増える |
| `chair_post_coordinate` | rideのuser ID | chair ID | PICKUPまたはARRIVEDが増えた場合だけ |
| `app_post_ride_evaluation` | user ID + chair stats revision | chair ID | COMPLETED、evaluation、updated_at、別userからも参照されるstatsが変わる |
| `post_initialize` | 全entry | 全entry | DB世代が入れ替わる |

座標更新は高頻度ですが、pickupまたはdestinationへ到着してstatusを追加したrequestだけ
invalidateします。通常座標ごとに通知payloadを捨てると、cache hitがほぼ消えるためです。

### なぜ `data: null` へ短絡しないか

未送信statusがないことは「payloadが変わっていない」と同義ではありません。
matcherはstatusを追加せず `rides.chair_id` を更新します。

```text
MATCHING / chairなし
  ↓ matcher
MATCHING / chairあり
```

app clientには同じ `MATCHING` を、今度はchair情報付きで返す必要があります。
そのため未送信statusがないだけで `data: null` を返す案は採用せず、matcher commit後の
明示的invalidationを入れました。

### 30msと100msを分ける

未送信statusがあるresponseは30msです。全status送信後またはrideなしのcacheable responseは
100msです。

100msは新しいstatusの発見を最大約70ms遅らせ得ます。一方、ISUCON14の通知反映要件は
3秒以内です。drive評価には150msの厳しい余分遅延予算があるため、すべてを100msへ
変えるのではなく、status列を配送中の区間は30msに残しました。

## 失敗した30ms固定cache

| run | pass | score | 最終評価数 | matching不満 | pickup不満 | drive不満 | error map |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | true | 77,341 | 1,061 | 52.9% | 45.3% | 63.6% | `CODE=26: 1` |
| 2 | true | 88,757 | 1,222 | 44.7% | 40.3% | 66.2% | 空 |
| 3 | true | 90,850 | 1,237 | 47.0% | 39.6% | 64.9% | `CODE=17: 1` |

中央値は88,757点で、直前101,037点を下回りました。

run 1のHTTP件数はapp notification 138,854、chair notification 88,236でした。
一方、変更前診断runは94,763 / 71,941です。計測構成とworldが同一ではないため単純な
倍率を因果値にはしませんが、cache hit後も30msで次pollを促し、HTTP requestが増える
仮説と整合します。

このrunでも通知本体SQLはapp約1.3万回、chair約1.3万回まで減り、COMMITは98,943回でした。
「DB query削減は成功したが、システム全体のscoreは失敗した」例です。局所指標だけで
採用してはいけない理由がここにあります。

## chair stats dependency追加前の100ms steady-state版

| run | pass | score | 最終評価数 | matching不満 | pickup不満 | drive不満 | error map |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | true | 114,996 | 1,661 | 53.6% | 32.9% | 64.2% | 空 |
| 2 | true | 103,957 | 1,434 | 50.4% | 36.9% | 62.6% | 空 |
| 3 | true | 112,156 | 1,562 | 46.7% | 36.9% | 65.0% | 空 |

この3走後の独立レビューで、app payloadにはchair statsが含まれ、別userが同じchairを
評価しても過去userのrecipient revisionは変わらないcross-key依存が見つかりました。
したがって上表は性能仮説の途中結果として残しますが、最終実装の代表値には使いません。

## cross-user dependency修正後の最終公式ベンチ

| run | pass | score | 最終評価数 | matching不満 | pickup不満 | drive不満 | error map |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | true | 111,798 | 1,548 | 54.4% | 38.9% | 62.5% | 空 |
| 2 | true | 103,727 | 1,423 | 48.9% | 39.6% | 63.6% | 空 |
| 3 | true | 109,443 | 1,547 | 60.7% | 39.6% | 61.8% | 空 |

| 指標 | Benchmark 25 | Benchmark 26 |
|---|---:|---:|
| 観測範囲 | 95,596–115,968 | 103,727–111,798 |
| 推定代表値 | 101,037 | 109,443 |
| 中央値差 | - | +8,406 |
| 推定改善率 | - | +8.3% |
| pass / error | 全run pass・error 0 | 全run pass・error 0 |

Benchmark 25の最大値115,968点は今回の最大値より4,170点高い一方、Benchmark 26は最小値と
中央値が上がりました。最高の1走ではなく分布の中央と正当性を採用根拠にします。

matching不満率は48.9–60.7%と高く、今回の変更でmatching policy自体は変えていません。
pickup不満率は38.9–39.6%、最終評価数は1,423–1,548件でした。score増加をpayload cache
だけへ断定せず、通知・DB負荷が下がった結果、coordinateと評価まで進む件数が増えたという
説明を、内部計測と合わせて採用します。

dependency追加前の中央値112,156点からは2,713点、約2.4%低下しました。worldのばらつきに加え、
chair stats変更時の正しいcache missも増えるため、性能だけを理由にdependency確認を外しません。

## 診断runの変更前後

通常スコアrunとは別に、`ISUCON_DIAGNOSTIC=1` でnginx JSON timing logを有効にしました。
計測はmacOSホストの `alp 1.0.21` でDocker containerのstdoutを集計しています。
表の診断runではrequest URI、method、status、request / upstream時間、response bytesを
記録しました。run後、次回の切り分け用にrequest bytes、upstream connect time、
connection ID、connection上のrequest回数も診断設定へ追加しています。追加項目は
nginx構文と出力形式を検証しますが、下表の変更前後比較には使っていません。
Cookie、token、request body本文、決済情報は記録しません。

変更前診断runは121,341点、変更後診断runは118,024点でした。各n=1でworldも異なるため、
このスコア差から性能を推定しません。endpoint内部値の方向を見る診断結果です。

| endpoint | 指標 | 変更前 | 変更後 | 変化 |
|---|---|---:|---:|---:|
| app notification | count | 94,763 | 107,954 | +13.9% |
| app notification | average | 113ms | 37ms | -67.3% |
| app notification | p50 | 96ms | 2ms | -97.9% |
| app notification | p95 | 274ms | 166ms | -39.4% |
| app notification | p99 | 344ms | 257ms | -25.3% |
| app notification | 累積 | 10,726.603秒 | 3,941.695秒 | -63.3% |
| chair notification | count | 71,941 | 75,993 | +5.6% |
| chair notification | average | 130ms | 51ms | -60.8% |
| chair notification | p50 | 119ms | 5ms | -95.8% |
| chair notification | p95 | 289ms | 181ms | -37.4% |
| chair notification | p99 | 352ms | 269ms | -23.6% |
| chair notification | 累積 | 9,357.486秒 | 3,887.219秒 | -58.5% |

変更後の499はapp 233件、chair 233件でした。nginx 499はclientがresponse完了前に接続を
閉じたことを表します。5xxは0件です。p95はまだ30ms tickを大きく超えるため、cacheを
入れたことで通知P0が完了したとは扱いません。

transactionは次でした。

| 指標 | 変更前診断 | 変更後診断 |
|---|---:|---:|
| BEGIN | 258,588回・4.260秒 | 142,097回・2.613秒 |
| COMMIT | 258,400回・307.922秒 | 141,956回・304.965秒 |
| COMMIT平均 | 1.192ms | 2.148ms |
| ROLLBACK | 177回・0.011秒 | 139回・0.012秒 |

COMMIT回数は約45.1%減りましたが、平均が増えたため累積は約1.0%減に留まります。
worldの処理量、同時実行、I/O競合が異なるため、「COMMIT回数を減らした分だけ累積時間も
比例して減る」とは判断しません。

変更後の終了時prepared statement snapshotでは、HTTP件数に対して最新ride存在確認SQLが
app 19,416回、chair 18,742回でした。毎pollでMySQLへ入っていないことを確認できます。
`Performance_schema_prepared_statements_lost=0`、`Connections=88`でした。ただし終了した
connectionのinstanceはsnapshotから消え得る制約があります。

## 正当性検証

Rust unit testは24件すべて成功しました。cache固有では次を確認しています。

- current revisionならpayloadを再利用できる
- appとchairのnamespaceを混同しない
- invalidation後に古いrevisionからinsertできない
- initialize後に前generationからinsertできない
- 別userの評価でchair stats revisionが進むと、同じchairを参照する過去userのentryが
  missになり、古いstats payloadを再insertできない

`scripts/test-status-notification-order.sh` は、created_atを意図的に逆転させてもapp / chairが
`MATCHING -> ENROUTE -> PICKUP -> CARRYING` の順に返すことを確認します。

さらに、CARRYINGの定常payloadを両cacheへ保存した後、coordinate APIでARRIVEDを追加し、
次のapp / chair通知が古いCARRYINGではなくARRIVEDを返す確認を追加しました。

```text
OK: app notification: MATCHING -> ENROUTE -> PICKUP -> CARRYING
OK: chair notification: MATCHING -> ENROUTE -> PICKUP -> CARRYING
OK: coordinate locking read: CARRYING -> ARRIVED
OK: app latest notification fallback: ARRIVED
OK: chair latest notification fallback: ARRIVED
```

このほか次が成功しています。

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
./scripts/test-chair-stats-transitions.sh
./scripts/smoke-test.sh
```

## なぜ別の案を先に採用しなかったか

### response全体のTTL cache

実装は簡単ですが、TTL中にchair割当やstatus追加を見落とします。今回の明示的invalidationと
revisionは、変更eventを即座にcache missへ戻します。TTLはwriter漏れの安全網にはなっても、
正当性の主な仕組みにはしません。

### 未送信statusなしなら `data: null`

同じMATCHING中にchair割当が追加されるため不正確です。clientがchair情報を得られず、
次の状態進行を壊す可能性があります。

### 未送信statusとpayloadを巨大な1 SQLへ統合

Benchmark 21ではstatus部分をCTEで1 SQLへ統合しましたが、対象SQL累積が変更前約32秒から
53.756秒へ増え、94,573点でした。SQL本数ではなく候補集合、sort、rows examinedを含む
累積costで不採用にしています。

### 全pollを100msへ変更

Benchmark 10では全pollを100msへ変えて14,611点で、当時の30ms 15,415点を上回りませんでした。
今回100msにするのは、未送信statusがなくpayload cacheを返せる定常区間だけです。

### SSE

protocolをSSEへ変えても、同じDB queryとpayload生成を同じ頻度で行えば負荷は減りません。
再接続時のreplay、cursor、proxy timeoutも新しい検証対象になります。まずJSON APIのまま
仕事量を減らす方が変更範囲を限定できます。

### 決済transaction分割

単発のevaluation latencyには有望ですが、pending paymentの永続状態とcrash recoveryが
必要です。今回の診断で累積寄与が最大だった通知を先に扱いました。次の独立施策として
TODOに残します。

## 残る制約と次の計測

### 複数process

cacheとrevisionはprocess内です。別processがDBを更新しても、このprocessのentryは
invalidateされません。現在のDocker構成はwebapp 1 processですが、水平分割する場合は
DBのpayload version、共有message bus、または短い世代確認が必要です。

### 同一recipientへの並行poll

revisionはwriterとのraceを防ぎますが、複数pollが同じ未送信statusを同時に読むclaimには
なっていません。benchmarkerの現行構成で正当性は通っています。並行pollを許す構成へ
広げる場合は、条件付きUPDATEまたはclaimを別に検証します。

### response配送前に進むcursor

`app_sent_at` / `chair_sent_at` はresponse配送のACKではなく、serverが返すstatusを選んだ
時点のcursorです。DB commit後からclient受信前に失敗すると、そのstatusを再送できません。
公式clientとの通常runでは順序テストと `pass=true` を確認していますが、切断時の
at-least-once保証ではありません。response lossの故障注入と、ACKを追加できない既存APIで
どこまで再送可能にするかはTODOへ残します。

### cache entryの回収

現在はinitializeまでentryを保持します。60秒runでは問題になりませんが、長期運用では
利用終了recipientのentry数とRSSを測り、上限または世代別回収を設計します。TTLで正当性を
保証するのではなく、memory回収だけへ使う方法が候補です。

### p95は30msを超える

変更後もapp p95 166ms、chair p95 181msです。次はcache missのphaseを、pool取得、
最新ride、未送信status、payload補助SQL、sent更新、commitへ分けます。

JSON long pollingを試す場合は、DB connectionを保持せず、version確認、waiter登録、
version再確認の順でlost wakeupを防ぎます。状態変更時だけ即時wakeし、timeout時に
定常payloadを返す案を、現行100ms pollingと3走比較します。

### 次のP0

診断runでは `POST /api/chair/coordinate` が78,879回、p95 165msでした。
`chair_current_locations` UPDATEは通常runで平均0.846ms、最大137.504msです。
通知cache後はcoordinateとnearbyが次の候補です。current rowのlock待ちを時系列で測り、
履歴完全性と3秒反映を維持できるcoalescingだけを単独比較します。
