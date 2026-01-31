# Reflected XSS ❌ (Docker環境で無効化)

**難易度:** ⭐⭐
**カテゴリ:** XSS
**目標:** URLパラメータを使ってスクリプトを実行する

## 試したこと (2026-01-31)

### 1. 従来の解法を試行

```
http://localhost:3000/#/track-result?id=<iframe src="javascript:alert(`xss`)">
```

**結果:** XSSペイロードがサニタイズされて `iframesrcjavascriptalertxss` と表示される。チャレンジは解決されず。

### 2. Docker環境でのチャレンジ状態を確認

```javascript
fetch('/api/Challenges')
  .then(r => r.json())
  .then(data => {
    const xss = data.data.filter(c => c.name.includes('XSS'));
    return xss.map(c => ({name: c.name, disabledEnv: c.disabledEnv}));
  });
```

**結果:**
| チャレンジ | disabledEnv |
|-----------|-------------|
| Reflected XSS | Docker |
| API-only XSS | Docker |
| Client-side XSS Protection | Docker |
| HTTP-Header XSS | Docker |
| Server-side XSS Protection | Docker |
| Video XSS | Docker |
| DOM XSS | null (有効) |

**Reflected XSS は Docker 環境で無効化されている**

## 解法 (非Docker環境用)

### 理論上の解法

ブラウザのアドレスバーに以下を入力:
```
http://localhost:3000/#/track-result?id=<iframe src="javascript:alert('xss')">
```

### なぜ動くはずなのか

**`<script>` ではなく `<iframe>` を使う理由:**

| タグ | 結果 | 理由 |
|------|------|------|
| `<script>` | ❌ 動かない | Angular が自動でブロック |
| `<img onerror=...>` | ❌ 動かない | サニタイズされる |
| `<iframe src=javascript:...>` | ✅ 動く | フィルタをすり抜ける |

### 攻撃シナリオ

```
攻撃者がこのURLをメールで送る
    ↓
被害者がクリックする
    ↓
被害者のブラウザでスクリプトが実行される
    ↓
Cookieを盗んだり、偽のログインフォームを表示できる
```

## ステータス

- [x] 従来の解法を試行
- [x] チャレンジがDocker環境で無効化されていることを確認
- [ ] 非Docker環境で検証
- [ ] チャレンジを解決

## 備考

Docker イメージを使用している場合、このチャレンジは**自動的に無効化**される。
解決するには:
1. ソースからビルドした Juice Shop を使用する
2. または Docker 環境以外で実行する

## 関連チャレンジ

- [DOM XSS](../difficulty-1/dom-xss.md) - Docker環境でも有効
- [HTTP-Header XSS](../difficulty-4/http-header-xss.md) - Docker環境で無効
