# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Quality

```sh
cargo fmt && cargo clippy -- -D warnings && cargo test
```

Run a single test: `cargo test <test_name>`

Run the GUI: `cargo run --release --bin yasume`

macOS `.app` bundle: `make bundle` / `make install`

## Architecture

**yasume** is a transparent macOS Pomodoro timer overlay (egui/eframe + wgpu Metal). Two binaries communicate via Unix domain socket IPC (`/tmp/yasume.sock`):

- `yasume` (GUI) — `src/main.rs` → `src/app.rs` (`PomodoroApp`). Owns `Timer`, `History`, IPC listener, and `Lang` state. The eframe `update()` loop processes IPC commands, ticks the timer, records completions, and triggers notifications.
- `yasume-ctl` (CLI) — `src/ctl.rs`. Sends JSON `Command` over socket, prints `Response`. Auto-launches GUI on `start` if not running.

Library modules (`src/lib.rs` re-exports):
- `timer` — `Timer` state machine (Idle→Running→Paused→Finished), `TimerPhase` cycling (Work×4→LongBreak), `TimerConfig`
- `ipc` — `Command`/`Response` serde enums, `StatusInfo`/`ReportInfo`, socket listener thread with mpsc channel bridge to main thread
- `i18n` — `Lang` enum (Ja default / En), `Strings` struct with all UI text, `strings(lang)` returns `&'static Strings`
- `history` — `CompletedTask` records persisted as JSON to `~/Library/Application Support/yasume/history.json`, date-range filtering
- `notification` — macOS notifications via `notify-rust` + `afplay` sounds, late-night (22:00+) and overwork escalation warnings

## Constraints

- Rust edition 2024, requires Rust 1.85+
- macOS only (wgpu Metal backend, `afplay` for sounds, CJK fonts from `/System/Library/Fonts/`)
- Japanese is the default language (`Lang::Ja`)

## Lessons Learned

### egui API
- `CornerRadius::same()` takes `u8`, not `f32` — use `const CORNER_RADIUS: u8 = 14;`
- `painter.rect_stroke()` requires 4th arg `egui::StrokeKind::Outside` (or `Inside`/`Middle`)
- Unicode glyphs (`\u{2715}` ✕) may not render in default/CJK fonts — use `\u{00D7}` (×) for close, `\u{2713}` (✓) for checkmark

### Clippy patterns
- `hour >= 22 || hour < 5` → `!(5..22).contains(&hour)` (manual_range_contains)
- Nested `if x { if let Some(y) = ... }` → `if x && let Some(y) = ...` (collapsible_if)
- `.map(f).flatten()` → `.and_then(f)` (map_flatten)

### IPC workflow
- CLI binary can be updated independently, but if `Command` enum changes, GUI must be restarted (`yasume-ctl quit` → relaunch)
- Always rebuild + restart GUI when adding new IPC commands

### GUI design
- Non-hovered state should be whisper-quiet (low alpha BG, muted colors, no glow, no border)
- Hovered state restores full interactivity (opaque BG, glow, colored border, controls)
- Use idle/hover color variants: `TEXT_COLOR` vs `TEXT_COLOR_IDLE`, `RING_BG_COLOR` vs `RING_BG_COLOR_IDLE`
- Japanese text: use `chars().count()` / `chars().take(n)` for truncation, never byte slicing

### i18n
- Adding a field to `Strings` requires updating both `JA` and `EN` const blocks — compiler won't catch missing fields if you only add to the struct
- `min_short` ("分"/"min") is useful as a standalone unit for history/log formatting

## Multi-Agent Strategy

複数タスクは Subagents で並列化する:
- **品質チェック**: rust-quality エージェントに委任（コード変更後は常に実行）
- **egui/eframe 調査**: egui-researcher エージェントに委任
- **IPC デバッグ**: ipc-debugger エージェントに委任
- **コードレビュー**: home-code-reviewer（グローバル）を使用
- **実装計画**: home-planner（グローバル）を使用
