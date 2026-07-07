# Workspace Docs

Cross-repository notes for the `/Users/nwiizo/ghq/github.com/nwiizo`
workspace. These files are indexes and synthesis notes; they should point to
source repositories instead of becoming a second source of truth. Current
technology claims belong in the dated technology map; repo status belongs in
the situation ledger.

## Current Files

| File | Purpose |
|---|---|
| [workspace-situation-2026-07-06.md](workspace-situation-2026-07-06.md) | Dirty-repo ledger, cleanup queue, and cross-repo risk map |
| [agentic-technology-map-2026-07-06.md](agentic-technology-map-2026-07-06.md) | Current map of Codex, Claude Code, MCP, A2A, and local tooling implications |
| [ccswarm-cleanup-2026-07-06.md](ccswarm-cleanup-2026-07-06.md) | ccswarm cleanup batches, worktree audit, and generated-artifact handling |

## Rules

- Keep dated snapshots dated. Do not silently turn them into evergreen claims.
- Prefer official or repo-local evidence for current technical claims.
- Put reusable agent configuration in `dotfiles`, not here.
- Put book-specific translation status in each book repo or `page-turners`, not
  here.
- Do not include secrets, local auth tokens, session transcripts, or generated
  runtime state.
