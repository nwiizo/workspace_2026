# CLAUDE.md

## Project Overview

Personal workspace for 2026. Systems programming, distributed systems, and AI infrastructure.

## Directory Structure

- `blogs/` - Blog articles (has its own CLAUDE.md)
- `tools/` - Tools and verification code
- `infrastructure/` - Infrastructure experiments
- `samples/` - Samples and experiments

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
