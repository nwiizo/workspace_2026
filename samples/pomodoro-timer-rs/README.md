# yasume

Transparent macOS Pomodoro timer overlay — おい、休め。

## Features

- Semi-transparent dark panel with always-on-top overlay
- Mouse passthrough — click through to apps underneath
- Hover to reveal controls (start/pause, reset, skip, close) and task input
- Drag to reposition
- Circular progress ring with phase-specific colors
- Late night mode (22:00+): amber ring color + escalating notifications
- Overwork escalation (2+/3+/4+ skipped breaks)
- Japanese/English language switching (`yasume-ctl lang ja|en`)
- Customizable durations (default: 25min Work / 5min Break / 15min Long Break)
- Auto-start next phase (configurable)
- Persistent task history with JSON storage
- CLI control via Unix socket (`yasume-ctl`)
- Auto-launch: `yasume-ctl start` launches GUI if not running
- macOS `.app` bundle support

## Build & Run

```sh
cargo run --release --bin yasume

# macOS .app bundle
make bundle
open yasume.app

# Install to /Applications
make install
```

## CLI Control

```sh
# Basic controls
yasume-ctl start          # auto-launches GUI if needed
yasume-ctl pause
yasume-ctl reset
yasume-ctl skip
yasume-ctl quit

# Status
yasume-ctl status

# Task management
yasume-ctl task "Write report"
yasume-ctl task --clear

# Language
yasume-ctl lang ja        # 日本語
yasume-ctl lang en        # English

# History
yasume-ctl list                                        # today
yasume-ctl list --week                                 # this week
yasume-ctl list --from 2026-02-10 --to 2026-02-14     # date range

# Reports
yasume-ctl report
yasume-ctl report --week

# Config
yasume-ctl times --work 50 --short-break 10 --long-break 20
yasume-ctl times --auto-start-work true --auto-start-break true
```

## Quality

```sh
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Requirements

- Rust 1.85+ (edition 2024)
- macOS (wgpu Metal backend)
