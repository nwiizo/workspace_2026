# Attack Techniques

CTF で学んだ攻撃手法のまとめ。

## JWT 操作

### alg: none 攻撃
```json
{"alg": "none", "typ": "JWT"}
```
→ 署名検証をバイパス

### RS256 → HS256 混乱攻撃
- 公開鍵を HMAC シークレットとして使用
- `/encryptionkeys/jwt.pub` から公開鍵取得

## パラメータ操作

### パラメータ省略
- `current` パラメータを省略してパスワード変更成功
- 必須パラメータのバリデーション不備

### 負の値
- `quantity: -100` で負の金額注文

### NoSQL 演算子
- `{"$ne": -1}` で条件バイパス

## クライアント操作

### Date オーバーライド
```javascript
window.Date = function(...args) {
  if (args.length === 0) return new Date('2019-03-08');
  return new OriginalDate(...args);
};
```

### sessionStorage 改ざん
```javascript
sessionStorage.setItem('couponDetails', 'WMNSDY2019-1551999600000');
```

## バイパス技術

### Allowlist Bypass
許可 URL をクエリパラメータとして付加:
```
/redirect?to=https://evil.com?x=https://allowed-domain.com
```

### Poison Null Byte
```
/ftp/file.bak%2500.md → file.bak をダウンロード
```

## OSINT

### セキュリティ質問
- キャラクター設定から推測 (Hunter x Hunter → Silence of the Lambs)
- 写真の EXIF/背景から推測

### 漏洩情報
- `/.well-known/csaf/` でセキュリティアドバイザリ
- `/rest/admin/application-configuration` で設定漏洩
- Stack Overflow / Pastebin でログ漏洩

## 脆弱ライブラリ

### 発見方法
- `package.json.bak` から依存関係を確認
- `sanitize-html 1.4.2` → 既知の XSS 脆弱性

### Typosquatting
- `epilogue-js` (正規: `epilogue`)
- `ngy-cookie` (正規: `ngx-cookie`)

## 効率的な攻略

1. **API 直接呼び出し**: UI より `fetch()` が速い
2. **トークン取得**: `localStorage.getItem('token')`
3. **エラー確認**: `r.text()` でレスポンス内容を確認
