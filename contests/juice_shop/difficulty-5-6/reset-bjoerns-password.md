# Reset Bjoern's Password ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 認証
**目標:** Bjoern (Gmail) のパスワードをリセット

## ヒント

- **ターゲット:** `bjoern.kimminich@gmail.com`
- **脆弱性:** OAuth 認証フローの問題
- **注意:** `bjoern@owasp.org` とは別アカウント

## OAuth フローの脆弱性

```
1. Google OAuth でログイン
2. トークン交換の脆弱性
3. アカウントハイジャック
```

## 調査ポイント

- `/api/Users` でユーザー情報を確認
- OAuth 関連のエンドポイント
- セキュリティ質問の設定状況

## SQLi で情報収集

```sql
-- Gmail ユーザーの情報
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users WHERE email LIKE '%gmail%'--
```

## 検証ポイント

- [ ] Gmail アカウントの存在を確認
- [ ] OAuth 設定を調査
- [ ] パスワードリセットを試行

## 解説

[未着手]
