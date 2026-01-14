# Ory Stack Blog Series

Ory Stack（Hydra, Kratos, Keto）を使った認証・認可システム構築のブログシリーズ。

## 記事一覧

| # | ファイル | 内容 |
|---|----------|------|
| 01 | [blog-01-introduction.md](./blog-01-introduction.md) | Ory Hydra入門 - OAuth2/OIDCの基礎と「自前で作ることの非合理性」 |
| 02 | [blog-02-implementation.md](./blog-02-implementation.md) | Login/Consent Providerの実装 - RustでHydraと連携 |
| 03-01 | [blog-03-01.md](./blog-03-01.md) | E2Eテストとバグ発見 - Playwright MCPによる自動テスト |
| 03-02 | [blog-03-02.md](./blog-03-02.md) | RBACとOWASP Top 10 - セキュリティ検証 |
| 04 | [blog-04-kratos.md](./blog-04-kratos.md) | Ory Kratosで認証を委譲 - 自前実装からの卒業 |
| 05 | [blog-05-keto.md](./blog-05-keto.md) | Ory Ketoで認可を実装 - Zanzibarモデル入門 |

## シリーズの流れ

```
認可サーバー（Hydra）
    ↓
認証の自前実装（Rust）
    ↓
E2Eテスト・セキュリティ検証
    ↓
認証の委譲（Kratos）
    ↓
認可の委譲（Keto）
```

## 関連リポジトリ

- [ory-hydra-rust](../ory-hydra-rust/) - Hydra + Rust Login Provider
- [ory-hydra-verification](../ory-hydra-verification/) - Hydra検証環境
- [ory-kratos-verification](../ory-kratos-verification/) - Kratos検証環境
- [ory-keto-verification](../ory-keto-verification/) - Keto検証環境

## 技術スタック

- **Ory Hydra** - OAuth2/OIDC認可サーバー
- **Ory Kratos** - ヘッドレスID管理システム
- **Ory Keto** - Zanzibarモデル認可サーバー
- **Rust** - Login/Consent Provider実装
- **PostgreSQL** - データストア
- **Docker Compose** - 開発環境

## 主なテーマ

- OAuth2認可コードフロー
- Login/Consent Provider実装パターン
- E2Eテストによるセキュリティ検証
- RBAC/ReBAC（Relation-Based Access Control）
- 「自前で作ることの非合理性」と外部サービス活用
