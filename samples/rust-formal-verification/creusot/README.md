# Creusot Discount Verification

`DiscountRate`の型不変条件と割引計算の事後条件をCreusot 0.12.0で検証するサンプルです。

- `passing/`は`u32`の中間値を使い、overflowしない実装です
- `failing/`は`u16`のまま乗算するため、入力によって中間値がoverflowします

## 通常のRust品質ゲート

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

## Creusot

このサンプルのflakeは、Alt-Ergoだけを含む最小構成です。公式`#free` packageがmacOSで構築できない問題を避け、正例と誤例を同じ環境で検証します。

正しい実装を検証します。

```sh
nix run path:. -- creusot -p creusot-discount-passing --no-cache
```

意図的に壊した実装を検証します。このコマンドは失敗することが期待値です。

```sh
nix run path:. -- creusot -p creusot-discount-failing --no-cache
```

## 2026-07-15の導入結果

Creusot 0.12.0のcore binaryはApple Silicon上で起動し、`cargo-creusot 0.12.0`とnightly `2026-04-21`を確認しました。正例からComaを生成する段階までは公式core packageだけでも成功しています。

```sh
nix run github:creusot-rs/creusot/v0.12.0#creusot \
  -- creusot -p creusot-discount-passing --only coma
```

solverを含む公式Nix環境は次の問題で構築できませんでした。

- 既定packageはunfreeのAlt-Ergoを含み、Nixの許可が必要
- `#free` packageはCoCoALibのbuild中にLinux向け`libgmp.so`を探し、macOSで失敗
- stable 0.12.0と2026-07-15時点のmasterで同じ失敗を再現
- CVC4とCVC5を除いた後も、Why3findのinstallが`codesign`を見付けられず失敗

`flake.nix`では、次の2点を明示的に補っています。

- proverをfree版Alt-Ergo 2.4.3だけに絞る
- macOSではWhy3findのnative build inputへ`darwin.sigtool`を加える

この環境での実測結果は次の通りです。

- 正例: `Proved (3 files)`、12.91秒
- 誤例: `Goal Coma.vc_apply_discount: ✘ (4/5)`、14.62秒

誤例の未証明subgoalは、生成されたComa上の`UInt16.mul`を含みます。検出力は確認できましたが、公式環境との差分を保守する必要があります。今回の新規サービスでは同じ契約用途にVerusを優先し、Creusotの採用は見送ります。
