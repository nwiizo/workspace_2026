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

---

## Security Principles (CTF からの学び)

### 入力は全て信頼しない

```
❌ クライアント側の検証のみ
✅ サーバー側でも必ず検証（型、長さ、形式、範囲）
```

- フロントエンドの `required` や `maxlength` は簡単にバイパスされる
- API は直接叩かれる前提で設計する

### 認証 ≠ 認可

```
認証 (Authentication): 「誰か」を確認
認可 (Authorization): 「何ができるか」を確認
```

- ログイン済みでも他人のリソースにアクセスできてはいけない
- `/api/users/123` → 必ず「このユーザーがID 123にアクセスできるか」を検証

### エラーメッセージは攻撃者への情報

```
❌ "ユーザー admin@example.com は存在しません"
❌ "パスワードが間違っています"
✅ "メールアドレスまたはパスワードが正しくありません"
```

- スタックトレースを本番環境で表示しない
- 内部構造（DB名、テーブル名）を漏らさない

### パラメータ化クエリを使う

```sql
❌ "SELECT * FROM users WHERE id = " + userId
✅ "SELECT * FROM users WHERE id = ?" with params [userId]
```

- 文字列結合でSQLを組み立てない
- ORMを使う場合も生SQLには注意

### 出力もエスケープする

```
❌ innerHTML = userInput
✅ textContent = userInput または適切なエスケープ
```

- HTMLコンテキスト、JavaScript、URL、SQLで異なるエスケープが必要
- テンプレートエンジンの自動エスケープを活用

### 最小権限の原則

- DBユーザーに `DROP` 権限を与えない
- APIトークンは必要なスコープのみ
- ファイルアクセスは必要なディレクトリのみ

### 依存関係の脆弱性

```sh
# 定期的に実行
npm audit / cargo audit / pip-audit / govulncheck
```

- 古いライブラリは攻撃対象になる
- Dependabot / Renovate で自動更新

### セキュリティヘッダー

```
Content-Security-Policy: default-src 'self'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Strict-Transport-Security: max-age=31536000
```

### チェックリスト

- [ ] 入力検証はサーバー側で行っているか
- [ ] 認可チェックは全エンドポイントにあるか
- [ ] SQLはパラメータ化されているか
- [ ] ユーザー入力の出力時にエスケープしているか
- [ ] エラーメッセージは情報を漏らしていないか
- [ ] 依存関係に既知の脆弱性はないか
- [ ] シークレットは環境変数で管理しているか
