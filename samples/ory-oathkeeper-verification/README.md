# Ory Oathkeeper Verification

Ory Oathkeeper と Keto を組み合わせた Zero Trust API Gateway パターンの検証環境。

## 概要

このプロジェクトは以下を検証します：

- Oathkeeper によるリクエストプロキシと認証・認可の実装
- Keto (Zanzibar モデル) との連携による細粒度アクセス制御
- Access Rules による柔軟なルーティング設定

## アーキテクチャ

```
┌─────────┐     ┌─────────────┐     ┌─────────┐
│ Client  │────▶│ Oathkeeper  │────▶│ Backend │
└─────────┘     │   (4455)    │     │ (nginx) │
                └──────┬──────┘     └─────────┘
                       │
                       ▼
                ┌─────────────┐
                │    Keto     │
                │   (4466)    │
                └─────────────┘
```

## クイックスタート

```sh
# サービス起動
docker compose up -d

# E2Eテスト実行
bash scripts/e2e-test.sh

# サービス停止
docker compose down
```

## Access Rules

| エンドポイント | 認証 | 認可 |
|---------------|------|------|
| `/health` | anonymous | allow |
| `/api/public` | anonymous | allow |
| `/api/protected` | noop | allow (ヘッダー転送) |
| `/api/documents/{id}` GET | noop | Keto viewer チェック |
| `/api/documents/{id}` PUT/DELETE | noop | Keto editor チェック |

## テストユーザー

| ユーザー | doc1 の権限 |
|---------|------------|
| alice | editor + viewer |
| bob | viewer のみ |
| charlie | 権限なし |

## ディレクトリ構造

```
.
├── docker-compose.yml      # サービス定義
├── oathkeeper/
│   ├── oathkeeper.yml      # Oathkeeper 設定
│   └── rules.yml           # Access Rules
├── keto/
│   └── keto.yml            # Keto 設定
├── backend/
│   ├── nginx.conf          # バックエンド設定
│   └── api.json            # ヘルスチェック用
└── scripts/
    └── e2e-test.sh         # E2Eテストスクリプト
```

## テスト結果

```
=== Verifying Keto permissions ===
PASS: Keto: alice is editor of doc1
PASS: Keto: bob is viewer of doc1
PASS: Keto: bob is NOT editor of doc1
PASS: Keto: charlie is NOT viewer of doc1

=== Testing Oathkeeper - Document View (GET) ===
PASS: Document: alice can GET doc1
PASS: Document: bob can GET doc1
PASS: Document: charlie cannot GET doc1
PASS: Document: anonymous cannot GET doc1

=== Testing Oathkeeper - Document Edit (PUT) ===
PASS: Document: alice can PUT doc1
PASS: Document: bob cannot PUT doc1
PASS: Document: charlie cannot PUT doc1

=== Summary ===
Passed: 17
Failed: 0
All tests passed!
```

## 参考

- [Ory Oathkeeper Documentation](https://www.ory.sh/docs/oathkeeper)
- [Ory Keto Documentation](https://www.ory.sh/docs/keto)
- [BeyondCorp: A New Approach to Enterprise Security](https://research.google/pubs/pub43231/)
