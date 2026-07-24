# herdr.nvim

`herdr.nvim` is a small Neovim control surface for persistent Codex and Claude Code agents managed by [Herdr](https://herdr.dev/).

It focuses on the job that matters once several agents are running: see which delegated work needs attention, intervene without touring terminal panes, and return to the same live agents after Neovim restarts.

> [!NOTE]
> This is an unofficial community integration. It is not maintained or endorsed by the Herdr project.

## Features

- Native scratch-buffer herd board grouped by Herdr workspace
- Attention-first ordering for `blocked`, `done`, `working`, `idle`, and `unknown`
- Notifications when an agent becomes blocked or completes work after the initial baseline
- Persistent Codex and Claude Code launches through Herdr
- Direct attach in a native Neovim terminal split
- Instructions, file references, visual selections, and LSP diagnostics sent to any agent
- Statusline summary and `:checkhealth herdr`
- Pure Lua with no runtime plugin dependency

Herdr owns PTYs, processes, semantic state, and persistence. `herdr.nvim` keeps no transcript or session database.

## Requirements

- Neovim 0.10+
- Herdr 0.7.4+
- macOS or Linux
- Codex CLI and/or Claude Code

Install the richer Herdr integrations once for state and session identity:

```sh
herdr integration install codex
herdr integration install claude
```

## Installation

For this workspace checkout with lazy.nvim:

```lua
{
  dir = "/path/to/workspace_2026/tools/herdr.nvim",
  name = "herdr.nvim",
  cmd = {
    "Herdr",
    "HerdrRefresh",
    "HerdrStart",
    "HerdrAttach",
    "HerdrSend",
    "HerdrSendVisual",
    "HerdrSendFile",
    "HerdrSendDiagnostics",
    "HerdrHealth",
  },
  opts = {},
}
```

For a standalone checkout, add its directory to `runtimepath` and call:

```lua
require("herdr").setup()
```

The first setup starts background state refresh. If the Herdr server is not reachable, the default configuration starts `herdr server` as a detached process. Closing Neovim never stops that server or its agents.

## Workflow

Open the board:

```vim
:Herdr
```

```text
Herdr  !1  *2  ✓1

workspace_2026
 ! reviewer   claude  blocked  approval required
 * api        codex   working
 ✓ tests      codex   done
```

Board mappings are buffer-local:

| Key | Action |
| --- | --- |
| `<CR>` | Attach to the selected live agent |
| `a` | Start Codex or Claude Code |
| `s` | Send an instruction |
| `r` | Refresh state immediately |
| `q` | Close the board |
| `?` | Toggle help |

No global mappings are defined.

## Commands

| Command | Description |
| --- | --- |
| `:Herdr` | Toggle the herd board |
| `:HerdrRefresh` | Refresh immediately |
| `:HerdrStart [codex\|claude]` | Start a named persistent agent at the current project root |
| `:HerdrAttach [target]` | Attach to an agent terminal |
| `:HerdrAttach! [target]` | Explicitly take over input from another direct-attach client |
| `:HerdrSend [target]` | Prompt for and send an instruction |
| `:[range]HerdrSend [target]` | Send complete lines from an explicit Ex range |
| `:'<,'>HerdrSendVisual [target]` | Send the exact character-, line-, or blockwise visual selection |
| `:HerdrSendFile [target]` | Send the saved current-file reference |
| `:HerdrSendDiagnostics [target]` | Send current-buffer LSP diagnostics |
| `:HerdrHealth` | Run `:checkhealth herdr` |

When the target is omitted, one agent is selected automatically or several are offered through `vim.ui.select()`.

## Statusline

The statusline API returns a plain, safe string:

```lua
require("herdr").statusline()
-- H !1 *2 ✓1
```

For lualine.nvim:

```lua
require("lualine").setup({
  sections = {
    lualine_x = { require("herdr").statusline },
  },
})
```

`~` marks last-known data after a refresh failure. Before the first successful snapshot the function returns an empty string.

## Configuration

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
    markers = {
      blocked = "!",
      working = "*",
      done = "✓",
      idle = "·",
      unknown = "?",
    },
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

Agent commands are argv lists, so wrappers and fixed flags remain shell-free:

```lua
agents = {
  codex = { command = { "codex", "--profile", "work" } },
}
```

## Safety and limitations

- External commands use argv arrays; editor content is never interpolated into a shell command.
- Instructions use Herdr's atomic `pane run` operation, so text is submitted with Enter instead of being left in the agent input box.
- Context larger than the configured limit is rejected rather than silently truncated.
- `HerdrSendFile` rejects unnamed and modified buffers so the agent does not read stale disk content.
- Attach takeover is only available through the explicit bang command.
- The initial release polls `herdr api snapshot`; raw socket subscriptions are deferred.
- Agent-to-Neovim MCP tools and native diff review are deferred to a later release.
- Windows is not supported in the initial release.

## Development

```sh
make test
make fmt-check
make lint
make check
```

Tests run in a clean headless Neovim and require no testing plugin.

Design and implementation records are in [`docs/`](docs/).

## License

Friend License (MIT-equivalent).
