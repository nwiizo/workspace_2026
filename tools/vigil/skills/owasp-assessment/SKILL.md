# OWASP Assessment — 詳細仕様

2つの OWASP 標準に基づく網羅的セキュリティ検査。

- **OWASP Top 10:2021** — Web アプリケーション向け（A01〜A10）
- **OWASP API Security Top 10:2023** — API 向け（API1〜API10）

各カテゴリに対して: 検査項目、CWE マッピング、grep パターン、判定基準、Opus 4.6 による深掘りポイントを定義する。

---

# Part 1: OWASP Top 10:2021（Web アプリケーション）

公式: https://owasp.org/Top10/

## A01:2021 — Broken Access Control

**概要:** アクセス制御の不備。ユーザーが許可された範囲を超えて操作できる。2021年版で1位に上昇。テスト対象の94%で検出。

**主要 CWE:**
- CWE-200: 機密情報の未認可アクターへの露出
- CWE-201: 送信データへの機密情報の挿入
- CWE-352: CSRF
- CWE-639: ユーザー制御キーによる認可バイパス
- CWE-862: 認可チェックの欠如
- CWE-863: 不正な認可
- CWE-22: パストラバーサル
- CWE-425: 強制ブラウジング
（34 CWE にマッピング）

**検査項目:**
1. IDOR: URL/パラメータのオブジェクト ID で他ユーザーリソースにアクセス可能か
2. 垂直権限昇格: 一般ユーザーが管理者機能にアクセス可能か
3. 水平権限昇格: 他ユーザーのデータを閲覧・変更可能か
4. メソッド制限: PUT/DELETE 等の意図しない HTTP メソッドが許可されるか
5. CORS 設定: `Access-Control-Allow-Origin: *` や不適切なオリジン許可
6. JWT/Cookie 操作: トークンの改ざんでアクセス制御をバイパス可能か
7. 強制ブラウジング: 認証なしで制限ページに直接アクセス可能か
8. パストラバーサル: `../` でファイルシステムを横断可能か

**grep パターン:**
```
# IDOR 候補
Grep("\\$_(GET|POST|REQUEST)\\[.*(id|user_id|account|uid|pid)")
Grep("params\\[:(id|user_id|account_id)\\]")
Grep("req\\.(params|query|body)\\.(id|userId|accountId)")

# 認可チェック関数の特定
Grep("is_admin|isAdmin|check_role|checkRole|authorize|can\\(|ability|permit")

# CORS
Grep("Access-Control-Allow-Origin.*\\*")
Grep("Access-Control-Allow-Credentials.*true")

# パストラバーサル
Grep("\\.\\./|\\.\\.\\\\/|\\.\\.%2[fF]")

# 強制ブラウジング
Grep("admin|dashboard|manage|internal")
```

**Opus 4.6 深掘り:**
- 各エンドポイントでの認可チェックの一貫性を追跡（チェック漏れを特定）
- IDOR のデータフロー分析（ID パラメータの取得 → DB クエリ → レスポンスの流れ）
- 多段階プロセスのステップスキップ可能性

## A02:2021 — Cryptographic Failures

**概要:** 暗号化の不備。以前の「機密データの露出」から焦点を絞り、暗号化そのものの失敗に特化。

**主要 CWE:**
- CWE-259: ハードコードされたパスワード
- CWE-327: 危殆化した暗号アルゴリズムの使用
- CWE-331: 不十分なエントロピー
- CWE-319: 機密データの平文送信
- CWE-338: 暗号的に弱い PRNG の使用
（29 CWE にマッピング）

**検査項目:**
1. パスワード保存: bcrypt/argon2/scrypt を使用しているか（MD5/SHA1 は不可）
2. ハードコードされた秘密情報: パスワード、API キー、暗号鍵がコードに埋め込まれていないか
3. 平文通信: HTTP での機密データ送信はないか
4. 弱い乱数: `rand()`, `mt_rand()`, `Math.random()` をセキュリティ用途に使っていないか
5. 鍵管理: 暗号鍵が環境変数/KMS から取得されているか
6. TLS 設定: 最新の TLS バージョン、Forward Secrecy が有効か
7. 初期化ベクトル: 固定鍵で IV を再利用していないか

**grep パターン:**
```
# 弱いハッシュ
Grep("\\bmd5\\s*\\(|\\bsha1\\s*\\(")
Grep("hashlib\\.(md5|sha1)")
Grep("MessageDigest\\.getInstance\\(['\"]MD5|SHA-1")

# ハードコードされた秘密
Grep("password\\s*=\\s*['\"](?!\\s*$)|secret\\s*=\\s*['\"]|api_key\\s*=\\s*['\"]")
Grep("ENCRYPTION_KEY|SECRET_KEY|JWT_SECRET|PRIVATE_KEY")

# 弱い乱数
Grep("\\brand\\s*\\(|\\bmt_rand\\s*\\(|Math\\.random\\(|random\\.random\\(")

# 平文通信
Grep("http://(?!localhost|127\\.0\\.0\\.1)")

# 非推奨暗号
Grep("DES|RC4|RC2|Blowfish|ECB")
Grep("SSLv[23]|TLSv1[^.]|TLSv1\\.0")
```

**Opus 4.6 深掘り:**
- パスワードのハッシュ → 保存 → 検証の全フローを追跡
- 暗号鍵の生成 → 使用 → ローテーションのライフサイクル検証

## A03:2021 — Injection

**概要:** インジェクション攻撃。SQLi, XSS, OS コマンドインジェクション等。テスト対象の94%で何らかのインジェクション脆弱性が検出。

**主要 CWE:**
- CWE-79: XSS（クロスサイトスクリプティング）
- CWE-89: SQL インジェクション
- CWE-73: 外部制御されたファイル名またはパス
- CWE-77: コマンドインジェクション
- CWE-78: OS コマンドインジェクション
- CWE-94: コードインジェクション
（33 CWE にマッピング）

**検査項目:**
1. SQL インジェクション: 文字列結合、エスケープ済み引用符なし数値、セカンドオーダー、エンコーディングバイパス
2. XSS: Reflected, Stored, DOM-based
3. OS コマンドインジェクション: ユーザー入力を含むシェルコマンド実行
4. LDAP/XPath インジェクション
5. テンプレートインジェクション（SSTI）
6. ORM インジェクション
7. Expression Language / OGNL インジェクション

**grep パターン:**
```
# SQLi
Grep("(mysql_query|mysqli_query|pg_query|sqlite_query).*\\$")
Grep("SELECT.*FROM.*WHERE.*\\$|INSERT.*INTO.*VALUES.*\\$")
Grep("sprintf.*SELECT|sprintf.*INSERT|sprintf.*UPDATE|sprintf.*DELETE")
Grep("\\.execute\\(.*%s|\\.execute\\(.*\\+|\\.execute\\(.*\\$")
Grep("\\.raw\\(|Arel\\.sql\\(")

# XSS
Grep("echo\\s+\\$_(GET|POST|REQUEST)|print\\s+\\$_(GET|POST|REQUEST)")
Grep("innerHTML\\s*=|document\\.write\\(|\\$\\(.*\\.html\\(")
Grep("dangerouslySetInnerHTML|v-html=|\\|safe|\\|raw|\\{!!.*!!\\}")
Grep("res\\.send\\(.*req\\.|res\\.write\\(.*req\\.")

# コマンドインジェクション
Grep("(system|exec|passthru|shell_exec|popen|proc_open)\\s*\\(.*\\$")
Grep("os\\.system\\(.*request|subprocess\\.(call|run|Popen).*request")
Grep("Runtime\\.getRuntime\\(\\)\\.exec.*request|ProcessBuilder.*request")

# SSTI
Grep("render_template_string|Template\\(.*request|Jinja2.*request")
Grep("\\{\\{.*request|\\$\\{.*request")
```

**Opus 4.6 深掘り:**
- セカンドオーダー SQLi: DB保存→取得→引用符なし使用の多段攻撃フロー追跡
- エンコーディングバイパス: 接続文字セットとエスケープ関数の組み合わせ分析
- DOM-based XSS: JavaScript のデータフロー追跡（source → sink）

## A04:2021 — Insecure Design

**概要:** 安全でない設計。実装の欠陥ではなくアーキテクチャ・設計レベルの欠陥。2021年版で新規追加。「安全な設計でも実装の欠陥により脆弱性が生じるが、安全でない設計は完璧な実装でも修正できない」。

**主要 CWE:**
- CWE-209: エラーメッセージによる機密情報の露出
- CWE-256: 認証情報の保護されていない保存
- CWE-501: 信頼境界違反
- CWE-522: 不十分に保護された認証情報
（40 CWE にマッピング）

**検査項目:**
1. 脅威モデリングの不足: セキュリティ要件が設計に反映されているか
2. レート制限の欠如: ログイン、パスワードリセット、API に制限があるか
3. ビジネスロジック欠陥: 価格操作、数量マイナス値、多段階プロセスのスキップ
4. 機密操作の再認証: パスワード変更、メールアドレス変更前に再認証があるか
5. リソース消費制限: ファイルアップロードサイズ、クエリ複雑度に制限があるか
6. テナント分離: マルチテナント環境でデータ分離が適切か
7. Bot 対策: 自動化攻撃への防御があるか

**grep パターン:**
```
# レート制限
Grep("rate_limit|throttle|RateLimit|slowDown|express-rate-limit")

# ビジネスロジック
Grep("price|amount|quantity|total|discount|coupon")
Grep("step|wizard|stage|phase|checkout")

# 再認証
Grep("confirm_password|current_password|re_authenticate|verify_password")

# リソース制限
Grep("max_size|maxFileSize|upload_max|MAX_UPLOAD|limit")
```

**Opus 4.6 深掘り:**
- ビジネスロジック全体のフロー追跡（SAST では原理的に検出不可能）
- 信頼境界の暗黙の前提の発見
- 多段階プロセスの各ステップの独立性検証

## A05:2021 — Security Misconfiguration

**概要:** セキュリティ設定の不備。テスト対象の90%で検出。XXE（旧 A04:2017）がこのカテゴリに統合。

**主要 CWE:**
- CWE-16: 設定の問題
- CWE-611: XML 外部エンティティ参照の不適切な制限（XXE）
（20 CWE にマッピング）

**検査項目:**
1. デバッグモード: 本番環境でデバッグが有効でないか
2. デフォルト認証情報: admin/admin 等が残っていないか
3. 不要な機能: 不要なポート、サービス、ページ、アカウントが有効でないか
4. セキュリティヘッダー: X-Frame-Options, CSP, HSTS, X-Content-Type-Options 等
5. エラーメッセージ: スタックトレースや内部情報が露出しないか
6. XXE: XML パーサーが外部エンティティを処理しないか
7. ディレクトリリスティング: Web サーバーの自動インデックスが有効でないか

**grep パターン:**
```
# デバッグ
Grep("debug\\s*=\\s*[Tt]rue|DEBUG\\s*=\\s*1|display_errors\\s*=\\s*[Oo]n")
Grep("APP_DEBUG|FLASK_DEBUG|DJANGO_DEBUG|NODE_ENV.*development")

# デフォルト認証情報
Grep("admin.*admin|root.*root|test.*test|default.*password")

# XXE
Grep("simplexml_load|DOMDocument|XMLReader|SAXParser|DocumentBuilder")
Grep("LIBXML_NOENT|FEATURE_EXTERNAL_ENTITIES")

# 情報漏洩
Grep("phpinfo\\s*\\(|server_info|ServerInfo")
Grep("stack_trace|stacktrace|backtrace|traceback")
Grep("error_reporting\\(.*E_ALL")

# セキュリティヘッダー（不在を確認）
Grep("X-Frame-Options|Content-Security-Policy|Strict-Transport-Security")
Grep("X-Content-Type-Options|X-XSS-Protection|Referrer-Policy")
```

## A06:2021 — Vulnerable and Outdated Components

**概要:** 既知脆弱性のあるコンポーネントの使用。

**主要 CWE:**
- CWE-937: 既知脆弱性のあるコンポーネントの使用（OWASP 2013）
- CWE-1035: 既知脆弱性のあるコンポーネントの使用（OWASP 2017）
- CWE-1104: メンテナンスされていないサードパーティコンポーネントの使用

**検査項目:**
1. バージョン管理: 全コンポーネント（直接 + 推移的依存）のバージョンを把握しているか
2. 既知脆弱性: 使用バージョンに CVE がないか
3. サポート状態: EOL（End-of-Life）のソフトウェアを使用していないか
4. 非推奨 API: 言語/フレームワークの非推奨関数を使用していないか
5. パッチ適用: セキュリティパッチがタイムリーに適用されているか

**grep パターン:**
```
# パッケージマニフェスト
Glob("**/composer.json", "**/package.json", "**/requirements.txt")
Glob("**/Gemfile", "**/go.mod", "**/Cargo.toml", "**/pom.xml")
Glob("**/package-lock.json", "**/yarn.lock", "**/Pipfile.lock")

# バージョン定数
Grep("version|VERSION|Version")

# PHP 非推奨関数
Grep("mysql_connect|mysql_query|ereg\\(|split\\(|session_register")

# Node.js 非推奨
Grep("new Buffer\\(|require\\(['\"]crypto['\"]\\)\\.createCipher\\b")

# Python 非推奨
Grep("cgi\\.escape|commands\\.getoutput|os\\.popen")
```

## A07:2021 — Identification and Authentication Failures

**概要:** 認証の不備。2017年版では2位だったが7位に下降（フレームワークの認証サポート向上による）。

**主要 CWE:**
- CWE-297: 証明書ホスト不一致の不適切な検証
- CWE-287: 不適切な認証
- CWE-384: セッション固定攻撃
（22 CWE にマッピング）

**検査項目:**
1. クレデンシャルスタッフィング/ブルートフォース: 自動化攻撃への対策があるか
2. デフォルト/弱いパスワード: `Password1`, `admin/admin` が許可されるか
3. パスワード回復: セキュリティ質問等の弱い回復メカニズムでないか
4. パスワード保存: 平文またはリバーシブルな暗号化で保存していないか
5. MFA: 多要素認証が実装されているか
6. セッション管理: セッション ID が URL に露出しないか、ログアウト後に無効化されるか
7. セッション固定: ログイン成功後にセッション ID を再生成しているか
8. セッションタイムアウト: 適切な有効期限が設定されているか

**grep パターン:**
```
# セッション管理
Grep("session_regenerate_id|session_destroy|session_start")
Grep("req\\.session\\.destroy|req\\.session\\.regenerate")
Grep("session\\.gc_maxlifetime|SESSION_TIMEOUT|session_timeout|maxAge")

# パスワードハッシュ
Grep("password_hash|password_verify|bcrypt|argon2|scrypt")
Grep("pbkdf2|PBKDF2")

# パスワードリセット
Grep("password_reset|forgot_password|reset_token|recovery")

# ログイン試行制限
Grep("login_attempts|failed_login|lockout|account_lock|max_attempts")
```

## A08:2021 — Software and Data Integrity Failures

**概要:** ソフトウェアとデータの完全性の不備。安全でないデシリアライゼーション（旧 A08:2017）を含む。CI/CD パイプライン、自動更新の整合性検証も対象。

**主要 CWE:**
- CWE-829: 信頼されない制御領域からの機能の取り込み
- CWE-494: 整合性チェックなしのコードダウンロード
- CWE-502: 信頼されないデータのデシリアライゼーション
（10 CWE にマッピング）

**検査項目:**
1. デシリアライゼーション: ユーザー入力を直接デシリアライズしていないか
2. 依存関係の整合性: パッケージの署名検証を行っているか
3. CI/CD パイプライン: アクセス制御、設定レビュー、署名検証があるか
4. 自動更新: 更新のデジタル署名を検証しているか
5. CDN/外部ソース: SRI（Subresource Integrity）を使用しているか

**grep パターン:**
```
# デシリアライゼーション
Grep("\\bunserialize\\s*\\(")
Grep("pickle\\.loads|yaml\\.load\\((?!.*Loader)|yaml\\.unsafe_load")
Grep("ObjectInputStream|readObject|XMLDecoder|XStream")
Grep("JSON\\.parse.*eval|jsonpickle|marshal\\.loads")

# 外部リソースの整合性
Grep("integrity=|SRI|subresource")
Grep("<script.*src=.*http|<link.*href=.*http")

# CI/CD
Glob("**/.github/workflows/*.yml", "**/.gitlab-ci.yml", "**/Jenkinsfile")
Glob("**/Dockerfile", "**/docker-compose*.yml")
```

## A09:2021 — Security Logging and Monitoring Failures

**概要:** セキュリティログと監視の不備。コミュニティ調査で3位。侵害の検出には平均287日かかる。

**主要 CWE:**
- CWE-117: ログの不適切な出力無害化
- CWE-223: セキュリティ関連情報の省略
- CWE-532: ログファイルへの機密情報の挿入
- CWE-778: 不十分なログ記録

**検査項目:**
1. ログ記録: ログイン試行、アクセス制御失敗、入力バリデーション失敗がログされるか
2. ログ品質: 警告・エラーが適切で明確なメッセージか
3. 監視: アプリケーションログの不審な活動が監視されているか
4. 集中管理: ログがローカルのみでなく集中管理されているか
5. アラート: リアルタイム/準リアルタイムの攻撃検出とアラートがあるか
6. 機密データ漏洩: ログにパスワード、トークン等が含まれていないか
7. ログインジェクション: ログデータが適切にエンコードされているか
8. 監査証跡: 高価値トランザクションに改ざん防止付き監査証跡があるか

**grep パターン:**
```
# ログの存在確認
Grep("\\blog\\(|logger\\.|logging\\.|error_log|syslog|console\\.(log|error|warn)")
Grep("Log\\.(info|warn|error|debug)|log\\.(info|warn|error|debug)")
Grep("audit|audit_log|access_log")

# ログへの機密データ混入
Grep("log.*(password|secret|token|api_key|credit_card|ssn)")
Grep("console\\.log.*(password|secret|token)")

# ログインジェクション対策
Grep("log.*sanitize|log.*escape|log.*encode")
```

## A10:2021 — Server-Side Request Forgery (SSRF)

**概要:** SSRF。コミュニティ調査で1位として2021年版に新規追加。クラウド環境の普及で深刻度が上昇。

**主要 CWE:**
- CWE-918: Server-Side Request Forgery

**検査項目:**
1. URL 入力: ユーザー指定の URL にサーバーサイドからアクセスする機能があるか
2. URL バリデーション: スキーム（file://, gopher://）、ホスト（内部 IP, localhost）、ポートが制限されているか
3. クラウドメタデータ: `169.254.169.254` 等のメタデータエンドポイントへのアクセスが防止されているか
4. リダイレクト: サーバーサイドで HTTP リダイレクトに自動追従しないか
5. DNS リバインディング: DNS 応答のキャッシュと再検証が行われているか
6. レスポンス: 生のサーバーレスポンスがクライアントに返されていないか

**grep パターン:**
```
# URL をパラメータで受け取る機能
Grep("file_get_contents\\(.*\\$|curl_setopt.*CURLOPT_URL.*\\$")
Grep("requests\\.(get|post|put|delete)\\(.*request\\.")
Grep("urllib\\.request\\.urlopen.*request|httplib.*request")
Grep("fetch\\(.*req\\.|axios\\.(get|post).*req\\.")
Grep("HttpClient.*request|WebClient.*request")
Grep("RestTemplate.*request|OkHttp.*request")

# 内部 IP チェック
Grep("127\\.0\\.0\\.1|localhost|0\\.0\\.0\\.0|169\\.254\\.|10\\.|172\\.(1[6-9]|2|3[01])\\.|192\\.168\\.")

# URL スキーム
Grep("file://|gopher://|dict://|ftp://|ldap://")
```

**Opus 4.6 深掘り:**
- URL パラメータ → fetch 関数 → レスポンス処理の全データフロー追跡
- リダイレクトチェーンの追跡（オープンリダイレクトとの連鎖）
- DNS リバインディング攻撃の可能性評価

---

# Part 2: OWASP API Security Top 10:2023

公式: https://owasp.org/API-Security/editions/2023/en/0x11-t10/

## API1:2023 — Broken Object Level Authorization (BOLA)

**概要:** API がオブジェクト ID に基づくアクセス制御を適切に行わない。エンドポイントはオブジェクト識別子を処理するため、オブジェクトレベルのアクセス制御問題の広い攻撃面を形成する。

**悪用容易性:** Easy | **普及度:** Widespread | **検出容易性:** Easy

**検査項目:**
1. オブジェクト ID 操作: API エンドポイントのパス/クエリ/ボディ内の ID を変更して他ユーザーのリソースにアクセス可能か
2. 認可チェック: 全てのオブジェクトアクセスで所有権/権限が検証されているか
3. ID の予測可能性: シーケンシャルな整数 ID が使用されていないか（GUID/UUID が推奨）
4. バッチ操作: 複数オブジェクトの一括操作で個別の認可チェックが行われているか

**grep パターン:**
```
# パスパラメータの ID
Grep("/:id|/<int:id>|/\\{id\\}|/\\{[a-z_]*_id\\}")
Grep("params\\[:id\\]|params\\['id'\\]|req\\.params\\.id")

# DB クエリでの ID 使用（認可チェックなし）
Grep("find\\(.*params|findById\\(.*params|where\\(.*id.*params")
Grep("SELECT.*WHERE.*id\\s*=.*params|SELECT.*WHERE.*id\\s*=.*req\\.")

# 認可関数
Grep("authorize|can\\?|has_permission|check_ownership|belongs_to")
```

**Opus 4.6 深掘り:**
- エンドポイントごとの認可チェック一貫性分析（A01 と連携）
- GraphQL のネストされたオブジェクト参照の権限検証

## API2:2023 — Broken Authentication

**概要:** 認証メカニズムの実装不備。認証はすべてのユーザーにアクセス可能であるため、主要な攻撃面となる。

**悪用容易性:** Easy | **普及度:** Common | **検出容易性:** Easy

**検査項目:**
1. ブルートフォース保護: ログイン試行にレート制限/ロックアウトがあるか
2. クレデンシャルスタッフィング: 既知の漏洩パスワードリストに対する保護があるか
3. 弱いパスワードポリシー: 最小長、複雑さ要件が設定されているか
4. トークン管理: JWT の署名検証、有効期限チェックが行われているか
5. 機密操作の再認証: パスワード変更等に現在のパスワードを要求するか
6. URL 内のトークン: 認証トークンが URL に含まれていないか
7. CAPTCHA/MFA: ボット対策と多要素認証があるか

**grep パターン:**
```
# JWT
Grep("jwt\\.decode|jwt\\.verify|jsonwebtoken|jose|JWT")
Grep("algorithm.*none|alg.*none")  # none アルゴリズム攻撃
Grep("JWT_SECRET|jwt_secret|token_secret")

# 認証エンドポイント
Grep("/login|/auth|/token|/signin|/signup|/register|/oauth")
Grep("/reset-password|/forgot-password|/verify-email")

# レート制限
Grep("rate.?limit|throttle|express-rate-limit|django-ratelimit|rack-attack")

# トークン in URL
Grep("token=|api_key=|access_token=|auth=")
```

## API3:2023 — Broken Object Property Level Authorization

**概要:** オブジェクトプロパティレベルの認可不備。旧 API3:2019（過剰なデータ露出）と旧 API6:2019（Mass Assignment）を統合。

**悪用容易性:** Easy | **普及度:** Common | **検出容易性:** Easy

**検査項目:**
1. 過剰なデータ露出: API レスポンスに不要な機密プロパティ（password_hash, internal_id, is_admin）が含まれていないか
2. Mass Assignment: クライアントが送信したプロパティがフィルタなしでオブジェクトに反映されないか
3. プロパティフィルタリング: API レスポンスで返すプロパティが明示的に選択されているか
4. 入力バインディング: リクエストボディのプロパティがホワイトリストで制限されているか

**grep パターン:**
```
# 全プロパティ返却
Grep("to_json|to_dict|as_json|serialize|toJSON\\(\\)|JSON\\.stringify")
Grep("select\\s*\\*|SELECT\\s*\\*")

# Mass Assignment
Grep("update_attributes|assign_attributes|fill\\(|mass_assignment")
Grep("req\\.body\\)|Object\\.assign.*req\\.body|\\{.*\\.\\.\\.req\\.body")

# ホワイトリスト/ブラックリスト
Grep("attr_accessible|attr_protected|fillable|guarded")
Grep("permit\\(|strong_parameters|allowed_params")
```

**Opus 4.6 深掘り:**
- API レスポンスの各フィールドの機密性評価
- Mass Assignment による `is_admin`, `role`, `balance` 等の権限昇格可能性

## API4:2023 — Unrestricted Resource Consumption

**概要:** リソース消費の制限なし。旧「Lack of Resources & Rate Limiting」から改名。API リクエスト処理に必要なリソース（帯域、CPU、メモリ、ストレージ）の制限不足。

**悪用容易性:** Average | **普及度:** Widespread | **検出容易性:** Easy

**検査項目:**
1. レート制限: API エンドポイントにリクエスト回数制限があるか
2. ペイロードサイズ: リクエストボディ、ファイルアップロードにサイズ制限があるか
3. ページネーション: レスポンスの返却件数に上限があるか
4. タイムアウト: API 処理に実行時間の上限があるか
5. バッチリクエスト: GraphQL のネストクエリ等に深度/複雑度制限があるか
6. サードパーティ費用: 外部 API 呼び出しに支出制限/アラートがあるか
7. OTP/パスワードリセット: 試行回数に制限があるか

**grep パターン:**
```
# ページネーション
Grep("per_page|page_size|pageSize|limit|offset|cursor")
Grep("LIMIT\\s+\\$|LIMIT.*request|LIMIT.*params")

# ファイルアップロード制限
Grep("max_size|maxFileSize|upload_max|MAX_UPLOAD|multer.*limits")

# GraphQL 深度制限
Grep("depthLimit|queryComplexity|maxDepth|maxComplexity")

# タイムアウト
Grep("timeout|TIMEOUT|requestTimeout|connectTimeout")

# レート制限（API 固有）
Grep("X-RateLimit|x-rate-limit|retry-after|429")
```

## API5:2023 — Broken Function Level Authorization

**概要:** 関数レベルの認可不備。複雑なアクセス制御ポリシー（階層、グループ、ロール）が認可の欠陥を招く。

**悪用容易性:** Easy | **普及度:** Common | **検出容易性:** Easy

**検査項目:**
1. 管理者 API: 管理者向けエンドポイントに一般ユーザーがアクセスできないか
2. HTTP メソッド変更: GET → PUT/DELETE にメソッドを変えて操作可能か
3. URL 推測: `/api/v1/users/export_all` のような管理者機能を URL 推測でアクセスできないか
4. 一貫した認可: 全エンドポイントで統一された認可モジュールが使われているか
5. デフォルト拒否: 明示的に許可されていないアクセスは拒否されるか

**grep パターン:**
```
# 管理者エンドポイント
Grep("/admin|/manage|/internal|/debug|/export|/import|/bulk")
Grep("admin_only|is_admin|require_admin|@admin_required")

# ロールチェック
Grep("role\\s*==|role\\s*===|has_role|check_role|@roles\\(|@authorize")
Grep("canActivate|AuthGuard|RolesGuard|PermissionGuard")

# HTTP メソッド制限
Grep("app\\.(get|post|put|delete|patch)\\(|router\\.(get|post|put|delete)")
Grep("@(Get|Post|Put|Delete|Patch)Mapping|RequestMethod\\.")
```

**Opus 4.6 深掘り:**
- ルーティング定義の全体マッピング → 認可ミドルウェアの適用漏れを検出
- 管理者機能と一般機能の境界分析

## API6:2023 — Unrestricted Access to Sensitive Business Flows

**概要:** 機密ビジネスフローへの無制限アクセス。API の自動化により、在庫の買い占め、チケット転売、スパム等のビジネス上の害を引き起こす。

**悪用容易性:** Easy | **普及度:** Widespread | **検出容易性:** Average

**検査項目:**
1. ビジネスフローの特定: 過剰な自動化が業務に害を与えるフローを特定しているか
2. Bot 対策: CAPTCHA、デバイスフィンガープリント、ヒューマン検出があるか
3. 振る舞い分析: 非人間的な利用パターンの検出ができるか
4. IP 制限: Tor 出口ノード、既知プロキシのブロックがあるか
5. B2B API の制限: マシン消費 API に適切なアクセス制御があるか

**grep パターン:**
```
# CAPTCHA
Grep("captcha|recaptcha|hcaptcha|turnstile")

# フィンガープリント
Grep("fingerprint|device_id|deviceId|browser_id")

# ビジネスクリティカルなフロー
Grep("purchase|checkout|reserve|book|transfer|withdraw|claim|redeem")
Grep("vote|review|comment|register|invite|referral")
```

**Opus 4.6 深掘り:**
- ビジネスフロー全体のモデリング（注文→決済→在庫減→発送）
- 自動化攻撃による業務影響の評価

## API7:2023 — Server Side Request Forgery (SSRF)

**概要:** API がユーザー提供の URI を検証せずにリモートリソースを取得する。ファイアウォール/VPN を迂回可能。

**悪用容易性:** Easy | **普及度:** Common | **検出容易性:** Easy

**検査項目:**
1. URL パラメータ: ユーザー指定の URL/URI にサーバーサイドからアクセスする機能があるか
2. Webhook: ユーザーが登録した Webhook URL への送信で内部リソースにアクセスされないか
3. ファイル取得: URL 指定でのファイルインポート/プレビュー機能
4. カスタム SSO: OAuth コールバック等の URL 検証
5. URL スキーム: `file://`, `gopher://` 等の危険なスキームが制限されているか
6. リダイレクト: サーバーサイドでのリダイレクト追従が制限されているか

**grep パターン:**
```
# Webhook
Grep("webhook|callback_url|notify_url|redirect_uri")

# URL パラメータ
Grep("url=|uri=|target=|dest=|redirect=|return_url=|next=")
Grep("fetch\\(.*params|requests\\.get\\(.*params|curl.*params")

# ファイルインポート
Grep("import_url|source_url|image_url|avatar_url|icon_url|feed_url")
```

**Opus 4.6 深掘り:**
- URL パラメータの全データフロー追跡（A10:2021 Web と同様だが API 固有のコンテキスト）
- クラウドメタデータエンドポイントへの到達可能性

## API8:2023 — Security Misconfiguration

**概要:** API スタック全体にわたるセキュリティ設定の不備。自動化ツールで検出・悪用可能。

**悪用容易性:** Easy | **普及度:** Widespread | **検出容易性:** Easy

**検査項目:**
1. TLS: 全 API 通信が暗号化されているか
2. CORS: API の CORS ポリシーが適切か（`*` でないか）
3. HTTP メソッド: 不要なメソッドが無効化されているか
4. Content-Type: 許容する Content-Type が制限されているか
5. エラーメッセージ: API エラーレスポンスにスタックトレースが含まれていないか
6. レスポンススキーマ: レスポンスのスキーマが定義・検証されているか
7. セキュリティヘッダー: 適切なセキュリティヘッダーが設定されているか
8. サーバーチェーン: リバースプロキシ/ロードバランサー間でリクエスト処理が統一されているか

**grep パターン:**
```
# CORS（API 固有）
Grep("cors|CORS|Access-Control")
Grep("origin.*\\*|allow_origins.*\\*|allowedOrigins.*\\*")

# Content-Type 制限
Grep("Content-Type|content_type|consumes|produces")
Grep("application/json|application/xml|multipart/form-data")

# API エラーハンドリング
Grep("stack.*trace|stackTrace|internal.*error.*detail")
Grep("res\\.status\\(500\\)\\.json|InternalServerError")

# API ドキュメント（本番で露出していないか）
Grep("swagger|openapi|api-docs|graphql.*playground|graphiql")
```

## API9:2023 — Improper Inventory Management

**概要:** API インベントリの不適切な管理。古い API バージョン、パッチ未適用のエンドポイント、弱いセキュリティ要件の非本番エンドポイントが残存。

**悪用容易性:** Easy | **普及度:** Widespread | **検出容易性:** Average

**検査項目:**
1. API バージョン管理: 古い API バージョン（v1, v2）が稼働し続けていないか
2. 廃止エンドポイント: 使われなくなったエンドポイントが残っていないか
3. 環境分離: ステージング/テスト環境の API が本番データにアクセスできないか
4. API ドキュメント: 全 API エンドポイントが文書化されているか
5. サードパーティ連携: データ共有先の第三者が明確で、共有の根拠があるか
6. セキュリティポリシーの一貫性: 全バージョンで同等のセキュリティ対策が適用されているか

**grep パターン:**
```
# API バージョニング
Grep("/v[0-9]+/|/api/v[0-9]+|version.*[0-9]+\\.[0-9]+")

# 非推奨/テスト用エンドポイント
Grep("deprecated|legacy|old_|_old|beta|alpha|test_|_test|staging")
Grep("/debug|/test|/demo|/sandbox|/dev/")

# API ドキュメント
Glob("**/swagger.*", "**/openapi.*", "**/api-docs*", "**/*.raml")
```

**Opus 4.6 深掘り:**
- ルーティング定義から全 API エンドポイントを抽出し、ドキュメントとの差分を検出
- 古いバージョンと新しいバージョンのセキュリティ対策の差を比較

## API10:2023 — Unsafe Consumption of APIs

**概要:** API の安全でない利用。開発者はサードパーティ API からのデータをユーザー入力より信頼しがちで、弱いセキュリティ基準を適用する。

**悪用容易性:** Easy | **普及度:** Common | **検出容易性:** Average

**検査項目:**
1. サードパーティデータの検証: 外部 API からのレスポンスをバリデーション/サニタイズしているか
2. TLS: サードパーティ API との通信が暗号化されているか
3. リダイレクト: 外部 API のリダイレクトを無条件に追従していないか
4. リソース制限: サードパーティのレスポンスサイズ/処理時間に制限があるか
5. タイムアウト: 外部 API 呼び出しにタイムアウトが設定されているか
6. 入力と同等の扱い: サードパーティデータをユーザー入力と同等にサニタイズしているか

**grep パターン:**
```
# サードパーティ API 呼び出し
Grep("requests\\.(get|post)|axios\\.(get|post)|fetch\\(|HttpClient|RestTemplate")
Grep("curl_exec|file_get_contents\\(.*http")
Grep("WebClient|OkHttp|Retrofit|Feign")

# レスポンスの直接使用（検証なし）
Grep("response\\.json\\(\\)|response\\.data|response\\.body")
Grep("json_decode\\(.*curl|JSON\\.parse\\(.*response")

# タイムアウト設定
Grep("timeout|connectTimeout|readTimeout|CURLOPT_TIMEOUT")
```

**Opus 4.6 深掘り:**
- サードパーティ API レスポンスのデータフロー追跡（受信 → 加工 → DB 保存/出力）
- サプライチェーン攻撃シナリオの構築（サードパーティが侵害された場合の影響評価）

---

# 出力フォーマット

## 統合レポート

```markdown
# OWASP Assessment Report

## 対象標準
- OWASP Top 10:2021（Web アプリケーション）
- OWASP API Security Top 10:2023（API）

## サマリー

### OWASP Top 10:2021

| カテゴリ | 検出数 | Critical | High | Medium | Low |
|---------|--------|----------|------|--------|-----|
| A01: Broken Access Control | N | n | n | n | n |
| A02: Cryptographic Failures | N | n | n | n | n |
| A03: Injection | N | n | n | n | n |
| A04: Insecure Design | N | n | n | n | n |
| A05: Security Misconfiguration | N | n | n | n | n |
| A06: Vulnerable and Outdated Components | N | n | n | n | n |
| A07: Identification and Authentication Failures | N | n | n | n | n |
| A08: Software and Data Integrity Failures | N | n | n | n | n |
| A09: Security Logging and Monitoring Failures | N | n | n | n | n |
| A10: Server-Side Request Forgery | N | n | n | n | n |

### OWASP API Security Top 10:2023

| カテゴリ | 検出数 | Critical | High | Medium | Low |
|---------|--------|----------|------|--------|-----|
| API1: Broken Object Level Authorization | N | n | n | n | n |
| API2: Broken Authentication | N | n | n | n | n |
| API3: Broken Object Property Level Authorization | N | n | n | n | n |
| API4: Unrestricted Resource Consumption | N | n | n | n | n |
| API5: Broken Function Level Authorization | N | n | n | n | n |
| API6: Unrestricted Access to Sensitive Business Flows | N | n | n | n | n |
| API7: Server Side Request Forgery | N | n | n | n | n |
| API8: Security Misconfiguration | N | n | n | n | n |
| API9: Improper Inventory Management | N | n | n | n | n |
| API10: Unsafe Consumption of APIs | N | n | n | n | n |

## 詳細

### [ID] 脆弱性タイトル

- **深刻度:** Critical / High / Medium / Low
- **ファイル:** `path:line`
- **OWASP:** A03:2021 / API1:2023（該当する両方を記載）
- **CWE:** CWE-89
- **コード:**
  \```
  // コード + 日本語コメント（// ← なぜ危険か）
  \```
- **攻撃シナリオ:** ...
- **推奨修正:** ...
```

## ID 命名規則

### Web（OWASP Top 10:2021）
`{カテゴリ}-{サブカテゴリ}-{連番}`
- `A01-IDOR-01`, `A03-SQL-01`, `A03-XSS-01`, `A07-SESS-01`

### API（OWASP API Security Top 10:2023）
`{カテゴリ}-{サブカテゴリ}-{連番}`
- `API1-BOLA-01`, `API2-AUTH-01`, `API3-MASS-01`, `API5-BFLA-01`

### Web と API の両方に該当する場合
両方の ID を併記: `A01-IDOR-01 / API1-BOLA-01`
