# Idiomatic Rust を Rust 2024 edition で書き直す

Manning 刊 *Idiomatic Rust* (Brenden Matthews) のパターン群を、Rust 2024 edition (1.85, 2025-02 stable) のベストプラクティスで再構成した検証プロジェクトです。
書籍のコード自体は MIT で公開されています ([brndnmtthws/rust-advanced-techniques-book](https://github.com/brndnmtthws/rust-advanced-techniques-book)) が、本プロジェクトのコードは書籍のパターンに依拠しつつ独自に書き起こしたものです。

`samples/async-rust-2024/` と同じ方針です: 章ごとに代表的な idiom を取り、edition で書き味が変わるところと変わらないところを実コードで示します。

## カバレッジ

| ファイル | 章 / 節 | 取り上げた idiom |
|----------|---------|------------------|
| `01_prelude_2024.rs` | 1 | `Future` / `IntoFuture` が prelude に追加 |
| `02_generic_bounds.rs` | 2.1 / 2.2 | generics + trait bound + `impl Trait` 引数/戻り値 |
| `03_let_else_if_let_chain.rs` | 3.1 | `let-else` (1.65) と `if let` chain (1.88, 2024 で実用) |
| `04_functional_iter.rs` | 3.2 | `try_fold` / `itertools::tuple_windows` |
| `05_raii_drop_guard.rs` | 4.1 | RAII guard と tail expression drop order |
| `06_thiserror_errors.rs` | 4.5 | `thiserror` + `#[from]` + `#[non_exhaustive]` |
| `07_oncelock_global.rs` | 4.6 | `LazyLock` / `OnceLock` (`lazy_static` 不要) |
| `08_typestate_builder.rs` | 5.3 / 7.6 / 8.1 | type-state builder + `PhantomData` + `#[must_use]` |
| `09_newtype.rs` | 5.7 | newtype + `From` / `Display` |
| `10_non_exhaustive.rs` | 6.8 | `#[non_exhaustive]` で SemVer 互換を残す |
| `11_const_generics.rs` | 7.1 | `Vector<const N: usize>` + `NonZero<T>` |
| `12_extension_trait.rs` | 7.3 / 7.4 | extension trait + sealed trait pattern |
| `13_marker_phantom.rs` | 7.5 / 7.6 | `PhantomData` で通貨を型で区別 |
| `14_macro_rules.rs` | 8.3 | declarative macro と `expr` matcher の 2024 拡張 |
| `15_cow_immutable.rs` | 9.7 | `Cow<'_, T>` で「変更時のみ allocate」 |
| `16_avoid_unwrap.rs` | 10.3 | `unwrap()` antipattern の 2024 流回避 |

## 適用した 2024 ベストプラクティス

`async-rust-2024/` と共通のものは省略し、idiomatic 寄りで効くものに絞って書きます。

### Cargo.toml

- `edition = "2024"` + `rust-version = "1.85"` (resolver = "3" を暗黙に有効化)
- `[lints.clippy]` で `pedantic` + `nursery` を warn 既定
- 教科書サンプル特有の許容: `missing_errors_doc` / `missing_panics_doc` / `must_use_candidate` / `module_name_repetitions`
- idiomatic 寄りで効かせるルール:
  - `unwrap_used = deny` (10章 antipattern を強制反映)
  - `manual_let_else = warn` (3章 `let-else` を促す)
  - `match_wildcard_for_single_variants = warn` / `single_match_else = warn`
  - `explicit_iter_loop = warn` / `items_after_statements = warn`
  - `return_self_not_must_use = warn` (builder で重要)

### コード側

- `lazy_static!` を `LazyLock` / `OnceLock` に置き換え
- `Box<dyn Error>` を `thiserror` 由来の型付きエラーに置き換え
- 公開 enum / struct には `#[non_exhaustive]`
- builder は type-state で「未設定で `build()` を呼べない」をコンパイル時保証
- 数値の「ゼロでない」を `NonZero<T>` で表現 (2024 で書き味が良くなった)
- declarative macro の `:expr` は 2024 では拡張された新仕様。`cargo fix --edition` で
  必要に応じて `:expr_2021` に書き換わる

## 動かし方

```sh
cargo build --examples
cargo clippy --examples --all-targets
cargo run --example 03_let_else_if_let_chain
cargo run --example 08_typestate_builder
# その他の番号も同様
```

検証環境: rustc 1.95.0 (2026-04-14), edition 2024, thiserror 2, itertools 0.14。

## TODO

書籍に近づけて拡張する候補。

- [ ] **ch5.1 procedural macros**: 別 crate に proc-macro を分離した最小例
- [ ] **ch6.11 library ergonomics**: `From` blanket impl との衝突を `#[diagnostic::do_not_recommend]` で抑える例 (2024 で安定)
- [ ] **ch7.3 extension trait**: orphan rule に引っかかる例と回避策
- [ ] **ch8.2 coroutines**: `async-rust-2024/07_gen_keyword.rs` と並置できるか検証
- [ ] **ch9.8 immutable data structures**: `im` / `rpds` クレートを使った永続データ構造
- [ ] **ch10.5 too many clones**: `Arc::clone(&x)` 明示と `Cow` の使い分けで具体改善
- [ ] **ch10.6 Deref polymorphism**: なぜ antipattern か、代替パターンを並置
- [ ] **ベンチ**: `Vec` vs `im::Vector` の構造共有コスト
- [ ] **fuzz**: parser に proptest / arbitrary を入れる
- [ ] CI に `cargo deny` / `cargo audit`

## 参考

- Rust Edition Guide 2024: <https://doc.rust-lang.org/edition-guide/rust-2024/>
- Rust 1.85 release notes: <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/>
- 書籍コード (MIT): <https://github.com/brndnmtthws/rust-advanced-techniques-book>
