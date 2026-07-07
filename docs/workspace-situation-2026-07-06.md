# Workspace Situation Ledger

Date: 2026-07-06
Scope: `/Users/nwiizo/ghq/github.com/nwiizo`
Status: living ledger for cleanup and technical synthesis.

This ledger summarizes the current multi-repository workspace so cleanup can
proceed by domain instead of by whichever dirty tree is loudest.

## Executive Summary

There are 31 dirty repositories in the workspace. The largest source of noise is
`ccswarm`: 8,033 status entries, of which 7,945 are tracked `target2/` build
artifacts staged for deletion. The meaningful work clusters are:

1. Agent/tooling modernization: `ccswarm`, `dotfiles`, `claudelytics`, `tfmcp`,
   `workspace_2026`, `bjj_2026`.
2. AI-agent and MCP translation/import work: `ai-agents-the`,
   `ai-agents-with-mcp`, `building-ai-agent-platforms`,
   `multimodal-real-time-ai-agent-systems`.
3. General translation imports: `simplicity`, `communication-patterns`,
   `the-c4-model`, plus smaller script utility additions across book repos.
4. Writing/manuscript work: `oitoriaezu-owarasero`, `wakaru-wo-watasu`,
   `hatena-blog-pull`, `x-posts`.
5. Personal operations: `kakutei-shinkoku-workspace`, `evaluation`,
   `vibe-ticket`.

The next cleanup should not start with one-off formatting. It should first
remove generated/cache noise, then group work into reviewable batches.

## Dirty Repository Snapshot

| Count | Repository | Classification | Immediate handling |
|---:|---|---|---|
| 8033 | `ccswarm` | Tool refactor + generated noise | `target2/` deletion is staged; keep real Rust/docs work separate |
| 186 | `simplicity` | Translation import | Validate scaffold and remove `__pycache__` before review |
| 173 | `communication-patterns` | Translation import | Treat as one book import batch |
| 171 | `multimodal-real-time-ai-agent-systems` | AI-agent translation import | Review as AI-agent knowledge source |
| 82 | `ai-agents-the` | AI-agent translation continuation | Check missing images and translation guardrails |
| 72 | `oitoriaezu-owarasero` | Manuscript/editorial | Keep separate from technical repos |
| 66 | `building-ai-agent-platforms` | AI-agent translation import | Review as AI-agent platform source |
| 66 | `3shake-marp-templates` | Slide/content assets | Verify asset provenance and slide references |
| 65 | `the-c4-model` | Translation continuation | Check chapter continuation and images |
| 58 | `ai-agents-with-mcp` | MCP translation import | Review as MCP knowledge source |
| 32 | `claudelytics` | Rust tool feature work | Separate Codex usage model work from UI/report changes |
| 18 | `kakutei-shinkoku-workspace` | Personal tax workspace | Keep private/operational, do not fold into tech summary |
| 16 | `hatena-blog-pull` | Writing-agent setup | Review agent/rule docs as reusable writing workflow |
| 14 | `wakaru-wo-watasu` | Manuscript | Keep editorial flow separate |
| 11 | `page-turners` | Translation parent repo | Reconcile submodules and translation status |
| 11 | `vibe-ticket` | Task metadata | Check whether generated state should be committed |
| 10 | `rust-best-practices` | Rust docs | Review for consistency and current Rust guidance |
| 8 | `dotfiles` | Agent/dev environment config | Validate symlinks/audit scripts before publishing |
| 4 | `workspace_2026` | Workspace tools/notes | Current ledger and technology map live here |
| 3 | `x-posts` | Short-form drafts | Keep separate from source-of-truth docs |
| 2 | `translation-architecture-modernization` | Translation QA/rules | Likely small safety-rule addition |
| 1 each | `looks-good-to-me`, `essential-test-driven-development`, `domain-driven-transformation`, `crafting-engineering-strategy`, `building-evolutionary-architectures` | Shared script utility additions | `.pyc` cache removed; review shared `scripts/utils.py` pattern |
| 1 | `tfmcp` | MCP/Rust tool | Inspect `src/main.rs` before grouping |
| 1 | `marp.nvim` | Neovim plugin | Standalone tool change |
| 1 | `cargo-coupling` | Rust tooling docs | Changelog-only update; keep separate from ccswarm cleanup |
| 1 | `bjj_2026` | BJJ notes/agents | Untracked `.agents/`; decide whether repo-local agents belong there |

Evidence command:

```bash
rtk proxy zsh -lc 'for d in */.git(N); do repo=${d:h}; n=$(git -C "$repo" status --porcelain 2>/dev/null | wc -l | tr -d " "); if [[ $n -gt 0 ]]; then printf "%5d %s\n" $n $repo; fi; done | sort -nr'
```

Snapshot note: counts can drift while this cleanup is running. Rerun the
evidence command before acting; this snapshot has been refreshed after adding
`docs/`, removing six Python cache intent-to-add entries, and staging the
`ccswarm/target2` deletion.

## Cross-Cutting Risks

### ccswarm worktree fan-out

`ccswarm` has 22 linked worktrees under `~/ghq/github.com/nwiizo/worktrees/`.
Read-only audit found all 22 dirty with unique `CLAUDE.md` diffs and untracked
`.claude.json`, so none should be removed directly. Archive first, then remove
the registered worktree only after the archive is checked.

Evidence command:

```bash
rtk git -C ccswarm worktree list
```

Detailed handling:
[ccswarm-cleanup-2026-07-06.md](ccswarm-cleanup-2026-07-06.md).

### No-commit translation imports

Several translation repos are newly imported or extended but not yet committed.
Do not combine them into one workspace commit. Commit or review them book by
book, because each has its own source provenance, image manifest, glossary, and
translation guardrail state.

### Local task state

`vibe-ticket` has `.vibe-ticket` state files. Treat these as local task metadata
until the repo's intended source-of-truth policy is confirmed. Do not infer that
all state files should be committed just because they are visible in `git
status`.

## Cleanup Strategy

### 1. Stop generated noise first

Priority targets:

- `ccswarm/target2/`: 7,945 tracked build artifacts are staged for deletion.
  Keep `target2/` ignored and commit this as a deliberate cleanup batch.
- `scripts/__pycache__/` and `*.pyc`: six intent-to-add cache files have been
  removed; keep `.gitignore` in those repos so only `scripts/utils.py` remains
  for review.
- `.vibe-ticket/`: decide whether task state is tracked project data or local
  runtime state before committing it.
- `examples/e2e-playwright/test-results/` and local generated queue/dry-run
  files: keep ignored in `ccswarm`.

Do this before reviewing feature work. Otherwise every review is polluted by
artifact churn.

### 2. Batch translation imports by scaffold

Many book repos share the same structure:

```text
AGENTS.md
CLAUDE.md
CODEX_TRANSLATION_BRIEF.md
README.md
content/en/
content/ja/
content/bilingual/
content/images/
scripts/{apply,check,create,extract,repair,translate}_*.py
scripts/utils.py
```

Review them in batches:

- AI-agent/MCP batch: `ai-agents-the`, `ai-agents-with-mcp`,
  `building-ai-agent-platforms`, `multimodal-real-time-ai-agent-systems`.
- General software batch: `simplicity`, `communication-patterns`, `the-c4-model`.
- Shared script utility batch: repos with only `scripts/utils.py` plus
  `__pycache__`.
- Parent/submodule batch: `page-turners` should reconcile `.gitmodules`,
  `TRANSLATION_STATUS.md`, and `translations/*` as either explicit submodules or
  tracked import manifests.

The review checklist should be: source files present, Japanese files present,
bilingual files present, images accounted for, guardrail script passes, and no
cache files are tracked.

Centralization candidate: the repeated translation scripts should graduate into
a shared scaffold or package once the current imports settle. Until then, avoid
changing every repo's scripts opportunistically.

### 3. Keep writing/manuscript separate

`oitoriaezu-owarasero`, `wakaru-wo-watasu`, `hatena-blog-pull`, and `x-posts`
are editorial workflows. They should not be mixed with technical tool commits.
Their useful reusable asset is the agent/rule design for writing review:
language, structure, style, quality, and reader agents.

### 4. Make `dotfiles` the source of reusable agent config

`dotfiles` already states that shared agent assets live under `.agents/` and
symlink into `.claude/` and `.codex/`. Therefore:

- Put reusable skills/rules/agents in `dotfiles`.
- Keep project-specific agents in the project only when they depend on local
  domain context.
- Run the repo's audit script before publishing agent config changes.

### 5. Treat `workspace_2026` as the cross-domain notebook

This repository is the best place for cross-repo situation ledgers and
experiments. It should not become the source of truth for book translation
status or dotfile configuration; it should point to those repos instead.

## Technical Themes

The current Codex, Claude Code, MCP, A2A, telemetry, and schema-output summary
lives in [agentic-technology-map-2026-07-06.md](agentic-technology-map-2026-07-06.md).
Keep this ledger focused on local cleanup implications:

- `ccswarm` should split generated cleanup from provider/A2A execution changes.
  Repo-local evidence: `ccswarm/crates/ccswarm/src/session/a2a.rs`,
  `ccswarm/crates/ccswarm/src/session/bridge.rs`,
  `ccswarm/docs/ARCHITECTURE.md`.
- `dotfiles` is the source for reusable agent configuration. Repo-local
  evidence: `dotfiles/.agents/README.md`, `dotfiles/.claude/README.md`,
  `dotfiles/.codex/README.md`, `dotfiles/AGENTS.md`.
- `remote-mcp-devkit` and `workspace_2026/tools/remote-mcp-devkit` need an
  explicit canonical/mirror/experiment decision. Repo-local evidence:
  `remote-mcp-devkit/README.md`, `remote-mcp-devkit/docs/development-spec.md`,
  `remote-mcp-devkit/src/client_dance.rs`.
- `tfmcp` is the Terraform MCP line. Repo-local evidence: `tfmcp/Cargo.toml`,
  `tfmcp/README.md`, `tfmcp/src/main.rs`.

## Next Cleanup Queue

1. `ccswarm`: keep the archived worktree evidence, commit the staged `target2/`
   cleanup batch, then split real Rust/docs work into product abstraction,
   provider execution, A2A, and e2e Playwright batches.
2. Translation imports: validate the AI-agent/MCP batch with guardrail scripts;
   Python cache files have already been removed from the small shared-script
   repos.
3. `dotfiles`: run agent-config audit and decide which project-local agent
   assets should move into reusable `.agents/`.
4. `page-turners`: reconcile `.gitmodules`, `TRANSLATION_STATUS.md`, and new
   `translations/*` entries.
5. Writing repos: review `hatena-blog-pull` and `oitoriaezu-owarasero` agent
   designs as possible reusable writing-review workflows, without mixing them
   into tool/translation commits.

## Evidence Commands

```bash
rtk proxy zsh -lc 'for d in */.git(N); do repo=${d:h}; n=$(git -C "$repo" status --porcelain 2>/dev/null | wc -l | tr -d " "); if [[ $n -gt 0 ]]; then printf "%5d %s\n" $n $repo; fi; done | sort -nr | sed -n "1,120p"'
rtk proxy zsh -lc 'for d in */.git(N); do repo=${d:h}; s=$(git -C "$repo" status --porcelain 2>/dev/null); [[ -n $s ]] || continue; printf "## %s\n" "$repo"; print -- "$s" | sed -n "1,35p"; c=$(print -- "$s" | wc -l | tr -d " "); [[ $c -gt 35 ]] && printf "... (%s total)\n" "$c"; done'
rtk git -C workspace_2026 status --short
rtk sed -n '1,260p' workspace_2026/README.md
rtk sed -n '1,240p' workspace_2026/AGENTS.md
```
