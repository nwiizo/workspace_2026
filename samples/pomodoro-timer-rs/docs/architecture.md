# アーキテクチャ

yasume は **macOS 上に常時表示される透明なポモドーロタイマー**です。画面の隅に浮かぶ円形のオーバーレイとして動作し、CLI からリモート操作できます。

このドキュメントでは、プロジェクト全体の構造とデータの流れを説明します。個別のクレートやモジュールの詳細は、対応するドキュメントを参照してください。

## 初めて読む人へ

yasume は **2つのプログラム** で構成されています:

1. **`yasume`（GUI）** — 画面にタイマーを表示するアプリ。バックグラウンドで常時動作する
2. **`yasume-ctl`（CLI）** — ターミナルからタイマーを操作するコマンドラインツール

この2つは **Unix ドメインソケット**（→ [unix-socket-ipc.md](unix-socket-ipc.md)）で通信します。CLI がコマンドを送ると、GUI がそれを受け取って実行し、結果を返します。

## モジュール構成

```
src/
├── lib.rs           # クレートルート（各モジュールを外部に公開する入口）
├── main.rs          # GUI エントリポイント (yasume バイナリ)
├── app.rs           # egui App 実装（GUI の描画ロジックと IPC ハンドラ）
├── ctl.rs           # CLI エントリポイント (yasume-ctl バイナリ)
├── timer.rs         # タイマーロジック（状態遷移、フェーズ管理）
├── history.rs       # 完了したセッションの永続化とクエリ
├── ipc.rs           # CLI ↔ GUI 間の通信プロトコル定義とソケットリスナー
├── notification.rs  # macOS 通知（フェーズ完了、深夜警告など）
└── i18n.rs          # 多言語対応（日本語/英語の全テキスト定義）
```

**各モジュールの役割:**

- **`app.rs`** が中心。毎フレーム呼ばれる `update()` の中で、タイマーの進行・IPC コマンドの処理・画面描画をすべて行う
- **`timer.rs`** は純粋なロジック層。GUI に依存せず、「今何秒残っているか」「どのフェーズか」だけを管理する
- **`ipc.rs`** は CLI と GUI を繋ぐパイプ。JSON 形式のメッセージをやり取りする
- **`history.rs`** は完了したセッションをファイルに保存し、日報やレポートに使う
- **`i18n.rs`** は UI と通知のテキストを日本語/英語で切り替える仕組み

**GUI のメインループ（`app.rs` の `update()`）** は以下の順序で処理を行います。これがアプリの「心臓部」です:

```rust
// eframe::App trait の実装。毎フレーム（≒毎秒60回以上）呼ばれる
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // 1. 終了フラグが立っていたらウィンドウを閉じる
    if self.should_quit {
        ctx.send_viewport_cmd(ViewportCommand::Close);
        return;
    }

    // 2. CLI から届いた IPC コマンドを処理する（try_recv でブロックしない）
    self.process_ipc_commands(ctx);

    // 3. タイマーを1フレーム進める。時間切れなら通知を発火
    if let Some(completed_phase) = self.timer.tick() {
        self.record_completion(completed_phase);           // 履歴に記録
        notify_phase_complete(completed_phase, ...);       // 通知を送信
        if self.timer.should_auto_start(completed_phase) { // 自動開始の判定
            self.timer.start();
        }
    }

    // 4. 再描画のスケジュール（Running 中は毎フレーム、それ以外は100ms間隔）
    if self.timer.status == TimerStatus::Running {
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    // 5. UI を描画（円弧、テキスト、ボタンなど）
    egui::CentralPanel::default().show(ctx, |ui| {
        // ... 描画コード
    });
}
```

## データフロー

CLI からコマンドを送ると、Unix ソケット経由で GUI に届き、GUI が処理結果を JSON で返します。

```
┌─────────────┐     Unix Socket      ┌──────────────┐
│ yasume-ctl  │ ──── JSON/行 ────→   │   yasume      │
│  (CLI)      │ ←── JSON/行 ────    │   (GUI)       │
└─────────────┘                      │               │
                                     │ ┌───────────┐ │
                                     │ │ Timer     │ │
                                     │ │ (状態機械)│ │
                                     │ └───────────┘ │
                                     │ ┌───────────┐ │
                                     │ │ History   │ │
                                     │ │ (JSON)    │ │
                                     │ └───────────┘ │
                                     └──────────────┘
```

**具体例:** `yasume-ctl start` を実行すると:
1. CLI が `{"command":"start"}` を `/tmp/yasume.sock` に送信
2. GUI の IPC リスナースレッドが受信し、`mpsc` チャネルでメインスレッドに転送
3. `app.rs` の `update()` 内で `Timer::start()` が呼ばれ、タイマーが開始
4. `{"ok":true}` がレスポンスとして CLI に返る

## Timer 状態遷移

タイマーは 4 つの状態を持つステートマシンです:

```
Idle ──start()──→ Running ──tick()完了──→ Finished
 ↑                  │                       │
 │      pause()     │         start()       │
 │         ↓        │          (advance)    │
 │       Paused     │              ↓        │
 │         │        │          Running      │
 │    start()       │                       │
 │         ↓        │                       │
 │       Running    │                       │
 │                  │                       │
 ←───reset()────────┴───────────────────────┘
```

- **Idle**: 初期状態。`start()` で開始
- **Running**: カウントダウン中。毎フレーム `tick()` で残り時間が減る
- **Paused**: 一時停止中。`start()` で再開
- **Finished**: 時間切れ。通知が鳴り、次のフェーズに進む（`start()` で次フェーズの Running へ）

`src/timer.rs` の `start()` メソッドの実際のコード。`match` で現在の状態に応じた振る舞いを分けています:

```rust
pub fn start(&mut self) {
    match self.status {
        // Idle, Paused, Finished のいずれかなら開始可能
        TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Finished => {
            if self.status == TimerStatus::Finished {
                // Finished からの start は「次のフェーズに進む」を意味する
                self.advance_phase();      // Work→ShortBreak など
                self.elapsed = Duration::ZERO;
            }
            if self.phase_started_at.is_none() {
                self.phase_started_at = Some(Local::now());  // 開始時刻を記録
            }
            self.status = TimerStatus::Running;
            self.last_tick = Some(Instant::now());  // 経過時間の基点
        }
        TimerStatus::Running => {} // すでに Running なら何もしない
    }
}
```

`tick()` は毎フレーム呼ばれ、時間切れになったら完了したフェーズを `Some` で返します:

```rust
pub fn tick(&mut self) -> Option<TimerPhase> {
    if self.status != TimerStatus::Running { return None; }  // Running 以外は何もしない

    self.accumulate_elapsed();                     // 前回の tick からの経過時間を加算
    self.last_tick = Some(Instant::now());

    if self.elapsed >= self.total_duration {       // 時間切れ？
        self.status = TimerStatus::Finished;
        let finished_phase = self.phase;
        if finished_phase == TimerPhase::Work {
            self.completed_sessions += 1;          // Work 完了回数をインクリメント
        }
        return Some(finished_phase);               // 完了したフェーズを呼び出し元に返す
    }
    None  // まだ時間がある
}
```

## フェーズサイクル

ポモドーロ・テクニックに基づき、作業と休憩を繰り返します:

```
Work → ShortBreak → Work → ShortBreak → Work → ShortBreak → Work → LongBreak → (リセット)
  1                   2                   3                   4
```

4回の Work セッション後に LongBreak。LongBreak 完了で `completed_sessions` がリセットされ、最初に戻ります。

## IPC プロトコル

通信は **1行1JSON、改行区切り** のシンプルなテキストプロトコルです。リクエストとレスポンスは同一コネクション内で同期的にやり取りされます。詳細は [serde-json.md](serde-json.md) を参照。

**コマンド一覧:**

| command | fields | 説明 |
|---------|--------|------|
| `start` | - | タイマー開始 |
| `pause` | - | 一時停止 |
| `reset` | - | リセット |
| `skip` | - | 次フェーズへ |
| `status` | - | 状態取得 |
| `set_task` | `name` | タスク名設定 |
| `clear_task` | - | タスク名クリア |
| `set_times` | `work`, `short_break`, `long_break`, `auto_start_*` | 設定変更 |
| `history` | `today_only` | 履歴取得 |
| `report` | `week` | レポート取得 |
| `quit` | - | アプリ終了 |
| `list` | `from`, `to` | 日付範囲で履歴一覧 |
| `set_lang` | `lang` | 言語切替 (Ja/En) |

## i18n 設計

日本語をデフォルトとし、英語に切り替え可能です:

- `Lang` enum（`Ja` / `En`）→ `strings(lang)` で対応する `&'static Strings` を取得
- GUI 側: `PomodoroApp.lang` に保持し、すべての UI テキストと通知文で参照
- CLI 側: `status` レスポンスに `lang` を含めて、サーバーの現在の言語設定を取得可能
- 切り替え: `yasume-ctl lang ja` / `yasume-ctl lang en`

## ファイルパス

| 用途 | パス | 備考 |
|------|------|------|
| ソケット | `/tmp/yasume.sock` | CLI ↔ GUI 通信用 |
| 履歴 | `~/Library/Application Support/yasume/history.json` | [dirs](dirs.md) クレートで取得 |
| バンドル | `yasume.app/Contents/MacOS/yasume` | `make bundle` で生成 |
