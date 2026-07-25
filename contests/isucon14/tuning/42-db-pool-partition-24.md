# Benchmark 42: coordinate 24 / general 26を診断比較

## 結論

総接続50のうちcoordinateへ24本を予約すると、Benchmark 41の16本よりcoordinateの
pool待ちが半分以下になり、診断runは152,128点、2,386完了ride、drive不満74.0%まで
改善しました。`pass=true`、error map空です。

一方、general 26本は通知、評価、matcherのburstでほぼ常時飽和しました。
この1走だけでは採用せず、20 / 30との中間比較と、通常モード3走を続けます。

## 仮説

Benchmark 41ではcoordinate pool 16本がidle 0だったsampleが75.3%で、idle 0時の取得平均は
93.703msでした。general 34本では通知のpool取得が6–8msまで下がったため、generalから8本を
coordinateへ移しても、全体としては改善余地があると考えました。

```text
Benchmark 41: general 34 + coordinate 16
Benchmark 42: general 26 + coordinate 24
```

総接続はどちらも50です。

## 実行条件

```sh
ISUCON_DB_COORDINATE_CONNECTIONS=24 \
ISUCON_DIAGNOSTIC=1 \
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b42.log \
./scripts/benchmark.sh 60
```

- Colima: 4 CPU / 4 GiB memory / 100 GiB disk
- benchmark: 60秒
- score: 152,128
- `pass=true`
- error map: 空
- 診断queue: `dropped_lines=0`

## 結果

### drive相関sampleで16本から24本へ増やした効果

| 指標 | coordinate 16 | coordinate 24 |
|---|---:|---:|
| score | 128,038 | 152,128 |
| 完了ride | 1,979 | 2,386 |
| drive不満率 | 79.5% | 74.0% |
| 実drive tick p50 | 47 | 31 |
| 実drive tick p95 | 230 | 158 |
| client coordinate平均 | 108.937ms | 103.005ms |
| client coordinate p95 | 343.301ms | 293.770ms |
| server pool取得平均 | 69.832ms | 33.852ms |
| server pool取得p95 | 245.659ms | 112.942ms |
| server total平均 | 81.104ms | 48.607ms |
| coordinate endpoint平均 | 107ms | 69ms |
| coordinate endpoint p95 | 295ms | 216ms |

poolを8本増やしただけですが、pool取得平均は約51.5%減りました。
接続待ちは線形ではありません。到着数が処理能力へ近づくと待ち行列が伸び、少しの余裕で
queueが大きく縮むためです。

この表のclient / server phaseはdrive中のhash選択rideです。周期sampleでは、
16本のpool取得平均71.623ms / p95 242.474msに対し、24本は
平均30.414ms / p95 108.400msでした。

周期sample 998件のうち、pool 24本・idle 0は695件、69.6%でした。まだ飽和していますが、
idle 0時の取得平均は42.739msで、16本の93.703msより短くなりました。

### general 26本の代償

| 指標 | general 34 | general 26 |
|---|---:|---:|
| app通知 initial acquire平均 | 6.129ms | 54.826ms |
| app通知 transaction acquire平均 | 6.538ms | 54.023ms |
| chair通知 initial acquire平均 | 7.978ms | 55.658ms |
| chair通知 transaction acquire平均 | 7.211ms | 55.338ms |
| 評価 preparation acquire平均 | 8.791ms | 62.607ms |
| 評価 completion acquire平均 | 6.754ms | 63.541ms |
| matcher pool begin平均 | 6.137ms | 44.028ms |

app通知のinitial acquireは440 sample中351件、transaction acquireは439件中359件が
pool 26本・idle 0でした。評価も準備253 / 299件、完了242 / 299件がidle 0です。

nginxでは通知のclient切断を示す499がapp 155件、chair 111件ありました。benchmarkの
error mapは空ですが、general側に十分な余裕があるとは評価できません。

### endpoint

| endpoint | count | avg | p95 | p99 | 5xx |
|---|---:|---:|---:|---:|---:|
| coordinate | 63,816 | 69ms | 216ms | 305ms | 0 |
| app notification | 117,629 | 55ms | 265ms | 374ms | 0 |
| chair notification | 78,370 | 87ms | 299ms | 397ms | 0 |
| evaluation | 2,386 | 483ms | 878ms | 970ms | 0 |
| matcher | 121 | 113ms | 349ms | 396ms | 0 |

coordinate件数と完了rideは増えましたが、general endpointは長くなっています。

## なぜscoreだけで即採用しないか

152,128点はこの時点の最高値ですが、診断instrumentation付き1走です。

- benchmarkの生成worldはrunごとに異なる
- 診断I/Oが通常モードにはない
- 24本ではgeneral側のp95と499が悪化
- 1走のscoreは外れ値を区別できない

そのため、次の2段階を残しました。

1. 20 / 30を診断し、両側の待ちを比較する
2. 採用候補を診断なしで3走し、中央値と正当性を確認する

## InnoDBログの読み方

fresh MySQL process lifetimeでrow-lock waitは2,682回、合計131,594ms、平均49msでした。
この値はcoordinateだけでなく全endpointの累積です。24本に増やしたためrow lockが増えた、
とはこの値だけでは言えません。

同じrunで見るべき関係は次です。

```text
pool待ち: application側で接続を借りる前
row lock: 接続を借り、SQLをMySQLへ送った後
```

今回coordinate server totalのうちpool待ちは平均33.852ms、その他は約14.755msです。
pool待ちは減りましたが0ではなく、SQL・COMMIT・row lock側も次の下限になります。

## 次の選択肢

- 20 / 30を中間点として測る
- 24 / 26を通常3走し、general悪化を含めても総scoreが安定するか確認する
- static partitionを採用後も、shared pool + admission controlを比較する
- 通知の同一request内2回acquireを1 connectionへまとめ、general 26本の圧力を下げる

最後の通知案はpool配分と同時に入れません。配分だけの効果を分離してから別benchmarkにします。
