# Security Advisory ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 情報漏洩
**目標:** CSAFプロバイダーメタデータを発見する

---

## 実行手順

ブラウザで以下にアクセス:
```
http://localhost:3000/.well-known/csaf/provider-metadata.json
```

## 解説

**CSAF（Common Security Advisory Framework）とは？**
- 脆弱性情報を公開するための標準フォーマット
- OASIS（オープン標準化団体）が策定
- セキュリティアドバイザリの自動化処理を可能にする

**provider-metadata.json の内容:**
- セキュリティアドバイザリの公開元情報
- 連絡先
- 公開ポリシー

**教訓:**
- 標準的なパス（`.well-known/`）に機密情報がある場合がある
- サイトの構成を調査する際はこれらのパスをチェック

## 関連チャレンジ

- [Security Policy](../difficulty-1/security-policy.md)
- [Confidential Document](../difficulty-1/confidential-document.md)
