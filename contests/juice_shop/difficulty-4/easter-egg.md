# Easter Egg ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** その他
**目標:** 隠されたイースターエッグを発見する

---

## 実行手順

1. `/ftp/eastere.gg%2500.md` でeastere.ggファイルを取得
2. Base64文字列をデコード:
   ```
   L2d1ci9xcmlmL25lci9mYi9zaGFhbC9ndXJsL3V2cS9uYS9ybmZncmUvcnR0L2p2Z3V2YS9ndXIvcm5mZ3JlL3J0dA==
   → /gur/qrif/ner/fb/shaal/gurl/uvq/na/rnfgre/rtt/jvguva/gur/rnfgre/rtt
   ```
3. ROT13変換:
   ```
   /the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg
   ```
4. このURLにアクセス:
   ```
   http://localhost:3000/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg
   ```
5. 「Welcome to Planet Orangeuze」が表示されれば成功

## 解説

**エンコードの連鎖:**
1. Base64エンコード
2. ROT13（アルファベットを13文字シフト）
3. 両方をデコードすると隠しURLが判明

**ROT13とは:**
- アルファベットを13文字ずらす単純な置換暗号
- A→N, B→O, ..., N→A
- 2回適用すると元に戻る

## コード/ペイロード

```javascript
// Base64デコード
atob('L2d1ci9xcmlmL25lci9mYi9zaGFhbC9ndXJsL3V2cS9uYS9ybmZncmUvcnR0L2p2Z3V2YS9ndXIvcm5mZ3JlL3J0dA==')
// → /gur/qrif/ner/fb/shaal/gurl/uvq/na/rnfgre/rtt/jvguva/gur/rnfgre/rtt

// ROT13変換
function rot13(str) {
  return str.replace(/[a-zA-Z]/g, c =>
    String.fromCharCode((c <= 'Z' ? 90 : 122) >= (c = c.charCodeAt(0) + 13) ? c : c - 26)
  );
}
```

## 関連チャレンジ

- [Poison Null Byte](poison-null-byte.md)
- [Score Board](../difficulty-1/score-board.md)
