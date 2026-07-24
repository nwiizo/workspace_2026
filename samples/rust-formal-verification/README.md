# Rust Formal Verification Samples

Rust向け形式検証ツールを、同じ料金計算とAxumサービスで実測したサンプルです。2026-07-15にApple Silicon macOSで、正例の受理、誤例の拒否、通常のCargo workflowとの共存を確認しました。

## サービスの検証境界

対象は`DiscountRate`と`apply_discount`からなる純粋なドメインcrateです。

- `DiscountRate`は0から100までの値だけを保持する
- `apply_discount`は`u16`の全価格で中間演算がoverflowしない
- 割引後の価格は元の価格を超えない
- 100%割引の結果は0になる

HTTP、JSON、status codeは証明対象に含めません。採用候補のKani、Verus、FluxにはAxum service crateを併設し、25%割引と不正な101%入力をTowerの結合テストで確認します。

## 評価項目

| 評価軸 | 確認内容 |
|---|---|
| 検出力 | 正例が通り、overflowを含む誤例が落ちるか |
| 保証の種類 | 反例探索、機能仕様、refinementのどれに向くか |
| サービス境界 | 検証済みcrateをAxumから通常の依存として呼べるか |
| 通常開発 | `cargo build`、Clippy、testを壊さないか |
| CI再現性 | verifier、nightly、solver、依存を固定できるか |
| 診断 | 反例または証明できない契約を修正へ結び付けられるか |
| 保守負担 | 注釈量、toolchain更新、チームの学習負担が妥当か |
| 信頼境界 | `assume`、trusted関数、未検証compilerを把握できるか |

計測時間はこの端末での1回の実行値であり、性能比較のbenchmarkではありません。導入時に待つ時間の規模を知るための記録です。

## 結果

| Tool | 方式 | 正例 | 誤例 | service | 判断 |
|---|---|---:|---:|---:|---|
| Kani 0.67.0 | bounded model checking | Proptest 256ケースと3 harness、Kaniは1.34秒 | Proptestの反例は一例として`656, 0`、Kaniはoverflowと境界誤りを具体値付きで検出 | Axum test 2件 | 採用 |
| Verus 0.2026.07.12.0b42f4c | 演繹的検証 | 6 verified、1.15秒 | 事後条件とoverflowを検出 | Axum test 2件 | 重要ドメインへ選択採用 |
| Flux `85ae8dc` | refinement type | 4 checked、1.55秒 | overflowを検出、0.55秒 | Axum test 2件 | 値域と局所契約へ選択採用 |
| Creusot 0.12.0 | 演繹的検証 | `Proved (3 files)`、12.91秒 | 乗算を含むVCが4/5で停止、14.62秒 | なし | proofは成功、採用は見送り |
| Prusti 0.2.2 | 演繹的検証 | 4 items、45.30秒 | overflowを検出、35.54秒 | なし | 新規採用は見送り |

## 選定

### Kani

小さな有限入力、unsafe境界、panic、overflow、index errorから具体的な反例を得たい場合の第一候補です。harnessの`assume`とloopのunwind boundは仕様の一部としてレビューします。

### Verus

金額、認可、quota、状態遷移のように、事前・事後条件と数式をproofとして長期保守するcrateへ限定します。今回の3候補では注釈と学習の負担が最も大きいため、一般的なCRUDへ広げません。

### Flux

percentage、ID、index、buffer lengthなど、値域や大小関係を関数境界へ自然に書ける場合に使います。`check_overflow = "strict"`を必須とし、Flux、Rust nightly、Liquid Fixpoint、Z3のversionをCIで検査します。

### Creusot

0.12.0のcore binaryはApple Siliconで起動し、正例からComaを生成できました。しかし、solverを含む公式Nixの`free`環境はmacOS上でCoCoALibがLinux向け`libgmp.so`を要求して構築に失敗しました。stableと2026-07-15時点のmasterで同じ問題を再現しています。

サンプルにはCVC4とCVC5を外し、Alt-Ergoだけを使う最小flakeを追加しました。macOSで不足する`codesign`もflake内で補い、正例と誤例のproofを再現できました。それでも公式環境との差分を保守する必要があり、今回の契約用途では公式arm64配布とCargo統合を持つVerusを優先します。

### Prusti

正例と誤例の検証には成功しました。ただし、最新の公式macOS releaseは2023-08-22のIntel向けで、Rosetta 2、x86_64 JDK 17、x86_64 nightlyをそろえる必要がありました。単純な正例にも45.30秒かかり、新規サービスの標準toolchainにはしません。

## ディレクトリ

- `kani/`: unit・property test、Kani harness、意図的な誤例、Axum service
- `verus/`: 契約とproof、意図的な誤例、Axum service
- `flux/`: refinement、意図的な誤例、Axum service
- `creusot/`: Creusotの正例・誤例とmacOS向け最小Nix flake
- `prusti/`: Prustiの正例・誤例とRosetta環境の記録

各ディレクトリのREADMEに、実行コマンドと期待結果を記載しています。Kani、Verus、Fluxの`scripts/verify.sh`は、通常testと正例の成功に加え、意図的な誤例を各検証器が拒否することまで成功条件にします。
