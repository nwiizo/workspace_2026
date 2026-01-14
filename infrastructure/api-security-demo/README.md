# API セキュリティデモ

Rustで実装された**OWASP API Security Top 10 (2023)**の脆弱性デモンストレーション。

各デモは**脆弱な**エンドポイントと**安全な**エンドポイントの両方を提供し、以下を示します：
1. 安全でない実装に対して攻撃がどのように機能するか
2. 適切なセキュリティ制御がこれらの攻撃をどのようにブロックするか

## 必要条件

- Rust（edition 2024）
- curl
- jq

## クイックスタート

```bash
# すべてのセキュリティテストを実行
./scripts/test_all.sh

# または個別のデモを実行
cargo run --release --bin bola-demo
```

## OWASP API Security Top 10 (2023) 対応表

| API# | 脆弱性 | バイナリ | 状態 |
|------|--------|----------|------|
| API1 | Broken Object Level Authorization | `bola-demo` | ✅ |
| API2 | Broken Authentication | `broken-auth-demo` | ✅ |
| API3 | Broken Object Property Level Authorization | `mass-assignment-demo` | ✅ |
| API4 | Unrestricted Resource Consumption | `rate-limit-demo` | ✅ |
| API5 | Broken Function Level Authorization | `bfla-demo` | ✅ |
| API6 | Unrestricted Access to Sensitive Business Flows | `business-flow-demo` | ✅ |
| API7 | Server Side Request Forgery | `ssrf-demo` | ✅ |
| API8 | Security Misconfiguration | `misconfiguration-demo` | ✅ |
| API9 | Improper Inventory Management | `inventory-demo` | ✅ |
| API10 | Unsafe Consumption of APIs | `unsafe-consumption-demo` | ✅ |

## デモ一覧

### 認可の脆弱性 (API1, API3, API5)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **BOLA** | `bola-demo` | オブジェクトレベル認可の不備 - ユーザーAがユーザーBのリソースにアクセス |
| **BFLA** | `bfla-demo` | 機能レベル認可の不備 - 一般ユーザーが管理者機能にアクセス |
| **Mass Assignment** | `mass-assignment-demo` | 攻撃者が保護されたフィールド（例：支払いステータス）を操作 |

### 認証の脆弱性 (API2, API4)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **Broken Auth** | `broken-auth-demo` | 期限切れ/無効なJWTトークンを受け入れる |
| **Rate Limiting** | `rate-limit-demo` | アカウントロックアウトによるブルートフォース保護 |

### ビジネスロジックとフロー (API6)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **Business Flow** | `business-flow-demo` | クーポン重複利用、チケット買い占め、自己紹介報酬の不正取得 |

### SSRF (API7)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **SSRF** | `ssrf-demo` | サーバーサイドリクエストフォージェリ - 内部リソースへのアクセス |

### 設定とインベントリ (API8, API9)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **Misconfiguration** | `misconfiguration-demo` | デバッグ情報露出、詳細エラー、過剰なCORS設定 |
| **Inventory** | `inventory-demo` | 廃止APIの残存、未ドキュメントAPI、バージョン不整合 |

### 外部API連携 (API10)

| デモ | バイナリ | 説明 |
|------|----------|------|
| **Unsafe Consumption** | `unsafe-consumption-demo` | Webhook署名未検証、外部データの信頼 |

### セキュリティインフラストラクチャ

| デモ | バイナリ | 説明 |
|------|----------|------|
| **JWT** | `jwt-demo` | HS256/RS256トークンの生成と検証 |
| **Observability** | `observability-demo` | セキュリティイベント監視（SQLi検出、認証失敗） |
| **Security Test** | `security-test-demo` | データ露出テスト、入力バリデーション |

## 例：BOLA攻撃

```bash
# デモサーバーを起動
cargo run --release --bin bola-demo

# Bobのトークンを取得
BOB_TOKEN=$(curl -s http://localhost:8080/token/bob | jq -r .access_token)

# 脆弱：BobがAliceの注文にアクセス（成功 - これが脆弱性）
curl -H "Authorization: Bearer $BOB_TOKEN" \
     http://localhost:8080/vulnerable/orders/1
# Aliceの注文データが返される

# 安全：BobがAliceの注文にアクセスしようとする（ブロック）
curl -H "Authorization: Bearer $BOB_TOKEN" \
     http://localhost:8080/orders/1
# 404を返す - このユーザーの注文が見つからない
```

## 例：Mass Assignment攻撃

```bash
# デモサーバーを起動
cargo run --release --bin mass-assignment-demo

# Aliceのトークンを取得
ALICE_TOKEN=$(curl -s http://localhost:8080/token/alice | jq -r .access_token)

# 脆弱：status=approvedで支払いを作成（攻撃成功）
curl -X POST http://localhost:8080/vulnerable/payments \
     -H "Authorization: Bearer $ALICE_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"amount": 100, "currency": "USD", "status": "approved"}'
# 返却: {"status": "approved", ...}

# 安全：statusフィールドは無視される
curl -X POST http://localhost:8080/payments \
     -H "Authorization: Bearer $ALICE_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"amount": 100, "currency": "USD", "status": "approved"}'
# 返却: {"status": "pending", ...}
```

## 例：SSRF攻撃

```bash
# デモサーバーを起動
cargo run --release --bin ssrf-demo

# 脆弱：SSRFを介して内部シークレットにアクセス
curl -X POST http://localhost:8080/vulnerable/fetch \
     -H "Content-Type: application/json" \
     -d '{"url":"http://localhost:8080/internal/secrets"}'
# 内部シークレットが返される

# 安全：HTTP URLはブロック、許可リストからのHTTPSのみ許可
curl -X POST http://localhost:8080/fetch \
     -H "Content-Type: application/json" \
     -d '{"url":"http://localhost:8080/internal/secrets"}'
# 返却: {"error": "Only HTTPS URLs are allowed"}
```

## テスト結果

`./scripts/test_all.sh`を実行すると20のセキュリティテストが実行されます：

```
==========================================
Test Results Summary
==========================================
PASS: 20
FAIL: 0

All security tests passed!
```

## プロジェクト構造

```
api-security-demo/
├── Cargo.toml
├── README.md
├── scripts/
│   └── test_all.sh              # 包括的なテストスクリプト
└── src/
    ├── lib.rs                   # 共有ライブラリ
    ├── auth.rs                  # JWT認証
    ├── db.rs                    # データベース操作
    ├── error.rs                 # エラーハンドリング
    ├── models.rs                # データモデル
    └── bin/
        ├── bola.rs              # API1: BOLAデモ
        ├── broken_auth.rs       # API2: 認証不備デモ
        ├── mass_assignment.rs   # API3: Mass Assignmentデモ
        ├── rate_limit.rs        # API4: レート制限デモ
        ├── bfla.rs              # API5: BFLAデモ
        ├── business_flow.rs     # API6: ビジネスフロー悪用デモ
        ├── ssrf.rs              # API7: SSRFデモ
        ├── misconfiguration.rs  # API8: 設定ミスデモ
        ├── inventory.rs         # API9: インベントリ管理デモ
        ├── unsafe_consumption.rs # API10: 外部API利用デモ
        ├── jwt.rs               # JWT処理デモ
        ├── observability.rs     # セキュリティ監視デモ
        └── security_test.rs     # セキュリティテストデモ
```

## 実装されているセキュリティ制御

### API1: BOLA対策
- アクセス前にリソース所有権を検証
- JWTクレームからユーザーコンテキストを使用
- 情報漏洩を避けるため403ではなく404を返す

### API2: 認証対策
- 適切な有効期限検証
- アルゴリズム制限
- 発行者/オーディエンス検証

### API3: Mass Assignment対策
- 入力用と内部データ用でDTOを分離
- 許可フィールドのホワイトリスト
- 機密フィールドのサーバーサイド初期化

### API4: レート制限
- 失敗試行後のアカウントロックアウト
- IP/ユーザーごとのリクエストレート制限
- 段階的なバックオフ

### API5: BFLA対策
- ロールベースアクセス制御（RBAC）
- 機能レベルの認可ミドルウェア
- 明示的な権限チェック

### API6: ビジネスフロー保護
- クーポンの重複利用防止
- ユーザーごとの購入上限
- 自己紹介報酬の防止

### API7: SSRF対策
- URL許可リストの検証
- プロトコル制限（HTTPSのみ）
- 内部/プライベートIP範囲のブロック

### API8: セキュリティ設定
- 本番環境でのデバッグ情報無効化
- 汎用エラーメッセージ（詳細はログのみ）
- 適切なCORS設定

### API9: インベントリ管理
- APIバージョニング戦略
- 廃止APIの段階的無効化
- 最新ドキュメントの維持

### API10: 外部API連携
- Webhook署名検証（HMAC-SHA256）
- 外部データのスキーマ検証
- リプレイ攻撃対策

## ライセンス

MIT
