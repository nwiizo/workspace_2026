# Repository Guidelines

## Project Structure & Module Organization
Source articles live in `contents/` and follow the `blog-0N-topic.md` naming pattern so that drafts line up with publication order. The top-level `blog.md` is the stitched master draft (backed up as `blog.md.backup`) that you can regenerate from approved articles. Writing aids such as `prh.yml` and `CLAUDE.md` sit in the root; keep private Claude agent presets inside `.claude/` (gitignored). Generated assets belong under `node_modules/`—never edit that folder directly.

## Build, Test, and Development Commands
Run `npm install` once per machine to pull Textlint and the Japanese technical-writing ruleset. Use `npx textlint blog.md contents/**/*.md` for a quick lint sweep, or append `--fix` when the rules can safely auto-correct phrasing. For focused iterations, lint an individual article: `npx textlint contents/blog-04-kratos.md`. These commands are the closest thing to “tests,” so run them before every commit.

## Coding Style & Naming Conventions
Write in Markdown with a single `#` title, nested `##` sections, and hyphen bullets. Inline citations should follow the existing `[https://example.com:embed:cite]` syntax so downstream tooling can expand them. Prefer short paragraphs, avoid figurative language the `prh.yml` rules ban, and keep headings in Japanese when the article is in Japanese. Filename slugs should stay lowercase with hyphens; reserve camelCase for local anchor IDs only.

## Testing Guidelines
Treat Textlint as the quality gate: submissions must pass with zero warnings. If you introduce new phrasing patterns, update `prh.yml` in the same change and document the reason inside the PR. When adding automation, mirror the existing command shape (direct `npx textlint …`) so CI additions later remain consistent.

## Commit & Pull Request Guidelines
History follows a conventional-commit variant (`feat(scope): summary`, `fix(scope): summary`). Keep scopes aligned with article or tooling names, e.g., `feat(blog-05-keto): add security hardening notes`. Pull requests should include: a one-paragraph change summary, the exact lint command/output, references to related issues or posts, and screenshots only when the Markdown renders unusually (diagrams, tables). Mention whether reviewers must update shared Claude commands or local configs.

## Security & Configuration Notes
Do not check in `.claude/` contents, API tokens, or unpublished vendor case studies. Validate embedded citation URLs before merging, and prefer environment variables over plaintext secrets whenever you document infrastructure steps.
