# Benchmark 39: pickup予測tickを最小化するmatcherを比較する

[チューニング目次へ戻る](../TUNING.md)

## 結果

空き椅子を単純な距離ではなく、modelのspeedを含む
`ceil(distance / speed)` で選ぶpolicyを比較しました。

通常60秒を3回実行し、すべて `pass=true`、error map空でした。しかし中央値は
Benchmark 38の134,428点から133,257点へ1,171点、約0.9%低下しました。
局所的なpickup予測時間は正しい目的ですが、現在のgreedy matcher全体の得点改善は
確認できなかったため、実装は戻しました。

| 条件 | run 1 | run 2 | run 3 | 観測範囲 | 中央値 |
|---|---:|---:|---:|---:|---:|
| Benchmark 38の距離優先 | 132,225 | 134,428 | 137,075 | 132,225–137,075 | 134,428 |
| pickup予測tick優先 | 134,611 | 126,948 | 133,257 | 126,948–134,611 | 133,257 |

両条件は別の乱数系列です。約0.9%を厳密な因果効果とは扱いませんが、少なくとも
「speedを加えれば明確に伸びる」という採用根拠はありません。最高得点を目指す現在の
mainには、より単純で中央値も高い距離優先を残します。

## 仮説

現行matcherは同じ地域の候補からマンハッタン距離が最小の椅子を選びます。
しかし距離だけでは、乗車地点へ着くまでの時間を表せません。

```text
候補A: 距離30 / speed 2 -> ceil(30 / 2) = 15 tick
候補B: 距離50 / speed 7 -> ceil(50 / 7) = 8 tick
```

距離優先はAを選びますが、理想的にはBの方が7 tick早く到着します。ベンチマーカーの
1 tickは約30msなので、差は約210msです。椅子が早くpickupへ着けば乗車中の距離を
早く稼ぎ、ride完了後に次の割当へ戻る時刻も早くなる、と仮定しました。

公式評価との関係は次のとおりです。

| 評価 | 条件 |
|---|---|
| matching | requestから割当まで100 tick未満 |
| dispatch | 割当時の距離が `10 × speed` 未満 |
| pickup | 実pickup時間と理想時間の差が15 tick未満 |
| drive | 実走行時間と理想時間の差が5 tick未満 |

`ceil(distance / speed)` は理想pickup時間へ直接対応します。ただし、通知や座標APIの
遅延を引いた「余分な時間」、matching待ち、乗車中の処理遅延は含みません。

## 実験した実装

候補queryで `chair_models` をJOINし、`speed` を取得しました。

```sql
SELECT chairs.id,
       chair_current_locations.latitude,
       chair_current_locations.longitude,
       chair_models.speed
FROM chairs
INNER JOIN chair_current_locations
        ON chair_current_locations.chair_id = chairs.id
INNER JOIN chair_models
        ON chair_models.name = chairs.model
```

1 rideに対する椅子の比較keyは次の辞書順です。

```text
1. ceil(distance / speed)
2. distance
3. chair ID
```

最古rideから処理する順序、地域ごとの最大64候補、距離200以下、全体最大64割当、
`FOR UPDATE SKIP LOCKED`、500ms pollingは変えていません。仮説を混ぜないため、
CPU、memory、SQLx pool、MySQL設定も変更していません。

0以下のspeedを除外し、整数除算の端数を切り上げる境界を純粋関数のテストへ
固定しました。距離8・speed 7は1ではなく2 tickです。整数の切り捨てを使うと
「まだ到着していない端数」を完了扱いするためです。

## 通常3走で見た不満率

| 条件 | run | matching | pickupまで | 実移動 | score |
|---|---:|---:|---:|---:|---:|
| 距離優先 | 1 | 36.3% | 31.6% | 69.7% | 132,225 |
| 距離優先 | 2 | 36.6% | 28.0% | 70.7% | 134,428 |
| 距離優先 | 3 | 36.9% | 28.9% | 71.0% | 137,075 |
| pickup tick優先 | 1 | 35.7% | 28.8% | 71.7% | 134,611 |
| pickup tick優先 | 2 | 43.1% | 28.5% | 70.3% | 126,948 |
| pickup tick優先 | 3 | 31.7% | 33.0% | 71.8% | 133,257 |

中央値同士ではmatching不満が36.6%から35.7%へ0.9ポイント、pickup不満が
28.9%から28.8%へ0.1ポイント改善しました。一方、実移動不満は70.7%から
71.7%へ1.0ポイント悪化しました。どれもrun間変動と同程度で、pickupだけを
明確に改善したとは判断できません。

## 診断runで確認したログ

診断runは154,568点、`pass=true`、error map空でした。instrumentation付きの1走なので
通常scoreの推定には使いません。initialize後から終了までのmatcher 104回は、すべて
`outcome=success / terminal_phase=complete` でした。

| 指標 | 観測値 |
|---|---:|
| matcher呼出し | 104 |
| 割当件数 | 2,772 |
| 平均pickup距離 | 31 |
| 最大pickup距離 | 156 |
| 距離200超 | 0 |
| 平均予測pickup tick | 9 |
| 最大予測pickup tick | 72 |
| dispatch距離条件を満たさない割当 | 820 |
| 最古pending待ち | 5,491ms |
| 30秒以上待ったsample | 0 |
| UPDATE競合 | 0 |

dispatch距離条件を満たさない割合は820 / 2,772、約29.6%です。予測tickを最小化しても
約3割残ったため、「選び方だけが悪く、近い高速椅子は十分に余っている」という仮説とは
一致しません。古いrideへ割り当てられる空き椅子自体が不足する時間帯では、
比較keyを変えても基準内の候補を作れません。

phaseは次のとおりでした。

| phase | 平均 | p95 |
|---|---:|---:|
| pool begin | 37,943µs | 130,603µs |
| pending query | 4,564µs | 13,518µs |
| available query | 39,248µs | 99,391µs |
| matching + UPDATE | 11,091µs | 31,333µs |
| 全体 | 90,865µs | 218,472µs |

104回中56回はmatcher開始時にpool size 50 / idle 0でした。pickup costのRust比較を
軽くするだけでは、pool待ちやavailable queryを解消できません。

Benchmark 37診断のavailable query平均38,409µs、全体平均87,235µsに対し、今回の
点観測は39,248µsと90,865µsでした。乱数とDB状態が異なるため、JOINがそれぞれ
2.2%と4.2%を悪化させたとは断定しません。ただし追加JOINが無料という証拠もなく、
通常3走の得点低下と合わせて採用を支持しません。

## なぜ局所的に正しい仮説が全体scoreへ出なかったか

### 空き椅子の供給が足りない

pending数がavailable数を上回ったsampleは104回中73回でした。比較できる椅子が少ない
状態では、距離優先と予測tick優先が同じ椅子を選ぶか、どちらも遠い候補しか持ちません。

### greedyな1 ride最適は将来の割当を見ない

高速椅子を現在のrideへ使うと、その椅子は次のrideでは使えません。現在のpickupを
数tick短縮しても、残った低速椅子が次の長距離pickupへ割り当てられればbatch全体では
遅くなる可能性があります。1 rideごとの最小値と、64 ride全体の最小値は別です。

### driveの遅延原因はmatcherだけではない

drive評価はpickup後の座標更新、status通知、評価APIまでを含みます。候補のspeedを
選び直しても、pool待ち、通知polling、coordinate処理が生む理想時間との差は減りません。
最終runの実移動不満が約70%あるため、次はこの区間のtick遅延を直接測る必要があります。

## 他に考えられる選択肢

### batch全体で最小費用matchingを行う

地域内のrideとchairを二部グラフにし、最大割当件数を先に保証した上で、
予測pickup tickの合計を最小化します。greedyの将来機会損失を減らせます。
ただし古いrideを捨てるとmatching不満が増えるため、未割当penaltyと待ち時間を
目的関数へ明示する必要があります。

### dispatch合格候補を優先する

`distance < 10 × speed` を満たす候補を先に選べます。ただし今回のtick最小化でも
基準外が約29.6%残り、供給不足時には選択順だけで解消しません。割当を次batchへ
保留する場合は、matching待ちとdispatch評価のtrade-offを3走で比較します。

### speedを永続化したcurrent-stateへ含める

modelはchair登録後に変わらないため、最新位置cacheやmatcher候補へspeedを一緒に
保持すれば、hot queryのJOINを避けられます。一方、今回の主問題はJOINだけではなく
局所目的関数なので、denormalizeだけを先に実装しません。

### drive区間をphase計測する

chairの `PICKUP -> CARRYING -> ARRIVED` について、理想tick、実tick、座標POST、
chair/app通知、pool取得を同じride IDで相関します。約70%の実移動不満を直接分解でき、
次の実装対象をmatcher以外も含めて選べます。

## 採否

実装は不採用です。比較用に追加した `chair_models` JOIN、speed field、予測tick選択、
診断fieldもmainから外し、Benchmark 38の距離優先へ戻しました。

残すものは次です。

- 3走のscoreと不満率
- 診断runの予測tick、dispatch条件、phase、供給不足
- 局所greedyとbatch全体最適化の違い
- drive区間を同一rideでphase計測するTODO

## 検証コマンド

```sh
cd webapp/rust
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets

cd ../..
shellcheck scripts/report-matcher-phases.sh
sh -n scripts/report-matcher-phases.sh
./scripts/benchmark.sh 60
```

Colimaは4 CPU / 4 GiB / 100 GiB、SQLx poolは50、matcher pollingは500msのままです。
