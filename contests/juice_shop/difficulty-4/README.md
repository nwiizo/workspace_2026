# 難易度4 チャレンジ (13/37 解決)

上級レベル: UNION SQLi、NoSQLi、Poison Null Byte など高度な攻撃手法を学びます。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 13 |
| ❌ 未解決 | 24 |

## チャレンジ一覧

| チャレンジ | カテゴリ | 状態 | ファイル |
|-----------|---------|------|----------|
| Database Schema | SQLi | ✅ | [database-schema.md](database-schema.md) |
| User Credentials | SQLi | ✅ | [user-credentials.md](user-credentials.md) |
| Christmas Special | SQLi+IDOR | ✅ | [christmas-special.md](christmas-special.md) |
| Poison Null Byte | バイパス | ✅ | [poison-null-byte.md](poison-null-byte.md) |
| Forgotten Developer Backup | 情報漏洩 | ✅ | [forgotten-developer-backup.md](forgotten-developer-backup.md) |
| Easter Egg | その他 | ✅ | [easter-egg.md](easter-egg.md) |
| HTTP-Header XSS | XSS | ✅ | [http-header-xss.md](http-header-xss.md) |
| NoSQL Manipulation | NoSQLi | ✅ | [nosql-manipulation.md](nosql-manipulation.md) |
| Access Log | 情報漏洩 | ✅ | [access-log.md](access-log.md) |
| Login Bjoern | 認証 | ✅ | [login-bjoern.md](login-bjoern.md) |
| Reset Bender's Password | OSINT | ✅ | [reset-benders-password.md](reset-benders-password.md) |
| Allowlist Bypass | バイパス | ✅ | [allowlist-bypass.md](allowlist-bypass.md) |

## Poison Null Byte リファレンス

```
%2500 = %00 のURLエンコード
/ftp/file.bak%2500.md → file.bak をダウンロード
```

## 主要ユーザーの認証情報

| ユーザー | MD5ハッシュ | パスワード | 元ネタ |
|---------|------------|-----------|--------|
| admin@juice-sh.op | 0192023a7bbd73250516f069df18b500 | admin123 | よくあるパスワード |
| jim@juice-sh.op | e541ca7ecf72b8d1286474fc613e5e45 | ncc-1701 | スタートレック |
| bender@juice-sh.op | 0c36e517e3fa95aabf1bbffc6744a4ef | OhG0dPlease1LubYou | Futurama |
