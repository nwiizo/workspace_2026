# serde / serde_json

**serde** は Rust の構造体を JSON やその他のフォーマットに変換（シリアライズ）し、逆に JSON から構造体に復元（デシリアライズ）するライブラリです。yasume では CLI ↔ GUI 間の IPC 通信と、履歴ファイルの保存・読み込みに使っています。

## serde の基本的な考え方

Rust の構造体と JSON の対応を `derive` マクロで自動生成します:

```rust
#[derive(Serialize, Deserialize)]  // この2行を付けるだけで JSON 変換が可能に
struct Task {
    name: String,
    duration: u64,
}

// 構造体 → JSON（シリアライズ）
let task = Task { name: "設計".into(), duration: 25 };
let json = serde_json::to_string(&task)?;
// → {"name":"設計","duration":25}

// JSON → 構造体（デシリアライズ）
let task: Task = serde_json::from_str(&json)?;
```

## このプロジェクトでの役割

### IPC コマンドプロトコル — タグ付き enum

CLI から GUI に送るコマンドは **enum** で定義されています。serde の `#[serde(tag = "command")]` を使うと、enum のどの variant かを JSON のフィールドで判別できます:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "command")]  // ← "command" フィールドで variant を判別する
pub enum Command {
    #[serde(rename = "start")]    // ← JSON 上の値は "start"（小文字）
    Start,
    #[serde(rename = "set_task")]
    SetTask { name: String },     // ← variant にフィールドがあれば JSON にも含まれる
    #[serde(rename = "list")]
    List { from: Option<String>, to: Option<String> },
}
```

上の定義から生成される JSON:

```
Rust の値                        JSON 表現
─────────────────────────────    ──────────────────────────────────────────────
Command::Start                 → {"command":"start"}
Command::SetTask {name: ".."}  → {"command":"set_task","name":"設計書レビュー"}
Command::List {from, to}       → {"command":"list","from":"2026-02-10","to":"2026-02-14"}
```

`#[serde(tag = "...")]` がないと `{"Start": null}` のような外部タグ形式になり、パースしにくくなります。

### レスポンス

`Option` フィールドに `skip_serializing_if` を付けると、`None` のときにフィールド自体を省略できます。`src/ipc.rs` の実際のコード:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,                                          // 成功/失敗
    #[serde(skip_serializing_if = "Option::is_none")]      // None なら JSON に含めない
    pub message: Option<String>,                           // メッセージ（"開始" など）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusInfo>,                        // status コマンド用の詳細情報
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<CompletedTask>>,               // history コマンド用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ReportInfo>,                        // report コマンド用
}

// ok: true, message: None, 他も None の場合 → {"ok":true}
// ok: true, message: Some("開始")  の場合   → {"ok":true,"message":"開始"}
```

ヘルパーメソッドで Response の生成を簡潔にしています:

```rust
impl Response {
    // 成功レスポンスを作る。impl Into<String> で &str でも String でも受け取れる
    pub fn ok(message: impl Into<String>) -> Self {
        Self { ok: true, message: Some(message.into()), status: None, history: None, report: None }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self { ok: false, message: Some(message.into()), status: None, history: None, report: None }
    }
}
```

### 履歴ファイル

完了したセッションの履歴は JSON ファイルとして保存されます:

```rust
// 保存: 構造体 → 整形された JSON 文字列 → ファイル書き込み
serde_json::to_string_pretty(&history)?;

// 読み込み: ファイル → JSON 文字列 → 構造体に復元
// ファイルが壊れていてもクラッシュさせず、空の履歴で続行する
serde_json::from_str(&data).unwrap_or_default()
```

## 注意点

- **`#[serde(tag = "command")]`（内部タグ形式）**: enum の variant 名が JSON の `"command"` フィールドの値になる。IPC プロトコルのように「1つのフィールドで種類を判別」したいときに便利
- **`#[serde(rename = "...")]`**: Rust の PascalCase（`SetTask`）を JSON の snake_case（`set_task`）に変換。これがないと variant 名がそのまま使われる
- **`DateTime<Local>` の serde**: chrono の `features = ["serde"]` を有効にすると自動実装される。ISO 8601 + タイムゾーンオフセット形式でシリアライズ
- **`unwrap_or_default`**: 履歴ファイルが壊れていても空の `History` で復帰する。データロスよりクラッシュ回避を優先する設計判断
- **改行区切り JSON（IPC）**: `writeln!` で1行1JSON。`BufReader::lines()` で読む。JSON 内に改行が入ると壊れるので注意
