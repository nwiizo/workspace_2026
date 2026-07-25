# Benchmark 36: chair通知のride選択を配送状態機械にする

[チューニング目次へ戻る](../TUNING.md)

## 結論

chair通知が最初に対象rideを選ぶ規則を、`rides.updated_at`の最大値から
「その椅子へどこまで通知を配送したか」に変更しました。

最初に試した「未送信statusを持つrideを常に優先する」規則は不十分でした。
あるrideの`MATCHING`をすでに椅子へ見せた後、次のstatusが追加されるまでの短い空白で、
別rideに残る未送信`MATCHING`へ切り替わったからです。診断runでは、現在のrideが
`ENROUTE`なのに別rideの`COMPLETED`を返す`CODE=12`を4件確認しました。

最終実装は次の3群をこの順で選びます。

1. `MATCHING`は送信済み、`COMPLETED`は未送信のride
2. `MATCHING`が未送信の新しい割当ride
3. それ以外の完了履歴

これは「未送信行を探すquery」ではなく、椅子から見たrideの配送状態機械です。
固定fixtureでは、現在のrideの`MATCHING`を送信した直後と、`ENROUTE`を送信した直後の
どちらでも別rideへ切り替わらないことを確認しました。

独立レビューでは、`COMPLETED`送信済みなのに古い`MATCHING`だけ未送信のrideを
優先度1から除外していない終端反例が見つかりました。修正前binaryへ追加回帰を当てると、
current rideの完了後にその古いrideへ切り替わる赤を再現しました。最終実装では
`completed_status.chair_sent_at IS NULL`も優先度1の条件にし、同じ回帰が緑になっています。

レビュー前候補の診断1走と通常3走はすべて`pass=true`でしたが、既知の正当性穴を含むため
最終実装の推定値には使いません。

| レビュー前候補 | score | 判定 | エラー |
|---|---:|---|---|
| 診断run | 113,046 | `pass=true` | `CODE=26` 153件、`CODE=27` 4件 |
| 通常run 1 | 91,603 | `pass=true` | `CODE=26` 130件、`CODE=27` 10件 |
| 通常run 2 | 94,301 | `pass=true` | `CODE=26` 136件 |
| 通常run 3 | 112,819 | `pass=true` | `CODE=26` 151件、`CODE=27` 49件 |

レビュー修正後の同条件3走は1走だけ合格し、2走はcriticalな`CODE=32`で
`pass=false`でした。

| 最終実装 | score | 判定 | エラー |
|---|---:|---|---|
| 通常run 1 | 86,532 | `pass=true` | `CODE=26` 144件、`CODE=27` 3件 |
| 通常run 2 | 43,980 | `pass=false` | `CODE=8` 24件、`CODE=26` 85件、`CODE=32` 1件 |
| 通常run 3 | 44,825 | `pass=false` | `CODE=26` 80件、`CODE=32` 1件 |

失敗runのscoreを通常構成の代表値へ混ぜないため、最終実装の推定代表値は出しません。
合格した実測`n=1`は86,532点・未推定です。`CODE=12`と`CODE=29`は最終3走でも
0件でしたが、全体として正当性gateを通過したとは扱いません。

固定回帰で反証できるride取り違えを防ぐ変更は残し、次はハード失敗を起こした
`CODE=32`を最優先で追います。その後に毎回80–144件出た`CODE=26`、
1走で24件出た`CODE=8`、最大3件の`CODE=27`を順に切り分けます。

ホストは4 CPU / 4 GiB / 100 GiB、SQLx pool上限は50のままで、CPU / memoryは
変更していません。

## はじめに知っておく用語

### 業務状態と配送状態

業務状態はrideそのものがどこまで進んだかを表します。

```text
MATCHING -> ENROUTE -> PICKUP -> CARRYING -> ARRIVED -> COMPLETED
```

配送状態は、その業務状態をappまたはchairへどこまで通知したかを表します。
`ride_statuses.chair_sent_at`が`NULL`なら未送信、時刻が入っていれば送信済みです。

同じ業務状態でも配送状態は異なり得ます。

```text
DBには ENROUTE まで存在
  MATCHING: chair_sent_atあり
  ENROUTE:  chair_sent_atなし
```

このときrideは業務上`ENROUTE`ですが、chairへ次に返すべきstatusも`ENROUTE`です。
ride選択を`updated_at`だけで行うと、この配送情報を見ないまま別rideを選べます。

### current ride

この記録でcurrent rideとは、椅子が現在処理しているrideです。単に
`updated_at`が最大のrideという意味ではありません。

椅子が`MATCHING`を受け取った時点で、そのrideは椅子のクライアント状態へ導入されます。
その後は`COMPLETED`を受け取るまで同じrideを追い続ける必要があります。次のstatusが
まだDBへ追加されていない瞬間でも、この関連付けは消えません。

### delivery gap

あるstatusを送信した後、次のstatusが作られるまでの空白をdelivery gapと呼びます。

```text
MATCHING送信済み
       |
       |  次の未送信statusが存在しない空白
       v
ENROUTE追加
```

「未送信statusがあるrideだけを優先する」と、この空白でcurrent rideの優先根拠を
失います。別rideに未送信行が残っていれば、椅子へ別userのpayloadを返します。
そのため、未送信行だけでなく「すでにこの椅子へ導入した未完了ride」を先に選びます。

### hidden pending status

同じ椅子の別rideに未送信statusがあるのに、最初のride選択で除外され、その後の
未送信status queryから見えなくなる状態です。

```text
chair
  ├─ ride A: updated_at最大、全status送信済み
  └─ ride B: updated_atは古い、MATCHING未送信
```

Benchmark 35の失敗後DBでは、この状態を25台の椅子で確認しました。以前の実装は
ride Aだけを選び、そのride内で`chair_sent_at IS NULL`を探していたため、ride Bは
何度pollしても配送されませんでした。

### fallback

未送信statusがないpollでも、クライアントは現在のrideと最新statusを必要とします。
ここでいうfallbackは「適当な過去ride」ではなく、最後に`MATCHING`を送った未完了rideを
維持して返す規則です。現在rideがない場合にだけ完了履歴へ落とします。

## 調査したログとDB

### Benchmark 35の失敗から立てた最初の仮説

Benchmark 35は52,564点、`pass=false`、`CODE=29` 142件でした。DBをchair単位で集計すると、
`updated_at`最大rideには未送信statusがなく、別rideに未送信statusがあるhidden pendingを
25件確認しました。

ここから最初は次の仮説を立てました。

> `chair_sent_at IS NULL`を持つrideを最優先すれば、`updated_at`に隠れた割当を配送できる。

固定fixtureで最初の`MATCHING`を返すところまでは成功しました。しかし、そのcursorを
進めた次のpollではcurrent rideに未送信行がなくなり、古い完了rideを返しました。
この時点で「未送信優先だけでは定常fallbackを表せない」と分かりました。

### 途中候補の診断run

そこで、未送信rideを優先し、送信済み時は`MATCHING.chair_sent_at`が新しいrideを
fallbackにする候補を試しました。この候補は21,767点、`pass=false`で、
`CODE=12` 4件、`CODE=26` 30件でした。

`CODE=12`の1例では、ベンチマーカーが現在ride
`01KYBAP67A1QMZH33C7P1GMDCG`の`ENROUTE`を期待している一方、応答は別rideの
`COMPLETED`でした。同じchairのDBを調べると次の順でした。

| ride | 状態 | chairへの配送 |
|---|---|---|
| 現在ride | `MATCHING`送信済み、`ENROUTE`未送信 | current ride |
| 別ride | `COMPLETED`送信済み、`MATCHING`未送信 | 異常な未送信履歴 |

「どこかに未送信行があれば最優先」という規則は、current rideの配送途中にも別rideへ
切り替わります。未送信の有無だけではなく、`MATCHING`から`COMPLETED`までの
配送ライフサイクルを表す必要があると判断しました。

## 最終実装

```sql
SELECT rides.*
FROM rides
LEFT JOIN ride_statuses AS matching_status
       ON matching_status.ride_id = rides.id
      AND matching_status.status = 'MATCHING'
LEFT JOIN ride_statuses AS completed_status
       ON completed_status.ride_id = rides.id
      AND completed_status.status = 'COMPLETED'
WHERE rides.chair_id = ?
ORDER BY CASE
    WHEN matching_status.chair_sent_at IS NOT NULL
     AND completed_status.chair_sent_at IS NULL THEN 0
    WHEN matching_status.id IS NOT NULL
     AND matching_status.chair_sent_at IS NULL
     AND completed_status.chair_sent_at IS NULL THEN 1
    ELSE 2
END,
matching_status.chair_sent_at DESC,
rides.updated_at DESC,
rides.created_at DESC,
rides.id DESC
LIMIT 1
```

### 優先度0: 導入済みで完了未配送

`MATCHING.chair_sent_at IS NOT NULL`は、そのrideを椅子へ一度見せたことを表します。
`COMPLETED.chair_sent_at IS NULL`は、まだ配送ライフサイクルを閉じていないことを表します。

`COMPLETED`行自体がまだない場合もLEFT JOIN結果は`NULL`なので、この群に入ります。
これによりdelivery gapの間もcurrent rideを維持します。

### 優先度1: 新しい割当

`MATCHING`行があり、その`chair_sent_at`が`NULL`で、かつ`COMPLETED`が配送済みでは
ないなら、まだ椅子へ導入していない割当です。current rideがないときにこの群を選び、
最初の`MATCHING`を配送します。

`COMPLETED`送信済みなのに古い`MATCHING`だけ未送信という異常履歴はこの群から除きます。
除かない場合、current rideの`COMPLETED`を配送した直後に古いrideへ切り替わります。

### 優先度2: 完了履歴

上のどちらでもないrideは、現在の配送対象ではありません。すべて完了済みの場合に
従来互換のpayloadを返すためのfallbackです。

### 同じ優先度内の順序

優先度0が複数ある異常状態では、`matching_status.chair_sent_at DESC`により最後に
椅子へ導入したrideを選びます。最後のfallbackで`rides.updated_at DESC`を使うのは、
配送状態が同じときだけです。そこまで同値なら`created_at DESC, id DESC`で全順序を
作ります。本番のIDは時系列順に並べられるULIDですが、作成時刻も明示して意味を保ちます。
`updated_at`を廃止したのではなく、意味を持つ範囲へ優先順位を下げました。

## INDEXと実行計画

### `rides(chair_id, created_at)`

```sql
INDEX idx_rides_chair_created_at (chair_id, created_at)
```

先頭列`chair_id`を等価条件で固定するため、MySQLは対象椅子のrideだけをrangeとして
読めます。今回の最終sortは`CASE`、`matching_status.chair_sent_at`、`updated_at`の
組み合わせなので、このINDEXだけでORDER BYを完成させることはできません。
それでも全ride表を走査せず、1 chairの小さい候補集合へ縮める役割があります。

### `ride_statuses(ride_id, status)`

```sql
INDEX idx_ride_statuses_ride_status (ride_id, status)
```

JOINはride IDと`MATCHING`または`COMPLETED`というstatusを両方等価条件で指定します。
複合INDEXの左から2列を固定できるため、それぞれのJOINは対象行へ直接lookupできます。

同じrideに同じstatusを複数作らないというアプリケーション不変条件があるので、
JOINで候補行数が掛け算に増えないことも重要です。将来この不変条件をDBで強制するなら
`UNIQUE (ride_id, status)`が候補ですが、既存データとの互換性とINSERT失敗時の扱いを
先に確認する必要があります。

### なぜCASE用INDEXを追加しなかったか

優先度は2表の配送時刻から計算する値であり、`rides`の単一INDEXへそのまま格納できません。
生成列やcurrent-state表へ優先度を物理化する方法はありますが、全status writerで
同時更新する必要があり、今回は正当性修正より変更範囲が大きくなります。

固定fixtureの`EXPLAIN ANALYZE`では、ride候補に
`idx_rides_chair_created_at`、2つのstatus JOINに
`idx_ride_statuses_ride_status`が使われました。失敗run由来の2 ride fixtureでは
0.182ms、7 rideの観測ではwarm時0.145msでした。これは局所的な1回の観測であり、
通常負荷の安定値や将来の保証値ではありません。

候補集合が1 chairあたり数十・数百へ増えてsortが支配的になったら、次を順に比較します。

1. `chair_current_ride`のような1 chair 1 rowのcurrent-state表
2. `rides`へ配送世代を持たせる
3. current ride IDをchair行へ持たせる

INDEXを増やすだけでは、複数表から導出する状態のsortは消せません。

## 回帰テスト

`scripts/test-chair-notification-pending-ride.sh`は次の3 rideを固定します。

- `updated_at`は新しいが全status送信済みの古いride
- `updated_at`は古いが`MATCHING`未送信のcurrent ride
- `COMPLETED`だけ送信済みで`MATCHING`が未送信という途中候補の反例

次をHTTP応答とDB cursorの両方で確認します。

1. 最初のpollはcurrent rideの`MATCHING`
2. `MATCHING`送信直後のfallbackも同じride
3. `ENROUTE`追加後のpollは同じrideの`ENROUTE`
4. `ENROUTE`送信直後のfallbackも同じride
5. current rideの`COMPLETED`送信後も、完了配送済みの古いrideに残る
   `MATCHING`へ切り替わらない
6. `matching_status.chair_sent_at`と`rides.updated_at`が同値でも、作成順でcurrent rideを選ぶ
7. `MATCHING`と`ENROUTE`で、返したstatusの`chair_sent_at`がcommit済み

旧`updated_at`規則では1が古い完了rideになり失敗しました。単純な未送信優先では
2で古い完了rideへ戻りました。最終実装ではすべて成功しています。

既存の状態順回帰`./scripts/test-status-notification-order.sh`、全体の
`./scripts/smoke-test.sh`も成功しました。

## レビュー前候補の診断run

終端反例の条件を加える前の候補は113,046点、`pass=true`でした。phaseの支配関係を
見る参考値として残しますが、最終実装のscoreや正当性を示す値には使いません。

| endpoint | 成功sample | cache hit | pending | steady | rideなし |
|---|---:|---:|---:|---:|---:|
| app通知 | 1,747 | 1,450 | 157 | 140 | — |
| chair通知 | 1,234 | 962 | 161 | 106 | 5 |

cache hit率はapp 82.9%、chair 77.9%でした。chairのpending pathは平均61.997ms・
p95 159.519ms、steady pathは平均54.456ms・p95 161.107msでした。
chairの新しいride選択queryは平均1.515ms、p95 5.533msで、request全体のpool待ちより
小さい値でした。

| endpoint | 初回acquire平均 / p95 | transaction acquire平均 / p95 | connection所有平均 / p95 |
|---|---:|---:|---:|
| app通知 | 25.107 / 77.790ms | 24.038 / 79.797ms | 10.692 / 25.058ms |
| chair通知 | 24.934 / 78.368ms | 23.685 / 75.754ms | 11.143 / 25.974ms |

同じrunのendpointログでは、coordinateが74,837件・平均54ms・p95 159ms、
app通知が111,793件・平均31ms・p95 168ms、chair通知が78,923件・平均43ms・
p95 186msでした。今回のJOIN sortだけを次の最大ボトルネックとは判断しません。

## 通常ベンチの読み方

最終実装は3走中2走が`pass=false`なので中央値を推定しません。レビュー前候補の
94,301点も、既知の終端反例を含むため採用スコアにはできません。

最終3走で`CODE=12`と`CODE=29`が0件だったことはride選択の改善を支持しますが、
`CODE=32`はベンチ全体を失格にするcritical errorです。スコアだけで終端反例のある
候補へ戻さず、同時に最終候補を「安定合格」とも記録しません。

- 固定回帰: 改善したため変更を保持
- 公式ベンチの正当性gate: 3走中2走失敗のため未通過
- throughput: 推定値を出せない
- 次の調査: `CODE=32`、`CODE=26`、`CODE=8`、`CODE=27`の順

## 他に考えられる選択肢

### 未送信statusがあるrideを常に優先

hidden pendingの最初の1件は救えますが、delivery gapでcurrent rideを失います。
途中候補の`CODE=12`で反証されたため不採用です。

### `updated_at DESC`を維持

queryは短いままですが、評価確定など別目的の更新で順序が変わります。
「最後に更新されたride」と「次に配送すべきride」は意味が異なるため不採用です。

### `matching_status.chair_sent_at DESC`だけで選ぶ

current rideのfallbackには有効ですが、まだ一度も`MATCHING`を送っていない新規割当は
時刻が`NULL`です。新しいrideを導入する条件を別に持つ必要があります。

### current-state表

`chair_id -> current_ride_id -> delivery phase`を1 rowで持てば、pollごとのJOINとsortを
O(1) lookupへできます。複数processでも共有できますが、matcher、全status writer、
initialize、故障回復を同じtransactionで更新する設計が必要です。
今回の正当性を維持した次の本命候補です。

### application ACK

`chair_sent_at`はserverが送信を決めた時点で進み、clientが受信した保証ではありません。
厳密なat-least-once配送には、次回pollで前回status IDをACKするprotocolが必要です。
公式clientとの互換性とDB write増加があるため、ride選択修正とは分離して比較します。

## 次に測ること

1. `CODE=32`のpending ride、地域、空きchair、matcher batch、UPDATE件数を
   同じtickで採取し、長時間MATCHINGの地点を特定する
2. `CODE=26`の同一chairについて、coordinate POST受信、DB commit、response完了、
   owner集計開始のwatermarkをrequest ID付きで記録する
3. owner responseが含めてよい最後のlocation IDを固定する赤・緑テストを作る
4. `CODE=8`が再発したらapp通知のride / user / cursorを同一requestで保存する
5. `CODE=27`のchairについて、DB current row、process cache revision、nearby responseを
   同じ時刻軸で採取する
6. critical errorが0件の通常3走へ戻ってから通知connection再利用を再比較する

今回の通常runはエラーmapが空ではないため、次の性能施策を重ねる前にこの順で
原因を切り分けます。
