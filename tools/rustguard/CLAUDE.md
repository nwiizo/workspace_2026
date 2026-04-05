# CLAUDE.md - RustGuard

## 概要

MIRレベル静的解析ツール。`rustc_driver` Callbacksでコンパイラに介入し、unsafe影響範囲マッピング・所有権パターン異常検出を行う。

## コマンド

```bash
# ビルド（nightly必須）
cargo build

# テスト
cargo test

# lint
cargo fmt && cargo clippy -- -D warnings

# 対象プロジェクトで実行
cargo rustguard
cargo rustguard --format json
cargo rustguard --config rustguard.toml
```

## アーキテクチャ

Dual-personality binary: `cargo rustguard` → `RUSTC_WORKSPACE_WRAPPER=self cargo check` → rustc_driver Callbacks

## 注意事項

- nightly Rust + `rustc-dev` component が必須
- `#![feature(rustc_private)]` で rustc 内部APIにリンク
- `after_analysis` callback で MIR にアクセス、`Compilation::Stop` で codegen をスキップ
