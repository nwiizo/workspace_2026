# Flux verification sample

`DiscountRate` の範囲を refinement type として表現し、料金計算の事後条件と算術安全性を検査します。

通常の Rust ビルドでは注釈が消去されるため、同じ crate をサービスから利用できます。Flux の検証では `check-overflow = "strict"` を明示し、符号なし整数の中間乗算も検査対象にします。

```console
cargo flux --Fcheck-overflow strict
cargo flux --features failing --Fcheck-overflow strict
```

後者は `u16` の中間乗算について、オーバーフローを報告する想定です。

正例の成功と誤例の失敗をまとめて確認するCI向けscriptも用意しています。

```sh
./scripts/verify.sh
```

scriptは検証前にFlux、Rust、Liquid Fixpoint、Z3のversionが下記の組み合わせと一致することも確認します。

## 固定した環境

- Flux commit `bd5de273235457a5922280cc180c4a6a2990418a`
- Rust nightly `2026-10-22`
- Liquid Fixpoint `0.9.6.3.8`（commit `0b924442ce26a8f270bffac18ba6295bc7411646`）
- Z3 `4.16`

Fluxの検証器も依存ライブラリと同じcommitから構築します。上流の`rust-toolchain.toml`に合わせたnightlyを使い、[公式のソースビルド手順](https://flux-rs.github.io/flux/guide/install.html)に従います。この版ではCargo metadataの設定名が`check-overflow`です。以前の`check_overflow`は受け付けません。

## 検証結果（2026-11-02、Apple Silicon macOS）

- 正例（サンプルcrate）: 8関数中8 checked、0 trusted、6 constraints
- 誤例: `src/bin/failing.rs`の中間乗算で`arithmetic operation may overflow`
- 通常のRust test: domain 2件、Axum integration 2件
- `./scripts/verify.sh`: version確認、fmt、Clippy、test、正例の成功、誤例の拒否をすべて確認

上記の関数集計はサンプルcrateの結果です。依存ライブラリには別途`trusted`関数が含まれ、検証済み関数としては数えません。サービスへ採用するときは`check-overflow = "strict"`を外せないCI規約にします。
