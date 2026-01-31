# 難易度4 チャレンジ (20/25 解決)

上級レベル: UNION SQLi、NoSQLi、Poison Null Byte など高度な攻撃手法を学びます。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 20 |
| ❌ 未解決 | 5 |

## チャレンジ一覧

### 解決済み ✅

| チャレンジ | カテゴリ | ファイル |
|-----------|---------|----------|
| Database Schema | SQLi | [database-schema.md](database-schema.md) |
| User Credentials | SQLi | [user-credentials.md](user-credentials.md) |
| Christmas Special | SQLi+IDOR | [christmas-special.md](christmas-special.md) |
| Poison Null Byte | バイパス | [poison-null-byte.md](poison-null-byte.md) |
| Forgotten Developer Backup | 情報漏洩 | [forgotten-developer-backup.md](forgotten-developer-backup.md) |
| Forgotten Sales Backup | 情報漏洩 | - |
| Misplaced Signature File | 情報漏洩 | - |
| Easter Egg | Cryptographic | [easter-egg.md](easter-egg.md) |
| Nested Easter Egg | Cryptographic | [nested-easter-egg.md](nested-easter-egg.md) |
| Access Log | 情報漏洩 | [access-log.md](access-log.md) |
| Ephemeral Accountant | SQLi | - |
| Login Bjoern | 認証 | [login-bjoern.md](login-bjoern.md) |
| NoSQL Manipulation | NoSQLi | [nosql-manipulation.md](nosql-manipulation.md) |
| Reset Bender's Password | OSINT | [reset-benders-password.md](reset-benders-password.md) |
| Reset Uvogin's Password | OSINT | - |
| Vulnerable Library | Components | - |
| Legacy Typosquatting | Components | - |
| Allowlist Bypass | バイパス | [allowlist-bypass.md](allowlist-bypass.md) |
| Steganography | Security Obscurity | [steganography.md](steganography.md) |
| Leaked Unsafe Product | 情報漏洩 | [leaked-unsafe-product.md](leaked-unsafe-product.md) |
| Expired Coupon | Input Validation | [expired-coupon.md](expired-coupon.md) |

### 未解決 ❌

| チャレンジ | カテゴリ | ファイル |
|-----------|---------|----------|
| GDPR Data Theft | 情報漏洩 | - |
| HTTP-Header XSS | XSS | [http-header-xss.md](http-header-xss.md) |
| NoSQL DoS | NoSQLi | - |
| CSP Bypass | バイパス | - |
| Server-side XSS Protection | XSS | - |

## クイックリファレンス

### Poison Null Byte

```
%2500 = %00 のURLエンコード
/ftp/file.bak%2500.md → file.bak をダウンロード
```

### 主要ユーザーの認証情報

| ユーザー | MD5ハッシュ | パスワード | 元ネタ |
|---------|------------|-----------|--------|
| admin@juice-sh.op | 0192023a7bbd73250516f069df18b500 | admin123 | よくあるパスワード |
| jim@juice-sh.op | e541ca7ecf72b8d1286474fc613e5e45 | ncc-1701 | スタートレック |
| bender@juice-sh.op | 0c36e517e3fa95aabf1bbffc6744a4ef | OhG0dPlease1LubYou | Futurama |

### セキュリティ質問

| ユーザー | 質問 | 答え |
|---------|------|------|
| bender@juice-sh.op | 勤務先 | Stop'n'Drop |
| uvogin@juice-sh.op | 好きな映画 | Silence of the Lambs |

### NoSQL Injection ペイロード

```javascript
// 条件バイパス
{"$ne": -1}
{"$gt": ""}

// Deluxe Membership 購入
{"paymentMode": {"$ne": ""}}
```

### 重要な管理 API

```
/rest/admin/application-configuration  # 設定情報
/support/logs                          # アクセスログ
```
