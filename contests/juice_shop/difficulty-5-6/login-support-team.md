# Login Support Team ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** 設定ミス
**目標:** サポートチームアカウントにログイン

## ヒント

- **複雑な調査が必要**
- **可能性:**
  - OAuth 設定の脆弱性
  - 設定ファイルからの情報漏洩
  - SQLi で認証情報を取得

## 調査ポイント

1. **SQLi でユーザー一覧取得**
```sql
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
```

2. **support チームのメールアドレスを探す**
```
support@juice-sh.op
support-team@juice-sh.op
```

3. **パスワードハッシュをクラック**
- MD5 ハッシュの場合は crackstation.net などで検索

## OAuth 調査

```
/api/Users でOAuth登録ユーザーを確認
Google/GitHub OAuth の設定を調査
```

## 設定ファイル

```
/.well-known/
/config/
/api-docs/
```

## 検証ポイント

- [ ] サポートチームのアカウント情報を特定
- [ ] パスワードを取得/クラック
- [ ] ログイン成功

## 解説

[未着手]
