# Benchmark 33: SQLx connection pool上限を50 / 75 / 100で比較

## 結論

SQLx connection poolの`max_connections`を50 / 75 / 100で比較し、50を維持しました。
通常60秒ベンチ3走の中央値は次のとおりです。

| pool上限 | 3走 | 中央値 | pool 50比 | 判定 |
|---:|---|---:|---:|---|
| 50 | 101,918 / 107,234 / 114,728 | 107,234 | - | 維持 |
| 75 | 105,867 / 118,846 / 99,700 | 105,867 | -1.3% | 不採用 |
| 100 | 103,720 / 107,229 / 95,129 | 103,720 | -3.3% | 不採用 |

全9 runが`pass=true`・error map空です。上限を75 / 100へ増やすとconnection取得待ちは
短縮しましたが、connection所有時間とInnoDB row-lock待ちが増え、通常中央値は1.3% /
3.3%下がりました。有限な4 CPU / 4 GiB環境では、DBへ同時に流す処理を増やし続けるより、
50でbackpressureを掛ける方がスコアへつながりました。

`ISUCON_DB_MAX_CONNECTIONS`を追加し、未指定時の既定値を50にしました。0や非数値は
起動時にエラーにします。ホストのCPU / memory / diskは4 CPU / 4 GiB / 100 GiBのまま
変更していません。

## はじめに知っておく用語

### connection pool

DB connectionをrequestごとに作り直さず、アプリ内で再利用する仕組みです。SQLxの
`Pool<MySql>`は複数taskから共有でき、handlerは`acquire()`で1本借り、処理後に返します。

`max_connections = 50`は「起動直後から必ず50本接続する」という意味ではありません。
必要に応じて増え、同時に所有できる上限が50本という意味です。`min_connections`を
指定しない現在の構成では、起動直後の不要なhandshakeを増やしません。

### backpressure

下流が処理できる量を超えたとき、上流を待たせて流量を制限する仕組みです。
pool上限に達した`acquire().await`は空きを待つため、pool自身がMySQL手前の
backpressureとして働きます。

待ちは常に悪いものではありません。待ちをゼロにするため接続を無制限に増やすと、
待ち場所がアプリのpoolからMySQLのrow lock、CPU scheduler、disk I/Oへ移るだけの場合が
あります。後者は実行途中のtransactionが資源を持ったまま待つため、全体にはより高価です。

### queueをどこへ置くか

上限50ではrequestがアプリ側でconnectionを待ちます。上限100ではより多くのrequestが
MySQLへ入り、row lockを待ちます。どちらにもqueueはありますが、性質が異なります。

```text
HTTP request
    |
    v
SQLx acquire待ち ---- pool上限で安全に待つ、まだDB transactionなし
    |
    v
MySQL実行・row lock待ち ---- connection、transaction、場合により他のlockを保持
```

最適値は「待ちをなくす値」ではなく、「安価な待ちとDBの実行並列を釣り合わせる値」です。

### Littleの法則

安定した系では、同時に存在する仕事量`L`は、おおむね到着率`λ`と平均滞在時間`W`の積です。

```text
L = λ × W
```

たとえばDB区間の平均滞在が長いまま到着率だけを増やすと、同時実行中のqueryやlock待ちが
増えます。Benchmark 32で外部決済をDB区間から外して`W`を先に94%短縮したからこそ、
Benchmark 33ではpool上限を増やす余地ができました。順序を逆にすると、約300msの決済待ちを
抱えたconnectionを100本まで増やすだけになります。

これは厳密な待ち行列モデルによる予測値ではありません。ベンチ負荷は時間とともに増え、
endpointごとにDB区間も異なるためです。「保持時間を先に短くし、その後で並列度を調整する」
理由を理解するための関係です。

### `size`、`idle`、`in_use`

- `size`: poolが現在持つconnection数
- `idle`: その瞬間に貸出可能と観測したconnection数
- `in_use`: この計測では`size - idle`で求めた値

`size`と`idle`は別々に読むため完全に原子的なsnapshotではありません。また`idle = 0`は
「必ず長い待ち行列がある」という意味ではなく、観測した瞬間に貸出可能なconnectionが
なかったことだけを表します。実際の`acquire_us`と組み合わせて判断します。

### MySQL `Max_used_connections`

MySQL process起動後に同時接続数が最大何本だったかを示す累積statusです。pool上限以外に、
初期化、matcher、診断用mysql clientも接続するため、上限より少し大きくなります。

今回の診断runではpool 50 / 75 / 100に対して51 / 77 / 101でした。設定が実際に負荷中の
接続数へ反映されたことを確認できます。ただし接続しているだけのthreadと、CPU上で実行中の
`Threads_running`は同じではありません。

## なぜこの検証を今行ったか

Benchmark 30ではcoordinateのpool acquire p95が113.156msで、SQL `BEGIN` p95 2.327msより
大幅に長く、sampleの78.1%でpool size 50 / idle 0でした。

しかし当時は評価APIが外部決済中もconnectionを平均319.754ms所有していました。上限を
増やす前にBenchmark 32で評価を準備transaction、transaction外決済、完了transactionへ
分け、所有平均を19.241msへ短縮しました。

それでも評価sampleの初回66.5%、完了時66.0%でsize 50 / idle 0だったため、残った待ちは
「長い不要な保持」だけではなく、短いDB区間の同時実行上限にも起因すると考えました。

## 仮説

pool 50から75へ増やすと、アプリ側acquire待ちが減り、coordinate、nearby、通知が
30ms tick内に返りやすくなると考えました。一方、100まで増やすと4 CPUのMySQLへqueryを
流し込みすぎ、row lockとquery自体の滞在時間が増える可能性があると予測しました。

採用条件は次のとおりです。

- 公式ベンチが`pass=true`、error map空
- 通常3走の中央値が50を上回る場合だけ上限を増やす
- 候補間では同じhot-path実装の通常3走中央値が高い
- acquireだけでなく、connection所有、endpoint latency、row-lockも確認する
- CPU / memoryを変えず、pool上限以外を同条件にする

## 実装

`ISUCON_DB_MAX_CONNECTIONS`をRust起動時に読み、正の`u32`へ変換して
`MySqlPoolOptions::max_connections`へ渡します。

```rust
let configured = match std::env::var("ISUCON_DB_MAX_CONNECTIONS") {
    Ok(value) => Some(value),
    Err(std::env::VarError::NotPresent) => None,
    Err(error) => {
        return Err(anyhow::Error::new(error)
            .context("ISUCON_DB_MAX_CONNECTIONS must contain valid Unicode"));
    }
};
let max_connections = parse_db_max_connections(configured.as_deref())?;

let pool = MySqlPoolOptions::new()
    .max_connections(max_connections)
    .connect_with(options)
    .await?;
```

未指定なら50です。`0`はpoolとして意味がなく、非数値や`u32`範囲外も設定ミスなので
暗黙に50へ戻さず起動を失敗させます。誤設定を隠すと、本番だけ別条件で動いたことに
気づけないためです。

Composeも同じ変数を渡します。

```yaml
ISUCON_DB_MAX_CONNECTIONS: "${ISUCON_DB_MAX_CONNECTIONS-50}"
```

比較時だけshellから50 / 75 / 100を指定でき、通常起動は維持した50になります。

## 比較条件と再計測の境界

Benchmark 32で記録したpool 50の通常3走は、今回の対照には流用していません。その後、
評価準備phaseへ`evaluation.is_some()`の再確認を追加しており、requestのhot pathが異なる
ためです。比較用のpool 50は、その再確認を含む状態で通常3走と診断1走を取り直しました。

50 / 75 / 100はすべて`ISUCON_DB_MAX_CONNECTIONS`へ値を明示し、同じhandler、SQL、
診断sampling率、60秒負荷で比較しています。通常ベンチは各3走、診断ベンチは各1走です。
有効な数値を環境変数からpoolへ渡す経路とrequest hot pathは3条件で同じです。

比較run後に、未指定時の既定値と非Unicode値の起動エラーをより明示するstartup-onlyの
読み取り処理へ整理しました。有効な`50`、`75`、`100`の変換と
`max_connections`への設定、request処理は変わりません。したがって通常比較には明示値を
使った9 runを採用し、最終構成では未指定時の50を短時間ベンチでも確認しました。

## 診断run

各条件は60秒、評価1/8・coordinate 1/64 sampling、nginx timing log付きです。
診断instrumentationを含むため、scoreは通常得点の推定には使いません。

ログの集計境界を再確認できるよう、ベンチ開始時のUTC時刻を残します。各runはこの時刻より
後のapplication / nginx / MySQLログを集計し、DB再起動後のstatus差分を確認しました。

| pool上限 | ベンチ開始（UTC） | 診断score | 判定 |
|---:|---|---:|---|
| 50 | 2026-07-24T23:45:40Z | 122,261 | `pass=true`、error map空 |
| 75 | 2026-07-24T23:17:35Z | 115,300 | `pass=true`、error map空 |
| 100 | 2026-07-24T23:20:07Z | 130,607 | `pass=true`、error map空 |

| 指標 | pool 50 | pool 75 | pool 100 |
|---|---:|---:|---:|
| 診断score | 122,261 | 115,300 | 130,607 |
| 評価成功sample | 219 | 202 | 232 |
| 初回acquire平均 | 32.447ms | 24.173ms | 20.848ms |
| 初回acquire p95 | 90.979ms | 71.158ms | 66.841ms |
| 完了acquire平均 | 28.783ms | 23.837ms | 20.108ms |
| 完了acquire p95 | 80.774ms | 74.322ms | 64.037ms |
| connection所有合計平均 | 18.637ms | 26.527ms | 30.410ms |
| connection所有合計p95 | 36.770ms | 59.012ms | 64.424ms |
| 初回size上限 / idle 0 | 162 / 219（74.0%） | 140 / 202（69.3%） | 143 / 232（61.6%） |
| 完了size上限 / idle 0 | 144 / 219（65.8%） | 122 / 202（60.4%） | 133 / 232（57.3%） |
| `Max_used_connections` | 51 | 77 | 101 |

上限を増やすとacquire平均は単調に短縮しました。しかしconnection所有平均は逆に
18.637→26.527→30.410msと増えています。connectionを取った後のMySQL処理が、同時実行と
競合の増加で長くなったためと考えられます。これは「pool待ちをDB内の待ちへ移した」
兆候です。

### endpoint

| endpoint | 指標 | pool 50 | pool 75 | pool 100 |
|---|---|---:|---:|---:|
| coordinate | 件数 | 75,809 | 79,453 | 83,931 |
| coordinate | 平均 | 62ms | 59ms | 55ms |
| coordinate | p95 | 175ms | 174ms | 152ms |
| nearby | 件数 | 12,687 | 13,609 | 16,165 |
| nearby | 平均 | 33ms | 32ms | 25ms |
| nearby | p95 | 126ms | 130ms | 97ms |
| evaluation | 件数 | 1,750 | 1,611 | 1,854 |
| evaluation | 平均 | 418ms | 415ms | 412ms |
| evaluation | p95 | 787ms | 787ms | 776ms |

100の診断runは最も多く処理し、130,607点でした。ただしworld、負荷到達度、決済retry回数が
runごとに異なる単発値です。後述の通常3走では100の中央値が75を下回ったため、診断の
最高単発scoreだけで100を採用しません。

### InnoDB row lock

| 指標 | pool 50 | pool 75 | pool 100 |
|---|---:|---:|---:|
| wait回数 | 3,880 | 4,975 | 5,439 |
| 累積wait時間 | 71,955ms | 117,733ms | 143,536ms |
| 1 wait平均 | 18ms | 23ms | 26ms |
| 最大wait | 146ms | 159ms | 147ms |

MySQL process lifetimeは94–96秒で近く、各runはDB再起動から集計しています。ただし処理件数も
異なり、全endpointのrow lockを含む累積値です。絶対値だけで因果を断定しません。それでも
上限増加とともに1 wait平均も18→23→26msへ増えたため、DB内競合が強くなる方向は
connection所有時間の増加と整合します。

## 通常60秒ベンチ

### pool 50

| run | score | matching / pickup / drive不満率 |
|---:|---:|---|
| 1 | 101,918 | 55.5% / 41.4% / 61.8% |
| 2 | 107,234 | 52.2% / 37.3% / 61.9% |
| 3 | 114,728 | 43.6% / 36.0% / 65.4% |

中央値107,234点、観測範囲101,918–114,728点です。

### pool 75

| run | score | matching / pickup / drive不満率 |
|---:|---:|---|
| 1 | 105,867 | 56.5% / 39.6% / 61.7% |
| 2 | 118,846 | 50.7% / 33.1% / 65.3% |
| 3 | 99,700 | 54.7% / 40.0% / 60.0% |

中央値105,867点、観測範囲99,700–118,846点です。pool 50中央値比-1.3%です。

### pool 100

| run | score | matching / pickup / drive不満率 |
|---:|---:|---|
| 1 | 103,720 | 47.2% / 38.3% / 63.3% |
| 2 | 107,229 | 52.1% / 38.0% / 60.0% |
| 3 | 95,129 | 60.5% / 43.7% / 56.8% |

中央値103,720点、観測範囲95,129–107,229点です。pool 50中央値比-3.3%です。

3条件ともrun間の分散が大きく、観測範囲も重なります。1.3%や3.3%の差を将来の保証値とは
扱いません。
同じhot-path実装で取り直した50の中央値が最も高く、上限追加でDB内競合指標も悪化するため、
50の維持がこのローカル条件で最も妥当です。

## なぜ50を維持するか

100はacquire平均だけを見ると最良で、75も50より短いです。しかし次が不利でした。

- 通常3走中央値は75が50より1.3%、100が3.3%低い
- connection所有平均は50の18.637msから26.527 / 30.410msへ増加
- InnoDBの1 wait平均は18msから23 / 26msへ増加
- `Max_used_connections`が101へ達し、4 CPUに対する同時接続が増える

pool待ちだけを短くするために、より高価なDB内滞在を増やす価値は通常scoreで確認できません。

## 他に考えられる選択肢

### `min_connections`を50にする

起動時にconnectionを事前作成し、最初のburstでhandshakeを避ける案です。今回の診断では
定常負荷中に上限まで増えた後のacquire待ちが主で、起動直後だけの問題ではありません。
MySQL再起動直後に50本を一斉作成する負荷も増えるため、今回は変更しません。

### `acquire_timeout`を短くする

待ちrequestを早く失敗させればtail latencyは打ち切れますが、公式ベンチではHTTP 500が
error budgetを消費します。仕事量を減らさず失敗へ変えるだけなので、先に採用しません。

### endpointごとにsemaphoreを置く

coordinateや通知など高頻度経路へ別の同時実行上限を設け、評価・owner queryとの干渉を
制御できます。pool全体の上限より細かいbackpressureですが、どのendpointを何件にするか
追加計測が必要です。次にDB競合が特定endpointへ偏ると分かった場合の候補です。

### 読取り用・書込み用poolを分ける

読取りが書込みのconnectionを使い切らないよう隔離できます。ただし両poolの上限合計、
transactionの接続先、initialize時の扱いが増え、MySQL全体の競合は消えません。
read replicaがない単一MySQLでは、優先度制御の効果を計測してから検討します。

### MySQL `max_connections`やbufferを同時に変える

複数要因を同時に変えると、pool上限の効果を説明できません。現在のMySQL
`max_connections=151`はpool 100にも足りています。buffer poolなどはquery・I/Oの
診断を別Benchmarkで行います。

### Tokio worker thread数を増やす

DB connection待ちはasyncなので、待機中にworker threadを占有しません。pool上限、
Tokio thread数、ホストCPUを同時に変える根拠はありません。今回ホストCPUは固定しました。

## 実行方法

通常は維持した50を使います。

```sh
./scripts/up.sh
./scripts/benchmark.sh 60
```

比較時だけ上書きできます。

```sh
ISUCON_DB_MAX_CONNECTIONS=50 ./scripts/benchmark.sh 60
ISUCON_DB_MAX_CONNECTIONS=75 ./scripts/benchmark.sh 60
ISUCON_DB_MAX_CONNECTIONS=100 ./scripts/benchmark.sh 60
```

診断runは次の形式です。

```sh
ISUCON_DIAGNOSTIC=1 \
ISUCON_DB_MAX_CONNECTIONS=50 \
./scripts/benchmark.sh 60
```

## 次のTODO

1. prepared statement digestとphase診断から、pool 50でconnectionを所有した後の
   p95を増やすqueryを特定する
2. `CODE=26`再発時に座標responseとowner累積距離を同じchair IDで採取する
3. 通知cache missのphaseを分け、poolを持たないlong pollingと比較する
4. pool 50は4 CPU / 4 GiBローカル環境の値なので、本番構成では各process / DBの
   CPUと接続上限を再計測する
