# Benchmark 41: DB poolをgeneral 34 / coordinate 16へ分割

## 結論

総接続上限50を変えず、`POST /api/chair/coordinate` 専用16本と、それ以外の
general 34本へ分ける最初の仮説は不採用でした。

診断runは128,038点、`pass=true`、error map空でした。通知、評価、matcherのgeneral側は
大幅に軽くなりましたが、毎秒約1,000回発生するcoordinateへ16本では足りません。
周期1/64 sampleのcoordinate pool取得は平均71.623ms、p95 242.474msとなり、
完了rideは1,979件、
drive不満率は79.5%まで悪化しました。

このrunから分かったのは「用途分離が無効」ではなく、「16本という予約数が小さすぎる」
ということです。

## 仮説

Benchmark 40では、共有pool 50本を使ったcoordinate handlerのserver時間平均76.515msのうち、
64.349ms、約84.1%が `pool.acquire()` でした。通知や評価がconnectionを待つburstと
coordinateを別poolへ分ければ、椅子の移動tickを他endpointから隔離できると考えました。

開始値16は、平均流量から求めた必要同時実行数に少し余裕を足した値です。

```text
coordinate流量      = 62,983 / 60秒 ≒ 1,050 request/秒
pool待ち以外の時間   = 76.515 - 64.349 ≒ 12.166ms
平均同時実行数       = 1,050 × 0.012166 ≒ 12.8
```

これはLittleの法則 `L = λW` を使った概算です。

- `L`: system内に同時に存在する処理数
- `λ`: 1秒あたりの到着数
- `W`: 1処理がsystem内にいる平均時間

ただし、この計算は平均しか表しません。benchmarkのrequestは均等間隔ではなく、
複数chairが同じ30ms tickで一斉にPOSTします。status遷移候補では通常coordinateよりSQLが増え、
MySQLの行lockやschedulerの揺れもあります。平均同時実行数12.8に対して16本あれば十分、
とは限りません。

## 実装

`ISUCON_DB_MAX_CONNECTIONS=50` を「各poolの上限」ではなく「process全体の接続予算」とし、
次のように差し引きました。

```text
coordinate = ISUCON_DB_COORDINATE_CONNECTIONS
general    = ISUCON_DB_MAX_CONNECTIONS - coordinate
```

Benchmark 41では `coordinate=16`、`general=34` です。

`AppState.pool` はgeneral用途として残し、`AppState.coordinate_pool` を追加しました。
`chair_post_coordinate` だけがcoordinate poolを使います。

| pool | 利用箇所 |
|---|---|
| coordinate | `POST /api/chair/coordinate` |
| general | 認証middleware、通知、status、評価、ride作成、owner API、matcher、initialize後のcache refresh、2秒ごとの座標cache reconciliation |

2つの `sqlx::MySqlPool` は同じ接続設定を使いますが、待ち行列と上限は別です。
上限16と34を足して50なので、MySQLへ最大84本作る変更ではありません。

## 実行条件

```sh
ISUCON_DB_COORDINATE_CONNECTIONS=16 \
ISUCON_DIAGNOSTIC=1 \
BENCHMARK_OUTPUT_FILE=/tmp/isucon14-b41.log \
./scripts/benchmark.sh 60
```

- Colima: 4 CPU / 4 GiB memory / 100 GiB disk
- benchmark: 60秒
- score: 128,038
- `pass=true`
- error map: 空
- 診断queue: `dropped_lines=0`

## 結果

### drive

| 指標 | 共有50（Benchmark 40） | general 34 / coordinate 16 |
|---|---:|---:|
| score | 146,727 | 128,038 |
| 完了ride | 2,310 | 1,979 |
| drive不満率 | 77.3% | 79.5% |
| 実drive tick p50 | 38 | 47 |
| 実drive tick p95 | 176 | 230 |
| 余分tick p50 | 22 | 33 |
| 余分tick p95 | 144 | 195 |

scoreはどちらも診断1走なので代表値ではありません。それでも完了ride、tick分布、pool phaseが
同じ方向へ悪化しており、16本を採用しない根拠になります。

### drive相関sampleのcoordinate

| 指標 | 共有50 | coordinate 16 |
|---|---:|---:|
| client request平均 | 106.873ms | 108.937ms |
| client p95 | 240.737ms | 343.301ms |
| server pool取得平均 | 64.349ms | 69.832ms |
| server pool取得p95 | 133.390ms | 245.659ms |
| server total平均 | 76.515ms | 81.104ms |
| endpoint平均 | 82ms | 107ms |
| endpoint p95 | 209ms | 295ms |

この表のclient / server phaseは、hashで選んだrideのうち
`picked_up_tick < world_tick < arrived_tick` にあるrequestです。上の結論で使った
71.623ms / 242.474msは、ride状態に依存しない周期1/64 sample 846件です。
母集団が違うため値は一致しません。配分間の通常coordinate負荷は周期sample、
drive tickとの因果はdrive相関sampleで比較します。

周期sample 846件のうち、poolが16本まで増えた後にidle 0だったものは637件、75.3%でした。
その637件のpool取得平均は93.703msです。16本を使い切るburstが例外ではなく、runの大半を
占めています。

### general

分割はgeneral側には効きました。

| 指標 | 共有50 | general 34 |
|---|---:|---:|
| app通知 initial acquire平均 | 47.720ms | 6.129ms |
| app通知 transaction acquire平均 | 49.757ms | 6.538ms |
| chair通知 initial acquire平均 | 49.868ms | 7.978ms |
| chair通知 transaction acquire平均 | 51.591ms | 7.211ms |
| matcher pool begin平均 | 参考値未同一表 | 6.137ms |
| 評価 preparation acquire平均 | 参考値未同一表 | 8.791ms |
| 評価 completion acquire平均 | 参考値未同一表 | 6.754ms |

coordinateが専用poolへ移ったことで、general 34本にはidleが生まれました。つまり、
共有poolで通知が待っていた主因の一つはcoordinateの高頻度取得です。

## どのログを見て判断したか

1. benchmark最終行
   - `pass=true`
   - score 128,038
   - error map空
2. `DRIVE_BENCHMARK_DIAGNOSTIC`
   - 1,979完了ride
   - drive不満79.5%
   - 実tickと余分tickのpercentile
3. `COORDINATE_CLIENT_DIAGNOSTIC`
   - drive中1,087 POST
   - failed POST 0
   - client p95 343.301ms
4. `COORDINATE_DIAGNOSTIC`
   - pool上限16を確認
   - idle 0の割合とpool取得時間を確認
5. `NOTIFICATION_DIAGNOSTIC`、`EVALUATION_DIAGNOSTIC`、`MATCHER_DIAGNOSTIC`
   - general側のpool待ちが短縮したことを確認
6. nginx診断JSON
   - coordinate 54,142件、5xx 0
   - endpoint p95 295ms

正当性errorがないため、score低下をエラー処理の影響とは判断していません。
coordinateの待ち、完了数、drive tickが同時に悪化したことから、接続配分を原因候補にしました。

## 次の選択肢

1. coordinate予約を24へ増やす
   - generalに26本残し、coordinate burstを吸収できるか確認する
2. 中間の20 / 30を測る
   - 24 / 26でgeneral starvationが出た場合の比較点にする
3. shared pool + admission control
   - pool自体は50本共有し、general用途だけ同時取得数を制限する
   - 片側がidleのとき、もう片側が借りられる利点がある
4. coordinateの非同期queue
   - pool分割後も30ms超過が残る場合の別施策
   - per-chair順序、全位置履歴、status遷移、3秒以内の可視性を守る必要がある

16本の失敗だけでpool分割全体を戻すと、general側の大幅改善という情報を捨てます。
次は接続総数を増やさず、配分だけを変えて比較します。
