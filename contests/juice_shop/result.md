# OWASP Juice Shop CTF

**チーム:** team2-takenoko | **進捗:** 66/172 (38%) | **順位:** 1/7

| 難易度 | 解決 | 詳細 |
|--------|------|------|
| ⭐ | 15/28 | [difficulty-1/](difficulty-1/) |
| ⭐⭐ | 13/23 | [difficulty-2/](difficulty-2/) |
| ⭐⭐⭐ | 20/44 | [difficulty-3/](difficulty-3/) |
| ⭐⭐⭐⭐ | 13/37 | [difficulty-4/](difficulty-4/) |
| ⭐⭐⭐⭐⭐+ | 7/40 | [difficulty-5-6/](difficulty-5-6/) |

**その他:** [advanced-techniques.md](advanced-techniques.md)

---

## 最近解決したチャレンジ

| チャレンジ | 難易度 | 解法 |
|-----------|--------|------|
| Login Amy | ⭐⭐⭐ | パスワード: `K1f.....................` (24文字) |
| Reset Jim's Password | ⭐⭐⭐ | セキュリティ質問: `Samuel` |
| Upload Size | ⭐⭐⭐ | 100KB以上のファイルをアップロード |
| Upload Type | ⭐⭐⭐ | 非PDF/ZIPファイルをアップロード |
| Meta Geo Stalking | ⭐⭐ | セキュリティ質問: `Daniel Boone National Forest` |
| Visual Geo Stalking | ⭐⭐ | セキュリティ質問: `ITsec` |
| Easter Egg | ⭐⭐⭐⭐ | Base64+ROT13でパスを解読 |
| Allowlist Bypass | ⭐⭐⭐⭐ | リダイレクトURLにクエリ文字列で許可ドメインを埋め込み |
| Security Advisory | ⭐⭐⭐ | `/.well-known/csaf/provider-metadata.json` |
| Repetitive Registration | ⭐ | APIでパスワード不一致でも登録可能 |
| Extra Language | ⭐⭐⭐⭐⭐ | `/assets/i18n/tlh_AA.json` (Klingon) にアクセス |

---

## 難易度別サマリー

### 難易度1 (15/28)
基本的なWebセキュリティの入門チャレンジ
- Score Board, DOM XSS, Confidential Document, Exposed Metrics, Zero Stars
- Missing Encoding, Mass Dispel, Web3 Sandbox, Bully Chatbot, Security Policy
- Outdated Allowlist, Error Handling, Privacy Policy, Bonus Payload, Repetitive Registration

### 難易度2 (13/23)
SQLインジェクションやXSSなど基本攻撃
- Login Admin, Admin Section, View Basket, Password Strength, Reflected XSS
- Login MC SafeSearch, Five-Star Feedback, Deprecated Interface, Weird Crypto
- Meta Geo Stalking, Visual Geo Stalking, Empty User Registration, Exposed Credentials

### 難易度3 (20/44)
SQLi応用、XXE、CAPTCHA Bypass
- Login Jim, Login Bender, Admin Registration, Forged Feedback, Product Tampering
- XXE Data Access, Manipulate Basket, Bjoern's Favorite Pet, Database Schema
- CAPTCHA Bypass, Forged Review, GDPR Data Erasure, Payback Time, API-only XSS
- Login Amy, Reset Jim's Password, Upload Size, Upload Type, Security Advisory

### 難易度4 (13/37)
UNION SQLi、NoSQLi、Poison Null Byte
- Database Schema, User Credentials, Christmas Special, Poison Null Byte
- Forgotten Developer Backup, Easter Egg, HTTP-Header XSS, NoSQL Manipulation
- Access Log, Login Bjoern, Reset Bender's Password, Allowlist Bypass

### 難易度5-6 (7/40)
JWT操作、2FAバイパス、NoSQL Exfiltration
- Unsigned JWT, Two Factor Authentication, NoSQL Exfiltration
- Change Bender's Password, Blockchain Hype, Extra Language
