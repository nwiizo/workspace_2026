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

## Multi-Agent Strategy

複数タスクは Subagents で並列化する:
- **品質チェック**: rust-quality エージェントに委任（コード変更後は常に実行）
- **egui/eframe 調査**: egui-researcher エージェントに委任
- **IPC デバッグ**: ipc-debugger エージェントに委任
- **コードレビュー**: home-code-reviewer（グローバル）を使用
- **実装計画**: home-planner（グローバル）を使用
