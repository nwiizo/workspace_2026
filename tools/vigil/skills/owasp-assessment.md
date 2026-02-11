# OWASP Assessment

2つの OWASP 標準に基づく網羅的セキュリティ検査を実行する。

## トリガー

- `/owasp-assessment` コマンド実行時
- セキュリティ監査の一環として OWASP マッピングが必要な場合
- API セキュリティ評価が必要な場合

## 対象標準

### OWASP Top 10:2021（Web アプリケーション）

公式: https://owasp.org/Top10/

| ID | カテゴリ | 主な検査内容 | 主要 CWE |
|----|---------|------------|----------|
| A01 | Broken Access Control | IDOR、認可チェック漏れ、パストラバーサル、CORS、CSRF | CWE-200, 352, 639, 862 |
| A02 | Cryptographic Failures | 弱いハッシュ、平文通信、ハードコードキー、弱い PRNG | CWE-259, 327, 331, 338 |
| A03 | Injection | SQLi、XSS、コマンドインジェクション、SSTI、ORM | CWE-79, 89, 77, 78, 94 |
| A04 | Insecure Design | ビジネスロジック欠陥、脅威モデリング不足、信頼境界違反 | CWE-209, 256, 501, 522 |
| A05 | Security Misconfiguration | デフォルト設定、不要機能有効、エラー情報漏洩、XXE | CWE-16, 611 |
| A06 | Vulnerable Components | 既知脆弱性のあるライブラリ、非サポートバージョン | CWE-937, 1035, 1104 |
| A07 | Auth Failures | ブルートフォース、弱いパスワードポリシー、セッション管理 | CWE-287, 297, 384 |
| A08 | Software & Data Integrity | デシリアライズ、CI/CD 汚染、未署名更新、SRI | CWE-502, 494, 829 |
| A09 | Logging & Monitoring | ログ不足、アラート欠如、監査証跡なし、ログインジェクション | CWE-117, 223, 532, 778 |
| A10 | SSRF | 内部リソースへのアクセス、URL 入力の検証不足 | CWE-918 |

### OWASP API Security Top 10:2023

公式: https://owasp.org/API-Security/editions/2023/en/0x11-t10/

| ID | カテゴリ | 主な検査内容 | 悪用容易性 |
|----|---------|------------|-----------|
| API1 | Broken Object Level Authorization (BOLA) | オブジェクト ID 操作による未認可アクセス | Easy |
| API2 | Broken Authentication | 認証メカニズムの実装不備、JWT 操作 | Easy |
| API3 | Broken Object Property Level Authorization | 過剰なデータ露出、Mass Assignment | Easy |
| API4 | Unrestricted Resource Consumption | レート制限なし、ペイロードサイズ無制限 | Average |
| API5 | Broken Function Level Authorization (BFLA) | 管理者 API への不正アクセス、HTTP メソッド操作 | Easy |
| API6 | Unrestricted Access to Sensitive Business Flows | ビジネスフローの自動化悪用（買い占め等） | Easy |
| API7 | Server Side Request Forgery | Webhook、URL パラメータによる内部アクセス | Easy |
| API8 | Security Misconfiguration | CORS、TLS、エラーメッセージ、API ドキュメント露出 | Easy |
| API9 | Improper Inventory Management | 古い API バージョン、未文書化エンドポイント | Easy |
| API10 | Unsafe Consumption of APIs | サードパーティ API データの未検証使用 | Easy |

## Web と API の重複マッピング

一部カテゴリは Web と API で重複する。両方の ID を併記する:

| Web (Top 10:2021) | API (Security Top 10:2023) | 共通テーマ |
|-------------------|---------------------------|-----------|
| A01: Broken Access Control | API1: BOLA + API5: BFLA | 認可の不備 |
| A02: Cryptographic Failures | API2: Broken Authentication | 認証・暗号の不備 |
| A05: Security Misconfiguration | API8: Security Misconfiguration | 設定の不備 |
| A07: Auth Failures | API2: Broken Authentication | 認証の不備 |
| A10: SSRF | API7: SSRF | SSRF |

## 詳細仕様

→ [SKILL.md](owasp-assessment/SKILL.md)
