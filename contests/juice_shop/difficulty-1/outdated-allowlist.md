# Outdated Allowlist ✅

**難易度:** ⭐
**カテゴリ:** リダイレクト
**目標:** 古いリダイレクト許可リストを悪用する

---

## 実行手順

1. ブラウザのアドレスバーに以下を入力:
   ```
   http://localhost:3000/redirect?to=https://blockchain.info/address/1AbKfgvw9psQ41NbLi8kufDQTezwG8DRZm
   ```
2. 外部サイト（blockchain.info）にリダイレクトされれば成功

## 解説

- リダイレクト機能は、信頼されたURLのみに制限するべき
- しかし、古いホワイトリスト（許可リスト）に blockchain.info が残っている
- これを悪用すると、ユーザーを任意のサイトに誘導できる

**オープンリダイレクトの危険性:**
- フィッシング攻撃に悪用される
- 信頼されたドメインからのリンクに見せかけて悪意あるサイトに誘導

## 関連チャレンジ

- [Allowlist Bypass](../difficulty-4/allowlist-bypass.md)
