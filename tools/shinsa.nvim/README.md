# shinsa.nvim

`shinsa.nvim` is a dependency-free Neovim plugin that turns a large working-tree diff into a review queue:
important changes first, explicit reviewed/unseen state, and a small concern ledger for handing work back.

It is not `rinkaku.nvim` and does not invoke rinkaku. The implementation only requires Neovim 0.10+ and Git.

## The job

> When an AI agent or a teammate produces a broad change, help me decide where to spend attention first,
> show what I have not inspected yet, and capture concrete follow-ups without forcing me into a second review tool.

Existing tools solve adjacent jobs well:

| Tool family | Primary job | How Shinsa uses that boundary |
| --- | --- | --- |
| [lazygit.nvim](https://github.com/kdheepak/lazygit.nvim) / [Neogit](https://github.com/NeogitOrg/neogit) | Inspect and mutate Git state | Optional `g` handoff; Shinsa itself never mutates Git |
| [CodeDiff](https://github.com/esmuellert/codediff.nvim) / [Diffview](https://github.com/sindrets/diffview.nvim) | Read a precise patch | Optional `d` handoff, with a built-in unified-diff fallback |
| [review.nvim](https://github.com/georgeguimaraes/review.nvim) / [Octo.nvim](https://github.com/pwntester/octo.nvim) | Write line comments or run a hosted PR review | Shinsa keeps only a lightweight local concern ledger |
| [rinkaku](https://github.com/hiro-o918/rinkaku) | Understand the structural outline of a change | Shinsa independently builds an attention queue and coverage state |

## What it does

- Compares the whole working tree (staged, unstaged, and optionally untracked files) with the merge base of a
  base branch.
- Maps changed lines to enclosing Tree-sitter symbols for Rust, Go, Python, TypeScript/TSX,
  JavaScript/JSX, and Lua.
- Falls back to a file-level item when a parser or symbol is unavailable, so unsupported changes remain visible.
- Uses NUL-delimited Git metadata so binary, empty, mode-only, deleted, and oversized changes remain in the queue
  even when no patch hunk is parsed. Symlinks are never followed during source analysis.
- Ranks items with explainable heuristics: signature/contract edits, additions or deletions, public surface,
  references between changed symbols, changed-test reachability, and change size.
- Tracks `unseen`, `reviewed`, and `concern` state for the current Neovim session.
- Exports concerns as agent-ready Markdown.

The score is an attention heuristic, not a correctness or security verdict. Reference counts and test paths are
derived from changed symbols only; Shinsa deliberately displays its reasons instead of pretending the score is
ground truth. Patch capture is memory-bounded from `max_file_bytes`; when that budget is exceeded, Shinsa keeps
metadata-only file items instead of loading an unbounded diff.

## Install

With `lazy.nvim`:

```lua
{
  dir = "/path/to/shinsa.nvim",
  cmd = { "Shinsa", "ShinsaToggle" },
  opts = {},
}
```

Or from a published repository:

```lua
{
  "nwiizo/shinsa.nvim",
  cmd = { "Shinsa", "ShinsaToggle" },
  opts = {},
}
```

## Use

Run `:Shinsa` from a file in a Git repository. Shinsa detects `origin/HEAD`, then tries `origin/main`, `main`,
`origin/master`, and `master`. Pass an explicit ref when needed:

```vim
:Shinsa origin/develop
```

Inside the queue:

| Key | Action |
| --- | --- |
| `<CR>` | Open the symbol in the source window |
| `d` | Open the exact diff in CodeDiff, Diffview, or the built-in viewer |
| `x` | Toggle reviewed state |
| `c` / `C` | Add/edit or clear a concern |
| `]u` / `[u` | Move to the next/previous unseen item |
| `s` | Resume at the highest-ranked unseen item |
| `y` | Yank concerns as Markdown |
| `g` | Open the repository in LazyGit or Neogit |
| `r` / `q` | Refresh or close the queue |

Commands: `:Shinsa [base]`, `:ShinsaRefresh`, `:ShinsaToggle`, `:ShinsaClose`, `:ShinsaReset`,
`:ShinsaYank`, and `:ShinsaHealth`.

## Configure

```lua
require("shinsa").setup({
  base = nil, -- string, nil for auto-detection, or function({ cwd })
  include_untracked = true,
  max_file_bytes = 2 * 1024 * 1024,
  panel = { side = "left", width = 64 },
  handoff = {
    diff = "auto", -- "codediff", "diffview", "builtin", false, or function(context)
    git = "auto", -- "lazygit", "neogit", false, or function(context)
  },
  clipboard_register = "+",
})
```

`diff = "auto"` prefers CodeDiff, then Diffview, then the built-in read-only unified diff. `git = "auto"`
prefers lazygit.nvim, then Neogit. Custom handoff functions receive repository, base, and merge-base context.

`User ShinsaReady` fires after a successful refresh with `{ root, base, item_count }` in `event.data`.

## Validate

```sh
make check
```

License: Friend License (MIT-equivalent).
