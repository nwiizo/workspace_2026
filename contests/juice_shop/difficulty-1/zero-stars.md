# Zero Stars ✅

**難易度:** ⭐
**カテゴリ:** 入力検証
**目標:** 0つ星のフィードバックを送信する（通常は1-5星のみ）

---

## 思考プロセス

**ステップ1: 画面の制限を確認**
```
「フィードバック画面で星を選ぶ」
    ↓
「1〜5個しか選べない」
    ↓
「でもこれはフロントエンド（画面）の制限」
    ↓
「APIに直接リクエストを送ったら？」
```

**ステップ2: APIリクエストを調査**
```
「DevTools → Network タブを開く」
    ↓
「フィードバック送信時のリクエストを観察」
    ↓
「POST /api/Feedbacks/ に JSON が送られている」
    ↓
「rating: 3 などの数値が含まれている」
```

**ステップ3: 値を改ざん**
```
「Console で直接APIを呼び出す」
    ↓
「rating: 0 を指定」
    ↓
「サーバーは受け入れた！」
    ↓
「フロントエンドの検証だけでは不十分という教訓」
```

## 実行手順

1. Juice Shop にログイン（まだアカウントがなければ登録）
2. キーボードの `F12` を押して DevTools を開く
3. 「Console」タブをクリック
4. 以下をコピー＆ペーストして Enter:
   ```javascript
   fetch('/api/Feedbacks/', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       comment: 'Zero stars!',
       rating: 0,
       captchaId: 0,
       captcha: '0'
     })
   }).then(r => r.json()).then(console.log)
   ```
5. `{status: "success"}` と表示されれば成功

## 解説

- 画面では1-5星しか選べないが、APIは0も受け付けてしまう
- フロントエンドだけでなく、バックエンドでも入力検証が必要
