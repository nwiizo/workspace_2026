# eframe / egui

**egui** は Rust の GUI ライブラリで、**eframe** はそれをデスクトップアプリとして動かすためのフレームワークです。yasume ではこの組み合わせで、画面の隅に浮かぶ透明なタイマーオーバーレイを実現しています。

## 普通の GUI と何が違うか — Immediate Mode

一般的な GUI フレームワーク（Qt, GTK, SwiftUI など）は **Retained Mode** と呼ばれる方式で、ボタンやラベルといった UI 要素をオブジェクトとして保持し、状態が変わったときにイベントで更新します。

egui は **Immediate Mode** という異なるアプローチを取ります:

- **毎フレーム、UI 全体を最初から組み立て直す**
- UI 要素はオブジェクトとして保持しない
- 「今この瞬間の状態」をもとに、描画コードがそのまま UI になる

```
// Retained Mode のイメージ（egui ではない）
let button = Button::new("Start");  // ボタンオブジェクトを作る
button.on_click(|| start_timer());  // イベントハンドラを登録

// Immediate Mode（egui のやり方）
if ui.button("Start").clicked() {   // 描画と判定が同時に起きる
    start_timer();
}
```

この仕組みのおかげで、**アプリの状態と UI が常に同期**します。ただし、描画に必要な状態は `App` 構造体に自分で保持する必要があります。

## このプロジェクトでの役割

yasume の GUI は `PomodoroApp` 構造体に `eframe::App` trait を実装しています。eframe が毎フレーム `update()` を呼び出し、その中で以下をすべて行います:

1. IPC コマンドの受信・処理
2. タイマーの `tick()`（残り時間の更新）
3. 円弧やテキストの描画
4. 通知の発火判定

### 透明オーバーレイウィンドウ

yasume の特徴は「透明で、他のウィンドウの上に常時表示される」ことです。以下の設定でこれを実現しています:

```rust
let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_transparent(true)       // ウィンドウ背景を透明にする
        .with_decorations(false)      // タイトルバー・枠線を消す
        .with_always_on_top()         // 常に最前面に表示
        .with_mouse_passthrough(true), // クリックを下のウィンドウに透過させる
    ..Default::default()
};
```

`clear_color` で `TRANSPARENT` を返すことで、egui の描画領域以外が完全に透明になります。

### マウスパススルーの動的切替

普段はクリックを透過させつつ、タイマー上にマウスが乗ったときだけ操作できるようにしています:

```rust
// マウスがタイマー領域にあるかどうかで、パススルーを切り替える
ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(!self.is_hovered));
```

**注意:** `hover_pos()` はマウスがウィンドウ外にあると `None` を返します。`is_some_and()` でガードしないとパニックの原因になります。

### ドラッグ移動

タイトルバーを消しているため、ドラッグでウィンドウを移動する仕組みを自前で実装しています:

```rust
// 指定した領域をドラッグ可能にする
let drag_response = ui.allocate_rect(drag_rect, egui::Sense::click_and_drag());
if drag_response.dragged() {
    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
}
```

### カスタム描画（円弧）

タイマーの残り時間を円弧（アーク）で表示しますが、egui には円弧のプリミティブがありません。そこで、三角関数で頂点を計算し、折れ線で近似しています:

```rust
fn draw_arc(painter, center, radius, start_angle, sweep, stroke) {
    // sweep（弧の角度）に応じてセグメント数を決定
    let segments = (sweep.abs() / TAU * 64.0).max(1.0) as usize;
    // 各セグメントの端点を計算して Shape::line で描画
}
```

### CJK フォント読み込み

日本語を表示するため、macOS のシステムフォントを直接読み込みます:

```rust
const CJK_FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
    // フォールバック用のパスが続く...
];
```

`FontDefinitions` の `families` に `push` すると、egui のデフォルトフォントで表示できない文字のフォールバックとして機能します。

### PomodoroApp 構造体 — 状態の保持

Immediate Mode では UI を毎フレーム作り直すため、アプリの状態は構造体に保持します。`src/app.rs` の `PomodoroApp` がそれにあたります:

```rust
pub struct PomodoroApp {
    timer: Timer,              // タイマーの状態（残り時間、フェーズなど）
    history: History,          // 完了したセッションの履歴
    is_hovered: bool,          // マウスがタイマーの上にあるか
    task_input: String,        // タスク名入力欄のテキスト
    cmd_rx: CommandReceiver,   // IPC コマンド受信チャネル（CLI からのコマンドを受け取る）
    _cmd_tx: CommandSender,    // IPC コマンド送信チャネル（Drop 防止のため保持）
    should_quit: bool,         // 終了フラグ
    lang: Lang,                // 現在の言語（Ja or En）
}
```

`_cmd_tx` に `_` プレフィックスが付いているのは「直接は使わないが、Drop させないために保持している」という意味です。これがドロップされると IPC リスナースレッドのチャネルが切れてしまいます。

### ボタンの実装例

egui にはスタイル付きボタンがないため、`allocate_rect` + `Sense::click()` で自前のボタンを作っています:

```rust
// 四角い領域を確保して、クリック可能にする
let response = ui.allocate_rect(rect, egui::Sense::click());

// ホバー時に背景色を変える
let bg = if response.hovered() {
    Color32::from_rgba_premultiplied(255, 255, 255, 35)
} else {
    Color32::TRANSPARENT
};

// 背景とテキストを手動で描画
ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, ...);

// クリックされたかを判定
if response.clicked() {
    // ボタンが押された時の処理
}
```

## 注意点

- **毎フレーム再構築**: UI の状態は `PomodoroApp` 構造体に保持する。UI 要素自体は毎フレーム作り直される
- **repaint の制御**: タイマー動作中は `request_repaint()`（毎フレーム再描画）、停止中は `request_repaint_after(100ms)` で CPU 使用率を抑える
- **安全な終了**: `should_quit` フラグを設定してから次フレームで `ViewportCommand::Close` を送る。即座に Close すると描画途中でクラッシュする可能性がある
- **wgpu Metal**: macOS では `wgpu` の Metal バックエンドが必要。`Cargo.toml` の features に明示指定
