# ccswarm Cleanup Notes

Date: 2026-07-06
Scope: `/Users/nwiizo/ghq/github.com/nwiizo/ccswarm` and linked worktrees.

This note separates cleanup batches so the large `ccswarm` dirty state can be
reviewed without mixing generated artifacts, stale docs, worktree scratch, and
real implementation changes.

## Current Shape

`ccswarm` has 8,033 status entries.

| Group | Count / paths | Handling |
|---|---:|---|
| Tracked `target2/` build artifacts | 7,945 staged deletions | Keep ignored; commit as one generated-artifact cleanup batch |
| Removed `crates/ai-session` crate | 39 staged deletions | Review with Cargo workspace changes and session code migration |
| Session/A2A implementation | `crates/ccswarm/src/session/*`, provider/workflow files | Review as functional refactor |
| Tests | new BDD/unit/e2e files plus removed ai-session bridge test | Review after implementation compiles |
| Docs | README, CLAUDE, ARCHITECTURE, MULTI_AGENT_REDESIGN, product plan | Keep current docs aligned with single-crate session design |
| Playwright runtime output | `examples/e2e-playwright/test-results/.last-run.json` | Delete/ignore as generated output |

## Worktree Audit

Read-only worktree audit found:

- 22 registered linked worktrees under `~/ghq/github.com/nwiizo/worktrees/`.
- All 22 are dirty.
- Every worktree has a modified tracked `CLAUDE.md`.
- Every worktree has an untracked `.claude.json`.
- `CLAUDE.md` diffs are unique in all 22 worktrees.
- Branches have no upstream configured.

Commit groups:

| Count | Prefix | Commit | Meaning |
|---:|---|---|---|
| 10 | `backend-agent/*` | `02d8151` | `test(workflow): Add 10 cross-module E2E integration tests` |
| 10 | `frontend-agent/*` | `02d8151` | same base commit |
| 1 | `backend-agent/*` | `b86690b` | `feat: post-pipeline OK/NG assisted flow (#52)` |
| 1 | `frontend-agent/*` | `b86690b` | same newer base commit |

The two newer-base worktrees are:

- `backend-agent-75c61e6c-58c1-4c24-9181-ebb9682641c6`
- `frontend-agent-6459b0c2-e1b1-49f9-a514-796539eb27cf`

Classification:

- Keep: none required by committed state alone.
- Archive first: all 22, because each has unique uncommitted `CLAUDE.md`.
- Remove candidate after archive: all 22; review the two `b86690b` worktrees
  slightly more carefully.

Do not use `rm -rf` on these worktrees. It would lose uncommitted content and
leave Git worktree metadata behind.

## Worktree Archive

Archive location, outside Git:

```text
/Users/nwiizo/ghq/github.com/nwiizo/.cleanup-artifacts/ccswarm-worktrees-2026-07-06/
```

Archive verification:

| Check | Result |
|---|---:|
| Worktree directories archived | 22 |
| `summary.tsv` rows | 22 |
| `CLAUDE.diff` files | 22 |
| `.claude.json` files | 22 |
| Empty `CLAUDE.diff` files | 0 |
| Unique `CLAUDE.md` diff hashes | 22 |
| Unique `.claude.json` hashes | 4 |
| Worktrees at `02d8151` | 20 |
| Worktrees at `b86690b` | 2 |

Each archived worktree directory contains:

- `status.txt`
- `CLAUDE.diff`
- `.claude.json`
- `HEAD`
- `branch`

The archive is enough to review the uncommitted worktree-local instruction
changes without keeping all 22 dirty worktrees open indefinitely. It is not a
commit and should not be treated as the final source of truth.

## Worktree Salvage Review

Archive review found that all 22 `CLAUDE.diff` files replace real project
guidance with per-worktree agent identity prompts. The added content is local
scratch state: role identity, absolute worktree path, session ID, narrow
backend/frontend boundaries, and a requirement to repeat identity in every
response.

Decision:

- Do not merge the added identity prompts into `ccswarm/CLAUDE.md`.
- Do not keep live worktrees for `.claude.json`; the files contain local Claude
  run config, not reusable repo config, and no visible secrets/tokens.
- Salvage only durable operational lessons from the two newer `b86690b`
  worktrees. Current `ccswarm/CLAUDE.md` now preserves the missing useful parts:
  `data-testid` guidance for Playwright-targeted tasks, timeout tuning, and
  provider resume caveats.

Removal recommendation: all 22 live worktrees can be removed after this archive
is retained. Use `git worktree remove` from the main `ccswarm` repository; do
not delete directories by hand.

## Safe Cleanup Already Applied

- `target2/` deletion is staged: 7,945 tracked Cargo build artifacts are in the
  index, with no remaining unstaged `target2/` diff.
- `docs/REFACTOR_PLAN.md` is now staged for deletion. It preserved an older
  `ai-session` boundary and overlaps the current product/redesign docs.
- `examples/e2e-playwright/test-results/.last-run.json` is now staged for
  deletion and covered by `.gitignore`.
- `.gitignore` includes `target2/`, Playwright `.work/`, `test-results/`, and
  generated queue/dry-run files.
- Dirty linked worktrees have been archived outside Git; removal can be decided
  from the archive instead of from the live directories.

## Recommended Batch Order

1. Archive worktrees, already done:

   ```bash
   rtk proxy zsh -lc 'archive=/Users/nwiizo/ghq/github.com/nwiizo/.cleanup-artifacts/ccswarm-worktrees-2026-07-06; for wt in /Users/nwiizo/ghq/github.com/nwiizo/worktrees/*; do repo=${wt:t}; mkdir -p "$archive/$repo"; git -C "$wt" diff -- CLAUDE.md > "$archive/$repo/CLAUDE.diff"; cp "$wt/.claude.json" "$archive/$repo/.claude.json" 2>/dev/null || true; done'
   ```

2. Review the archive outside Git, especially the two `b86690b` worktrees.
3. Remove stale worktrees with `git worktree remove` only after the archive is
   verified and the removal decision is explicit.
4. Commit `target2/` removal as a dedicated generated-artifact cleanup batch.
5. Review and test the real `ccswarm` refactor separately.

## Evidence Commands

```bash
rtk git -C ccswarm status --short
rtk git -C ccswarm diff --cached --name-status
rtk git -C ccswarm diff --name-status
rtk git -C ccswarm worktree list
rtk proxy zsh -lc 'cd ccswarm && git status --porcelain -- target2 | wc -l'
rtk proxy zsh -lc 'cd ccswarm && git diff --name-only -- target2 | wc -l'
rtk proxy zsh -lc 'for wt in /Users/nwiizo/ghq/github.com/nwiizo/worktrees/*; do printf "## %s\n" "$wt"; git -C "$wt" status --short --branch --untracked-files=all; done'
```
