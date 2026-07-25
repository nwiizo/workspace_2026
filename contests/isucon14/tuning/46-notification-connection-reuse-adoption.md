# Benchmark 46: 通知connection再利用の採用判定

[チューニング目次へ戻る](../TUNING.md)

## 結論

[Benchmark 45](./45-notification-connection-reuse-diagnostics.md)で、rideあり通知の2回目の
pool取得を878 / 878 sampleで削除し、過去の不採用理由だった`CODE=29`が再発しないことを
確認しました。通常60秒3走のscore中央値は139,198点で、直前のDB pool分離版
138,027点から1,171点、約0.85%上がりました。

| 構成 | scores | 中央値 | 範囲 |
|---|---|---:|---:|
| 変更前: general 26 / coordinate 24 | 138,027 / 142,851 / 133,797 | 138,027 | 133,797–142,851 |
| 変更後: 通知connection再利用 | 134,732 / 150,117 / 139,198 | 139,198 | 134,732–150,117 |

差はrun間のばらつきより小さいため、scoreが0.85%確実に改善するとは断定しません。
一方で、同じ通知requestが飽和したgeneral poolへ2回並ぶ構造を確実に除去でき、
SQL・配送cursor・transaction境界・総接続数を変えていません。中央値も悪化していないため、
general poolの資源効率を改善する施策として採用します。

通常run 2 / 3では既知のowner距離不整合`CODE=26`が77 / 64件再発しました。
通知差分をjjへ退避した完全な`main`対照でも94件再現したため、通知再利用が作ったerrorとは
判断しません。ただし正当性問題が解消したわけではありません。次のP0として、
owner距離の更新時刻省略数、対象履歴行数、query時間を診断します。

## 条件

| 項目 | 値 |
|---|---|
| ホスト | Apple Silicon |
| Colima | 4 CPU / 4 GiB / 100 GiB |
| DB総接続上限 | 50 |
| general pool | 26 |
| coordinate pool | 24 |
| 各run | DB volumeを初期化して60秒 |
| 診断 | 通常3走は無効 |

CPU、memory、disk、MySQL設定、matcher、通知poll間隔は変更していません。

## 通常3走

| run | score | pass | error map | 最終評価数 | matching不満 | pickup不満 | pickup + drive合算不満 |
|---|---:|---|---|---:|---:|---:|---:|
| 1 | 134,732 | true | 空 | 2,068 | 52.6% | 28.6% | 65.8% |
| 2 | 150,117 | true | `CODE=26: 77` | 2,289 | 59.9% | 26.3% | 64.2% |
| 3 | 139,198 | true | `CODE=26: 64` | 2,128 | 60.7% | 29.7% | 64.7% |

「最終評価数」は終了直前の`eval reqs`です。run終了後の最終集計までに完了したrequestと
完全に同義ではありませんが、同じbenchmark logの同じ位置から取得しているため、
処理進行の比較指標になります。

run 2が150,117点まで伸びる一方でrun 1は134,732点です。1走だけを採用根拠にすると、
乱数seed、container scheduling、DB競合のばらつきを施策効果と誤認します。
そのため事前に決めた3走の中央値139,198点を推定代表値に使います。

## `CODE=26`の対照実験

### なぜ通知errorと分けるのか

`CODE=26`はowner椅子一覧の`total_distance`と
`total_distance_updated_at`の組み合わせを検証するwarningです。通知payloadのride ID、
user ID、status順を検証する`CODE=12/29`とは経路が異なります。

ただし「別機能だから無関係」と推測だけで除外はできません。通知がconnectionを長く
所有すると、owner queryの実行時刻をずらし、既存競合を増減させる可能性があるためです。

そこで通知差分をjjの実験commitへ退避し、`main` bookmarkの親から作った空のworking copyで
追加の60秒対照を実行しました。

| 構成 | score | pass | error map | 最終評価数 | matching / pickup / pickup + drive合算不満 |
|---|---:|---|---|---:|---|
| 通知再利用なしの完全な`main` | 135,807 | true | `CODE=26: 94` | 2,026 | 44.1% / 31.4% / 68.6% |

対照でも同じerrorがより多く発生しました。これで分かるのは、通知再利用がなくても
現在のowner距離公開境界で`CODE=26`が発生することです。

一方、1回の対照だけで「通知再利用は発生頻度へ全く影響しない」とまでは証明できません。
直前Benchmark 44の実効設定4走はerror map空だったのに対し、今回は候補2 / 3走と
追加対照1 / 1走で再発しています。workloadの進み方によって露出する非決定的競合として、
owner側を直接計測します。

## 比較値の扱い

### 事前に決めた比較

主比較はBenchmark 44の通常3走中央値138,027点と、今回の通常3走中央値139,198点です。

```text
(139,198 - 138,027) / 138,027 × 100
  = 約0.85%
```

### 追加確認を含む記述値

Benchmark 44には既定値確認132,756点、今回は因果分離用の対照135,807点があります。
変更前の実効設定5走を記述的に並べると次のとおりです。

```text
132,756 / 133,797 / 135,807 / 138,027 / 142,851
中央値 = 135,807
```

今回の139,198点との差は3,391点、約2.50%です。ただし後から追加した確認走を混ぜた値なので、
主たる改善率には使いません。比較群を後から選び直して都合のよい中央値を作らないためです。

## なぜ採用するのか

採用理由はscore差だけではありません。

| 観点 | 証拠 |
|---|---|
| 無駄な待ち | rideあり878 / 878 sampleで2回目の取得0 |
| DB仕事量 | SQL本数、query、cursor UPDATEは不変 |
| transaction | 存在確認は従来どおりautocommit、payload処理だけtransaction |
| 通知正当性 | 固定回帰成功、診断・通常3走で`CODE=12/29`なし |
| 接続予算 | 総50、general 26、coordinate 24を維持 |
| 得点 | 3走中央値は+0.85%、明確な悪化なし |
| 既知error | 差分なし対照でも`CODE=26: 94`を再現 |

run間分散が大きいため、得点改善の統計的な確度は高くありません。しかし、飽和時に同じ
requestを2回queueへ並べる構造は常に削減されます。今後general poolへadmission controlを
入れる場合も、permitまたはpool queueへ重複して並ぶ回数を先に減らしておく方が、
制御対象を単純にできます。

## なぜconnectionを長く持っても採用できるのか

変更前は存在確認connectionを平均約0.7–0.8ms所有して一度返し、数十ms待って別connectionを
借り、残りのtransactionを処理していました。変更後の連続所有は平均app 9.875ms、
chair 10.906msです。

```text
変更前のrequest視点
  所有A -> 待ちB -> 所有B

変更後
  所有A+B
```

pool全体から見ると、変更後は待っている間に別requestへconnectionを貸す機会が減ります。
これが公平性のtrade-offです。一方、変更前は返したrequestがもう一度queueへ入り、
通知request数に対してpool取得試行を増やします。

今回の採用は「connectionを長く持つ方が常に速い」という一般則ではありません。

- transaction内で外部HTTPを待つ
- retry sleepを含む
- 大量のCPU処理やserializeを含む

このようなhandlerではconnectionを分割して返す方がよい場合があります。評価APIで決済HTTPを
transaction外へ出したBenchmark 32は、その代表例です。通知は連続所有が約10msで、
再取得待ちが数十msだったため判断が逆になります。

## 実装上の注意

### rideなしでは必ず返す

通知pollingはrideがない利用者・椅子からも高頻度で届きます。rideなしまでtransactionを
開くと空polling改善を失うため、存在確認直後に明示的にdropします。

### cache hitはDBへ行かない

revision付きpayload cacheがhitした場合は、この変更前後ともpoolを取得しません。
connection再利用はcache missかつrideありだけの最適化です。

### 診断の0と未到達を分ける

再利用時は`transaction_pool_acquire_us = Some(0)`、rideなしやcache hitは`None`です。
reportは未到達sampleを除外し、0msで再利用できた件数を表示します。

### cancellation

requestが途中でcancelされると`Transaction`と`PoolConnection`がdropされ、未commitの
transactionはrollbackされます。診断guardも`Drop`でterminal phaseと所有時間を出します。
診断値を保持する構造がconnectionそのものを所有しないため、計測追加で返却を遅らせません。

## 他に考えられる選択肢

### appだけ、chairだけ採用する

片方ずつなら影響をさらに分離できます。しかし両方で同じ二重取得があり、診断では
app 436件・chair 442件とも100%除去でき、通知errorもありませんでした。片方を残す根拠が
ないため同時に採用します。

### 公平なqueueへ置き換える

SQLx pool内部の公平性へ依存せず、endpoint別Semaphoreやpriority queueを置く案です。
priority inversion、permit cancellation、二重queueを設計する必要があります。
まず重複取得を消し、その後shared pool + general permitを独立比較します。

### connectionではなくpayloadをさらにcacheする

現在も状態不変payloadは100ms cacheしています。未送信statusがある間はcursorを進めるため
DB transactionが必要です。cache時間を一律に延ばすとstatus発見が遅れ、closed-loopの
次行動とscoreを落とすため、Benchmark 10 / 26の結果どおり分けています。

## 検証log

| log | 内容 |
|---|---|
| `/tmp/isucon14-b46-run1.log` | 候補通常run 1 |
| `/tmp/isucon14-b46-run2.log` | 候補通常run 2 |
| `/tmp/isucon14-b46-run3.log` | 候補通常run 3 |
| `/tmp/isucon14-b46-control.log` | 通知差分なしの追加対照 |

各runは最終行の`pass`、score、error mapと、終了直前の`eval reqs`、最終不満率を確認しました。
途中の一時的な不満率や高い単発scoreだけを代表値にはしていません。

## 次のTODO

1. `CODE=26`再発時の更新時刻省略数、対象座標行数、owner query時間を診断sampleへ追加する
2. 同じchair IDでowner request開始境界、採用watermark、最新座標を相関する
3. 正当性を戻した後、shared pool 50 + general permitをstatic 26 / 24と比較する
4. 初回通知pool取得のp95約234–244msを、permit待ち・pool待ち・MySQL実行へ分ける
