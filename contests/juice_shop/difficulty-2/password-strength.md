# Password Strength ✅

**難易度:** ⭐⭐
**カテゴリ:** 認証
**目標:** 管理者のパスワードを推測してログインする

---

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email: `admin@juice-sh.op`
3. Password: `admin123`
4. ログインできれば成功

## 解説

- 管理者が非常に弱いパスワードを使用している
- `admin123` は最もよく使われるパスワードの一つ
- パスワード強度ポリシーの欠如

**弱いパスワードの例:**
- `admin`, `admin123`, `password`, `123456`
- ユーザー名と同じ
- 辞書に載っている単語
- 短すぎるパスワード

**対策:**
- 最小長（12文字以上）の強制
- 大文字・小文字・数字・記号の組み合わせ要求
- 既知の流出パスワードとの照合
- 多要素認証（MFA）の導入

## 関連チャレンジ

- [Login Admin](login-admin.md)
- [Login MC SafeSearch](login-mc-safesearch.md)
