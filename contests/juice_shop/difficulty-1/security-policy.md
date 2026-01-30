# Security Policy ✅

**難易度:** ⭐
**カテゴリ:** 情報
**目標:** セキュリティポリシーファイルを見つける

---

## 実行手順

1. ブラウザのアドレスバーに以下を入力:
   ```
   http://localhost:3000/.well-known/security.txt
   ```
2. セキュリティ連絡先情報が表示されれば成功

## 解説

- `security.txt` は脆弱性を発見した人が連絡先を見つけるための標準的なファイル
- 多くの企業がこのファイルを公開している
- RFC 9116 で標準化されている

**security.txt に含まれる情報:**
- 脆弱性報告先のメールアドレス
- 報告ポリシーへのリンク
- 暗号化用の公開鍵
- 有効期限

## 関連チャレンジ

- [Security Advisory](../difficulty-3/security-advisory.md)
