# herdr.nvim Implementation Plan

Date: 2026-07-21

Design: [`../specs/2026-07-21-herdr-nvim-design.md`](../specs/2026-07-21-herdr-nvim-design.md)

## Goal

Deliver the approved first release of `herdr.nvim`: a dependency-free Neovim control surface that uses Herdr for persistent Codex and Claude Code agents, routes attention through a live board and notifications, attaches to agent terminals, and sends bounded editor context.

## Constraints

- Keep Herdr as the only persistent state and PTY owner.
- Support Neovim 0.10+, Herdr 0.7.4+, macOS, and Linux.
- Use argv arrays for every process invocation; never invoke a shell with user data.
- Define no global keymaps.
- Do not stop Herdr or close agent panes during Neovim cleanup.
- Preserve compatibility with unknown response fields and reject malformed required data atomically.

## Work Breakdown

### 1. Runtime and test foundation

- Add `plugin/herdr.lua` command registration.
- Add `lua/herdr/init.lua` as the public facade.
- Add a dependency-free headless test harness under `tests/`.
- Add fixture helpers that can replace process calls without executing a shell.
- Add formatting configuration and stable `make test` / `make check` entry points.

Verification:

```sh
nvim --headless --clean -u tests/minimal_init.lua -l tests/run.lua
stylua --check lua plugin tests
```

### 2. Configuration and Herdr client

- Implement copied deep-merge defaults and precise validation errors.
- Implement `herdr api snapshot` decoding and normalization.
- Implement required-field checks for agents and safe optional display fields.
- Implement asynchronous `agent start` and atomic `pane run` submission wrappers.
- Implement detached `herdr server` startup with a single shared attempt.
- Centralize timeout, stderr normalization, and callback scheduling.

Tests:

- default and custom configuration;
- caller table immutability;
- invalid enum/range/type errors;
- snapshot success, unknown fields, malformed JSON, and missing fields;
- exact argv for snapshot/start/send;
- command timeout and process failure;
- shared bounded server startup.

### 3. State, polling, events, and notifications

- Store only the latest valid normalized snapshot.
- Index agents by `terminal_id` and retain Herdr order.
- Compute status counts and attention ordering.
- Detect semantic transitions without notifying on bootstrap.
- Emit `HerdrUpdated` and `HerdrAgentStatusChanged` user events.
- Poll at board/background intervals without overlapping requests.
- Preserve last-good state and mark stale failures.
- Notify configured `blocked` and `done` transitions, including newly observed attention states after bootstrap.

Tests:

- bootstrap behavior;
- transitions, revisions, removal, and reappearance;
- ordering/grouping/counts;
- stale-state retention;
- notification deduplication;
- timer interval switching and cleanup.

### 4. Editor context and send flows

- Resolve project-relative paths.
- Format file references, ranges, visual selections, and diagnostics.
- Enforce byte and line ceilings before starting a process.
- Reject unnamed or modified buffers for file-reference sending.
- Select a target automatically for one agent or through `vim.ui.select()` for several.
- Prompt for plain instruction text through `vim.ui.input()`.

Tests:

- Unicode-safe byte limits;
- one-based line/column formatting;
- partial-line visual selections;
- diagnostic severity/source/code formatting;
- zero, one, and multiple target flows;
- empty, unnamed, modified, and oversized inputs.

### 5. Board and terminal attachment

- Render the scratch-buffer board grouped by workspace.
- Apply state highlight groups without requiring a Nerd Font.
- Preserve selection by `terminal_id` across refreshes.
- Add buffer-local mappings and inline help.
- Reuse an existing valid attach buffer.
- Open native right/left terminal splits at the configured width.
- Implement explicit bang-only takeover.
- Release terminal bookkeeping safely on buffer deletion and exit.

Tests:

- buffer options and deterministic rendered lines;
- attention ordering and empty/error/stale views;
- local mappings without global mappings;
- selection preservation;
- terminal command argv, reuse, invalid buffer recovery, and takeover.

### 6. Commands, health, and documentation

- Register every approved Ex command with completion and range/bang behavior.
- Add `vim.health` checks for versions, server state, agents, and integrations.
- Document lazy.nvim installation, setup, commands, board keys, statusline usage, and limitations.
- Mark the plugin as an unofficial Herdr community integration.
- Add `herdr.nvim` to `tools/CLAUDE.md`.

Tests:

- command existence and argument parsing;
- health behavior with available and missing executables;
- setup idempotency and cleanup.

### 7. Final verification

- Run formatting and the complete headless suite.
- Run `:checkhealth herdr` headlessly where possible.
- Exercise a read-only snapshot against the installed Herdr 0.7.4 server.
- Run a manual disposable-agent smoke test only if it can be cleaned up without touching existing agents.
- Review the complete diff for scope, security, lifecycle leaks, and user-owned changes.

## Definition of Done

- All design success criteria are implemented or explicitly reported as a verified platform limitation.
- Automated tests and formatting pass from a clean Neovim process.
- No external plugin is required at runtime or in tests.
- Existing repository changes outside the approved files remain untouched.
