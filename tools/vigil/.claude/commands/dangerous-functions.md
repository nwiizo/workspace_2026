# Dangerous Functions — 危険関数の全数検査 + Web 到達性判定

対象プロジェクト内の危険関数を網羅的に検出し、各関数が Web リクエストから到達可能かを判定する。

## 実行手順

### Step 1: 言語の特定

プロジェクトで使用されている言語を特定:

```
Glob("**/*.php", "**/*.py", "**/*.rb", "**/*.js", "**/*.ts", "**/*.java", "**/*.go", "**/*.jsp")
```

### Step 2: 言語別の危険関数検索

**PHP — コマンド実行:**
```
Grep("\\b(system|exec|passthru|shell_exec|popen|proc_open|pcntl_exec)\\s*\\(")
Grep("\\b(backtick|`).*\\$")  # バッククォート内の変数展開
```

**PHP — コード実行:**
```
Grep("\\b(eval|assert|create_function|call_user_func|call_user_func_array)\\s*\\(")
Grep("preg_replace\\s*\\(.*['\"]/.*/e")  # /e 修飾子
Grep("\\$\\$")  # 可変変数（間接的コード実行）
```

**PHP — ファイル操作:**
```
Grep("\\b(include|require|include_once|require_once)\\s*\\(.*\\$")
Grep("\\b(file_get_contents|file_put_contents|fopen|readfile|unlink|rmdir)\\s*\\(.*\\$")
Grep("move_uploaded_file")
```

**PHP — 情報漏洩:**
```
Grep("\\b(phpinfo|var_dump|print_r|debug_backtrace)\\s*\\(")
Grep("display_errors.*=.*[Oo]n")
```

**PHP — DB（非プリペアド）:**
```
Grep("mysql_query|mysqli_query.*\\$")
Grep("pg_query.*\\$")
```

**PHP — デシリアライズ:**
```
Grep("\\bunserialize\\s*\\(")
```

**Python — コマンド/コード実行:**
```
Grep("\\b(eval|exec|compile)\\s*\\(")
Grep("os\\.(system|popen)|subprocess\\.(call|run|Popen)")
Grep("__import__\\s*\\(")
```

**JavaScript/TypeScript — コード実行:**
```
Grep("\\beval\\s*\\(")
Grep("new\\s+Function\\s*\\(")
Grep("child_process")
Grep("dangerouslySetInnerHTML")
```

**Go — コマンド実行:**
```
Grep("exec\\.Command")
Grep("os\\.Exec")
```

**Java — コマンド/デシリアライズ:**
```
Grep("Runtime\\.getRuntime\\(\\)\\.exec")
Grep("ProcessBuilder")
Grep("ObjectInputStream|readObject|XMLDecoder")
```

**Ruby — コマンド/コード実行:**
```
Grep("\\b(system|exec|`)\\s*[\\(\"]")
Grep("\\beval\\s*\\(")
Grep("\\bsend\\s*\\(.*params")
```

### Step 3: Web 到達性の判定

検出された各危険関数について、以下のフローで Web 到達性を判定:

```
1. ファイルに shebang (#!/usr/bin/...) があるか？
   → YES: CLI 専用 → Web 到達性: なし

2. ファイルが Web ルートに含まれるか？
   → NO: Web 到達性: なし

3. ファイルがルーティング/コントローラから参照されるか？
   Grep("require.*{filename}|include.*{filename}|import.*{module}")
   → NO: 孤立ファイル → Web 到達性: 低（直接URLアクセスの可能性を確認）

4. ルーティングテーブルに含まれるか？
   → YES: Web 到達性: あり

5. 呼び出し元がコメント内/デッドコードか？
   → YES: Web 到達性: なし（デッドコード）

6. 危険関数の引数にユーザー入力が含まれるか？
   → YES: Web 到達性: あり + ユーザー制御可能
   → NO: Web 到達性: あり（固定引数）
```

### Step 4: 出力

危険関数マトリクスを生成:

| 関数 | ファイル:行 | Web 到達性 | ユーザー入力 | リスク |
|------|-----------|-----------|-------------|--------|
| `eval()` | `lib/parser.php:42` | あり | あり（$_POST['code']） | Critical |
| `system()` | `scripts/backup.sh:10` | なし（CLI） | なし | Low |
| `unserialize()` | `session.php:88` | あり | あり（Cookie） | Critical |
| `exec()` | `admin/tools.php:15` | あり | あり（$_GET['cmd']） | Critical |
| `phpinfo()` | `info.php:1` | あり | なし | Medium |

リスク判定基準:
- **Critical**: Web 到達性あり + ユーザー入力が引数に到達
- **High**: Web 到達性あり + ユーザー入力が間接的に影響
- **Medium**: Web 到達性あり + 固定引数（設定変更リスク）
- **Low**: Web 到達性なし or デッドコード
- **Info**: テスト/開発用コード
