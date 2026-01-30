# Exposed Credentials ✅

**難易度:** ⭐⭐
**カテゴリ:** 機密データ
**目標:** ソースコードからテスト認証情報を発見する

---

## 実行手順

1. DevTools（F12）を開く
2. 「Sources」タブで `main.js` を開く
3. `Ctrl+F` で「password」や「testing」を検索
4. テスト用認証情報を発見:
   ```javascript
   testingUsername="testing@juice-sh.op"
   testingPassword="IamUsedForTesting"
   ```
5. この認証情報でログインすると、チャレンジ解決

## 解説

**なぜこれが危険？**
- 本番コードにテスト用認証情報が残っている
- ソースコードは誰でも閲覧可能なため、攻撃者に悪用される
- ハードコードされた認証情報はセキュリティ上の大きなリスク

**よくある問題:**
- 開発用の認証情報が本番に残る
- APIキーがソースコードに直書き
- データベース接続情報が設定ファイルに平文で保存

**対策:**
- 環境変数を使用する
- シークレット管理ツール（AWS Secrets Manager, HashiCorp Vault など）
- コードレビューで認証情報の混入をチェック
- git-secrets などのツールでコミット前にスキャン

## 関連チャレンジ

- [Access Log](../difficulty-4/access-log.md)
- [Leaked API Key](../difficulty-5-6/leaked-api-key.md)
