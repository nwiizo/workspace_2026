# Leaked API Key ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 機密データ
**目標:** ソースコードから漏洩した API キーを発見

## ヒント

- **場所:**
  - `main.js` (フロントエンドバンドル)
  - `/ftp` ディレクトリ
  - ソースマップ (`main.js.map`)
- **形式:** `API_KEY`, `SECRET`, `TOKEN` などの変数名

## 調査方法

```bash
# main.js から API キーを検索
curl http://localhost:3000/main.js | grep -iE "api.?key|secret|token"

# /ftp ディレクトリを確認
curl http://localhost:3000/ftp/

# ソースマップを取得
curl http://localhost:3000/main.js.map
```

## よくあるパターン

```javascript
// ハードコードされた API キー
const API_KEY = "sk-xxxxxxxxxxxxxxxx";
const SECRET = "secret123";
const AWS_ACCESS_KEY = "AKIA...";
```

## 検索キーワード

```
api_key, apiKey, API_KEY
secret, SECRET, secretKey
token, accessToken, access_token
password, pwd, pass
key, privateKey, private_key
```

## 検証ポイント

- [ ] main.js を検索
- [ ] /ftp の設定ファイルを確認
- [ ] 発見した API キーの形式を確認
- [ ] チャレンジ完了に必要なアクションを実行

## 解説

[未着手]
