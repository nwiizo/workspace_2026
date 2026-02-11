# Webshell Hunt — Web シェル・バックドア探索

対象プロジェクトのファイルシステムを検査し、Web シェル・バックドア・不正ファイルを探索する。言語非依存。

## 実行手順

### Step 1: 既知パターンの検索

言語ごとの Web シェルシグネチャを検索:

**PHP:**
```
Grep("eval\s*\(.*\$_(GET|POST|REQUEST|COOKIE)")
Grep("eval\s*\(\s*base64_decode")
Grep("eval\s*\(\s*gzinflate")
Grep("eval\s*\(\s*str_rot13")
Grep("assert\s*\(.*\$_(GET|POST|REQUEST)")
Grep("preg_replace\s*\(.*['\"]/.*/e['\"]")
Grep("create_function\s*\(")
Grep("call_user_func(_array)?\s*\(.*\$_(GET|POST|REQUEST)")
Grep("\$\$")  # 可変変数
Grep("shell_exec|passthru|system\s*\(|popen\s*\(|proc_open")
Grep("move_uploaded_file.*\.\.\\/")
```

**JSP/Java:**
```
Grep("Runtime\.getRuntime\(\)\.exec")
Grep("ProcessBuilder")
Grep("ScriptEngine.*eval")
```

**ASP/ASPX:**
```
Grep("eval\s*\(\s*Request")
Grep("Execute\s*\(\s*Request")
Grep("cmd\.exe|powershell")
```

**Python:**
```
Grep("exec\s*\(\s*request\.")
Grep("os\.system\s*\(\s*request\.")
Grep("subprocess.*request\.")
Grep("__import__\s*\(.*request")
```

### Step 2: 難読化検出

多重エンコーディングのパターン:

```
Grep("base64_decode\s*\(\s*base64_decode")  # 二重 Base64
Grep("gzinflate\s*\(\s*base64_decode")       # 圧縮 + Base64
Grep("str_rot13\s*\(\s*base64_decode")        # ROT13 + Base64
Grep("\\\\x[0-9a-fA-F]{2}")                  # 16進数エンコード文字列
Grep("chr\s*\(\s*\d+\s*\)\s*\.\s*chr")       # chr() 連結
Grep("pack\s*\(\s*['\"]H\*['\"]")            # pack() による難読化
```

エントロピー指標:
- 長い1行の文字列（200文字超）が含まれるファイル
- ASCII 印字可能文字のみで構成された長い文字列

### Step 3: 不正配置の検出

ファイルが本来あるべきでない場所にないか:

```
# 画像ディレクトリ内のスクリプト
Glob("**/images/**/*.php", "**/img/**/*.php", "**/uploads/**/*.php")
Glob("**/images/**/*.jsp", "**/uploads/**/*.jsp")
Glob("**/images/**/*.asp*", "**/uploads/**/*.asp*")

# 二重拡張子
Glob("**/*.jpg.php", "**/*.gif.php", "**/*.png.php")
Glob("**/*.jpg.jsp", "**/*.gif.asp*")

# 隠しファイル
Glob("**/.[!.]*\.php", "**/.[!.]*\.jsp")
```

### Step 4: GIF/画像ヘッダ偽装の検出

画像ファイルとして偽装されたスクリプトを検出:

```
# GIF89a ヘッダ + PHP コード
Grep("GIF89a.*<\?php", glob: "**/*.gif")

# JPEG/PNG ヘッダの後にスクリプトが続くファイル
# → Read で先頭数行を確認し、マジックバイト後にコードがあるかチェック
```

### Step 5: 404 偽装パターンの検出

```
Grep("die\s*\(\s*['\"]404")
Grep("header\s*\(\s*['\"]HTTP.*404")
Grep("REQUEST_METHOD.*POST.*eval")
```

GET リクエストでは 404 を返し、POST でペイロードを実行するパターン。

### Step 6: タイムスタンプ異常の検出

Bash で最終更新日時が他のファイルと大きく異なるファイルを検出:

```bash
# 最近変更されたファイル（他のファイルが古い場合に有効）
find . -name "*.php" -newer [基準ファイル] -type f

# 極端に古い・新しいタイムスタンプ
find . -name "*.php" -printf '%T+ %p\n' | sort
```

### Step 7: 出力

検出結果を以下の形式で報告:

| 深刻度 | ファイル | 検出パターン | 詳細 |
|--------|---------|------------|------|
| Critical | `uploads/img/gifimg.php` | 404偽装 + eval(base64_decode) | POST で任意コード実行 |
| High | `tmp/.cache.php` | 隠しファイル + shell_exec | シェルコマンド実行 |
| Medium | `images/logo.gif` | GIF89a + PHP タグ | 画像偽装スクリプト |

各検出項目に対して:
- **難読化の解読結果**（該当する場合）: 実際に実行されるコードを提示
- **Web 到達性**: URL から直接アクセス可能か
- **推奨対処**: 削除、隔離、調査継続
