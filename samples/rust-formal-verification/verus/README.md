# Verus Discount Verification

`DiscountRate` の Smart Constructor と割引計算の事後条件を Verus 0.2026.07.12.0b42f4c で検証するサンプルです。

正しいドメイン crate を Cargo 経由で検証し、Axum サービスから同じ crate を呼び出します。

```sh
cargo verus build -p verus-discount-verification
cargo test --workspace --all-targets
```

意図的な中間演算のオーバーフローを検出します。このコマンドは失敗することが期待値です。

```sh
cargo verus focus -p verus-discount-verification --features failing
```

正例の成功と誤例の失敗をまとめて確認するCI向けscriptも用意しています。

```sh
./scripts/verify.sh
```

## 実測結果

Apple Silicon向け公式release `0.2026.07.12.0b42f4c`と、対応する`vstd = 0.0.0-2026-07-12-0122`を使いました。

- 正例: `6 verified, 0 errors`、clean後1.15秒
- 誤例: `postcondition not satisfied`と`possible arithmetic underflow/overflow`を検出
- 通常のRust test: domain 2件、Axum integration 2件

`service` crateは`verify = false`とし、検証済みdomain crateだけをVerusへ渡します。HTTP境界はTowerの結合テストで検査します。
