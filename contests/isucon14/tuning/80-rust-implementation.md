# Rust / sqlx 実装から学べること

[チューニング目次へ戻る](../TUNING.md)

この文書はベンチマーク結果ではなく、ISUCON14 の Rust 実装を読み、変更し、検証するときに役立つ Rust 固有の知識をまとめたものです。スコアの変化は各 Benchmark 文書へ分離します。

## `Pool`、`Connection`、`Transaction` の違い

3つは似ていますが、責務が違います。

| 型・概念 | 役割 | たとえ |
|---|---|---|
| `MySqlPool` | 再利用可能な接続を管理する | 複数の窓口を管理する受付 |
| `PoolConnection<MySql>` | poolから一時的に借りた1接続 | 今使っている1つの窓口 |
| `Transaction<'_, MySql>` | 1接続上の複数SQLをcommit/rollbackまでまとめる | 途中で分割できない一連の手続き |

`&pool` をqueryへ渡すと、sqlxが必要な接続をpoolから借りて返します。複数SQLが同じ接続・同じtransactionになるとは限りません。

```rust
let chair = sqlx::query_as("SELECT ...")
    .fetch_one(&pool)
    .await?;
```

`pool.acquire().await?` で接続を明示的に借りると、複数SQLに同じ通信路を使えます。しかし、それだけではtransactionではありません。MySQLのautocommitが有効なら、各SQLは文ごとに確定します。

```rust
let mut conn = pool.acquire().await?;
query_a.execute(&mut *conn).await?;
query_b.execute(&mut *conn).await?;
```

`pool.begin().await?` は接続を借り、さらにtransactionを開始します。

```rust
let mut tx = pool.begin().await?;
query_a.execute(&mut *tx).await?;
query_b.execute(&mut *tx).await?;
tx.commit().await?;
```

## `&mut *tx` は何をしているのか

sqlxのquery実行には、`Executor` として使える可変参照が必要です。`Transaction` は内部にDB接続を持ち、`DerefMut` を通してその接続を参照できます。

```rust
query.fetch_one(&mut *tx).await?;
```

読み方は次のとおりです。

1. `*tx`: `Transaction` が保持する接続を参照する
2. `&mut`: その接続への可変参照を作る
3. queryへ渡し、同じtransaction上でSQLを実行する

これは「値をコピーしている」のではありません。借用なので、queryの実行後もtransactionは同じ接続を保持します。

ヘルパー関数へ `&mut Transaction` を渡している場合は、関数内でさらに `&mut **tx` が必要になることがあります。参照が一段増えるためです。型エラーが出たときは記号を闇雲に増やさず、現在の変数が `Transaction` 本体か、その可変参照かを確認します。

## transactionを `await` をまたいで保持する意味

Rustの非同期処理では、`.await` 中にOSスレッドを占有し続けるとは限りません。しかし、transactionが借りたDB接続はcommit/rollbackまでpoolへ戻りません。

```text
リクエストA: BEGIN ─ SQL ─ await ─ SQL ─ COMMIT
                    ↑ この区間は接続を保持する ↑
リクエストB: poolに空きがなければ待つ
```

したがって「asyncだから接続待ちは問題にならない」とは言えません。スレッドを空けられても、限られたDB接続という別の資源は占有します。

一方、速さだけを理由にtransactionを外すと、複数SELECTが別時点のデータを見る可能性があります。Benchmark 02では実際に通知の椅子統計が期待値とずれるCODE=33を検出しました。

判断手順は次のとおりです。

1. 複数SQLが同じ時点のデータを見る必要があるか確認する
2. 複数更新を全部成功または全部失敗へまとめる必要があるか確認する
3. 不要な分岐だけをtransaction開始前へ移せないか考える
4. ベンチマーカーの正当性エラーを確認する

## `Option` と早期returnで正常な「データなし」を表す

sqlxの `fetch_optional` は、0行なら `None`、1行なら `Some(row)` を返します。「ライドがまだない」はサーバー障害ではないため、`Option` と相性がよい処理です。

```rust
let ride: Option<(String,)> = sqlx::query_as("SELECT id FROM rides ...")
    .fetch_optional(&pool)
    .await?;

if ride.is_none() {
    return Ok(Json(response_without_notification));
}
```

今回の通知処理では、この軽い存在確認をtransactionの前へ置きます。空pollingだけを早期returnし、通知データを組み立てる複数SQLはtransaction内へ残します。

## N+1をRustのloopだけで解決しない

初期実装の椅子統計は、最初にridesを取得し、Rustの `for` 内でrideごとのstatusを取得します。

```text
1回: 椅子のrides一覧
N回: 各rideのstatus一覧
```

コードは直感的ですが、対象rideが増えるほどSQL往復も増えます。計算を1つの集約SQLへ寄せられる場合は、MySQLがINDEXと集合演算を使えます。

ただし、書き換え前に次を確認します。

- `ARRIVED`、`CARRYING`、`COMPLETED` がすべて必要という元の条件
- `evaluation` がNULLのrideをどう扱うか
- 0件時の平均値が `0.0` になること
- 同じtransaction snapshotで評価する必要があるか

「SQLを1回にした」だけでは正しさの証明になりません。元の分岐を真理値表またはテストケースへ落としてから集約します。

## Rustの型をSQL結果の仕様書として使う

`sqlx::query_as` の結果はRustの構造体またはtupleへ変換されます。集約SQLでは、MySQLの `COUNT` が符号なし整数、`AVG` がNULLになり得るなど、型の違いに注意します。

```rust
#[derive(sqlx::FromRow)]
struct ChairStatsRow {
    total_rides_count: i64,
    total_evaluation_avg: f64,
}
```

APIレスポンスが `i32` を要求するなら、無条件な `as i32` より `i32::try_from` を使うと、将来件数が範囲を超えた場合に壊れ方を明示できます。ISUCONでは速度だけでなく、変換失敗をどのHTTPエラーへするかも設計対象です。

## `Cargo.lock` と再現可能なDocker build

このRust実装はアプリケーションなので、`Cargo.lock` をコミットします。Dockerfileは次のオプションを使います。

```sh
cargo build --release --locked --frozen
```

- `--locked`: `Cargo.lock` を勝手に更新しない
- `--frozen`: lock更新とnetworkアクセスが必要なら失敗する
- `--release`: 最適化した本番向けバイナリを作る

`Cargo.lock` が履歴にないのにローカルだけに存在すると、自分のマシンでは成功してfresh cloneでは `COPY Cargo.lock` または `--locked` が失敗します。Dockerが再現性を保証するのではなく、build contextへ必要な入力をすべて含めて初めて再現可能になります。

## debug buildとrelease buildを混同しない

`cargo test` は通常debug profileを使います。テストが速く回る一方、最適化レベルやコード生成はreleaseと異なります。

品質確認と性能確認の役割を分けます。

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
cargo build --release --locked
```

- fmt: 書式
- clippy: Rustで起きやすいバグや読みにくい表現
- test: 期待動作
- release build: 本番と同じ最適化でbuild可能か
- benchmark: 実際の負荷下での正しさと性能

どれか1つだけで他の性質まで保証することはできません。

## 実測: Docker内のrelease再ビルドを高速化する

### 症状

Rustソースを変更した後のlegacy Docker buildで、次の時間がかかりました。

| 段階 | 時間 |
|---|---:|
| `lib.rs` のrelease生成 | 約14分35秒 |
| libとbinを含む `cargo build --release` 全体 | 30分52秒 |

このときbuild contextは `.dockerignore` により約32KBまで削減済みでした。したがって、遅い場所はDockerへのファイル転送ではなく、コンテナ内のRustコード生成とlinkです。

`docker top` と `docker stats` では、次も確認しました。

- buildコンテナ内で `rustc -C opt-level=3` が実行中だった
- 既存matcherが旧アプリへpollingし、約141% CPUを使用していた
- 既存MySQLが約1.4GiBを保持していた
- buildコンテナは約53% CPU、約320MiBだった

ここから「build cacheが使えない」問題と「旧stackとの資源競合」を分けて直しました。

### 公式資料から確認したこと

[Cargoのrelease profile](https://doc.rust-lang.org/stable/cargo/reference/profiles.html) の既定値は、`opt-level=3`、`incremental=false`、`codegen-units=16`、`lto=false` です。incrementalを有効にすると追加情報を `target/` へ保存し、同じcrateの再コンパイルで再利用できます。

[Cargoのbuild性能ガイド](https://doc.rust-lang.org/cargo/guide/build-performance.html) は、実際のworkflowを計測すること、代替linkerとしてLLDやmoldを検討することを案内しています。特にLinuxの `x86_64-unknown-linux-gnu` 以外はsystem linkerが遅い場合があると説明しています。今回のコンテナは `aarch64-unknown-linux-gnu` です。

[Dockerのcache最適化ガイド](https://docs.docker.com/build/cache/optimize/) は、Rust向けにCargo cacheと `target/` を `RUN --mount=type=cache` で永続化する例を示しています。通常のlayer cacheと違い、ソースCOPYでbuild命令が再実行されてもcache directoryの内容を再利用できます。

### 根本原因

初期Dockerfileも、Cargo.tomlとCargo.lockを先にCOPYし、依存crateのlayerを分けていました。これは依存の再コンパイルを避けますが、アプリ自身の過去の `target/` は次のsource変更へ引き継ぎません。

```text
legacy layer cache
  ├─ dependencies: 再利用できる
  └─ isuride crate: source変更でlayerごと無効 → libとbinを全生成
```

さらに、認証情報を空にするプロジェクト専用 `DOCKER_CONFIG` がHomebrewのCLI plugin directoryも見えなくしていました。ComposeはBuildxを発見できず、legacy builderへフォールバックしていました。

### 修正

1. プロジェクト専用Docker設定へHomebrewのplugin directoryを追加
2. Dockerfile frontendをBuildKitへ変更
3. Cargo registry、Cargo Git、Rust `target/` を別々のcache mountへ保存
4. Docker build内だけ `CARGO_INCREMENTAL=1` を有効化
5. Rust toolchain同梱のLLDを使い、追加packageをinstallせずlink
6. `cargo build --timings` で毎回timing reportを生成
7. cache mount外の `/usr/local/bin/isuride` へ完成binaryをinstall
8. ベンチ前のbuild中だけ、前回のISUCON stackを正常停止

`target/` はcache mountなので、その中のbinaryはimage layerへ自動では残りません。そのため同じ `RUN` の中でcache外へcopyする必要があります。

```dockerfile
RUN --mount=type=cache,target=/home/isucon/webapp/rust/target \
    CARGO_INCREMENTAL=1 cargo build --release --locked --frozen --timings \
    && install -m 0755 target/release/isuride /usr/local/bin/isuride
```

releaseの `opt-level=3` は変更していません。コンパイルを速くするためにアプリの実行時最適化を下げると、前後のISUCONスコアを同じ条件で比較できないためです。

### 途中で失敗したこと

manifestだけをCOPYして `cargo fetch` した最初の試行は、270.98秒後に失敗しました。

```text
no targets specified in the manifest
either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present
```

Cargoはtargetのないmanifestをpackageとして解決できません。依存fetchの間だけ最小の `src/main.rs` を作り、fetch後に削除する形へ修正しました。

また、matcherを `docker pause` した試行は、Composeがpaused containerをstartできず失敗しました。現在は `docker compose stop` で正常停止し、`up.sh` で再開します。

### 結果

ホストおよびColimaの4 CPU / 4 GiBは変更していません。

| build | Cargo時間 | Docker build壁時計 | 備考 |
|---|---:|---:|---|
| 変更前のsource再build | 30分52秒 | 30分超 | legacy builder、旧stack稼働 |
| BuildKit cache初回作成 | 4分08秒 | 6分15.24秒 | 全依存compileとcrate取得を含む |
| owner SQL変更後の再build | 7.03秒 | 11.02秒 | incremental target cache hit |
| nearby SQL変更後の再build | 6.79秒 | 未分離 | 小さいsource変更、cache hit |
| chair stats変更後の再build | 10.34秒 | 未分離 | 小さいsource変更、cache hit |
| batch matcher変更後の再build | 42.89秒 | 未分離 | 同じcacheでもホスト負荷により増加 |
| 近傍優先matcher変更後の再build | 39.77秒 | 未分離 | 同じ4 CPU / 4 GiB、cache hit |
| 座標更新変更後の再build | 13.36秒 | 未分離 | query統合と結果型追加、cache hit |

最良の実測では、source変更後のDocker build壁時計は約168分の1になりました。その後のCargo時間は6.79〜42.89秒と幅があります。座標更新変更後も13.36秒で再buildできました。cache hitは「必ず同じ秒数になる」という意味ではなく、変更範囲、link対象、同じホストのI/O・CPU負荷でも変わります。それでもlegacy buildの30分52秒より大幅に短く、チューニングの再計測を現実的な時間で回せました。

初回と再buildを混ぜると誤解を招くため、fresh cloneでは最初にcache作成時間が必要なこと、再build時間は複数回の分布で見ることを明記します。

生成したbinaryは次を通しました。

- `cargo test`: 成功
- Docker smoke test: `GET /` 200、`POST /api/initialize` 正常
- owner SQL時点の60秒公式ベンチ: `pass=true`、スコア5,601、エラー0
- 近傍優先matcherまで含む60秒公式ベンチ: `pass=true`、スコア16,909、エラー0
- 座標更新とcoupon code INDEXまで含む60秒公式ベンチ: `pass=true`、スコア15,415、エラー0

### 注意点と他の選択肢

| 方法 | 利点 | 注意点 |
|---|---|---|
| BuildKit cache mount + incremental | source変更後の再buildが非常に速い | builderのcache削除後は初回compileが必要 |
| LLD | GNU system linkerより速い | C/C++依存を含むprojectでは互換性確認が必要 |
| `opt-level` を下げる | code generationが速い | runtime性能が変わるため今回不採用 |
| `codegen-units` を増やす | 並列化しやすい | 生成コードが遅くなる可能性 |
| crateを分割する | 変更していないcrateを再利用しやすい | 設計変更が大きく、境界設計が必要 |
| sccache | CIや複数buildで再利用しやすい | changed crate全体が必ずhitするわけではない |

cacheは正しさの前提にしません。Docker公式資料が説明するように、cacheは消去されてもbuildが成功する必要があります。`Cargo.lock` と `--locked --frozen` は引き続き再現性のために必要です。

## 警告を性能変更へ混ぜない

未使用フィールドの削除や括弧の整理は大切ですが、SQL変更と同じコミットへ混ぜると、ベンチ結果の原因を追いにくくなります。

この作業では次の単位に分けます。

1. 動作を変えないRust品質修正
2. 1つの性能仮説に対応する実装修正
3. その仮説のベンチマーク記録

小さいコミットは単に行数が少ないだけでなく、「なぜ変えたか」「何で確認したか」を1つに説明できる単位であることが重要です。

## この実装での高速化の優先順位

2026-07-24時点のlockfileでは、主な実行時依存は `sqlx 0.8.2`、`tokio 1.42.0`、`axum 0.7.9` です。現在の処理を見ると、Rustの数命令を減らすより、まずDBとの境界を短くする方が改善余地は大きいと考えられます。

| 優先度 | 観測・変更候補 | 理由 |
|---:|---|---|
| 1 | SQL回数、走査行数、実行時間を減らす | N+1や集約SQLは1リクエストで数十〜数百回の往復差になり得る |
| 2 | transactionとconnectionの保持時間を短くする | pool待ちはDBを使う全APIへ波及する |
| 3 | 同じ結果を返す重複queryと `SELECT *` を減らす | 通信量、decode、所有する `String` 等をまとめて減らせる |
| 4 | request単位のログ量と同期I/Oを確認する | 高頻度経路ではログの整形・出力先待ちが累積する |
| 5 | Tokio workerを止めるblocking処理を探す | 1つの長い同期処理が同じworker上のtaskを遅らせる |
| 6 | CPU profileに現れたallocationやcloneを減らす | 根拠なく借用を増やすと複雑さだけが増えやすい |
| 7 | LTO、codegen units、CPU固有命令を比較する | I/O待ちが支配的な間は改善がスコアに現れにくい |

この順序は固定ではありません。CPU使用率が1 coreに張り付き、MySQLとpoolに余裕があるという観測が得られたら、CPU profileの優先度を上げます。

## 「asyncにしたから並列」ではない

`.await` は「この処理の完了まで他のtaskを動かしてよい」という中断点です。次のloopは非同期I/Oを使っていますが、query自体は1件ずつ直列です。

```rust
for ride in rides {
    let status = get_latest_ride_status(&pool, &ride.id).await?;
    // 次のrideは、このqueryが完了してから処理する
}
```

独立した処理なら `tokio::try_join!` 等で同時に待てますが、ISUCONのN+1を機械的に並列化するのは危険です。

- 同じ `&mut Transaction` は同時に可変借用できず、1本のMySQL connectionもqueryを同時実行する通信路ではない
- `&pool` からqueryごとに別connectionを借りれば並列化できるが、同じtransaction snapshotではなくなる
- N本を一斉実行するとpoolとMySQLへN本分の仕事を押し込み、他のHTTP requestを遅らせる
- query回数とDBの総仕事量は減らない

まずJOIN、subquery、集約、bulk queryで往復そのものを減らします。意味を保ったままSQLへまとめられず、かつ独立性が確認できた場合だけ、同時数を制限して並列化します。

## Tokio workerをblocking処理で止めない

Tokioはtaskが `.await` へ到達することで他のtaskを進めます。async handler内で長いCPU計算、同期ファイルI/O、同期HTTP client、`std::thread::sleep` 等を実行すると、その間はworker threadが他のtaskを進められません。

Tokioの [`spawn_blocking`](https://docs.rs/tokio/1.42.0/tokio/task/fn.spawn_blocking.html) は、終了する同期処理をblocking専用threadへ逃がすためのAPIです。ただし、何でも包めば速くなるわけではありません。

- sqlxとreqwestのasync APIは、そのまま `.await` する。`spawn_blocking` で包まない
- 数回の整数演算である `calculate_distance` は移動コストの方が大きくなり得るため、包まない
- CPU-bound処理を多数投げる場合はSemaphore等で同時数を制限する
- 開始済みの `spawn_blocking` は通常のasync taskのようにはcancelできない
- 長時間常駐する処理は専用threadや別processを検討する

現在の `post_initialize` は `tokio::process::Command` を使っているため、子processの終了待ち自体はasyncです。将来、handlerへ圧縮、画像処理、大量JSON変換、同期SDK等を追加した場合に再確認します。

![blocking処理をTokio worker上で実行する場合と専用poolへ逃がす場合](./images/tokio-blocking-worker.webp)

*async workerは短いtaskを進める役割に保ち、長い同期処理だけを制限付きのblocking poolへ分離します。*

## connection poolは「50なら速い」わけではない

現在は `max_connections(50)` です。[sqlxのPool](https://docs.rs/sqlx/0.8.2/sqlx/struct.Pool.html) は上限に達して全connectionが貸出中なら、返却されるまで `acquire()` を待たせます。上限は同時処理の目標値ではなく、防波堤です。

上限を増やしたときに起きることは、次のどちらかです。

1. MySQLに余裕があり、Rust側だけが絞りすぎていたなら待ちが減る
2. MySQLがすでに飽和しているなら、同時queryが増えて各queryが遅くなる

したがって `50 → 100` のような変更だけを先に行いません。ベンチ中に次を同じ時系列で観測します。

```rust
let size = pool.size() as usize;
let idle = pool.num_idle();
let in_use = size.saturating_sub(idle);
```

毎requestでログへ出すと観測自体が負荷になります。1秒ごとのsampling、または一時的なmetrics endpointで `size`、`idle`、`in_use` を採取します。合わせてMySQLの実行中thread数、CPU、statement時間を見ます。

[`PoolOptions`](https://docs.rs/sqlx/0.8.2/sqlx/pool/struct.PoolOptions.html) には次の診断・調整手段があります。

- `acquire_timeout`: connection取得全体の待ち時間上限
- `acquire_slow_threshold`: 遅い取得をログにする閾値
- `min_connections`: 起動時に一定数をwarm upする候補
- `max_connections`: このprocessが保持する上限

`acquire_timeout` は遅い処理を速くする設定ではなく、長時間待つ代わりに失敗を早く返す設定です。ベンチの正当性エラーを増やす可能性があります。`min_connections` も開始直後の接続確立を減らすだけで、定常状態のSQL量は減らしません。

## ログは消す前に量と出力先を測る

`main.rs` は `TraceLayer::new_for_http()` を追加し、環境変数がなければ `tower_http=debug` を有効にします。一方、ローカルの `compose.yaml` は既定で `RUST_LOG=info` を渡します。どのlevelが実際に出ているかは、起動方法を含めて確認する必要があります。

比較は同じbinary、同じベンチ条件で行います。

```sh
RUST_LOG=info ./scripts/benchmark.sh 60
RUST_LOG=warn ./scripts/benchmark.sh 60
```

確認するのはスコアだけではありません。

- 60秒間のlog行数とbyte数
- webappのCPU使用率
- p50 / p95 / p99 latency
- エラーの種類と件数

`warn` で改善した場合も、必要なエラーまで無条件に捨てません。request成功log、SQL debug log、エラー時のbacktraceを分け、負荷走行用filterを明示します。逆に差がなければ、ログ削除を主要な高速化として扱いません。

## release binaryをprofileする

Rust Performance Bookは、最適化対象を推測せずhotな箇所をprofilerで特定することを勧めています。debug binaryは本番とコード生成が違うため、release binaryへ最低限の行情報を付けます。

```toml
[profile.release]
debug = "line-tables-only"
```

Linuxなら `perf` / `cargo-flamegraph`、macOSならInstrumentsや `samply` が候補です。stackが欠ける場合だけ、次のようにframe pointerを残したbinaryを別に作ります。

```sh
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release --locked
```

profile用instrumentationにはoverheadがあります。profile採取runと、最終スコアを記録するrunは分けます。[Tokio Console](https://tokio.rs/tokio/topics/tracing-next-steps) は長くpollされるtaskやresource待ちの調査に使えますが、現在の依存へ一時的なfeatureとsubscriberを追加するため、診断用変更を入れたままスコア比較しません。

flame graphを読むときは、次の境界を区別します。

- `mysql`、socket、poll待ちが中心: SQLとpoolを調べる
- serdeや文字列処理が中心: response構築と返す列を調べる
- `malloc` / `free` が中心: allocation箇所を調べる
- tracingの整形・writerが中心: filterと出力先を調べる
- 特定の純粋Rust関数が中心: algorithmとデータ構造を調べる

![profileで支配的なhotspotを特定し、そこだけ直して再計測する流れ](./images/profile-before-optimize.webp)

*細かな最適化を広く試す前に、release profileで時間を占める箇所を絞ります。*

## allocationとcloneはhot pathだけ直す

heap allocationにはコストがありますが、すべての `String` や `clone()` が問題ではありません。[Rust Performance Bookのheap allocationの章](https://nnethercote.github.io/perf-book/heap-allocations.html) も、profileでhotと判明した箇所を対象にするよう勧めています。

この実装で先に確認する候補は次です。

- `SELECT *` で使わない文字列列までdecodeしていないか
- 件数が分かっているresponse `Vec` に `Vec::with_capacity` を使えているか
- loop内の `format!`、`to_string()`、所有 `String` がprofileへ現れるか
- 大量rowを一度 `Vec` に集めずstream処理すべきか

一方、`AppState` 内の `MySqlPool::clone()` は接続群を複製しません。sqlxのpoolは参照countされたhandleで、cloneは安価です。Router構築時の `app_state.clone()` を借用へ直すことは優先しません。

`SELECT *` の削減はallocationだけでなく、MySQLからの転送量、sqlxのdecode、Rust structの所有値を同時に減らせます。細かな `clone` 削除より先に、必要な列だけを返すqueryを検討します。

## Cargoのrelease profileは候補ごとに比較する

現在のDocker buildは `cargo build --release` なので、Cargo既定の `opt-level = 3`、`lto = false`、`codegen-units = 16` を使います。[Cargo profileの公式資料](https://doc.rust-lang.org/stable/cargo/reference/profiles.html) によると、LTOはcrateをまたいだ最適化を増やす代わりにlink時間が伸び、codegen unitsを減らすと並列compileが減る代わりに最適化しやすくなります。

比較候補は、通常のrelease profileを基準に1項目ずつ試します。

| 候補 | `Cargo.toml` へ追加する設定 |
|---|---|
| A | `[profile.release]` の `lto = "thin"` |
| B | `[profile.release]` の `codegen-units = 1` |

AとBを同時に入れず、それぞれ既定releaseと比較してから、両方を組み合わせる価値があるか判断します。

予想ではなく、次を記録します。

| 指標 | 既定release | 候補profile |
|---|---:|---:|
| clean build時間 | 計測 | 計測 |
| source 1行変更後のbuild時間 | 計測 | 計測 |
| binary size | 計測 | 計測 |
| 60秒スコア | 計測 | 計測 |
| p95 latency / error | 計測 | 計測 |

`panic = "abort"` はbinary sizeを小さくできる候補ですが、handler内のpanicでprocess全体が終了する意味変更を伴います。通常経路を速くする設定として先に入れません。

`RUSTFLAGS="-C target-cpu=native"` はbuild hostのCPU向け命令を使います。[rustcの資料](https://doc.rust-lang.org/rustc/codegen-options/index.html#target-cpu) にあるとおり、別CPUでは動かないbinaryを作る可能性があります。大会サーバー上でbuildしてそのサーバーだけで実行する場合に限って比較し、Apple Silicon上で作ったimageをamd64環境へ持ち込むような運用とは混ぜません。

PGOは代表的な負荷から分岐情報を集めて再buildする手法ですが、手順とbuild時間が大きく増えます。基本的なSQL、pool、ログ、profile由来のhotspotを解消した後の候補です。

## Dockerfile高速化は3種類に分ける

Dockerfileの「高速化」は、同じものを指していません。

| 指標 | 速くする対象 | 60秒ベンチへの直接効果 |
|---|---|---|
| build context | Docker builderへ送るfile量 | なし |
| image build | crate downloadとcompileの反復時間 | なし |
| final image | pull、展開、配布、起動前準備 | 通常はなし |
| application runtime | 起動後のHTTP/SQL処理 | あり |

buildが10分から1分になれば試行回数を増やせますが、それ自体をスコア改善として記録しません。final imageを小さくしても、起動済みprocessのSQLが速くなるわけではありません。

![Cargo cache再利用とmulti-stage buildを組み合わせる設計候補](./images/rust-docker-build-pipeline.webp)

*この検証で試したDockerfileは、左側のBuildKit cache再利用までを採用しました。右側のruntime stage分離は、image縮小を目的に追加できる別の候補です。*

### 変更前Dockerfileが使っていたlayer cache

変更前の `development/dockerfiles/Dockerfile.rust` は、先に `Cargo.toml` と `Cargo.lock` だけをcopyし、ダミーの `src/main.rs` で依存crateをrelease buildしていました。

```dockerfile
COPY ./Cargo.toml ./Cargo.lock ./
RUN mkdir src \
 && echo 'fn main() {}' > ./src/main.rs \
 && cargo build --release --locked \
 && rm src/main.rs target/release/deps/isuride-*

COPY ./src/ ./src/
RUN cargo build --release --locked --frozen
```

sourceだけを変えた場合、前半の依存build layerを再利用できるのが利点でした。この検証では [実測: Docker内のrelease再ビルドを高速化する](#実測-docker内のrelease再ビルドを高速化する) で説明したとおり、ダミーsourceの依存buildをやめ、`cargo fetch` とBuildKit cache mountを試しました。

変更前方式には次の注意点がありました。

- package構成、binary、build script、workspaceが増えるとダミーsourceの保守が難しくなる
- release成果物を同じstageへ残すため、Rust toolchainと巨大な `target/` layerがfinal imageにも残る
- Docker layer cacheが消えたfresh buildでは全依存を再download・compileする

2026-07-24のローカルimageでは、`docker image ls` の表示は約3.7GBでした。これは転送・保存・展開の問題であり、起動後のHTTP throughputを示す数値ではありません。

### `cargo-chef` はダミーsource方式の保守性を上げる

[`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) は、`cargo chef prepare` でmanifestとtarget構成から `recipe.json` を作り、`cargo chef cook` で依存だけをbuildする外部toolです。変更前のダミー `main.rs` と目的は同じですが、workspace member、複数binary、`src/lib.rs` 等の構成をrecipeへ自動反映します。

```dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-1.83-bookworm AS chef
WORKDIR /home/isucon/webapp/rust

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /home/isucon/webapp/rust/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked \
 && cp target/release/isuride /tmp/isuride
```

`cargo chef cook` と最終 `cargo build` は同じ `WORKDIR`、同じRust versionで実行します。絶対pathやcompiler versionが変わると、期待した依存cacheを使えないためです。

`latest-rust-1.83-bookworm` tagのmanifestは2026-07-24時点で確認できましたが、`latest` は将来別の `cargo-chef` を指し得ます。採用時は実際にbuildできたimage digestへ固定します。

現在は単一crateなので、追加toolを入れずBuildKit cache mountだけを使う方が単純です。workspace化やbinary追加で依存buildの再現が難しくなった場合、またはCIでDocker layer cacheを共有する場合の候補として、source rebuild時間を比較して選びます。

### BuildKit cache mountでCargo cacheを再利用する

Docker公式の[build cache最適化資料](https://docs.docker.com/build/cache/optimize/)は、Rust向けにCargo homeと `target/` をcache mountする例を示しています。

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:1.83-bookworm AS build

WORKDIR /home/isucon/webapp/rust
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/home/isucon/webapp/rust/target,sharing=locked \
    --mount=type=cache,target=/var/cache/cargo,sharing=locked \
    CARGO_HOME=/var/cache/cargo \
    cargo build --release --locked \
 && cp target/release/isuride /tmp/isuride
```

通常のlayer cacheは命令と入力が一致したときだけ再利用されます。cache mountはbuild命令が再実行されても、Cargoが前回download・compileした変更のない部分を再利用できます。

重要なのは、cache mountの中身がfinal image layerへ保存されないことです。そのため同じ `RUN` の中で、完成binaryをmount外の `/tmp/isuride` へcopyします。これを忘れると、次のstageからbinaryをcopyできません。

cacheは正解データではなく、消えてもbuildできる補助データとして扱います。BuildKitのgarbage collectionで消える可能性があるため、cacheが空でも `Cargo.lock` から再構築できる必要があります。この例で `--frozen` を付けないのは、空cacheからはnetwork downloadが必要だからです。事前の `cargo fetch --locked` で依存を揃える構成なら、その後のbuildだけを `--frozen` にできます。CIのbuilderが毎回使い捨てなら、`buildx --cache-to` / `--cache-from` によるexternal cacheを別途検討します。

### multi-stage buildでcompilerをfinal imageから外す

[Dockerのmulti-stage build](https://docs.docker.com/build/building/multi-stage/) は、build用stageから必要な成果物だけをruntime stageへcopyします。このアプリでは `scratch` をそのまま使えません。

- `post_initialize` が `../sql/init.sh` を起動する
- `init.sh` がMySQL CLIを使う
- 現在のGNU/Linux向けbinaryはglibc等のruntime libraryを必要とする

したがって、まずは同系統の `debian:bookworm-slim` をruntime stageにして必要packageだけを入れる案が小さい変更です。

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:1.83-bookworm AS build

WORKDIR /home/isucon/webapp/rust
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/home/isucon/webapp/rust/target,sharing=locked \
    --mount=type=cache,target=/var/cache/cargo,sharing=locked \
    CARGO_HOME=/var/cache/cargo \
    cargo build --release --locked \
 && cp target/release/isuride /tmp/isuride

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install --no-install-recommends -y \
      bash \
      ca-certificates \
      default-mysql-client-core \
      gzip \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /home/isucon/webapp/rust
COPY --from=build /tmp/isuride ./isuride
EXPOSE 8080
CMD ["./isuride"]
```

これは設計例であり、未計測・未適用です。採用前に次を確認します。

```sh
# binaryが要求する共有library
ldd target/release/isuride

# 初期化scriptを含む起動確認
./scripts/up.sh
./scripts/benchmark.sh 10

# image size
docker image ls isucon14-rust-webapp
```

ローカルComposeは `webapp/sql` を `/home/isucon/webapp/sql` へmountするため、`WORKDIR` と `../sql/init.sh` の相対位置を変えません。公式環境ではvolume mountではなくホスト上の配置を使う可能性があるため、systemd起動も別に確認します。

Alpineや `scratch` まで小さくするにはmusl向けbuild、TLS証明書、DNS、timezone、shell、MySQL clientの扱いが変わります。imageをさらに小さくする目的だけで、競技中にruntime互換性まで同時に変更しません。

### Docker buildの比較条件

cacheの有無で数字の意味が変わるため、最低3種類を分けます。

| 条件 | 測るもの |
|---|---|
| fresh build | base image以外のcacheがない状態からの再現時間 |
| source rebuild | `.rs` だけを1行変更した反復時間 |
| dependency rebuild | `Cargo.toml` / `Cargo.lock` 変更後の時間 |

各buildで次を残します。

- build contextのbyte数
- `cargo build` stepの時間
- cache hit / missしたstep
- final image size
- build hostのarchitecture
- application benchmarkとは別の記録であること

`docker builder prune` は他projectのcacheも消す破壊的な操作です。fresh buildを測るために共有builder全体を無造作に消さず、専用builderや別cache keyを使います。

## ISUCON14 Rust実装の確認順

次の順序なら、効果の大きい境界から狭められます。

1. `performance_schema` とaccess logで、回数と累積時間が大きいendpoint / SQLを特定する
2. `app_handlers.rs` のrides・nearby、`owner_handlers.rs` のsales・chairsにあるloop内queryを集合SQLへ寄せる
3. `chair_handlers.rs` のcoordinate・notificationでtransactionとconnectionの保持区間を測る
4. `middlewares.rs` の認証queryが高頻度経路で占める割合を測る
5. poolの `size` / `num_idle` とMySQLの同時実行数を同時採取する
6. `RUST_LOG=info` と `warn` を同条件で比較する
7. release binaryのCPU / allocation profileを採る
8. profileに出た箇所だけ、列削減、capacity確保、clone削減、algorithm変更を行う
9. 最後にrelease profileを1設定ずつ比較する
10. Docker buildはsource rebuild、dependency rebuild、final image sizeを別々に測る

## 避けるショートカット

- N+1を無制限の `tokio::spawn` で隠す
- asyncなsqlx queryを `spawn_blocking` で包む
- pool上限とTokio worker数を根拠なく同時に増やす
- cacheを `std::sync::Mutex` で守り、そのguardを保持したまま `.await` する
- `unsafe`、独自allocator、全箇所のborrow化をprofileなしで導入する
- LTO、`target-cpu=native`、ログlevel、SQL変更を1回のベンチへまとめる
- multi-stage化によるimage縮小を、アプリruntimeのスコア改善として数える
- cache mount内にだけ完成binaryを置き、次stageへ残ると思い込む

高速化は「速そうな構文」へ書き換える作業ではありません。待っている資源を観測し、その待ちを生む仕事量または保持時間を1つずつ減らし、同じ条件で正しさとスコアを再計測する作業です。

## 参考資料

- [Rust Performance Book: Profiling](https://nnethercote.github.io/perf-book/profiling.html)
- [Rust Performance Book: Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html)
- [Cargo Book: Profiles](https://doc.rust-lang.org/stable/cargo/reference/profiles.html)
- [rustc book: Codegen options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- [Tokio: `spawn_blocking`](https://docs.rs/tokio/1.42.0/tokio/task/fn.spawn_blocking.html)
- [Tokio: Next steps with Tracing / Tokio Console](https://tokio.rs/tokio/topics/tracing-next-steps)
- [sqlx 0.8.2: `Pool`](https://docs.rs/sqlx/0.8.2/sqlx/struct.Pool.html)
- [sqlx 0.8.2: `PoolOptions`](https://docs.rs/sqlx/0.8.2/sqlx/pool/struct.PoolOptions.html)
- [Docker Docs: Optimize cache usage in builds](https://docs.docker.com/build/cache/optimize/)
- [Docker Docs: Multi-stage builds](https://docs.docker.com/build/building/multi-stage/)
- [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef)
