# relax-jit

[Relax](https://makepad.github.io/relax/)（Rust用のJITとライブコンパイラ）と、Rustのビルド待ちを減らす定番の手段
（[Cargo Bookのビルド性能の手引き](https://doc.rust-lang.org/cargo/guide/build-performance.html)の各項目、
profileの`opt-level`、リンカ、sccache、cargo-nextest）を、同じ編集・実行・テストで比べる検証コード。

## 記事の数値を測った環境

cargoの各手段はLinuxのコンテナ（`scripts/linux-all.sh`ほか）、Relaxとの比較は同じマシンのmacOS（`scripts/macos-all.sh`ほか）で測った。
Relaxは`c1301db`の時点でaarch64のLinuxに対応しておらず、ソースの`compile_error!`でビルドが止まる。

| 項目 | 値 |
|---|---|
| ホスト | Apple M4 Pro（14コア）、メモリ48GB。ColimaのVM（aarch64、10 vCPU、32GiB、Ubuntu 24.04、Linux 6.8） |
| コンテナ | `rust:1.100-bookworm` |
| Rust | rustc 1.100.0（stable）、rustc 1.102.0-nightly (5f4effd57 2026-11-13) + `rustc-codegen-cranelift-preview` |
| リンカ | GNU ld 2.40（既定）、LLD 14.0.6、mold 1.10.1（Debianのパッケージ）、wild 0.11.0 |
| ツール | sccache 0.19.0、cargo-nextest 0.9.152、hyperfine 1.15.0 |
| Relax | makepad/relax `c1301db6264986df964cc79ff1f6a9afda3b7fe6`（2026-11-16） |

## 構成

| パス | 内容 |
|---|---|
| `cli-deps/` | clap（derive）・regex・serde。`Cargo.lock`に33パッケージ。依存のあるCLIの代表 |
| `live-counter/` | `../hotreload-rs/part1-hot-lib-reloader`と同じ`State`・`step`・`render`を1つのbinにしたもの。実行中に編集する |
| `borrowck-bg/` | rustcが拒否する借用（E0502）を含む関数。Relaxが実行前に止めるか |
| `perf-rounds/` | ソートとエラトステネスの篩を8ラウンド。ラウンドごとの時間でRelaxの段階的な最適化を見る |
| `closure-capture/` | Relaxで値が変わる最小の再現。型をstructのフィールドから推論させた`f64`をクロージャで読む |
| `probes/gp_probe.rs` | `../rust-grokking-ai-algorithms`の`GaussianProcess`の中間値を出す例 |
| `hello/`、`with-serde/` | macOSでの予備の計測に使った小さなパッケージ |
| `scripts/linux-all.sh` | Linuxでの各手段の計測。コンテナの中で実行する |
| `scripts/linux-best.sh`、`scripts/linux-round2.sh` | 効いた手段の組み合わせ（1回目・2回目）。`linux-all.sh`のコンテナを保存したイメージで実行する |
| `scripts/linux-tests.sh` | `--no-fail-fast`で`cargo test`とcargo-nextestを同じ設定で比べる |
| `scripts/macos-all.sh` | macOSでRelaxとcargoを比べる。ツールチェインは`~/relax-bench/mac`に隔離する |
| `scripts/macos-firstrun.sh`、`scripts/macos-checkfirst.sh` | 新しい実行ファイルの初回起動と、Relaxの`--check-first`の有無を測る |
| `scripts/run-remote.sh`、`scripts/remote-sequence.sh` | ソースをSSHで別のMacへ送り、上の計測を同時に走らないよう順に起動する |
| `scripts/*.sh`（ほか） | macOSで先に試した計測。手元のVMやほかのプロセスの負荷で値が2〜4割ぶれたため、記事の数値には使っていない |

`vendor/`（Relaxのclone）、`results/`（計測結果と作業コピー）、各`target/`はGitの対象外。

## 実行

```sh
# Docker（Colima）が使える別のMacへソースを送り、Linuxのコンテナで計測を始める
HOST=user@host PORT=22 scripts/run-remote.sh
# そのMacで、残りの計測（組み合わせ、macOSでのRelaxとの比較など）を順に流す
ssh -p 22 user@host 'nohup bash ~/relax-bench/src/relax-jit/scripts/remote-sequence.sh > ~/relax-bench/sequence.log 2>&1 &'
# 結果を取り寄せる
rsync -a -e "ssh -p 22" user@host:relax-bench/out/ results/linux-all/
rsync -a -e "ssh -p 22" user@host:relax-bench/mac/out/ results/macos/
```

TLSを復号して再署名するネットワーク（SSL検査）では、`static.rust-lang.org`とcrates.ioに接続できない。
`CA_NAME`にそのルート証明書の名前を渡すと、macOSのシステムキーチェーンから取り出してコンテナに渡し、
コンテナの中でだけ信頼させる。ホストの設定は変えない。

```sh
CA_NAME=<ルート証明書の名前> HOST=user@host PORT=22 scripts/run-remote.sh
```

## 編集の方法

- `../rust-grokking-ai-algorithms`: `src/search.rs`の`binary_search`にある`(0, values.len())`を`(0 * <乱数>, values.len())`へ書き換える
- `cli-deps`: `summarize`の`lines.len() + 0 * 1`の数字を乱数へ書き換える

どちらも結果は変わらず、関数1つの本体だけが変わる。元のsampleには触らず、コンテナ内のコピーを編集する。

## 値が変わる最小の再現

```sh
cd closure-capture
cargo run -q
../vendor/relax/target/release/relax run --quiet closure-capture
```

rustcは全行で`rhs = [-1.0, 1.0]`。Relax（`c1301db`）は`inferred`の行だけ`rhs`がずれる。
