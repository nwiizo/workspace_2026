# requirements-evidence-rust

AI時代の要求工学を、口座間送金の小さなRustドメインで実行するサンプルである。

中心にあるのは、検証技法の数ではない。`Software Requirements Essentials`の20プラクティスを使って
問題、判断者、利用場面、品質属性、業務ルールを整理し、要求IDから型、テスト、Kani、Fuzzing、性能、
攻撃者視点のテストまでを辿れるようにした。

## 最初に読むもの

1. [`DESIGN.md`](DESIGN.md): 20プラクティスを設計段階から適用した判断記録。
2. [`REQUIREMENTS.md`](REQUIREMENTS.md): 要求ベースライン、業務ルール、証拠の追跡表。
3. [`src/lib.rs`](src/lib.rs): `parse -> authorize -> execute`を表す公開API。
4. [`SECURITY.md`](SECURITY.md): 自動化した攻撃ケースと、独立評価へ残した範囲。

## 公開APIが示す処理順

```rust
let parsed = ParsedTransfer::parse(input)?;
let authorized = ledger.authorize(authenticated_actor, parsed)?;
let receipt = authorized.execute()?;
```

`ParsedTransfer`は直接実行できない。`AuthorizedTransfer`は`&mut Ledger`を保持するため、認可と実行の間に
同じ台帳を変更できない。IDと金額は`NonZeroU64`を内包する別々のnewtypeであり、送金額の上限と異なる
2口座という規則はSmart Constructorに閉じ込めた。

## 通常の品質ゲート

Rust 1.97.1で確認した。

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
cargo test --doc
cargo test --test adversarial
```

テスト構成は、受け入れ7件、攻撃者視点6件、Proptest 3件、無視指定の性能1件、rustdocのコンパイル失敗
1件である。Proptestは各性質を512ケース生成する。

## Kaniによるモデル検査

[Kaniの公式導入手順](https://model-checking.github.io/kani/install-guide.html)に従い、0.67.0を使う。

```sh
cargo install --locked kani-verifier --version 0.67.0
cargo kani setup
cargo kani \
  --harness successful_transition_preserves_total \
  --harness transition_result_matches_preconditions
```

Kaniへ渡すのは、2つの`u64`残高と`TransferAmount`から次の残高を計算する純粋な`transition`である。
成功時の合計保存と、失敗理由が事前条件に一致することを検査する。台帳、認証、永続化を含むシステム全体の
正しさを示すものではない。

## Fuzzing

[cargo-fuzzの公式手順](https://rust-fuzz.github.io/book/cargo-fuzz/setup.html)に従う。libFuzzerのsanitizerを
使うためnightlyが必要だが、既定toolchainを変更する必要はない。

```sh
rustup toolchain install nightly
cargo install --locked cargo-fuzz --version 0.13.2
RUSTUP_TOOLCHAIN=nightly cargo fuzz run parse_transfer -- \
  -max_total_time=60 -max_len=512
```

Fuzz targetは任意バイト列を解析し、panicがないことに加え、成功した値の入力長、異なる2口座、送金額の
値域をassertする。時間制限を外せば継続探索できる。

## 性能テスト

品質属性QR-3は、10万件の逐次送金が開発機のrelease buildで2秒未満であることを求める。

```sh
cargo test --release --test performance -- --ignored --nocapture
cargo bench --bench transfer
```

この閾値は本番SLOではない。変更前後を同じマシンで比較し、回帰を見つけるためのサンプルである。

## まとめて実行する

Kani、cargo-fuzz、nightlyを導入した後は、次のスクリプトで全ゲートを実行できる。

```sh
./scripts/verify.sh
```

スクリプトはローカル検証用に15秒のFuzzingを行う。継続的なFuzzingや独立したセキュリティ評価は
別ジョブとして扱う。

## 実測結果

Apple M3、macOS 26.5.2、Rust 1.97.1、Kani 0.67.0、cargo-fuzz 0.13.2で次を観測した。

| Gate | Result |
|---|---|
| 通常テスト | 16 passed、1 ignored |
| rustdoc | compile-fail 1 passed |
| Kani | 2 harnesses verified、0 failures |
| Fuzzing | 61秒、61,736,411 runs、crashなし |
| 性能テスト | 10万件が8.985084ms、89ns/transfer |

数値はこの実行環境の結果であり、別環境での再現値を保証しない。
