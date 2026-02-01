# Zero Stars ✅

**難易度:** ⭐
**カテゴリ:** 入力検証 / クライアントサイドバイパス
**目標:** 0つ星のフィードバックを送信する（通常は1-5星のみ）

---

## 背景知識

### クライアントサイド検証の限界

Webアプリケーションでは、入力値の検証を**クライアント（ブラウザ）側**と**サーバー側**の両方で行う必要がある。しかし、クライアント側の検証だけでは**セキュリティにならない**。

```
┌─────────────────────────────────────────────────────────────────┐
│                  クライアント vs サーバー検証                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【クライアント側検証（ブラウザ）】                                │
│  ┌────────────────────────────────────────┐                    │
│  │ <select>                               │                    │
│  │   <option value="1">⭐</option>        │                    │
│  │   <option value="2">⭐⭐</option>      │                    │
│  │   <option value="3">⭐⭐⭐</option>    │                    │
│  │   ...                                  │                    │
│  │ </select>                              │                    │
│  │                                        │                    │
│  │ ✗ ユーザーがDevToolsで変更可能        │                    │
│  │ ✗ APIを直接叩けばバイパス可能         │                    │
│  │ ✗ 「見た目」の制限に過ぎない           │                    │
│  └────────────────────────────────────────┘                    │
│             │                                                   │
│             │ リクエスト送信（fetch/XHR）                        │
│             ▼                                                   │
│  【サーバー側検証（バックエンド）】                                │
│  ┌────────────────────────────────────────┐                    │
│  │ if (rating < 1 || rating > 5) {       │                    │
│  │   return error("Invalid rating")      │                    │
│  │ }                                      │                    │
│  │                                        │                    │
│  │ ✓ ユーザーが直接変更できない          │                    │
│  │ ✓ 本当のセキュリティはここで実装       │                    │
│  └────────────────────────────────────────┘                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

オンラインアンケートを想像してください:

- **クライアント側検証**: 画面上で1〜5の選択肢しか表示されない
- **サーバー側検証**: 送信されたデータが1〜5の範囲内か確認する

画面に1〜5しかなくても、攻撃者はリクエストを直接送れば**0や-100や999**を送信できる。

### なぜこれが問題か

```
┌─────────────────────────────────────────────────────────────────┐
│                     影響範囲                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【評価システムへの影響】                                        │
│  - 平均評価の計算が狂う（0や負数が含まれると）                   │
│  - ビジネスロジックの破綻                                       │
│                                                                 │
│  【より深刻な例】                                               │
│  - 数量欄に負数 → 返金処理の悪用                                │
│  - 価格欄に0 → 無料で商品購入                                   │
│  - 年齢欄に-1 → 年齢制限のバイパス                              │
│                                                                 │
│  【共通点】                                                     │
│  サーバーが「画面から来たデータだから正しいはず」と信頼してしまう │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

**ステップ1: 画面の制限を確認**
```
「フィードバック画面で星を選ぶ」
    ↓
「1〜5個しか選べない」
    ↓
「でもこれはフロントエンド（画面）の制限」
    ↓
「APIに直接リクエストを送ったら？」
```

**ステップ2: APIリクエストを調査**
```
「DevTools → Network タブを開く」
    ↓
「フィードバック送信時のリクエストを観察」
    ↓
「POST /api/Feedbacks/ に JSON が送られている」
    ↓
「rating: 3 などの数値が含まれている」
```

**ステップ3: 値を改ざん**
```
「Console で直接APIを呼び出す」
    ↓
「rating: 0 を指定」
    ↓
「サーバーは受け入れた！」
    ↓
「フロントエンドの検証だけでは不十分という教訓」
```

## 実行手順

1. Juice Shop にログイン（まだアカウントがなければ登録）
2. キーボードの `F12` を押して DevTools を開く
3. 「Console」タブをクリック
4. 以下をコピー＆ペーストして Enter:
   ```javascript
   fetch('/api/Feedbacks/', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       comment: 'Zero stars!',
       rating: 0,
       captchaId: 0,
       captcha: '0'
     })
   }).then(r => r.json()).then(console.log)
   ```
5. `{status: "success"}` と表示されれば成功

## Juice Shop の脆弱なコードパターン

### 脆弱なコード（推定）

```typescript
// ❌ 脆弱なコード
// routes/feedback.ts
export function createFeedback() {
  return async (req: Request, res: Response) => {
    const { comment, rating, captchaId, captcha } = req.body

    // CAPTCHA 検証
    if (!validateCaptcha(captchaId, captcha)) {
      return res.status(401).json({ error: 'Invalid captcha' })
    }

    // ❌ rating の範囲チェックがない！
    // フロントエンドで 1-5 に制限しているから大丈夫...と思っている

    await FeedbackModel.create({
      comment,
      rating,  // 0 でも -100 でも保存される
      UserId: req.user?.id
    })

    res.json({ status: 'success' })
  }
}
```

### 問題点

1. **サーバー側の検証なし**: クライアント側の `<select>` を信頼している
2. **型チェックのみ**: 数値であればどんな値でも受け入れる
3. **ビジネスロジック無視**: 1-5星という仕様が守られていない

---

## 安全な実装

```typescript
// ✅ 安全なコード
// routes/feedback.ts
export function createFeedback() {
  return async (req: Request, res: Response) => {
    const { comment, rating, captchaId, captcha } = req.body

    // 1. CAPTCHA 検証
    if (!validateCaptcha(captchaId, captcha)) {
      return res.status(401).json({ error: 'Invalid captcha' })
    }

    // 2. rating の型チェック
    if (typeof rating !== 'number') {
      return res.status(400).json({ error: 'Rating must be a number' })
    }

    // 3. rating の範囲チェック（これが重要！）
    if (!Number.isInteger(rating) || rating < 1 || rating > 5) {
      return res.status(400).json({ error: 'Rating must be between 1 and 5' })
    }

    // 4. comment のサニタイズ
    const sanitizedComment = sanitizeHtml(comment, {
      allowedTags: [],
      allowedAttributes: {}
    })

    await FeedbackModel.create({
      comment: sanitizedComment,
      rating,
      UserId: req.user?.id
    })

    res.json({ status: 'success' })
  }
}
```

### バリデーションライブラリを使う場合

```typescript
// Joi を使った例
import Joi from 'joi'

const feedbackSchema = Joi.object({
  comment: Joi.string().max(500).required(),
  rating: Joi.number().integer().min(1).max(5).required(),
  captchaId: Joi.number().required(),
  captcha: Joi.string().required()
})

// 使用時
const { error, value } = feedbackSchema.validate(req.body)
if (error) {
  return res.status(400).json({ error: error.details[0].message })
}
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **型チェック** | 期待する型（number, string 等）を確認 |
| ✅ **範囲チェック** | 許容される範囲内（1-5）を確認 |
| ✅ **整数チェック** | 小数点が不要なら `Number.isInteger()` |
| ✅ **必須チェック** | null/undefined を拒否 |
| ✅ **サニタイズ** | HTML/SQL インジェクション対策 |

---

## バイパステクニック集

攻撃者がクライアント側の制限を回避する方法:

### 1. Console から fetch API

```javascript
// 最も一般的な方法
fetch('/api/Feedbacks/', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ rating: 0, comment: 'test' })
})
```

### 2. DevTools で HTML を編集

```html
<!-- Before -->
<option value="1">⭐</option>

<!-- After (value を変更) -->
<option value="0">⭐</option>
```

### 3. curl で直接リクエスト

```bash
curl -X POST http://localhost:3000/api/Feedbacks/ \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer YOUR_TOKEN' \
  -d '{"rating": 0, "comment": "Zero stars"}'
```

### 4. Burp Suite/ZAP でリクエスト改ざん

プロキシでリクエストをインターセプトし、送信前に値を変更。

---

## 解説

- 画面では1-5星しか選べないが、APIは0も受け付けてしまう
- フロントエンドだけでなく、バックエンドでも入力検証が必要
- **クライアント検証はUX向上のため**、**サーバー検証はセキュリティのため**

---

## OWASP との関連

このチャレンジは以下に該当:

- **A03:2021 - Injection**: 不正な入力値を受け入れてしまう
- **A04:2021 - Insecure Design**: クライアント側検証のみに依存

---

## 関連チャレンジ

- [Payback Time](../difficulty-3/payback-time.md) - 数量に負数を入力
- [Forged Feedback](../difficulty-3/forged-feedback.md) - UserId の改ざん
- [Deprecated Interface](../difficulty-2/deprecated-interface.md) - ファイル種類の制限バイパス

## 参考リンク

- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
