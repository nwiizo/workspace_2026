# Attack Surface — 攻撃面の列挙と分類

対象プロジェクトの攻撃面を網羅的に列挙し、リスク分類する。

## 実行手順

### Step 1: エンドポイント列挙

フレームワーク/言語に応じてルーティング定義を検索:

```
# 汎用パターン
Grep("route|router|get\(|post\(|put\(|delete\(|patch\(|@app\.|@router\.")
Grep("app\.(get|post|put|delete|patch|use)\(")

# PHP
Grep("\\$_(GET|POST|REQUEST|FILES|COOKIE|SERVER)")
Grep("case ['\"]")  # switch/case ルーティング

# Python (Django/Flask/FastAPI)
Grep("urlpatterns|path\(|re_path\(|@app\.(route|get|post)")

# Go
Grep("HandleFunc|Handle\(|mux\.(Get|Post|Put|Delete)")

# Ruby on Rails
Grep("(get|post|put|patch|delete|resources|resource) ")
```

各エンドポイントに対して:
- HTTP メソッド
- 認証要否（認証チェックの有無を確認）
- 入力パラメータ

### Step 2: 外部入力ポイント

認証なしでアクセス可能なすべての外部入力を列挙:

1. **HTTP パラメータ**: クエリ文字列、リクエストボディ、ヘッダー
2. **ファイルアップロード**: 受付箇所、拡張子チェック、MIME 検証
3. **Cookie / セッション**: 設定・読み取り箇所
4. **WebSocket**: メッセージハンドラ
5. **メール受信**: メール解析処理
6. **外部 API コールバック**: Webhook、OAuth コールバック
7. **環境変数・設定ファイル**: 起動時に読み込む外部入力

### Step 3: 認証なしエンドポイントの特定

認証チェック（ミドルウェア、デコレータ、手動チェック）がないエンドポイントを列挙:

```
# 認証パターンの検索
Grep("auth|login|session_check|is_authenticated|@login_required|middleware")
Grep("require_auth|check_token|verify_jwt|isLoggedIn")
```

認証チェックを行う関数/ミドルウェアを特定 → 各エンドポイントでの適用有無を確認。

### Step 4: ファイルシステムアクセス

1. **公開ディレクトリ**: Web サーバーから直接アクセス可能なファイル
2. **アップロードディレクトリ**: ユーザーが配置可能なファイル
3. **設定ファイル**: 公開ディレクトリ内の `.env`, `config.*`, `*.conf`
4. **バックアップファイル**: `.bak`, `.old`, `.swp`, `~` suffix

```
Glob("**/.env*", "**/*.bak", "**/*.old", "**/*.swp", "**/*~")
Glob("**/config.*", "**/*.conf", "**/*.ini", "**/*.yaml", "**/*.yml")
```

### Step 5: 依存関係の攻撃面

1. **既知脆弱性のあるライブラリ**: バージョン番号を確認
2. **非推奨 API**: 非推奨の関数・メソッドの使用
3. **デフォルト設定**: フレームワークのデフォルトシークレット、デバッグモード

### Step 6: 出力

以下の形式で攻撃面レポートを生成:

| カテゴリ | 項目 | 認証 | リスク | 備考 |
|---------|------|------|--------|------|
| HTTP エンドポイント | `POST /api/upload` | なし | High | 拡張子チェックなし |
| ファイル | `.env` in webroot | - | Critical | 認証情報露出 |
| 依存関係 | PHPMailer 5.2.16 | - | Critical | CVE-2016-10033 |
| ... | ... | ... | ... | ... |
