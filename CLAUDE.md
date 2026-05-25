# CLAUDE.md

## Project Overview

Personal workspace for 2026. Systems programming, distributed systems, and AI infrastructure.

## Directory Structure

- `blogs/` - Blog articles (has its own CLAUDE.md)
- `tools/` - Tools and verification code
- `infrastructure/` - Infrastructure experiments
- `samples/` - Samples and experiments
- `contests/` - Programming contests and security challenges (CTF, etc.)

## Code Style

### Rust

```sh
cargo fmt && cargo clippy -- -D warnings && cargo test
```

- No `.unwrap()` in production code
- Use `thiserror` for error types
- Prefer `Result<T, E>` over panics

### Go

```sh
go fmt ./... && golangci-lint run && go test -race ./...
```

### TypeScript

```sh
npx prettier --write . && npx eslint . --fix && npx tsc --noEmit
```

### Python

```sh
uv run --frozen ruff format . && uv run --frozen ruff check . && uv run --frozen pytest
```

## Git

- Feature branches for development
- Conventional commits: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Run linter and tests before commit

## Rules

- Write tests for new features
- Prefer editing existing files over creating new ones
- Keep documentation minimal and accurate
- No hardcoded secrets or API keys
- **Documentation files**: Only `CLAUDE.md` and `README.md` in each directory (no other docs)

## Engineering Discipline (pre-implementation gate)

曖昧/大きい/不確実なタスクには、コードを書く前に以下を実行:

1. **前提を surface**: 仮定した前提を 3〜5 個列挙。複数解釈があれば全部出す。不明なら止まって質問
2. **検証可能ゴールに変換**: "バグ修正" → "再現テスト → 通す" / "機能追加" → "受け入れテスト → 通す"
3. **複数ステップなら計画を 1 度だけ出す**: `[step] → verify: [check]` 形式

実装後、コミット前に必ず両方 yes:
- **Senior engineer test**: 過剰実装と言われないか
- **Traceability test**: diff の全行がユーザ要求に直接トレースできるか

詳細は skill `karpathy-guidelines`。自明なタスクには適用しない（caution > speed の tradeoff）。

## Security (on demand)

- [security-catalog](.claude/docs/security-catalog.md) - CTF学習メモ・攻撃手法カタログ・セキュリティチェックリスト
