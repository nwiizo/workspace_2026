# infrastructure/

インフラ検証・セキュリティデモ

## 用途

- セキュリティ検証（WAF, コンテナセキュリティ, API Security）
- データベース活用（PostgreSQL, TimescaleDB, pgvector）
- モニタリング・オブザーバビリティ
- Kubernetes / Istio 検証
- 低レベルシステムプログラミング

## プロジェクト例

```
infrastructure/
├── api-security-demo/      # OWASP API Security検証
├── container-security/     # コンテナセキュリティ
├── rust-postgres/          # PostgreSQL + Rust
├── waf-test/              # WAF検証
└── monitoring/            # モニタリング構成
```

## 新規プロジェクト作成

```bash
cd infrastructure/
mkdir project-name
cd project-name

# Docker Compose環境
touch docker-compose.yml

# Rustプロジェクト
cargo init
```
