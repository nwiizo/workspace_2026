# blogs/ ✍️

Where mass production meets prose.

## What's Here

Blog articles, drafts, and the occasional existential crisis about whether anyone actually reads these.

## Workflow

1. Write something
2. Regret half of it
3. Publish anyway
4. Repeat

## Local Config

Shared editorial guidance lives in `~/.local/share/blog-agents/`. Codex and Claude Code use links under `~/.agents/skills/` and `~/.claude/skills/`. This workspace links to the shared rules and docs; `AGENTS.md` describes the layout.

## Textlint

Run these commands from `blogs/`. Restore the local dependencies from the lockfile before the first check:

```sh
npm ci --include=dev --ignore-scripts --no-audit --no-fund
npx --no-install textlint contents/senior-engineer-learning.md
```

Replace the article path as needed. The workspace's `.textlintrc.json` loads the Japanese rules and `prh.yml`. A global Textlint installation alone does not provide these local dependencies. If Textlint reports `No rules found`, add `--debug` to the check command to see which configuration or rule failed to load.

The prose replacements in `prh.yml` exclude headings, which may contain official book titles. Preserve those titles and verify them against their sources; the other Textlint rules still check headings.

## Pro Tips

- First drafts are supposed to be bad. That's what makes them first drafts.
- "I'll polish it later" is a valid strategy (sometimes).
- The best blog post is the one you actually publish.
