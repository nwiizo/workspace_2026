# 難易度2 チャレンジ (13/23 解決)

SQLインジェクションやXSSなど、Webセキュリティの基本的な攻撃手法を学ぶチャレンジです。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 13 |
| ❌ 未解決 | 10 |

## チャレンジ一覧

| チャレンジ | カテゴリ | 状態 | ファイル |
|-----------|---------|------|----------|
| Login Admin | SQLi | ✅ | [login-admin.md](login-admin.md) |
| Admin Section | アクセス制御 | ✅ | [admin-section.md](admin-section.md) |
| View Basket | IDOR | ✅ | [view-basket.md](view-basket.md) |
| Password Strength | 認証 | ✅ | [password-strength.md](password-strength.md) |
| Reflected XSS | XSS | ✅ | [reflected-xss.md](reflected-xss.md) |
| Login MC SafeSearch | OSINT | ✅ | [login-mc-safesearch.md](login-mc-safesearch.md) |
| Five-Star Feedback | アクセス制御 | ✅ | [five-star-feedback.md](five-star-feedback.md) |
| Deprecated Interface | 設定ミス | ✅ | [deprecated-interface.md](deprecated-interface.md) |
| Weird Crypto | 暗号 | ✅ | [weird-crypto.md](weird-crypto.md) |
| Meta Geo Stalking | OSINT | ✅ | [meta-geo-stalking.md](meta-geo-stalking.md) |
| Visual Geo Stalking | OSINT | ✅ | [visual-geo-stalking.md](visual-geo-stalking.md) |
| Empty User Registration | 入力検証 | ✅ | [empty-user-registration.md](empty-user-registration.md) |
| Exposed Credentials | 機密データ | ✅ | [exposed-credentials.md](exposed-credentials.md) |
| NFT Takeover | 機密データ | ❌ | [nft-takeover.md](nft-takeover.md) |

## 認証情報

```
admin@juice-sh.op / admin123
mc.safesearch@juice-sh.op / Mr. N00dles
john@juice-sh.op / newpassword123 (リセット後)
emma@juice-sh.op / newpassword123 (リセット後)
testing@juice-sh.op / IamUsedForTesting
```

## XSSペイロード

```html
<!-- Reflected XSS -->
/#/track-result?id=<iframe src="javascript:alert('xss')">

<!-- DOM XSS -->
/#/search?q=<iframe src="javascript:alert('xss')">
```
