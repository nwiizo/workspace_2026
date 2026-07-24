# Prusti Discount Verification

`DiscountRate` の Smart Constructor と割引計算の事後条件を Prusti 0.2.2 で検証するサンプルです。

## 通常の Rust 品質ゲート

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

## Prusti

2023-08-22 の macOS リリースは Intel 向けです。Apple Silicon では Rosetta 2、同梱版と一致する x86_64 Rust toolchain、x86_64 JDK が必要です。

正しい実装を検証します。

```sh
prusti-rustc --edition=2021 --crate-type=lib src/lib.rs
```

意図的な中間演算のオーバーフローを検出します。このコマンドは失敗することが期待値です。

```sh
prusti-rustc --edition=2021 examples/failing.rs
```

## 実測結果

Apple Silicon上でRosetta 2を使い、Prusti 0.2.2、x86_64 nightly `2023-08-15`、x86_64 JDK 17を組み合わせました。

- 正例: `Successful verification of 4 items`、45.30秒
- 誤例: `attempt to multiply with overflow`、35.54秒

検出力は確認できましたが、公式macOS releaseが2023-08-22のIntel向けであることと、単純なcrateでも実行時間が長いことから、新規サービスへの採用は見送ります。
