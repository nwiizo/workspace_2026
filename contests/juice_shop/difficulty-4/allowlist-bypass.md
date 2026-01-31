# Allowlist Bypass ✅

**難易度:** ⭐⭐⭐⭐
**カテゴリ:** Unvalidated Redirects
**目標:** リダイレクト許可リストを回避して任意のサイトに誘導する

## 思考プロセス

1. `/redirect?to=` エンドポイントは許可されたURLにのみリダイレクト
2. 許可リストに `github.com/juice-shop/juice-shop` が含まれている
3. URL検証ロジックが「許可URLが含まれているか」をチェック
4. クエリパラメータとして許可URLを付加すれば検証をバイパス可能

## 実行手順

### ブラウザでアクセス

```
http://localhost:3000/redirect?to=https://owasp.slack.com?pwned=https://github.com/juice-shop/juice-shop
```

### Playwright MCP

```javascript
browser_navigate({
  url: "http://localhost:3000/redirect?to=https://owasp.slack.com?pwned=https://github.com/juice-shop/juice-shop"
});
```

## 解説

### 脆弱な検証ロジック

```javascript
// 脆弱なコード（概念）
function isAllowedRedirect(url) {
  const allowlist = ['github.com/juice-shop/juice-shop', 'blockchain.info'];
  return allowlist.some(allowed => url.includes(allowed));
}
```

**問題点:**
- `includes()` は URL 全体のどこかにマッチすれば true を返す
- クエリパラメータにも許可 URL を含められる

### バイパスの仕組み

```
https://owasp.slack.com?pwned=https://github.com/juice-shop/juice-shop
^^^^^^^^^^^^^^^^^^^^^^^        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
実際のリダイレクト先            許可リストマッチ用（クエリパラメータ）
```

- `owasp.slack.com` にリダイレクトされる
- `?pwned=...` の部分で許可リストのチェックをパス

### 許可された URL

設定から取得できる許可リスト:
- `github.com/juice-shop/juice-shop`
- `blockchain.info/address/` (Bitcoin)
- `explorer.dash.org/address/` (Dash)
- `etherscan.io/address/` (Ethereum)

### 攻撃シナリオ

1. **フィッシング**: 信頼されたドメインからのリンクに見せかける
2. **マルウェア配布**: 悪意のあるサイトにユーザーを誘導
3. **認証情報の窃取**: 偽のログインページにリダイレクト

### 対策

1. **完全一致検証**: URL 全体を許可リストと完全一致で検証
2. **ホスト名のみ検証**: スキーム + ホスト名のみを抽出して検証
3. **URL パース**: URL を適切にパースしてからホスト名を検証

```javascript
// 安全なコード
function isAllowedRedirect(url) {
  try {
    const parsedUrl = new URL(url);
    const allowedHosts = ['github.com', 'blockchain.info'];
    return allowedHosts.includes(parsedUrl.hostname);
  } catch {
    return false;
  }
}
```

## 関連チャレンジ

- [Outdated Allowlist](../difficulty-1/outdated-allowlist.md) - 古い暗号通貨アドレスへのリダイレクト

## 参考リンク

- [OWASP Unvalidated Redirects](https://cheatsheetseries.owasp.org/cheatsheets/Unvalidated_Redirects_and_Forwards_Cheat_Sheet.html)
- [CWE-601: URL Redirection to Untrusted Site](https://cwe.mitre.org/data/definitions/601.html)
