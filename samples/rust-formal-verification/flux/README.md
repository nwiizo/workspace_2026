# Flux verification sample

`DiscountRate` の範囲を refinement type として表現し、料金計算の事後条件と算術安全性を検査します。

通常の Rust ビルドでは注釈が消去されるため、同じ crate をサービスから利用できます。Flux の検証では `check_overflow = "strict"` を明示し、符号なし整数の中間乗算も検査対象にします。

```console
cargo flux --Fcheck-overflow strict
cargo flux --features failing --Fcheck-overflow strict
```

後者は `u16` の中間乗算について、オーバーフローと事後条件違反を報告する想定です。

正例の成功と誤例の失敗をまとめて確認するCI向けscriptも用意しています。

```sh
./scripts/verify.sh
```

scriptは検証前にFlux、Rust、Liquid Fixpoint、Z3のversionが下記の組み合わせと一致することも確認します。

## 固定した環境

- Flux commit `85ae8dca62b561bd49ed0a96d057625451539ad1`
- Rust nightly `2025-11-25`
- Liquid Fixpoint `0.9.6.3.7`
- Z3 `4.16`

## 実測結果

- 正例: 8関数中4 checked、4 trusted、6 constraints、1.55秒
- 誤例: `arithmetic operation may overflow`、0.55秒
- 通常のRust test: domain 2件、Axum integration 2件

`trusted`を検証済み関数として数えないでください。サービスへ採用するときは`check_overflow = "strict"`を外せないCI規約にします。
