---
name: jj-agent-spawn
description: Spawn a parallel coding-agent session in its own jj workspace. Uses `jj_agent` if the nwiizo/jujutsu.fish plugin is installed, otherwise falls back to raw `jj workspace add`. Use when starting a second (or Nth) agent on a different task without disturbing the current working copy. Invoke manually; creates a workspace directory on disk.
disable-model-invocation: true
---

# jj Agent Spawn

Create a new jj workspace (the equivalent of `git worktree add`) and open it ready for an agent to start work.

## When to use

- User wants to run two or more agents on different tasks in parallel.
- User wants to try an alternative approach without contaminating the main working copy.
- User wants the agent's output isolated in its own `@` for later review.

## Prerequisites

- `.jj/` exists in the repo root.
- `$TMUX` is set if `--tmux` mode is desired.
- `$EDITOR` is set (or an explicit editor is provided).

## Steps

1. Pick a workspace name. Short, descriptive, kebab-case:
   - Good: `fix-auth`, `review-pr-42`, `try-approach-b`.
   - Bad: `agent1`, `tmp`, `wip`.

2. Pick the base revision — usually `@` (current) or `trunk()`.

3. Spawn. Prefer the plugin wrapper if available:
   ```fish
   # Preferred: with nwiizo/jujutsu.fish installed
   jj_agent <name> -r 'trunk()'
   jj_agent <name> --tmux           # opens a new tmux window
   jj_agent <name> -e 'claude'      # override editor

   # Fallback: plain jj
   jj workspace add --name <name> -r 'trunk()' ../<name>
   cd ../<name>
   $EDITOR .
   ```

4. Confirm the workspace registered:
   ```fish
   jj workspace list          # or `jj_agent_list` if the plugin is installed
   ```
   The new workspace should appear with an empty `@`.

## Lifecycle follow-ups

Plugin-available shortcuts (otherwise run the underlying jj commands):

- Compare two agents' output: `jj_agent_diff <A> <B>` (or `jj diff --from <A>@ --to <B>@`).
- Close out a workspace: `jj_agent_done <name> --push-pr --forget` (or summarize + push + `jj workspace forget <name>`).
- Cleanup of empty abandoned workspaces: `jj_agent_prune --dry-run` then `jj_agent_prune`.

## Notes for the invoking agent

- The spawned session is an **independent** jj workspace. Operations there record into the **shared** op log, so the main agent can see what the spawned agent did via `jj op log` from any workspace.
- Do not invoke this skill programmatically just to create a workspace — the skill exists so the user explicitly chooses the name, base, and mode.
