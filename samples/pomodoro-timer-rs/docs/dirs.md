# dirs

**dirs** は、OS ごとに異なる標準ディレクトリのパスを取得するクレートです。yasume では履歴ファイルの保存先を決めるために使っています。

## なぜパスをハードコードしないのか

履歴ファイルの保存先を `/Users/nwiizo/Library/Application Support/yasume/history.json` のようにハードコードすると:

- **他のユーザーで動かない**: ユーザー名がパスに入っている
- **OS が変わると壊れる**: macOS は `~/Library/Application Support/`、Linux は `~/.local/share/` とルールが違う
- **ホームディレクトリが標準と異なる環境で壊れる**: Docker コンテナや CI など

`dirs` クレートを使えば、**実行環境に応じた正しいパスを自動で取得**できます。

## このプロジェクトでの役割

履歴ファイルのパスを OS に適したディレクトリに配置しています:

```rust
fn history_path() -> Option<PathBuf> {
    // dirs が返すベースパスに、アプリ名とファイル名を追加
    dirs::data_local_dir().map(|d| d.join("yasume").join("history.json"))
}
```

### macOS でのパス

| 関数 | 返すパス |
|------|---------|
| `dirs::data_local_dir()` | `~/Library/Application Support` |
| `dirs::config_dir()` | `~/Library/Application Support` |
| `dirs::cache_dir()` | `~/Library/Caches` |

実際の履歴ファイル: `~/Library/Application Support/yasume/history.json`

### 実際の load / save の流れ

`src/history.rs` の実際のコードです。`dirs` で取得したパスをどう使うかを見てみましょう:

```rust
impl History {
    pub fn load() -> Self {
        // dirs::data_local_dir() が None を返す可能性があるので let-else で早期 return
        let Some(path) = history_path() else {
            return Self::default();  // パスが取得できなければ空の履歴で続行
        };
        if !path.exists() {
            return Self::default();  // ファイルがなければ空の履歴
        }
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),  // JSON が壊れていても空で続行
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = history_path() else { return };

        // ★ dirs はパスを返すだけなので、ディレクトリは自分で作る
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&self) {
            Ok(json) => { let _ = std::fs::write(&path, json); }
            Err(e) => eprintln!("Failed to save history: {e}"),
        }
    }
}
```

ポイントは **どの段階でもクラッシュさせない** 設計です。`unwrap()` を使わず、エラーはすべて空の履歴やログ出力で処理しています。

## 注意点

- **`Option` を返す**: 環境変数 `$HOME` が未設定だと `None` になる。必ず `Option` として扱い、`unwrap()` しない
- **ディレクトリは自動で作られない**: `dirs` はパスを返すだけ。ファイル保存前に `create_dir_all()` でディレクトリを作成する必要がある
- **macOS の特殊事情**: `data_local_dir()` と `config_dir()` が同じパス（`~/Library/Application Support`）を返す。将来 `config.json` を追加しても同じ `yasume/` ディレクトリ内に置ける
- **テスト時の注意**: `History::load()` / `save()` は実ファイルに読み書きする。テストでは `History::default()` を使えばファイル I/O をスキップできる
