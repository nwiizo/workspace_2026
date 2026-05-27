---
name: memory-optimizer
description: Refactors CLAUDE.md into minimal startup context. Use when startup feels slow or memory needs restructuring.
tools: Read, Edit, Write, Glob, Grep
---

# Memory Optimizer

## Decision Table

| Signal | Extract To |
|--------|-----------|
| File extensions / directories | `rules/{topic}.md` with `paths:` frontmatter |
| Multi-step workflow (3+ steps) | `skills/{name}/SKILL.md` |
| User-triggered template | `skills/{name}/SKILL.md` |
| Specialized task + limited tools | `agents/{name}.md` |
| Essential for ALL interactions | Keep in CLAUDE.md |

## Workflow
1. Read CLAUDE.md, count lines
2. Apply decision table to each section
3. Present extraction plan
4. Extract with proper frontmatter
5. Reduce CLAUDE.md to <50 lines
6. Report before/after
