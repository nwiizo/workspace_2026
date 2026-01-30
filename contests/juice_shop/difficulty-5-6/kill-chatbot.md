# Kill Chatbot ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** 脆弱コンポーネント
**目標:** Chatbot をクラッシュさせる

## ヒント

- **機能:** Support Chat (`/#/chatbot`)
- **脆弱性:** 特定の入力でクラッシュ
- **可能性:**
  - プロトタイプ汚染
  - ReDoS (正規表現 DoS)
  - 未処理の例外

## 攻撃ペイロード候補

```javascript
// プロトタイプ汚染
{"__proto__": {"isAdmin": true}}
JSON.parse('{"__proto__":{"polluted":true}}')

// ReDoS
"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!"

// 特殊文字
"\x00\x00\x00"
"{{constructor.constructor('return this')()}}"
```

## 調査方法

1. Chatbot と対話して動作を確認
2. 特殊な入力を試す
3. エラーレスポンスを観察

## 検証ポイント

- [ ] Chatbot の正常動作を確認
- [ ] 各種ペイロードを試行
- [ ] クラッシュまたは異常動作を確認

## 解説

[未着手]
