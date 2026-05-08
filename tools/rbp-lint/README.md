# rbp-lint

[`rust-best-practices`](https://github.com/nwiizo/rust-best-practices) のルールを
[`rowan`](https://github.com/rust-analyzer/rowan) ベースのロスレス構文木で検査する
シンプルな Rust リンター。

`rowan` 上に rust-analyzer が構築している `ra_ap_syntax` を使って Rust ソースを
パースし、各 lint が AST を走査して違反を報告する。

## ビルド

```sh
cd tools/rbp-lint
cargo build --release
```

## 使い方

```sh
# 単一ファイル
./target/release/rbp-lint path/to/file.rs

# ディレクトリ再帰
./target/release/rbp-lint src/

# JSON 出力（CI 連携用）
./target/release/rbp-lint --format json src/

# 警告もエラー扱いにして exit 1
./target/release/rbp-lint --deny-warnings src/
```

## 実装済み Lint

| ID | Severity | 元ルール | 内容 |
|----|----------|----------|------|
| `no-unwrap` | error | security.md NEVER | プロダクションコードでの `.unwrap()` を禁止（test 配下は除外） |
| `no-expect` | warning | security.md チェックリスト | プロダクションでの `.expect()` を警告。空メッセージは常に警告 |
| `no-panic` | warning | security.md | `panic!` / `unimplemented!` / `todo!` / `unreachable!` を警告 |
| `dead-code-comment` | warning | code-quality.md | `#[allow(dead_code)]` 直前にコメントが無い場合に警告 |
| `tracing-format` | note | code-quality.md | `tracing::info!("msg {}", x)` 形式を構造化フィールドに誘導 |
| `arc-clone-explicit` | note | code-quality.md | `let x = Arc::new(..)` の後の `x.clone()` を `Arc::clone(&x)` に誘導 |
| `hardcoded-secret` | error | security.md NEVER | `sk-...`, `ghp_...`, `AKIA...` 等のリテラルを検出 |
| `unsafe-safety-comment` | warning | security.md | `unsafe { .. }` 直前に `// SAFETY:` コメントが無い場合に警告 |
| `debug-print` | warning | code-quality.md | `println!`/`eprintln!`/`dbg!`/`print!`/`eprint!` をプロダクションで警告 |
| `string-as-error` | warning | error-handling.md | `Err("...")` / `Err(format!(..))` 等を typed error に誘導 |
| `unbounded-channel` | warning | async-patterns.md | `mpsc::unbounded_channel()` を bounded channel に誘導 |
| `unwrap-or-default-call` | note | code-quality.md | `.unwrap_or(Default::default())` → `.unwrap_or_default()` |
| `mod-rs-file` | warning | project-structure.md | Edition 2018+ で `mod.rs` を `foo.rs + foo/` に誘導 |
| `needless-return` | note | rust-coding-style.md | 関数末尾の `return expr;` を tail expression に誘導 |
| `lazy-static-macro` | warning | idiomatic-rust-2024 | `lazy_static!` を `LazyLock` / `OnceLock` に誘導 |
| `manual-let-else` | note | idiomatic-rust-2024 | `if let X = e { .. } else { return .. }` を `let-else` に誘導 |
| `pub-field-newtype` | note | idiomatic-rust-2024 | `pub struct X(pub T);` の内部公開を警告 |
| `non-exhaustive-pub-error` | warning | idiomatic-rust-2024 | `pub enum *Error` で `#[non_exhaustive]` 欠如 |
| `raw-id-field` | note | rust-types-as-walls | `pub *_id: String/u64` を newtype に誘導 |
| `status-string-field` | note | rust-types-as-walls | `pub status: String` を `enum` に誘導 |
| `bool-option-pair` | warning | rust-types-as-walls | `is_paid: bool` + `payment_id: Option<_>` の illegal-state を検出 |

`test_in_context` ヒューリスティックは `#[test]` 関数 / `#[cfg(test)]`
モジュール / `mod tests`・`mod test` 配下を test とみなす。

## Lint の追加

1. `src/lints/<your_lint>.rs` を作成し、`LintRule` を実装
2. `src/lints/mod.rs` の `pub mod` 宣言と `all_lints()` に登録
3. `tests/fixtures/` に good/bad のフィクスチャを追加

## 設計メモ

- `ra_ap_syntax::SourceFile::parse(src, Edition::Edition2021)` でパース
- 失敗トレラント: 構文エラーがあっても部分木で lint は走る
- 各 lint は `LintContext` を受けて `Diagnostic` を vec に追加するだけの責務
- 型情報は使わない。`arc-clone-explicit` 等は構文ヒューリスティックのみで検出

## ライセンス

MIT
