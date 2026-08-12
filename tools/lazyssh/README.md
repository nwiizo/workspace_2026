# lazyssh

`lazyssh` is a read-only, keyboard-first TUI for hosts in OpenSSH config. It is
written in Rust and inspired by [lazygit](https://github.com/jesseduffield/lazygit)
and [Adembc/lazyssh](https://github.com/Adembc/lazyssh).

The first version focuses on one safe workflow: find a configured host and hand
its alias to the system `ssh` executable.

```text
┌ lazyssh   3/3 hosts ───────────────────────────────────────────────┐
│ /Users/me/.ssh/config                                              │
├ Hosts ─────────────────────┬ Connection ───────────────────────────┤
│ › prod-web   10.0.0.8      │ Alias        prod-web                 │
│   bastion    bastion...    │ Destination  deploy@10.0.0.8:2222    │
│   dev-api    dev.example   │ ProxyJump    bastion                  │
└────────────────────────────┴────────────────────────────────────────┘
↑↓/jk navigate  / filter  Enter connect  r reload  ? help  q quit
```

## Features

- Lists concrete `Host` aliases in `~/.ssh/config`
- Recursively expands `Include` paths (including `~` and globs) and guards against include cycles
- Ranks fuzzy matches by alias, hostname, user, or jump host with `nucleo-matcher`
- Shows source location and common connection fields
- Suspends the TUI and launches the native OpenSSH client for an interactive session
- Reloads config without restarting
- Never writes SSH config or stores credentials

Wildcard and negated `Host` patterns are not shown as destinations. They still
take effect when OpenSSH resolves the selected alias. The details panel shows
values declared directly in concrete host blocks; `ssh` remains the authority
for wildcard defaults and other advanced options.

## Build and run

Rust 1.88 or newer is required.

```sh
cargo build --release
./target/release/lazyssh
```

Use another config file with:

```sh
lazyssh --config ./examples/ssh_config
```

The example aliases use documentation-only addresses. Press `q` instead of
`Enter` when exploring the fixture.

## Key bindings

| Key | Action |
| --- | --- |
| `↑` / `k`, `↓` / `j` | Move selection |
| `g`, `G` | Jump to first or last host |
| `/` | Start fuzzy filtering |
| `Enter` | Accept a filter or connect to the selected host |
| `Esc` | Cancel and clear the current filter |
| `r` | Reload the config and included files |
| `?` | Toggle help |
| `q` / `Ctrl-C` | Quit |

## Security model

`lazyssh` does not implement SSH and never invokes a shell. It starts the
system `ssh` binary with `-F <config>` and the selected alias as literal process
arguments, so your existing agent, keys, `ProxyJump`, and OpenSSH policy remain in control.
Aliases beginning with `-` are not listed, preventing them from being treated
as command-line options.

## Library choices

- [`ratatui`](https://ratatui.rs/) 0.30 provides the immediate-mode layout,
  widgets, differential rendering, and headless `TestBackend` used here.
- [`crossterm`](https://github.com/crossterm-rs/crossterm) 0.29 is Ratatui's
  portable backend for terminal input, raw mode, and alternate-screen control.
- [`nucleo-matcher`](https://github.com/helix-editor/nucleo) supplies the
  Unicode-aware fuzzy scoring used by Helix instead of a hand-written matcher.

Only the Ratatui features needed for the Crossterm 0.29 backend are enabled.
Higher-level component frameworks are not needed for this single-screen state
machine; parsing, input transitions, rendering, and process launch remain
separately testable.

## Scope

This MVP is intentionally read-only. Adding, editing, deleting, pinging, and
file transfer are outside the initial scope because safe SSH config mutation
requires comment-preserving edits, atomic replacement, permission preservation,
and tested backup recovery.
