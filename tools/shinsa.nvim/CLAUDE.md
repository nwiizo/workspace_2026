# shinsa.nvim

## Intent

`shinsa.nvim` turns the current Git working tree into a risk-ranked review queue. It is an independent
Neovim plugin, not a frontend for rinkaku or another review CLI.

The core job is: when an AI or a person produces a large change, help the reviewer inspect consequential
changes first, record explicit coverage, and hand unresolved concerns back without leaving Neovim.

## Boundaries

- Neovim 0.10+ and Git are required.
- There are no required Lua plugin dependencies.
- Tree-sitter symbol extraction currently supports Rust, Go, Python, TypeScript/TSX, JavaScript/JSX, and Lua.
- Unsupported languages, unavailable parsers, binary files, deleted files, and top-level edits become file-level
  review items rather than disappearing.
- The ledger is session-local. Do not add persistence without defining its compatibility and privacy contract.
- Analysis is read-only. Git mutation belongs to LazyGit or Neogit; detailed diff navigation belongs to CodeDiff
  or Diffview when installed.

## Checks

```sh
make check
```
