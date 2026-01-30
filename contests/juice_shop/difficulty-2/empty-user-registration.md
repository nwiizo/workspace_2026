# Empty User Registration ✅

**難易度:** ⭐⭐
**カテゴリ:** 入力検証
**目標:** 空のデータでユーザーを登録する

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
fetch('/api/Users/', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({})
})
// email: null, password: undefined のユーザーが作成される
```

## 解説

- 空のJSONオブジェクトを送信すると、ユーザーが作成される
- `email: null`, `password: undefined` のユーザーが存在することに
- サーバー側で必須フィールドの検証が行われていない

**脆弱性の原因:**
```javascript
// サーバー側のコード（推測）
const user = await User.create(req.body);
// req.body が {} だと、デフォルト値または null で作成される
```

**対策:**
- 必須フィールドの存在チェック
- フィールドの型チェック
- フィールドの値の範囲チェック

## 関連チャレンジ

- [Repetitive Registration](../difficulty-1/repetitive-registration.md)
- [Zero Stars](../difficulty-1/zero-stars.md)
