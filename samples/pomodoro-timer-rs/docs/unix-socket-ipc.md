# Unix ドメインソケット IPC

**IPC（Inter-Process Communication: プロセス間通信）** は、別々に動作する2つのプログラムがデータをやり取りする仕組みです。yasume では、GUI アプリと CLI ツールがこの仕組みで通信します。

## なぜソケットを使うのか

CLI から GUI を操作する方法はいくつか考えられます:

| 方法 | 特徴 | 不採用の理由 |
|------|------|-------------|
| ファイル監視 | ファイルに書き込んで検知 | ポーリングが必要、レスポンスが遅い |
| HTTP サーバー | TCP でリクエスト/レスポンス | ポート番号の管理、オーバーヘッドが大きい |
| **Unix ドメインソケット** | **ファイルパスで接続、低オーバーヘッド** | — |
| D-Bus / XPC | OS のメッセージバス | macOS では過剰、外部依存が増える |

Unix ドメインソケットは **ネットワークを経由しないローカル専用の通信路** です。ファイルシステム上のパス（`/tmp/yasume.sock`）を「住所」として使い、TCP/IP のようなオーバーヘッドなしに高速にデータを送受信できます。`std::os::unix::net` で外部クレート不要で使えるのも利点です。

## このプロジェクトでの役割

GUI（サーバー）がソケットを作成して待ち受け、CLI（クライアント）が接続してコマンドを送り、レスポンスを受け取ります。

```
yasume (GUI)                      yasume-ctl (CLI)
  │                                    │
  ├─ UnixListener::bind()              │  ← GUI がソケットファイルを作成
  │   /tmp/yasume.sock                 │
  │                                    │
  │  ← UnixStream::connect() ─────────┤  ← CLI が接続
  │  ← JSON command (1行) ────────────┤  ← CLI がコマンドを送信
  │  → JSON response (1行) ───────────┤  ← GUI がレスポンスを返す
  │                                    │
  ├─ mpsc::channel() で                │
  │  リスナースレッド → メインスレッド    │  ← 受信したコマンドを GUI に転送
```

### リスナー側（GUI）

GUI は専用のスレッドでソケット接続を待ち受け、受信したコマンドを `mpsc` チャネルでメインスレッドに転送します。以下は `src/ipc.rs` の実際のコードです:

```rust
pub fn start_listener(cmd_tx: CommandSender) {
    // --- 1. stale ソケットの検出と多重起動防止 ---
    if std::path::Path::new(SOCKET_PATH).exists() {
        // 既存のソケットに接続を試みる
        if UnixStream::connect(SOCKET_PATH).is_ok() {
            // 接続できた = 別インスタンスが動作中 → 終了
            eprintln!("Another instance is already running on {SOCKET_PATH}");
            std::process::exit(1);
        }
        // 接続できない = 前回クラッシュの残骸 → 削除して続行
        let _ = std::fs::remove_file(SOCKET_PATH);
    }

    // --- 2. ソケットの作成 ---
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind IPC socket: {e}");
            return;
        }
    };

    // --- 3. 別スレッドで接続を待ち受ける ---
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream { Ok(s) => s, Err(_) => continue };

            // try_clone() が必要: BufReader が stream の所有権を取るため、
            // レスポンス書き込み用に clone を作っておく
            let reader = BufReader::new(stream.try_clone().unwrap());
            let line = reader.lines().next().unwrap().unwrap(); // 1行読む

            // JSON → Command にデシリアライズ
            let cmd: Command = serde_json::from_str(&line).unwrap();

            // レスポンス返送用の一時チャネルを作成
            let (resp_tx, resp_rx) = mpsc::channel();
            cmd_tx.send((cmd, resp_tx)).unwrap();  // GUI スレッドに転送

            // GUI がレスポンスを送り返すのを待つ
            let resp = resp_rx.recv().unwrap();
            writeln!(stream, "{}", serde_json::to_string(&resp).unwrap()).unwrap();
        }
    });
}
```

（実際のコードではエラーハンドリングが入っていますが、流れを理解しやすくするため簡略化しています）

### クライアント側（CLI）

`src/ctl.rs` の実際のコードです。接続→送信→受信のシンプルな流れ:

```rust
fn send_command(cmd: &Command) -> Result<Response, Box<dyn std::error::Error>> {
    // ソケットに接続（GUI が起動していないと Err になる）
    let mut stream = UnixStream::connect(SOCKET_PATH)?;

    // Command を JSON 文字列にして、改行付きで送信
    let json = serde_json::to_string(cmd)?;
    writeln!(stream, "{json}")?;
    stream.flush()?;  // ← これがないとバッファに溜まったまま送信されないことがある

    // GUI からのレスポンスを1行読んでデシリアライズ
    let reader = BufReader::new(&stream);
    let line = reader.lines().next().ok_or("No response")??;
    let resp: Response = serde_json::from_str(&line)?;
    Ok(resp)
}
```

GUI が起動していない場合は `connect()` が失敗するため、自動起動してリトライする仕組みもあります:

```rust
fn send_with_auto_start(cmd: &Command) -> Result<Response, Box<dyn std::error::Error>> {
    match send_command(cmd) {
        Ok(resp) => Ok(resp),  // 接続成功ならそのまま返す
        Err(_) => {
            // GUI が起動していない → yasume バイナリを起動
            if !try_auto_start() {
                return Err("Failed to start yasume".into());
            }
            // 最大3秒（100ms × 30回）リトライ
            for _ in 0..30 {
                thread::sleep(Duration::from_millis(100));
                if let Ok(resp) = send_command(cmd) {
                    return Ok(resp);
                }
            }
            Err("Failed to connect to yasume".into())
        }
    }
}
```

### チャネル連携（GUI 内部）

IPC リスナースレッドと GUI メインスレッドの橋渡しに `mpsc` チャネルを使います:

```rust
// 送信側の型: (受け取ったコマンド, レスポンス返送用の一時チャネル)
pub type CommandSender = mpsc::Sender<(Command, mpsc::Sender<Response>)>;
```

各 IPC 接続ごとに一時的な `mpsc::channel()` を作り、GUI がレスポンスをそこに送り返します。こうすることで、リスナースレッドは GUI の処理完了を待ってからクライアントにレスポンスを返せます。

## 注意点

- **ソケットファイルの後始末**: アプリ終了時に `Drop` で `cleanup_socket()` を呼び、`/tmp/yasume.sock` を削除する。残っていると次回起動時に bind できない
- **stale ソケット対策**: 前回クラッシュでソケットが残る場合がある。起動時に `connect()` を試し、失敗すれば stale と判断して削除する
- **多重起動防止**: `connect()` が成功 = 別インスタンスが動作中 → エラーを表示して `exit(1)`
- **1行プロトコル**: `writeln!` + `lines().next()` でメッセージの区切りを判定。JSON 内に改行を含めてはいけない
- **GUI スレッドとの同期**: egui の `update()` 内で `cmd_rx.try_recv()` を使う。`recv()`（ブロッキング）を使うと GUI がフリーズする
- **自動起動**: CLI で `send_command` が失敗（GUI 未起動）した場合、`yasume` バイナリを `spawn()` して最大3秒リトライする
- **`flush()` 必須**: `writeln!` だけではバッファに溜まったまま送信されないことがある。必ず `flush()` する
