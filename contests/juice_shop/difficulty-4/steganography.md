# Steganography ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** Security through Obscurity
**目標:** 店舗に潜む悪名高いキャラクターを Contact フォームで報告

## 思考プロセス

1. チャレンジの説明は「Rat out a notorious character hiding in plain sight in the shop」
2. 「hiding in plain sight」= 画像にステガノグラフィで隠されている可能性
3. しかし、実際には商品画像に Rick and Morty の「Pickle Rick」が隠れている
4. Contact フォームでキャラクター名を正確に報告する必要がある

## 実行手順

### 方法1: Contact フォームで直接報告

```
1. /#/contact にアクセス
2. Comment に「Pickle Rick」と入力
3. CAPTCHA を解いて送信
```

### 方法2: API で直接送信

```javascript
// CAPTCHA を取得
const captcha = await fetch('/rest/captcha/').then(r => r.json());
const answer = eval(captcha.captcha);

// フィードバック送信
await fetch('/api/Feedbacks', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    comment: 'Pickle Rick',
    rating: 3,
    captchaId: captcha.captchaId,
    captcha: String(answer)
  })
});
```

## 解説

### なぜ「Pickle Rick」なのか

- Rick and Morty のエピソード「Pickle Rick」は非常に有名
- 商品画像のどこかに Pickle Rick が隠されている（ステガノグラフィ）
- 「notorious character」= 悪名高いキャラクター = Pickle Rick

### 脆弱性の本質

このチャレンジは技術的な脆弱性というより、隠された情報を見つけ出す OSINT/ステガノグラフィの演習。実際のセキュリティでは、画像に機密情報が埋め込まれている可能性を認識することが重要。

### 対策

- 公開前に画像メタデータとステガノグラフィをスキャン
- 信頼できないソースからの画像を注意深く扱う
- DLP（Data Loss Prevention）ツールでステガノグラフィを検出

## Playwright MCP での自動化

```javascript
// CAPTCHA 取得と送信を自動化
browser_evaluate(() => {
  const token = localStorage.getItem('token');
  return fetch('/rest/captcha/')
    .then(r => r.json())
    .then(captchaData => {
      const answer = eval(captchaData.captcha);
      return fetch('/api/Feedbacks', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer ' + token
        },
        body: JSON.stringify({
          comment: 'Pickle Rick',
          rating: 3,
          captchaId: captchaData.captchaId,
          captcha: String(answer)
        })
      }).then(r => r.json());
    });
});
```
