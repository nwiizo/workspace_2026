# Forged Review ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** IDOR / パラメータ改ざん
**目標:** 他のユーザーとしてレビューを投稿する

---

## 背景知識

### IDOR（Insecure Direct Object Reference）とは

ユーザーが送信するデータ（ID、メールアドレス等）を**サーバーが検証せずに信頼**してしまう脆弱性。攻撃者は他人のIDやメールアドレスを指定することで、なりすましが可能になる。

```
┌─────────────────────────────────────────────────────────────────┐
│                     IDOR によるなりすまし                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【正常なフロー】                                                │
│  ユーザー: alice@example.com                                    │
│       │                                                         │
│       │ レビュー投稿: { author: "alice@example.com", ... }      │
│       ▼                                                         │
│  サーバー: 「Aliceさんからのレビューですね」→ 保存              │
│                                                                 │
│  【攻撃フロー】                                                  │
│  攻撃者: attacker@evil.com                                      │
│       │                                                         │
│       │ レビュー投稿: { author: "ceo@company.com", ... }        │
│       │               ↑ 勝手に他人のメールを指定                 │
│       ▼                                                         │
│  サーバー: 「CEOさんからのレビューですね」→ 保存 😱             │
│                                                                 │
│  【問題点】                                                     │
│  サーバーが「author の値は本当にこのユーザーか？」を確認していない│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### なぜ author を信頼してはいけないか

```
┌─────────────────────────────────────────────────────────────────┐
│                     信頼できるデータソース                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【信頼できない】ユーザーからの入力                              │
│  - リクエストボディ { author: "..." }                           │
│  - URLパラメータ ?author=...                                    │
│  - Cookie（改ざん可能）                                         │
│                                                                 │
│  【信頼できる】サーバー側で管理されたデータ                       │
│  - JWT トークンから取得したユーザーID                            │
│  - セッションに保存されたユーザー情報                            │
│  - 認証ミドルウェアが設定した req.user                          │
│                                                                 │
│  【原則】                                                       │
│  「誰が」の情報は、ユーザー入力ではなく認証情報から取得すべき     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

レストランで「〇〇様からのご注文です」と言って別人の名前で注文を入れることができてしまう状態。本来は「このテーブルの人が注文した」という事実を確認すべき。

---

## 思考プロセス

### ステップ1: レビュー投稿APIを調査

```
「商品ページでレビューを投稿してみる」
    ↓
「DevTools → Network で送信内容を確認」
    ↓
「PUT /rest/products/1/reviews に JSON が送られている」
    ↓
「{ message: "...", author: "自分のメールアドレス" }」
```

### ステップ2: author フィールドに着目

```
「author が リクエストボディに含まれている」
    ↓
「これを別の値に変えたらどうなる？」
    ↓
「jim@juice-sh.op に変更してみよう」
```

### ステップ3: なりすまし成功

```
「リクエストを送信」
    ↓
「レビューが jim@juice-sh.op として投稿された！」
    ↓
「サーバーは author を検証していない」
```

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
    message: "This product is amazing! - Definitely Jim",
    author: "jim@juice-sh.op"  // 他人のメールアドレス
  })
}).then(r => r.json()).then(console.log)
```

### 確認方法

1. 商品ページでレビュー一覧を確認
2. `jim@juice-sh.op` として投稿されたレビューが表示される

---

## Juice Shop の脆弱なコードパターン

### 脆弱なコード（推定）

```typescript
// ❌ 脆弱なコード
// routes/productReviews.ts
export function createReview() {
  return async (req: Request, res: Response) => {
    const productId = req.params.id
    const { message, author } = req.body  // ❌ author を信頼

    // 商品が存在するか確認
    const product = await ProductModel.findByPk(productId)
    if (!product) {
      return res.status(404).json({ error: 'Product not found' })
    }

    // ❌ author がリクエスト送信者のものか検証していない！
    await ReviewModel.create({
      productId,
      message,
      author  // 攻撃者が指定した任意のメールアドレス
    })

    res.json({ status: 'success' })
  }
}
```

### 問題点

1. **author をリクエストから取得**: ユーザー入力をそのまま使用
2. **認証情報との照合なし**: JWT のユーザーと author の一致を確認していない
3. **なりすまし可能**: 任意のメールアドレスでレビューを投稿できる

---

## 安全な実装

```typescript
// ✅ 安全なコード
// routes/productReviews.ts
export function createReview() {
  return async (req: Request, res: Response) => {
    const productId = req.params.id
    const { message } = req.body  // ✓ author はリクエストから取得しない

    // 商品が存在するか確認
    const product = await ProductModel.findByPk(productId)
    if (!product) {
      return res.status(404).json({ error: 'Product not found' })
    }

    // ✓ 認証済みユーザーの情報を使用
    const author = req.user?.email  // JWT から取得した信頼できる値

    if (!author) {
      return res.status(401).json({ error: 'Authentication required' })
    }

    await ReviewModel.create({
      productId,
      message,
      author  // サーバー側で取得した正しいメールアドレス
    })

    res.json({ status: 'success' })
  }
}
```

### さらに安全な実装（ID参照）

```typescript
// ✅ メールアドレスではなくユーザーIDで管理
await ReviewModel.create({
  productId,
  message,
  userId: req.user?.id  // メールアドレスではなくIDを保存
})

// 表示時にユーザー情報を JOIN で取得
const reviews = await ReviewModel.findAll({
  where: { productId },
  include: [{
    model: UserModel,
    attributes: ['email', 'username']  // 表示に必要な情報のみ
  }]
})
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **認証情報から取得** | ユーザーIDやメールは JWT/セッションから取得 |
| ✅ **リクエスト入力を信頼しない** | body.author, body.userId を使わない |
| ✅ **ID で管理** | メールアドレスではなく内部IDで参照 |
| ✅ **外部キー制約** | DB レベルで userId の存在を保証 |

---

## ビジネスへの影響

偽レビュー投稿は実際のビジネスで深刻な問題になりうる:

```
┌─────────────────────────────────────────────────────────────────┐
│                     ビジネスへの影響                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【競合への攻撃】                                               │
│  - 競合商品に低評価の偽レビューを大量投稿                        │
│  - 有名人になりすまして「この商品は最悪」と投稿                  │
│                                                                 │
│  【自社への不正】                                                │
│  - 自社商品に高評価の偽レビューを投稿                            │
│  - インフルエンサーになりすまして宣伝                            │
│                                                                 │
│  【法的リスク】                                                  │
│  - 偽レビューは多くの国で違法（景品表示法等）                    │
│  - なりすましは名誉毀損・詐欺罪に該当する可能性                  │
│                                                                 │
│  【信頼性の低下】                                                │
│  - プラットフォーム全体の信頼性が損なわれる                      │
│  - ユーザー離れにつながる                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 類似の脆弱性パターン

| 機能 | 脆弱なパターン | 安全なパターン |
|------|--------------|---------------|
| レビュー投稿 | `body.author` を使用 | `req.user.email` を使用 |
| コメント投稿 | `body.userId` を使用 | `req.user.id` を使用 |
| 注文作成 | `body.customerId` を使用 | JWT から取得 |
| メッセージ送信 | `body.senderId` を使用 | セッションから取得 |

---

## OWASP との関連

- **A01:2021 - Broken Access Control**: 他人としてアクションを実行できてしまう

---

## 関連チャレンジ

- [Forged Feedback](forged-feedback.md) - フィードバックのなりすまし
- [View Basket](../difficulty-2/view-basket.md) - 他人のカートを閲覧
- [Manipulate Basket](manipulate-basket.md) - 他人のカートに商品を追加

## 参考リンク

- [OWASP IDOR](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/05-Authorization_Testing/04-Testing_for_Insecure_Direct_Object_References)
- [CWE-639: Authorization Bypass Through User-Controlled Key](https://cwe.mitre.org/data/definitions/639.html)
- [PortSwigger - IDOR](https://portswigger.net/web-security/access-control/idor)
