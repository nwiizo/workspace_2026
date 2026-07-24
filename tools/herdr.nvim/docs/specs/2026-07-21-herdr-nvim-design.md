# herdr.nvim Design

Date: 2026-07-21

Status: Approved for implementation planning

## Summary

`herdr.nvim` is a dependency-free Neovim control surface for persistent coding agents managed by Herdr. The first release supports Codex and Claude Code. It lets a developer see which delegated work needs attention, start and reattach to agents, and send editor context without leaving Neovim.

Herdr remains the process, PTY, session, and semantic agent-state owner. The plugin does not duplicate Herdr persistence or keep a second session database.

## Job to Be Done

When a developer delegates work to multiple coding agents, they need to know which agent requires attention without polling terminal panes, intervene without losing their editing flow, and return to the same live work after Neovim restarts.

The first release succeeds when it supports this loop:

1. Start a named Codex or Claude Code agent for the current project.
2. Continue editing while the agent runs in Herdr.
3. Notice `blocked` or `done` without manually visiting the terminal.
4. Attach to the relevant live agent or send it editor context.
5. Restart Neovim and rediscover the same Herdr-managed agents.

The primary value is attention routing, not merely launching terminal commands.

## Product Scope

### Included in the first release

- Herdr as a required backend.
- Codex and Claude Code launch presets.
- A native Neovim herd board grouped by Herdr workspace.
- Semantic states: `blocked`, `working`, `done`, `idle`, and `unknown`.
- Background notifications for transitions into `blocked` and `done`.
- A statusline function that summarizes state counts.
- Direct terminal attachment to a selected Herdr agent.
- Plain-text instructions sent to a selected agent.
- Current-file, visual-selection, and LSP-diagnostic context sent from Neovim to an agent.
- Health checks for Neovim, Herdr, the Herdr server, Codex, Claude Code, and their Herdr integrations.
- User autocmds for state refresh and agent-state changes.
- Headless, dependency-free automated tests.

### Deferred

- An MCP server or agent-to-Neovim tools.
- Agent-initiated file navigation and diff review.
- Raw Herdr socket event subscriptions.
- Remote Herdr session configuration UI.
- Worktree creation or deletion.
- A plugin-owned chat transcript.
- A plugin-owned persistence database.
- Windows support.

The module boundaries must allow a later MCP bridge without requiring the board, state store, or Herdr client to be replaced.

## Requirements and Compatibility

- Neovim 0.10 or newer, for `vim.system()` and the current Lua APIs.
- Herdr 0.7.4 or newer.
- macOS or Linux.
- At least one of `codex` or `claude` on `PATH` for agent launch.
- Herdr's matching `codex` or `claude` integration is recommended for session identity and richer state behavior.

The plugin is an unofficial community integration and must say so in its README.

## Architecture

The project lives at `tools/herdr.nvim/` and uses focused Lua modules.

```text
herdr.nvim/
├── lua/herdr/
│   ├── init.lua
│   ├── config.lua
│   ├── client.lua
│   ├── state.lua
│   ├── board.lua
│   ├── terminal.lua
│   ├── context.lua
│   ├── notifier.lua
│   └── health.lua
├── plugin/herdr.lua
├── tests/
├── README.md
└── LICENSE
```

### Public entry point

`init.lua` exposes only:

- `setup(opts)`
- `toggle()`
- `start(agent_kind, opts)`
- `attach(target, opts)`
- `send(target, text, opts)`
- `refresh()`
- `statusline()`

Commands call this API instead of reaching into internal modules.

### Configuration

`config.lua` owns defaults, merge behavior, and validation. Invalid enum and numeric values fail during `setup()` with a precise option path. User tables are copied before merging and are never mutated.

### Herdr client

`client.lua` is the only module that invokes Herdr. It uses argv arrays and never constructs shell commands. Bounded commands use `vim.system()`; the long-lived auto-started server uses `vim.fn.jobstart({ "herdr", "server" }, { detach = true })`.

Read state with one `herdr api snapshot` call. The response supplies agents, workspaces, tabs, panes, layouts, focused IDs, server version, and protocol version. Normalize only documented fields needed by the UI, and ignore unknown fields for forward compatibility.

Required agent fields are `terminal_id`, `agent_status`, `workspace_id`, `tab_id`, `pane_id`, `focused`, and `revision`. Optional display fields include `name`, `display_agent`, `agent`, `title`, `foreground_cwd`, and `cwd`. A response missing a required field is rejected without replacing the last valid state.

Mutation methods wrap:

- `herdr agent start`
- `herdr pane run` for atomic text-plus-Enter submission
- `herdr agent attach`

Only one snapshot request may be in flight. Each process has a finite timeout and a callback that is scheduled before using Neovim APIs.

### State store

`state.lua` owns the latest valid normalized snapshot, the prior state per `terminal_id`, refresh timestamps, stale state, and polling timers.

`terminal_id` is the stable internal identity. Names and agent labels are presentation and command-target conveniences, not table keys.

On the first successful snapshot, the store establishes a baseline without notifications. Later refreshes compare semantic status by `terminal_id`. A transition emits `User HerdrAgentStatusChanged`; every accepted snapshot emits `User HerdrUpdated`.

The store does not persist to disk. A new Neovim instance bootstraps from Herdr.

### Herd board

`board.lua` renders a non-modifiable scratch buffer in a native split. It groups rows by `workspace_id` using the workspace label when available. Agents are ordered by attention priority:

1. `blocked`
2. `done`
3. `working`
4. `idle`
5. `unknown`

Within a status group, retain the stable order received from Herdr. The board preserves the selected `terminal_id` across renders when that agent still exists.

Default board-local mappings are:

| Key | Action |
| --- | --- |
| `<CR>` | Attach to selected agent |
| `a` | Start a Codex or Claude Code agent |
| `s` | Send a plain instruction |
| `r` | Refresh immediately |
| `q` | Close the board |
| `?` | Toggle board help |

The plugin defines no global keymaps.

### Terminal attachment

`terminal.lua` tracks one Neovim terminal buffer per Herdr `terminal_id`. Attaching focuses a valid existing buffer or opens a native split running `herdr agent attach <target>`.

Closing or wiping that buffer ends only the attach client. It must not close the Herdr pane or agent process. If another direct-attach client owns input, normal attach reports the conflict and offers `:HerdrAttach!`; takeover occurs only after the explicit bang form.

### Editor context

`context.lua` creates bounded, human-readable prompts:

- Current file: repository-relative or cwd-relative path, without copying the whole file.
- Visual selection: path, one-based line range, filetype, and selected text in a fenced block.
- Diagnostics: path, line and column, severity, source, code when present, and message.

Selection context is limited to 500 lines and 65,536 bytes by default. Diagnostics are limited by the same byte ceiling. Oversized context fails visibly and is never silently truncated. `SendFile` rejects unnamed or unsaved buffers.

Context is sent as one argv element to `herdr pane run`, which submits text plus Enter atomically; no shell quoting is involved. `herdr agent send` is not used because it writes literal text without submitting it.

### Notifications

`notifier.lua` listens to accepted state transitions. By default it uses `vim.notify()` when an agent enters `blocked` or `done`, including an attention-requiring agent first observed after bootstrap. It deduplicates notifications by `terminal_id`, status, and revision. Existing states discovered during the initial bootstrap do not notify.

### Health checks

`health.lua` uses `vim.health` and reports:

- supported Neovim version;
- resolved Herdr executable and client version;
- Herdr server reachability and protocol compatibility;
- resolved Codex and Claude Code executables;
- Herdr integration status for Codex and Claude Code;
- actionable remediation commands where possible.

Health checks are diagnostic only. They do not install or update software.

## User Interface

The default board is a 44-column left split:

```text
Herdr  !1  *2  ✓1

workspace_2026
 ! reviewer   claude  blocked  approval required
 * api        codex   working
 ✓ tests      codex   done

dotfiles
 · docs       claude  idle
```

Default font-independent state markers are `!`, `*`, `✓`, `·`, and `?`; they do not require a Nerd Font. Highlight groups link to existing Neovim diagnostic groups so a colorscheme can control appearance. Users may override markers and highlight links.

`statusline()` returns a plain string such as `H !1 *2 ✓1`. It returns an empty string before the first successful snapshot or when the Herdr executable is missing. After a later refresh failure it retains the last counts and adds `~` to indicate stale data. Statusline evaluation never raises an error.

## Commands

- `:Herdr` toggles the board.
- `:HerdrRefresh` requests an immediate snapshot.
- `:HerdrStart [codex|claude]` starts an agent in the current project root.
- `:HerdrAttach[!] [target]` attaches to an agent; bang explicitly permits takeover.
- `:[range]HerdrSend [target]` sends complete lines from an explicit Ex range; without a range it asks for instruction text with `vim.ui.input()`.
- `:'<,'>HerdrSendVisual [target]` sends the exact character-, line-, or blockwise visual selection. A separate command avoids guessing whether a generic Ex range originated in Visual mode.
- `:HerdrSendFile [target]` sends the current file reference.
- `:HerdrSendDiagnostics [target]` sends diagnostics for the current buffer.
- `:HerdrHealth` opens `:checkhealth herdr`.

When a target is omitted and exactly one agent exists, use it. When several agents exist, use `vim.ui.select()`. When none exist, explain how to start one.

`HerdrStart` determines cwd with `vim.fs.root(0, ".git")`, falling back to the current working directory. It asks for an editable agent name whose default combines the agent kind and project directory. It starts without stealing Herdr UI focus, refreshes after success, and leaves Neovim focus unchanged.

## Refresh and Server Lifecycle

The first plugin operation checks the Herdr server. When `auto_start_server` is enabled and no server is reachable, start `herdr server` as a detached job and retry after 100, 300, and 1,000 ms. Concurrent callers share the same start attempt. After the third failed retry, background polling stops until a later explicit action or refresh.

Refresh intervals are:

- 1,000 ms while the board is visible;
- 5,000 ms while the board is hidden.

Polling is suspended when Neovim is exiting. Overlapping snapshot requests are skipped. Closing Neovim stops plugin timers and attach clients but never invokes `herdr server stop`, `pane close`, or any agent-closing command.

Raw socket event subscriptions are deferred. The polling boundary remains inside `state.lua` so a later subscriber can replace polling without changing UI consumers.

## Default Configuration

```lua
require("herdr").setup({
  herdr_cmd = "herdr",
  auto_start_server = true,

  agents = {
    codex = { command = { "codex" } },
    claude = { command = { "claude" } },
  },

  refresh = {
    board_ms = 1000,
    background_ms = 5000,
    timeout_ms = 3000,
  },

  board = {
    side = "left",
    width = 44,
  },

  terminal = {
    side = "right",
    width = 0.4,
    auto_insert = true,
  },

  notifications = {
    blocked = true,
    done = true,
  },

  context = {
    max_lines = 500,
    max_bytes = 65536,
  },
})
```

## Failure Handling

- Missing Herdr executable: notify once, stop polling, and render installation guidance when the board opens.
- Unreachable server: auto-start once when enabled; otherwise render the exact server-start command.
- Server start failure: perform bounded retries, report stderr, and do not enter a restart loop.
- Snapshot timeout or failure: keep the last valid state, mark it `stale`, and avoid repeated identical notifications.
- Invalid JSON or missing required fields: reject the response atomically and keep the last valid state.
- Unknown response fields: ignore them.
- Unsupported protocol: report through health and actions; do not guess at response shapes.
- Mutation failure: show concise stderr and leave the board and selection intact.
- Attach ownership conflict: never take over automatically.
- Oversized context: reject before invoking Herdr.
- No diagnostics: report an informational message instead of sending an empty prompt.
- Callback after buffer/window deletion: check validity and exit without error.

## Security and Data Handling

- All external commands use argv arrays.
- User text, file paths, and diagnostics are never interpolated into shell code.
- The plugin writes no prompts, terminal output, or agent state to disk.
- Logs exclude prompt and selection bodies by default.
- Health output may show executable paths and versions but not environment variables or tokens.
- No command automatically installs integrations, modifies Herdr configuration, or takes over another terminal client.

## Testing Strategy

Tests run with headless Neovim and require no testing plugin at runtime. A small test harness provides assertions and module reset helpers. A fake Herdr executable supplies deterministic JSON and records argv safely.

### Unit tests

- default config, deep merge, validation, and immutability;
- snapshot response decoding and required-field validation;
- unknown-field tolerance;
- attention-priority sorting and workspace grouping;
- stable selection across board renders;
- first snapshot does not notify;
- transitions and post-bootstrap newly observed agents in `blocked` or `done` notify;
- event data for `HerdrUpdated` and `HerdrAgentStatusChanged`;
- file, range, selection, and diagnostic prompt formatting;
- one-based line reporting and byte/line limits;
- statusline output for empty, healthy, and stale states.

### Component tests

- `vim.system()` receives the expected argv without a shell;
- non-overlapping refresh behavior and timeout handling;
- last-good-state retention after malformed JSON or process failure;
- server auto-start is bounded and shared by concurrent callers;
- board buffer options, rendering, and local mappings;
- target selection behavior for zero, one, and multiple agents;
- attach buffer creation, reuse, normal close, and explicit takeover;
- buffer/window deletion during callbacks;
- health results with missing and fake executables.

### Verification commands

The repository will expose stable commands equivalent to:

```sh
nvim --headless --clean -u tests/minimal_init.lua -l tests/run.lua
stylua --check lua plugin tests
```

### Manual acceptance

With Herdr 0.7.4 and current Codex and Claude Code installations:

1. Start one Codex and one Claude Code agent from different projects.
2. Verify both appear under their owning Herdr workspace with the expected cwd and status.
3. Cause one agent to block for input and verify a single notification.
4. Send text, a visual selection, a file reference, and diagnostics.
5. Attach, detach, and reattach without stopping the agent.
6. Verify attach ownership requires explicit takeover.
7. Close and reopen Neovim and verify both agents are rediscovered.
8. Verify Herdr agents continue running after Neovim exits.

## Success Criteria

- A developer can identify attention-requiring agents from the board or statusline without visiting every terminal.
- `blocked` and `done` transitions become visible within the configured background refresh interval.
- Starting, sending, attaching, and refreshing do not block the Neovim UI.
- Neovim restart loses no Herdr-managed agent process or session.
- Closing a plugin terminal never closes its Herdr agent.
- All automated tests and formatting checks pass.
- `:checkhealth herdr` provides an actionable result on both healthy and intentionally incomplete setups.

## References

- [Herdr home](https://herdr.dev/)
- [Herdr agent behavior](https://herdr.dev/docs/agents/)
- [Herdr CLI reference](https://herdr.dev/docs/cli-reference/)
- [Herdr socket API](https://herdr.dev/docs/socket-api/)
- [Herdr session state and restore](https://herdr.dev/docs/session-state/)
- [coder/claudecode.nvim](https://github.com/coder/claudecode.nvim)
- [Codex MCP documentation](https://learn.chatgpt.com/docs/extend/mcp)
- [Claude Code MCP documentation](https://docs.anthropic.com/en/docs/claude-code/mcp)

The MCP references inform the deferred bidirectional integration boundary; MCP is not part of the first release.
