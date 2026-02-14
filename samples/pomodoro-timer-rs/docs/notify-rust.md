# notify-rust

**notify-rust** は、OS のデスクトップ通知を Rust から送るためのクレートです。yasume ではポモドーロのフェーズ完了時や、深夜の警告を通知するために使っています。

## macOS の通知の仕組み

macOS では、アプリが **通知センター** にメッセージを送ると、画面右上にバナーが表示されます。notify-rust はこの仕組みを Rust から簡単に使えるようにするラッパーです。

ただし、macOS の通知にはいくつかの制約があります:

- **初回の許可が必要**: アプリを初めて実行したとき「通知を許可しますか？」ダイアログが出る。ユーザーが拒否すると通知は表示されない
- **バンドルが必要な場合がある**: `.app` バンドルにしないと通知センターに表示されないことがある。`Info.plist` の `CFBundleIdentifier` が正しく設定されている必要がある

## このプロジェクトでの役割

### 基本的な通知の送信

```rust
notify_rust::Notification::new()
    .summary("おつかれ。")   // 通知のタイトル
    .body("休め。")          // 通知の本文
    .sound_name("Glass")     // macOS のシステムサウンド名
    .show()                  // 通知を送信（Result を返す）
```

### サウンドの使い分け

macOS のシステムサウンドを `.sound_name()` で指定しています:

| サウンド | 用途 | 音のイメージ |
|---------|------|------------|
| `Glass` | フェーズ完了通知 | 軽い通知音 |
| `Sosumi` | 警告（overwork、深夜作業） | 注意を引く音 |

### afplay による追加サウンド — なぜ両方使うのか

`notify-rust` のサウンドは通知バナーと連動しているため、**通知が拒否されている環境では音も鳴りません**。そこで macOS の `afplay` コマンドでも音を鳴らし、通知設定に関係なく確実にユーザーに知らせます。`src/notification.rs` の実際のコード:

```rust
fn play_sound(phase: TimerPhase) {
    // フェーズによって音を変える（作業完了 → Glass、休憩完了 → Tink）
    let sound = match phase {
        TimerPhase::Work => "/System/Library/Sounds/Glass.aiff",
        TimerPhase::ShortBreak | TimerPhase::LongBreak => "/System/Library/Sounds/Tink.aiff",
    };
    // spawn() でバックグラウンド実行。let _ = で Result を無視（音が鳴らなくてもクラッシュさせない）
    let _ = ProcessCommand::new("afplay").arg(sound).spawn();
}
```

### 深夜判定と段階的な警告

`src/notification.rs` では、時間帯に応じて異なるメッセージを出します:

```rust
pub fn notify_late_night(lang: Lang) {
    let hour = Local::now().hour();  // chrono で現在時刻を取得
    let s = strings(lang);           // 現在の言語のテキストを取得

    // 22時台、23時台、0〜4時台で異なるメッセージ。それ以外は何もしない
    let (summary, body) = match hour {
        22 => (s.late_22_summary, s.late_22_body),       // "そろそろいい時間だ。"
        23 => (s.late_23_summary, s.late_23_body),       // "おい、寝ろ。"
        0..=4 => (s.late_0_summary, s.late_0_body),      // "何時だと思ってる。"
        _ => return,  // ← 5〜21時は何もせず return で終了
    };

    // 通知を送信（エラーは eprintln! で出力するだけ）
    if let Err(e) = notify_rust::Notification::new()
        .summary(summary).body(body).sound_name("Sosumi").show()
    {
        eprintln!("Notification failed: {e}");
    }
}

/// 22:00 以降かどうかを判定。GUI の色を変えるのにも使う
pub fn is_late_night() -> bool {
    let hour = Local::now().hour();
    !(5..22).contains(&hour)  // 5時〜21時以外 = 深夜
    // ↑ clippy が hour >= 22 || hour < 5 からこの形に書き換えを推奨する
}
```

- `spawn()` を使うことで音の再生完了を待たずに処理を続行
- `let _ =` で戻り値を無視。音が鳴らなくてもアプリをクラッシュさせない
- 通知音と afplay の両方が鳴ると2回音がするが、**通知ブロック時のフォールバック** として意図的にこうしている

## 注意点

- **エラーハンドリング**: `.show()` の `Err` は `eprintln!` で出力するのみ。通知の失敗でアプリ自体を止めない
- **`LSUIElement`**: `Info.plist` で `true` に設定すると Dock に表示されないバックグラウンドアプリになる。通知は引き続き送信可能
