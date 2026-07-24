# Kani Discount Verification

`DiscountRate` の境界条件と割引計算の整数安全性を Kani 0.67.0 で検証するサンプルです。

## 通常の Rust 品質ゲート

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

通常のunit testに加え、Proptest 1.11.0で同じ事後条件を256ケース検査します。これは生成した入力を試すテストであり、入力範囲の網羅を証明するものではありません。`failing` featureには、意図的に壊した実装から反例を縮小するignored testも置いています。

## Kani

正しい実装だけを検証します。

```sh
cargo kani --harness valid_rate_round_trips \
  --harness invalid_rate_is_rejected \
  --harness discounted_price_never_increases
```

意図的な境界条件の誤りを検出します。このコマンドは失敗することが期待値です。

```sh
cargo kani --harness detects_off_by_one_constructor
```

意図的な中間演算のオーバーフローを検出します。このコマンドも失敗することが期待値です。

```sh
cargo kani --harness detects_intermediate_overflow
```

正例の成功と誤例の失敗をまとめて確認するCI向けscriptも用意しています。

```sh
./scripts/verify.sh
```

## 実測結果

Apple Silicon上のKani 0.67.0で、正例3 harnessは1.34秒で成功しました。誤例は1.37秒で失敗し、concrete playbackから次の入力を得ました。

- constructorの境界誤り: `raw = 101`
- `u16`中間乗算のoverflow: `price = 2024`, `discount = 0`

通常のRust testは、domain 2件、property test 1件、Axum integration 2件の計5件です。

Axumサービスも同じdomain crateを呼びます。

```sh
cargo run -p kani-discount-service
curl -H 'content-type: application/json' \
  -d '{"price":1000,"discount_percent":25}' \
  http://127.0.0.1:3000/quotes
```

実際の応答は`{"original_price":1000,"discount_percent":25,"final_price":750}`でした。101%の入力はHTTP 422と`{"code":"invalid_discount_percent"}`を返します。
