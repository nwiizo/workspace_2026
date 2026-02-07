# Security Principles & 攻撃手法カタログ (CTF からの学び)

## Security Principles

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

## 攻撃手法カタログ (Juice Shop CTF)

### エンコーディング攻撃

| 手法 | ペイロード例 | 対策 |
|------|-------------|------|
| Double URL encode | `%252F` (/) | 複数回デコードしない |
| Null byte injection | `%00`, `%2500` | 拡張子検証前にnull除去 |
| Z85 encoding | クーポンコード偽造 | サーバー側で生成・検証 |
| Unicode normalization | `＜script＞` | 正規化後に検証 |

### Allowlist バイパス

```javascript
// 脆弱なコード
if (url.includes("github.com")) { redirect(url); }

// 攻撃ペイロード
https://evil.com?pwned=github.com  // includes() を騙す
https://github.com@evil.com        // credential injection
```

**対策**: `URL` オブジェクトでパース、`hostname` を厳密に比較

### JWT 攻撃

| 攻撃 | 手法 | 対策 |
|------|------|------|
| alg: none | 署名検証バイパス | アルゴリズム固定 |
| 弱い秘密鍵 | ブルートフォース | 強力な秘密鍵 (256bit+) |
| kid injection | ファイルパス操作 | kid をホワイトリスト |

### SSRF バイパス

```
127.0.0.1 の別表現:
- 2130706433 (decimal)
- 0x7f000001 (hex)
- 0177.0.0.1 (octal)
- localhost, [::1], 127.1

DNS rebinding:
- localtest.me → 127.0.0.1
- 127.0.0.1.nip.io → 127.0.0.1
```

**対策**: IP アドレスを正規化してからブラックリスト比較

### ファイルアップロード攻撃

| 攻撃 | 手法 | 対策 |
|------|------|------|
| 拡張子バイパス | `shell.php.jpg`, `shell.php%00.jpg` | Magic bytes 検証 |
| Zip Slip | `../../../tmp/evil.txt` | パス正規化、ディレクトリ制限 |
| VTT XSS | 字幕ファイルに `<script>` | HTMLエスケープ |
| XXE | DTD外部エンティティ | XML外部エンティティ無効化 |

### NoSQL Injection

```javascript
// MongoDB
{ "email": {"$ne": ""}, "password": {"$ne": ""} }

// 対策
- 入力型を検証 (文字列のみ許可)
- $where, $regex 等の演算子をフィルタ
```

### Prototype Pollution

```javascript
// 攻撃
{"__proto__": {"isAdmin": true}}

// 対策
- Object.create(null) を使用
- 入力キーをホワイトリスト
```

### rectitude ライブラリの活用

```rust
use rectitude::payloads::{sqli, xss, ssrf, traversal, redirect};
use rectitude::helpers::{coupon_helpers, osint_helpers};

// SQLi ペイロード
for payload in sqli::auth_bypass_payloads() { ... }

// SSRF IP バイパス
let variants = ssrf::ip_bypass_variants(127, 0, 0, 1);

// Allowlist バイパス
let bypass = redirect::allowlist_bypass("https://evil.com", "github.com");

// クーポン偽造
let coupon = coupon_helpers::generate_z85_coupon("JAN", 26, 90);
```
