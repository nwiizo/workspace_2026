# souther-style-rust

[Souther](https://github.com/souther-lang/souther) が JVM 上で与える保証を、Rust の型と crate 構成だけで作れるかを確かめるサンプルです。題材は Souther のチュートリアルと同じ出張申請の提出で、10 万円を超える申請を却下します。

## Souther の保証との対応

| Souther | このサンプル | 確かめ方 |
| --- | --- | --- |
| `data Amount = Int invariant value >= 0` | `nutype` の `Amount` | Kani が全 `i64` で「0 以上だけ受理」を証明 |
| `constructs Submitted` | `Submitted` のフィールドを非公開にする | `compile_fail,E0451` の doctest |
| `-> Submitted \| Rejected` | `SubmitOutcome` enum を返し、`Result` を使わない | 網羅的 `match` |
| `depends on currentTime` | `Clock` trait を引数で受ける | 領域 crate は `no_std` なので時刻を自分で取れない |
| 派生 decoder | `serde` の `Deserialize` | 負の額と社員番号 0 の JSON が拒否されるテスト |
| `example` 行 | `tests/trip_examples.rs` の `ROWS` 表 | `submit_rows` |
| signature 網羅性 | `strum` で出力ケースを列挙し `ROWS` と比較 | `every_outcome_has_a_row` |
| border 網羅性 | ON 点と OFF 点の行 + cargo-mutants | ON 点の行を外すと変異を取り逃す |

例の実行と網羅性の検査が同じ行を読む必要があるので、例は `rstest` の `#[case]` ではなく定数の表に置いています。`#[case]` は実行時に列挙できず、網羅性の検査から参照できません。

## 実行

```sh
./scripts/verify.sh
```

`cargo kani` には [Kani](https://github.com/model-checking/kani) 0.67.0 が必要です。

## 実測結果

2026-09-23 に Apple Silicon macOS、Rust 1.98.1、cargo-mutants 27.1.0、Kani 0.67.0 で確認しました。

| 検査 | 結果 |
| --- | --- |
| `cargo test` | 統合テスト 3 件、doctest 2 件が成功 |
| `cargo mutants` | 8 件中 3 件を検出、5 件はコンパイル不能 |
| `cargo mutants --features without-border` | `>` を `>=` に変えた変異 1 件を取り逃し |
| `cargo kani` | 2 harness が成功 |
| 閾値を `>=` に壊した実装への `cargo kani` | `raw > PRE_APPROVAL_LIMIT` の検査で失敗 |

コンパイル不能の 5 件は、`Default` を持たない領域型を `Default::default()` に置き換える変異です。領域型に `Default` を付けないこと自体が、不正な値を作らせない制約として働いています。

## Souther に届かない点

Souther の `souther examples` は partition と border の欠落を、行を書く前に名前付きで報告します。このサンプルでは欠落は変異テストを回した後に「生き残った変異」として間接的に分かるだけです。
