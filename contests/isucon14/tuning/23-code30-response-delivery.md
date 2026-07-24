# Benchmark 23: 評価レスポンス配送境界と `CODE=30`

[チューニング目次へ戻る](../TUNING.md)

> 後続の[Benchmark 24](./24-owner-sales-completion-boundary.md)で、evaluationと
> `rides.updated_at` のwriteを外部決済成功後へ移しました。本書の「外部決済より前に
> updated_atが決まる」という説明はBenchmark 23計測時点の実装を示します。
> nearbyのresponse配送境界と1秒leaseは、完了writeの順序変更後も別の境界対策として
> 維持しています。

## 結論

認証SQLをcache化して処理量が増えた後、nearbyが「DBでは空きだが、ベンチマーカーでは
評価完了をまだ受信していない椅子」を返し、`CODE=30` が6–20件発生しました。

原因は、評価transactionのcommitだけでなく、次の2つの世界が切り替わる時刻の差です。

- サーバーの世界: `rides.evaluation` がcommitされ、椅子を空きと判定できる
- ベンチマーカーの世界: 評価HTTPレスポンスを最後まで受信し、`Evaluated=true` を設定する

評価中の椅子をresponse bodyのdropまで保持する既存RAII guardだけでは、Hyperへ小さな
JSON bodyを渡した時点と、clientが受信を終える時点の間を閉じられませんでした。
そこで、次を組み合わせました。

1. nearby開始時にtrackerのrevisionと除外対象をsnapshotする
2. SQL待機中に開始・終了した評価をcompletion revisionで検出する
3. response body drop後も、実測に基づく1秒のdelivery lease中は再掲載しない
4. guardとsnapshotへgenerationを持たせ、initialize前後の世代を分離する
5. live snapshotが不要になった期限切れcompletionを安全に回収する

最終コードの60秒ベンチ3走では、`CODE=30` はすべて0件でした。

| run | pass | score | error map | `CODE=30` | matching不満 | pickup不満 | drive不満 |
|---:|---|---:|---|---:|---:|---:|---:|
| 1 | true | 105,002 | 空 | 0 | 28.6% | 40.2% | 71.0% |
| 2 | true | 103,046 | 空 | 0 | 20.5% | 40.7% | 72.6% |
| 3 | true | 96,542 | 空 | 0 | 26.3% | 42.9% | 69.7% |

- 観測範囲: 96,542–105,002点
- 推定代表値: 中央値103,046点
- 直前の認証cache版中央値104,612点との差: -1,566点、約-1.5%
- 全run `pass=true`
- `CODE=30`: 3走合計0件

中央値の低下よりも、処理量増加後に繰り返し出ていた整合性エラーを除くことを優先して
採用します。generation/prune追加前の候補runでは `CODE=17` が1件ありましたが、最終3走では
再現しませんでした。ユーザー登録へのHTTP 500であり、今回変更したnearbyと評価trackerとは
別経路なので、再発時にrequest IDとMySQL errorを取る診断対象として残します。

## はじめに知っておく用語

### サーバーの状態とベンチマーカーの状態

分散したプログラムでは、同じ出来事を同時に観測するとは限りません。評価処理では、
サーバーはDB commitを見て完了を知り、ベンチマーカーはHTTPレスポンスを受けて完了を
知ります。ネットワークとtask schedulingがあるため、両者の間には短い差があります。

ここで重要なのは「DBが正しいか」だけではなく、「API利用者が成功を観測するより先に、
その成功を前提とする別の情報を公開してよいか」です。

### response body lifecycle

Axum handlerが `Response` を返しても、その時点でclientがbodyを受信したとは限りません。
概略は次の順です。

```text
handlerがResponseを返す
  -> Hyperがbodyをpollする
  -> socketへ書き出す
  -> clientが受信する
  -> clientがJSONをdecodeする
  -> client側の状態を更新する
```

`ActiveRideEvaluationBody` が観測できるのは、Hyperがbodyを消費またはdropするところまで
です。client側のdecode完了は観測できません。

### ACK

ACKは「相手が受け取った」と送り手が確認できる応答です。TCPにも確認応答はありますが、
アプリケーションがJSONをdecodeし、`Evaluated=true` を設定したことを意味しません。
このAPIには評価レスポンスを処理し終えたことをclientから返すapplication ACKがないため、
サーバーだけでclientの状態変更時刻を厳密に知ることはできません。

### grace / lease

graceは、境界直後の短い揺らぎを吸収する猶予時間です。leaseは、期限まで対象を利用不可
として扱う権利・予約です。今回の1秒は「評価処理開始から1秒」ではなく、
response body guardがdropした時点から始まります。

基準点が重要です。外部決済より前の `rides.updated_at` から1秒を測ると、決済に1秒以上
かかった時点でcommit前に期限切れになります。body dropを基準にすれば、少なくとも
サーバー内の評価処理時間には依存しません。

### revision

revisionは、評価完了イベントごとに増える単調な番号です。wall-clock時刻とは異なり、
同時刻や時計補正を考えずに「nearby開始後に完了したか」を比較できます。

```text
nearby開始時 revision = 40
SQL待機中に評価完了 revision = 41
SQL後に 41 > 40 を確認できる
```

### snapshot

snapshotは、ある時点のrevisionと除外対象の組です。nearbyのSQL前後でactive IDを1回だけ
読むと、評価がSQL待機中に始まって終わった場合を見落とします。開始時snapshotと終了時の
revisionを比較すれば、その「両方の観測点の間で完結したイベント」も検出できます。

### happens-beforeとTOCTOU

happens-beforeは、ある処理の結果が別の処理から確実に見える順序関係です。DB commitは
DB内の順序を作りますが、clientの `Evaluated=true` より前であることまで保証しません。

TOCTOU（time of check to time of use）は、確認した時点と利用する時点の間で状態が変わる
問題です。今回のnearbyでは、tracker確認とSQL、レスポンス構築の間に評価が変わり得ます。
snapshotとrevisionは、この区間に重なった評価を保守的に除外します。

### `Instant` とwall-clock

Rustの `Instant` は経過時間を測る単調時計です。UTC日時を表す型ではなく、NTP補正や
手動の時計変更で逆戻りしません。1秒のlease期限には「何時だったか」より「どれだけ
経過したか」が必要なので、`SystemTime` ではなく `Instant` を使います。

## `CODE=30` の検査内容

ベンチマーカーのnearby検査は、nearbyリクエスト直前の時刻を基準に、返された椅子の
直前rideを調べます。直前のマッチ受諾が十分古く、ベンチマーカー内部でそのrideがまだ
`Evaluated=false` なら、返してはいけないbusy chairとして `CODE=30` になります。

これは座標の最大3秒遅延とは別です。座標は多少古くても許されますが、「空きかどうか」
は即時の整合性が求められます。nearbyレスポンス全体を3秒cacheする方法を採れない理由も
ここにあります。

## 調査で確認したログ

### 1. WARN本文と対象ride

まずbenchmarkのWARNから、エラー番号だけでなくchair IDと判定理由を保存しました。

```text
取得した付近の椅子情報に不備があります (CODE=30):
ID:...の椅子は既にライド中です
```

そのchairについて次を突き合わせました。

- `rides.evaluation`
- 最新status
- `rides.updated_at`
- 評価APIの開始・commit・response body drop
- nearbyのリクエスト開始
- benchmarkerが評価レスポンスを受けた時刻
- benchmarker内部の `Evaluated` 更新段階

### 2. 一時的な診断instrumentation

原因を推測だけで決めないため、benchmarkerの診断用作業ツリーに評価処理のphaseを
一時追加しました。

| phase | 意味 |
|---:|---|
| 1 | `SendEvaluation` を呼び、HTTPレスポンスを待っている |
| 2 | HTTPレスポンスの受信・decodeが終わった |
| 3 | benchmarker内部の `Evaluated=true` 更新が終わった |

診断後はこの変更をすべて戻し、最終3走は無変更の公式benchmarkerで行いました。
`git diff -- bench/benchmarker` が空であることも確認しています。

### 3. 仮説ごとの結果

| 条件 | score | `CODE=30` | 観測 |
|---|---:|---:|---|
| response body guardのみ | 97,809 | 27 | 27件すべてphase 1。clientはまだresponse待ち |
| SQL前後のactive snapshot | 112,242 | 1 | 大幅に減ったが、残りもphase 1 |
| revision overlap試作 | 105,348 | 4 | SQL中の開始・完了は拾えるが配送境界が残る |
| 詳細時刻付きrevision試作 | 94,454 | 7 | body drop後もclient受信まで差がある |
| revision + body drop基準1秒lease | 最終中央値103,046 | 0 | generation/pruneを含む公式benchmarker 3走で再現なし |

診断runのscoreはinstrumentationと走行揺らぎを含むため、性能比較の代表値には使いません。
何件がどのphaseにいたかという因果の切り分けにだけ使います。

### 4. body dropからclient受信までの実測差

同一ride IDでサーバーログと診断benchmarkerログを対応付けると、代表例は次でした。

| 例 | body guard drop | client response完了 | 差 |
|---:|---|---|---:|
| 1 | 18:34:54.666 | 18:34:54.801 | 約135ms |
| 2 | 18:34:57.588 | 18:34:57.744 | 約156ms |
| 3 | 18:35:01.665 | 18:35:01.719 | 約55ms |
| 4 | 18:35:03.270 | 18:35:03.402 | 約132ms |
| 5 | 18:35:06.049 | 18:35:06.725 | 約677ms |

確認できた最大値は約677msでした。小さなJSON bodyでも、socketへの書出し、runtimeの
scheduling、client側の受信処理により、body wrapperのdropとclient観測は同時では
ありません。このログにより、「body guardがあるからclient受信まで保持される」という
仮説を棄却しました。

## なぜ以前のcooldownでは不十分だったか

以前比較した500ms / 1秒cooldownは、`rides.updated_at` を起点にしていました。

```text
UPDATE rides SET evaluation = ...  <- updated_atが決まる
chair stats更新
外部決済HTTP / retry sleep
COMMIT
response body送信
client受信
```

外部決済が長いほど、椅子がDB上で空きになる前にcooldown時間を消費します。つまり、
短い決済では不要に椅子を隠し、長い決済では必要なときに期限が切れる設計です。

今回も1秒という時間は使いますが、起点が異なります。

```text
評価開始             -> active guardで除外
DB commit             -> active guardで除外
body poll / drop      -> ここから1秒lease
client受信・状態更新  -> 実測では最大約677ms
```

これは評価transactionの処理時間ではなく、観測したresponse配送差へ対応します。

## 実装

### trackerの状態

`ActiveRideEvaluationTracker` は次を1つの短い同期mutexで保護します。

```text
active_counts:
  chair ID -> 同時guard数

completed_evaluations:
  chair ID -> { completion revision, unavailable_until }

revision:
  最後に完了した評価イベント番号

generation:
  initializeで増えるDB世代番号

live_snapshot_revisions:
  実行中nearbyの開始revisionと参照数
```

同じchairにguardが重なっても、最初のdropでactive状態を消さないよう参照数を使います。
最後のguardがdropしたときだけrevisionを増やし、1秒leaseを記録します。

### nearbyの判定

```text
1. SQL前に { revision, active IDs, lease中IDs } をsnapshot
2. DBでactive chairと未評価rideのantijoinを実行
3. SQL後に次を除外集合へ加える
   - 開始snapshotに入っていたchair
   - 現在activeなchair
   - 開始revisionより後に評価完了したchair
   - 現在もdelivery lease中のchair
4. 最新座標を合成し、距離内のchairだけ返す
```

開始時にlease中だったchairは、SQL中に期限が切れてもそのnearbyレスポンスでは除外します。
リクエスト開始時の判断がresponse構築中に変わるTOCTOUを避けるためです。

### initialize

`POST /api/initialize` はDBを別世代へ全置換します。前世代のchair IDとleaseを残すと、
新しいDBの椅子を理由なく隠す可能性があります。maintenance write lockを取得し、DBの
破壊的初期化より前に認証cacheと評価trackerをclearします。

ただしmaintenance lockが待つのはhandlerが `Response` を作るまでで、旧response bodyの
dropまでは待ちません。次の順序を世代情報なしで許すと、旧guardが新しいcountを減らします。

```text
旧世代guardをbodyが保持
  -> initializeでtrackerをclear
  -> 同じchair IDの新世代guardをbegin
  -> 旧bodyをdrop
  -> 旧guardが新世代countを誤って減らす
```

guardとsnapshotは作成時のgenerationを保持し、drop時のcurrent generationと一致する場合
だけstateを変更します。initializeはgenerationを増やしてmapを空にするため、遅れてdropした
旧世代の値は新世代に触れません。この反例は独立レビューで見つかり、交差順序をunit testへ
追加しました。

### 期限切れ記録の回収とmemory

完了記録はイベントごとに追加せず、chair IDごとに最新1件を上書きします。そのため、
評価回数には比例しません。ただし期限切れ記録をinitializeまで残すだけでも、長時間稼働で
動的chair数に比例したscanがnearbyごとに発生します。

そこで実行中nearbyの開始revisionを参照数付き `BTreeMap` で追跡します。completionは、
次の両方を満たすときだけ回収できます。

1. delivery leaseが期限切れ
2. 最古のlive snapshotより前に完了済み、またはlive snapshotがない

開始時にlease中だったchair IDはsnapshot自身が所有しています。開始後に完了した記録は
`completion revision > snapshot revision` の間だけ残します。snapshotは通常の合成完了時
だけでなくfuture cancellation時も `Drop` で参照数を減らすため、途中returnで永続的に
pruneを止めません。次のnearby開始時に安全な期限切れ記録を回収し、mutex内のscan対象を
直近のleaseと実行中requestに必要な範囲へ戻します。

## なぜ1秒にしたか

診断で確認した最大配送差は約677msでした。1秒はそこへ約323msの余裕を加えた値です。
500msでは実測最大値を覆えません。2秒以上は正しさの余裕を増やしますが、評価済みの
椅子を空きとして再利用するまでを長くし、matchingとscoreを下げる可能性があります。

ただし1秒は数学的保証ではなく、この固定4 CPU / 4 GiB Docker環境で得た実測値です。
ホスト負荷、ネットワーク構成、複数process化が変われば再計測が必要です。

## 他に考えられる選択肢

### application ACK

clientが評価レスポンスを処理した後にACK endpointを呼び、そのACKまで椅子を隠す方法が
最も境界を明確にできます。しかし公式APIとbenchmarkerを変更できないため採用できません。

### DBまたはRedisの共有lease

複数webapp processで同じavailabilityを即時共有できます。owner、期限、世代を保存し、
process crash時は期限で回収できます。一方でnearbyごとの共有storage readとlease writeが
増えるため、単一processの現構成ではprocess内trackerを先に採用しました。

### response送信完了hookだけを使う

body wrapperはhandler scopeより正確ですが、今回の計測ではclient受信まで最大約677msの
差が残りました。単独では十分ではありません。

### nearbyを常に保守的に遅延させる

すべての評価済みchairを2–3秒隠せば再現率は下がりますが、空き椅子数とmatching機会を
不要に減らします。対象chairだけを、観測した境界に限定する方が影響範囲を狭くできます。

### `rides.updated_at` を使う

時刻が外部決済より前に決まるため不採用です。また `updated_at` は利用者履歴の完了時刻と
owner売上の期間判定にも使われるため、この問題のためだけに後から更新できません。

## 検証

### Rust

```sh
cd webapp/rust
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

unit testでは次を固定しました。

- guardの参照数と最後のdrop
- nearby開始前・開始中・開始後の評価overlap
- bodyを正常に最後まで消費したときのdrop
- client切断相当でbodyをdropしたときのcleanup
- leaseが有効な間の除外
- SQL中にlease期限が切れても開始snapshotが残ること
- initialize相当のclearで前世代を消せること
- clear後に旧guardがdropしても新世代のactive countを壊さないこと
- 期限切れcompletionをlive snapshotのrevision条件に従って回収すること
- 新しいsnapshotがpruneしても、古いlive snapshotに必要なcompletionを残すこと

最終結果は14 test成功、Clippy error 0件です。依存crate由来の将来互換warningは
今回の変更対象外で、`-D warnings` の対象となる自crate warningはありません。

### 公式ベンチ

```sh
./scripts/benchmark.sh 60
```

最終3走は無変更の公式benchmarkerで実施しました。ColimaのCPU / memoryは全期間を通じて
4 CPU / 4 GiBのままです。

## 判断と次のTODO

今回の仮説は「DB commitとclient状態更新の間をbody lifecycleだけでは閉じられない」
でした。phase診断と同一rideの時刻相関で仮説を支持し、revision + delivery lease後の
公式3走で `CODE=30` 0件を確認しました。

一方、次は未解決です。

1. `CODE=17` が再現したrunで、登録request ID、MySQL error、deadlock履歴を同時採取する
2. 以前1件だけ出たowner salesの `CODE=24` が、同じ「commit後・client観測前」の
   境界問題かをphase付きで確認する
3. 複数process化する前に、共有leaseとprocess crash回収を設計する
4. 環境負荷が変わった場合、body drop→client受信差のp95 / p99 / 最大値を再計測する

`CODE=30` が再発した場合はleaseを推測で延ばさず、対象ride IDでbody drop、socket側、
benchmarker受信完了の時刻を再び相関させます。
