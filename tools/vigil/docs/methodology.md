# vigil — 監査方法論リファレンス

Opus 4.6 によるセキュリティ監査の方法論。SAST ツールとの併用を前提とし、セマンティック分析でしか発見できない脆弱性に注力する。

## 監査フェーズ

### Phase 1: 偵察（Reconnaissance）

**目的:** 対象プロジェクトの技術スタック、アーキテクチャ、データフローを理解する。

**手順:**
1. 言語・フレームワーク・ライブラリの特定
2. エントリポイントの列挙（ルーティング定義）
3. DB 接続方式の確認
4. 認証・認可メカニズムの把握
5. 外部サービスとの通信の確認
6. 文字エンコーディングの確認

**ツール:** Glob, Grep, Read

### Phase 2: 攻撃面の列挙（Attack Surface Enumeration）

**目的:** 外部から到達可能なすべてのインターフェースを列挙する。

**手順:**
1. 認証なしエンドポイントの列挙
2. 外部入力ポイント（HTTP パラメータ、ファイルアップロード、Cookie、ヘッダー）
3. 公開ディレクトリ内のファイル
4. 依存関係の既知脆弱性

**ツール:** Glob, Grep, Read, `/attack-surface` コマンド

### Phase 3: 脆弱性評価（Vulnerability Assessment）

**目的:** 各入力ポイントから到達可能な脆弱性を特定・評価する。

**手順:**
1. 入力トレーシング（入力 → 処理 → 出力の追跡）
2. SQL インジェクション分析（4パターン — 後述）
3. XSS 分析（出力エスケープの検証）
4. 認証・認可ロジックの検証
5. ファイル操作の検証
6. セカンドオーダー攻撃の検証
7. API 固有の脆弱性評価（BOLA, BFLA, Mass Assignment, リソース消費 — 後述）
8. 攻撃チェーンの構築

**ツール:** vulnerability-assessor エージェント

### Phase 4: 侵害調査（Compromise Investigation）

**目的:** 既に侵害されている痕跡がないかを調査する。

**手順:**
1. Web シェル・バックドアの探索
2. 危険関数の全数検査 + Web 到達性判定
3. 不審ファイルの検出
4. ハードコードされた認証情報の検出

**ツール:** compromise-investigator エージェント, `/webshell-hunt` コマンド

### Phase 5: 報告と修正計画（Reporting & Remediation）

**目的:** 発見事項を整理し、実行可能な修正計画を作成する。

**手順:**
1. OWASP Top 10:2021（Web）へのマッピング
2. OWASP API Security Top 10:2023 へのマッピング（API を含む場合）
3. 深刻度の評価（Critical / High / Medium / Low）
4. 修正優先度の決定（Phase 1-5）
5. Before/After コードの作成

**ツール:** remediation-planner エージェント, owasp-assessment スキル

---

## OWASP 標準リファレンス

### OWASP Top 10:2021（Web アプリケーション）

公式: https://owasp.org/Top10/

| ID | カテゴリ | 概要 | 主要 CWE |
|----|---------|------|----------|
| A01 | Broken Access Control | IDOR、認可バイパス、パストラバーサル、CORS、CSRF | CWE-200, 352, 639, 862, 863 (34 CWE) |
| A02 | Cryptographic Failures | 弱いハッシュ（MD5/SHA1）、ハードコード鍵、平文通信、弱い PRNG | CWE-259, 327, 331, 338 (29 CWE) |
| A03 | Injection | SQLi、XSS、OS コマンド、SSTI、ORM、LDAP | CWE-79, 89, 77, 78, 94 (33 CWE) |
| A04 | Insecure Design | ビジネスロジック欠陥、信頼境界違反、脅威モデリング不足 | CWE-209, 256, 501, 522 (40 CWE) |
| A05 | Security Misconfiguration | デバッグモード、デフォルト認証情報、XXE、エラー情報漏洩 | CWE-16, 611 (20 CWE) |
| A06 | Vulnerable and Outdated Components | 既知脆弱性のあるライブラリ、EOL ソフトウェア | CWE-937, 1035, 1104 |
| A07 | Identification and Authentication Failures | ブルートフォース、弱いパスワード、セッション固定 | CWE-287, 297, 384 (22 CWE) |
| A08 | Software and Data Integrity Failures | デシリアライズ、CI/CD 汚染、未署名更新 | CWE-502, 494, 829 (10 CWE) |
| A09 | Security Logging and Monitoring Failures | ログ不足、アラート欠如、ログへの機密データ混入 | CWE-117, 223, 532, 778 |
| A10 | Server-Side Request Forgery (SSRF) | URL 入力による内部リソースアクセス、クラウドメタデータ | CWE-918 |

### OWASP API Security Top 10:2023

公式: https://owasp.org/API-Security/editions/2023/en/0x11-t10/

| ID | カテゴリ | 概要 | 悪用容易性 |
|----|---------|------|-----------|
| API1 | Broken Object Level Authorization (BOLA) | オブジェクト ID 操作で他ユーザーリソースにアクセス | Easy |
| API2 | Broken Authentication | 認証実装の不備、JWT 操作、ブルートフォース | Easy |
| API3 | Broken Object Property Level Authorization | 過剰なデータ露出 + Mass Assignment（旧 API3+API6:2019 統合） | Easy |
| API4 | Unrestricted Resource Consumption | レート制限・ペイロードサイズ・ページネーション制限の欠如 | Average |
| API5 | Broken Function Level Authorization (BFLA) | 管理者 API への不正アクセス、HTTP メソッド操作 | Easy |
| API6 | Unrestricted Access to Sensitive Business Flows | ビジネスフローの自動化悪用（買い占め、スパム） | Easy |
| API7 | Server Side Request Forgery | Webhook/URL パラメータによる内部アクセス | Easy |
| API8 | Security Misconfiguration | CORS 設定、TLS、エラーメッセージ、API ドキュメント露出 | Easy |
| API9 | Improper Inventory Management | 古い API バージョン、未文書化エンドポイント、環境分離不足 | Easy |
| API10 | Unsafe Consumption of APIs | サードパーティ API データの未検証使用、サプライチェーンリスク | Easy |

### Web と API の重複カテゴリ

| Web (Top 10:2021) | API (Security Top 10:2023) | 共通テーマ |
|-------------------|---------------------------|-----------|
| A01: Broken Access Control | API1: BOLA + API5: BFLA | 認可の不備 |
| A02: Cryptographic Failures | API2: Broken Authentication | 認証・暗号の不備 |
| A05: Security Misconfiguration | API8: Security Misconfiguration | 設定の不備 |
| A07: Auth Failures | API2: Broken Authentication | 認証の不備 |
| A10: SSRF | API7: SSRF | SSRF |

脆弱性が Web と API の両方に該当する場合は、両方の ID を併記する（例: `A01-IDOR-01 / API1-BOLA-01`）。

---

## API 固有の脆弱性パターン

OWASP API Security Top 10:2023 に基づく、API 特有の検査観点。Web アプリケーションの検査に加えて実施する。

### Pattern 1: BOLA（Broken Object Level Authorization）

```
# 危険な API 実装（言語非依存の疑似コード）
GET /api/v1/users/{id}/profile

def get_profile(id):
    user = db.find_user(id)    # ← id はリクエストパスから取得
    return user.to_json()       # ← 認可チェックなしで他ユーザーのプロファイルを返却
    # ↑ id=1, id=2, ... と列挙するだけで全ユーザーのデータが取得可能
```

**検出:** 各 API エンドポイントでの認可チェック（所有権検証）の一貫性を追跡
**SAST での検出:** 困難（認可ロジックはビジネス固有で、パターンマッチングでは判定不可）
**Opus の役割:** エンドポイント全体の認可チェック一貫性を分析

### Pattern 2: Mass Assignment

```
# 危険な API 実装
PUT /api/v1/users/me

def update_profile(request):
    user = get_current_user()
    user.update(request.body)    # ← リクエストボディの全プロパティをそのまま適用
    # ↑ {"name": "Alice", "role": "admin"} を送信すると権限昇格
    # ← フィルタリングなしで role, is_admin, balance 等も変更可能
```

**検出:** リクエストボディ → オブジェクト更新のデータフローで、プロパティフィルタリングの有無を確認
**SAST での検出:** フレームワーク依存（Rails の `strong_parameters` 等なら検出可能だが、カスタム実装は困難）
**Opus の役割:** API レスポンスの全プロパティを分析し、変更されるべきでないプロパティを特定

### Pattern 3: サードパーティ API の安全でない利用

```
# 危険な実装
def get_weather(city):
    response = requests.get(f"https://weather-api.example.com/{city}")
    data = response.json()
    db.execute(f"INSERT INTO cache (city, temp) VALUES ('{data['city']}', {data['temp']})")
    # ← サードパーティ API のレスポンスを信頼し、サニタイズなしで SQL に挿入
    # ↑ weather-api が侵害された場合、SQLi が成立
```

**検出:** サードパーティ API レスポンスのデータフロー追跡（受信 → 加工 → DB/出力）
**SAST での検出:** 非常に困難（データソースの信頼性を判定できない）
**Opus の役割:** サプライチェーン攻撃シナリオの構築

---

## SQL インジェクション 4パターン

言語非依存の SQLi 分類。各パターンの特徴と検出方法。

### Pattern 1: 直接結合（Direct Concatenation）

```
# 危険なコード（言語非依存の疑似コード）
query = "SELECT * FROM users WHERE name = '" + user_input + "'"
# ← ユーザー入力を直接文字列結合。' を含む入力で SQL 構造を破壊
```

**検出:** `Grep("SELECT.*\\+.*\\$|SELECT.*\\..*\\$|SELECT.*%s")`
**SAST での検出:** 容易（パターンマッチングで十分）
**Opus の役割:** SAST に委譲

### Pattern 2: エスケープ済み・引用符なし数値コンテキスト

```
# 危険なコード
id = escape(user_input)
query = "SELECT * FROM users WHERE id = " + id
# ← エスケープしているが引用符で囲っていない。数値以外の入力で SQL 構造を破壊
# ← escape() はシングルクォートをエスケープするが、引用符外では無意味
```

**検出:** エスケープ関数の戻り値が引用符なしで使われるケースを追跡
**SAST での検出:** 困難（エスケープ関数の存在で安全と誤判定）
**Opus の役割:** データフロー追跡で検出

### Pattern 3: セカンドオーダー（Second-Order）

```
# Step 1: 安全に保存（プリペアドステートメント使用）
stmt = prepare("INSERT INTO profiles (name) VALUES (?)")
stmt.execute(user_input)

# Step 2: 取得後に引用符なしで使用（別の処理）
name = query("SELECT name FROM profiles WHERE id = ?", profile_id)
query2 = "SELECT * FROM orders WHERE customer_name = " + name
# ← DB から取得した値を「信頼済み」として扱い、引用符なしで結合
# ← 保存時に O'Brien のような名前が入っていると SQL 構造を破壊
```

**検出:** DB 保存と取得のデータフローを横断的に追跡
**SAST での検出:** 非常に困難（ファイル/関数をまたぐデータフロー）
**Opus の役割:** マルチファイル分析で検出（最大の差別化ポイント）

### Pattern 4: エンコーディングバイパス

```
# 危険な構成
# 接続文字セット: EUC-JP (or Shift_JIS, GBK)
# エスケープ関数: mysql_escape_string（接続文字セットを無視）

value = mysql_escape_string(user_input)
query = "SELECT * FROM t WHERE col = '" + value + "'"
# ← EUC-JP の 0xbf5c は2バイト文字だが、0x5c = バックスラッシュとして解釈される
# ← エスケープで追加された \ (0x5c) が多バイト文字に吸収され、' が生き残る
```

**検出:** 文字エンコーディング設定とエスケープ関数の組み合わせを確認
**SAST での検出:** 非常に困難（エンコーディング設定の理解が必要）
**Opus の役割:** 設定ファイル + コードの横断分析で検出

---

## Web シェル検出チェックリスト

### 1. シグネチャベース検出

- [ ] `eval()` + ユーザー入力（$_GET, $_POST, $_REQUEST, $_COOKIE）
- [ ] `assert()` + ユーザー入力
- [ ] `create_function()` + 動的引数
- [ ] `call_user_func()` / `call_user_func_array()` + ユーザー入力
- [ ] `preg_replace()` with `/e` modifier
- [ ] 可変変数（`$$var`）による間接的コード実行
- [ ] `system()` / `exec()` / `passthru()` / `shell_exec()` + ユーザー入力

### 2. 難読化検出

- [ ] 多重 Base64（`base64_decode(base64_decode(...))`）
- [ ] gzinflate + Base64
- [ ] str_rot13 + Base64
- [ ] chr() 連結による文字列構築
- [ ] 16進数エスケープ（`\x41\x42\x43`）
- [ ] pack() / unpack() による変換
- [ ] 変数名の意味のないランダム文字列

### 3. 隠蔽手法検出

- [ ] 404 偽装（GET → 404, POST → 実行）
- [ ] GIF/JPEG/PNG ヘッダ偽装（`GIF89a` + PHP コード）
- [ ] 二重拡張子（`.jpg.php`, `.gif.php`）
- [ ] 画像ディレクトリ・キャッシュディレクトリへの配置
- [ ] 隠しファイル名（`.xxx.php`）
- [ ] 正規ファイル名の模倣（`wp-config.php`, `.htaccess`）

### 4. 振る舞い検出

- [ ] HTTP メソッドによる分岐（GET vs POST で異なる動作）
- [ ] 特定のパラメータ/ヘッダーの存在チェック後に実行
- [ ] パスワード保護された実行ゲート
- [ ] User-Agent による分岐

---

## 危険関数 Web 到達性判定フロー

```
入力: 危険関数の呼び出し箇所（ファイル:行番号）

Step 1: ファイル種別の判定
  ├─ shebang (#!/usr/bin/...) あり → CLI 専用 → Web 到達性: なし [END]
  ├─ テストディレクトリ内 (test/, tests/, spec/) → テスト → Web 到達性: なし [END]
  └─ 上記以外 → Step 2 へ

Step 2: Web ルート内か
  ├─ Web ルート外 (vendor/, node_modules/, etc.) → ライブラリ → Step 3 へ
  └─ Web ルート内 → Step 3 へ

Step 3: エントリポイントからの到達パス
  ├─ 直接 URL アクセス可能なファイル → Web 到達性: あり → Step 5 へ
  ├─ include/require で読み込まれている → 読み込み元を確認 → Step 4 へ
  └─ どこからも参照されていない → 孤立ファイル → Web 到達性: 低 [END]

Step 4: ルーティングからの到達
  ├─ ルーティングテーブルに含まれる → Web 到達性: あり → Step 5 へ
  ├─ コメント内/デッドコード内 → Web 到達性: なし [END]
  └─ 条件分岐内（認証チェック後等） → Web 到達性: 条件付き → Step 5 へ

Step 5: 引数のソース判定
  ├─ ユーザー入力が引数に到達 → リスク: Critical
  ├─ DB/ファイルからの値が引数 → リスク: High（セカンドオーダーの可能性）
  ├─ 設定値が引数 → リスク: Medium（設定変更攻撃の可能性）
  └─ ハードコード値のみ → リスク: Low
```

---

## ドキュメント記述スタイルガイド

### コードブロック

脆弱なコードには必ず日本語コメントを付記:

```php
$sql = "SELECT * FROM users WHERE id = " . $id;
// ← $id が引用符で囲まれていない。数値以外の入力で SQL 構造を破壊
// ↑ mysql_real_escape_string() を通しても、引用符外では防御にならない
```

### Before/After 形式

```markdown
**Before（脆弱）:**
\```php
$name = $_POST['name'];
echo "Hello, " . $name;
// ← ユーザー入力を直接出力。<script>alert(1)</script> で XSS
\```

**After（修正）:**
\```php
$name = $_POST['name'];
echo "Hello, " . htmlspecialchars($name, ENT_QUOTES, 'UTF-8');
// ← htmlspecialchars で HTML エンティティに変換。XSS 不可
\```
```

### 脆弱性 ID の命名規則

**Web（OWASP Top 10:2021）:**
`{カテゴリ}-{サブカテゴリ}-{連番}`
- `A03-SQL-01`: SQL インジェクション 1件目
- `A03-XSS-03`: XSS 3件目
- `A01-IDOR-02`: IDOR 2件目
- `A07-SESS-01`: セッション管理の脆弱性 1件目

**API（OWASP API Security Top 10:2023）:**
`{カテゴリ}-{サブカテゴリ}-{連番}`
- `API1-BOLA-01`: BOLA 1件目
- `API2-AUTH-01`: 認証不備 1件目
- `API3-MASS-01`: Mass Assignment 1件目
- `API5-BFLA-01`: BFLA 1件目

**Web と API の両方に該当する場合:**
両方の ID を併記: `A01-IDOR-01 / API1-BOLA-01`

### 深刻度の定義

| レベル | 基準 |
|--------|------|
| **Critical** | RCE、認証バイパス、全データ漏洩。攻撃が容易で影響が甚大 |
| **High** | SQLi（限定的）、権限昇格、個人情報漏洩。攻撃に条件あり |
| **Medium** | Stored XSS、CSRF、情報漏洩（部分的）。悪用にユーザー操作が必要 |
| **Low** | Reflected XSS（限定的）、情報漏洩（技術情報のみ）、ベストプラクティス違反 |
| **Info** | 改善推奨だがセキュリティ影響なし |
