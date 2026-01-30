# View Basket ✅

**難易度:** ⭐⭐
**カテゴリ:** IDOR
**目標:** 他のユーザーのカートの中身を見る

---

## 思考プロセス

**ステップ1: 自分のカートのURLを確認**
```
「カートページを開く」
    ↓
「Network タブでAPIリクエストを観察」
    ↓
「/rest/basket/1 というリクエストを発見」
    ↓
「1 は自分のユーザーID... じゃあ 2 は？」
```

**ステップ2: IDを変えてみる**
```
「/rest/basket/2 にリクエストを送ってみる」
    ↓
「他のユーザーのカート内容が見えた！」
    ↓
「サーバーが "このカートはあなたのものか？" を確認していない」
```

**ステップ3: なぜこれが危険？**
```
「IDを1,2,3,...と総当たりすれば全ユーザーのカートが見える」
    ↓
「購入履歴、住所、支払い情報まで漏洩する可能性」
    ↓
「アクセス制御の欠如 = IDOR 脆弱性」
```

## 実行手順

1. まず普通にログイン
2. `F12` で DevTools を開く
3. 「Console」タブで以下を実行:
   ```javascript
   fetch('/rest/basket/2', {
     headers: {'Authorization': 'Bearer ' + localStorage.getItem('token')}
   }).then(r => r.json()).then(console.log)
   ```
4. 他のユーザーのカート内容が表示されれば成功

## 解説

**IDORとは？**
- Insecure Direct Object Reference（安全でない直接オブジェクト参照）
- URLの数字を変えるだけで、他人のデータにアクセスできてしまう脆弱性

**対策:**
- リソースへのアクセス時に所有権を確認する
- 予測困難なID（UUIDなど）を使用する
- アクセス制御リスト（ACL）を実装する

## 関連チャレンジ

- [Forged Feedback](../difficulty-3/forged-feedback.md)
- [Manipulate Basket](../difficulty-3/manipulate-basket.md)
