# tools/

ツール開発・ブログ記事の検証コード

## 用途

- ブログ記事用の検証コード
- CLIツール開発
- ライブラリ実装
- MCP（Model Context Protocol）関連

## プロジェクト命名規則

```
# ブログ記事の検証コード
2026-rust-xxxx/
xxxx-blog/

# ツール
tool-name/
```

## 新規プロジェクト作成

```bash
# Rustプロジェクト
cd tools/
cargo new project-name
cd project-name

# Node.jsプロジェクト
mkdir project-name && cd project-name
npm init -y
```

## .gitignore

このディレクトリ専用の.gitignoreで以下を除外：

- Rust: `target/`, `Cargo.lock`
- Node.js: `node_modules/`
- Python: `.venv/`, `__pycache__/`
- Terraform: `.terraform/`, `*.tfstate`
