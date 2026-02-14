# clap

**clap** は Rust の CLI 引数パーサーです。yasume-ctl（CLI ツール）で、ユーザーが入力したサブコマンドやオプションを解析するために使っています。

## derive マクロによる定義

clap の特徴は、**構造体や enum の定義がそのまま CLI の仕様になる**ことです。`#[derive(Parser)]` マクロを付けると、フィールドの型や属性から自動的に引数パーサーが生成されます:

```rust
#[derive(Parser)]
#[command(name = "yasume-ctl", about = "yasume — control the timer")]
struct Cli {
    #[command(subcommand)]
    command: CtlCommand,  // サブコマンドを持つ
}
```

## このプロジェクトでの役割

### サブコマンドの定義

`yasume-ctl start` や `yasume-ctl task <name>` のようなサブコマンドを enum で定義します:

```rust
#[derive(Subcommand)]
enum CtlCommand {
    /// タイマーを開始する（← この doc comment が --help のテキストになる）
    Start,
    /// タスク名を設定する
    Task {
        name: Option<String>,    // 位置引数（省略可能）: yasume-ctl task "設計"
        #[arg(long)]
        clear: bool,             // フラグ: yasume-ctl task --clear
    },
    /// タイマー時間を設定する
    Times {
        #[arg(long)]
        work: Option<u64>,       // 名前付き引数: yasume-ctl times --work 30
    },
}
```

### CLI コマンドと IPC コマンドの対応

`src/ctl.rs` の `main()` で、clap が解析した `CtlCommand` を IPC 用の `Command` に変換しています:

```rust
let (cmd, use_auto_start) = match cli.command {
    CtlCommand::Start => (Command::Start, true),  // start だけ自動起動を有効に
    CtlCommand::Pause => (Command::Pause, false),
    CtlCommand::Reset => (Command::Reset, false),
    CtlCommand::Task { name, clear } => {
        // --clear フラグがあるか、name が None なら ClearTask
        let cmd = match name {
            Some(name) if !clear => Command::SetTask { name },
            _ => Command::ClearTask,
        };
        (cmd, false)
    }
    // ...
};
```

対応の概要:

```
ユーザーの入力                  clap が解析          IPC に送る Command
───────────────────────────    ─────────────────    ──────────────────────────
yasume-ctl start             → CtlCommand::Start  → Command::Start
yasume-ctl task "設計"        → CtlCommand::Task   → Command::SetTask { name }
yasume-ctl task --clear      → CtlCommand::Task   → Command::ClearTask
yasume-ctl times --work 30   → CtlCommand::Times  → Command::SetTimes { work: 30 }
```

### 引数の種類

| パターン | Rust の型 | 属性 | CLI での使い方 |
|---------|----------|------|---------------|
| フラグ | `bool` | `#[arg(long)]` | `--clear` |
| 名前付き引数 | `Option<T>` | `#[arg(long)]` | `--work 30` |
| 位置引数 | `String` | （属性なし） | `task "設計"` |
| doc comment | — | `///` | `--help` に表示 |

## 注意点

- **`features = ["derive"]`**: `Cargo.toml` で有効にしないと `#[derive(Parser)]` が使えない
- **サブコマンド名の自動変換**: enum の variant 名が自動で kebab-case に変換される（`SetTask` → `set-task`）。本プロジェクトでは1語のサブコマンドのみ使用しているので変換は発生しない
- **`Option<bool>` の罠**: `--auto-start-work true` のように値として渡す必要がある。`--auto-start-work` だけだと `None` になり、直感に反する
- **exit code**: clap の引数パースエラーは自動で `exit(2)`。アプリ側のエラーは `std::process::exit(1)` で明示的に設定
