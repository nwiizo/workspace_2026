# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OWASP API Security Top 10の脆弱性デモンストレーション。各デモは**脆弱な**エンドポイントと**安全な**エンドポイントの両方を提供し、攻撃手法とその対策を示す。

## Build & Run Commands

```bash
# Build all demos
cargo build --release

# Run specific demo (e.g., BOLA)
cargo run --release --bin bola-demo

# Run all security tests
./scripts/test_all.sh

# Run unit tests
cargo test

# Run single test
cargo test test_order_authorization

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Architecture

### Demo Pattern

各デモは独立したバイナリで、同一パターンに従う：

1. **トークン生成エンドポイント**: `/token/{user_id}` - テスト用JWTを発行
2. **脆弱なエンドポイント**: `/vulnerable/...` - 攻撃が成功するバージョン
3. **安全なエンドポイント**: `/...` - 適切なセキュリティ制御を実装したバージョン

```
src/bin/
├── bola.rs              # BOLA: 他ユーザーのリソースにアクセス
├── bfla.rs              # BFLA: 一般ユーザーが管理者機能を実行
├── mass_assignment.rs   # 保護フィールドの不正操作
├── broken_auth.rs       # 期限切れ/無効なJWTを受け入れ
├── rate_limit.rs        # ブルートフォース保護
├── ssrf.rs              # 内部リソースへのSSRF攻撃
├── jwt.rs               # HS256/RS256トークン処理
├── observability.rs     # セキュリティイベント監視
└── security_test.rs     # データ露出テスト
```

### Shared Library (`src/lib.rs`)

```
src/
├── auth.rs    # JWT生成/検証、AuthenticatedUser/VulnerableAuthUser extractors
├── db.rs      # SQLite操作、get_order_by_id (脆弱) vs get_order_by_id_for_user (安全)
├── error.rs   # AppError enum、axum IntoResponse実装
└── models.rs  # データモデル、CreatePaymentRequest (安全) vs UnsafePaymentRequest (脆弱)
```

### Key Security Patterns

**認可チェック**: `AuthenticatedUser` extractor（安全）と `VulnerableAuthUser` extractor（脆弱）を使い分け

**Mass Assignment対策**: 入力DTOと内部DTOを分離（`CreatePaymentRequest` vs `UnsafePaymentRequest`）

**BOLA対策**: `db.get_order_by_id()` (脆弱) vs `db.get_order_by_id_for_user()` (安全)

## Testing with curl

```bash
# Get token
TOKEN=$(curl -s http://localhost:8080/token/bob | jq -r .access_token)

# Test vulnerable endpoint
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/vulnerable/orders/1

# Test secure endpoint
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/orders/1
```

## Dependencies

- **axum 0.8**: Webフレームワーク
- **jsonwebtoken 9**: JWT処理
- **rusqlite**: インメモリSQLite
- **governor**: レート制限
