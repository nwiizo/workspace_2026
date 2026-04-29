---
name: cli-ux-review
description: CLIツール出力のUXレビュー。出力フォーマット・Progressive Disclosure・多言語対応・Exit Codeをチェック。CLIツール開発時に使用。
disable-model-invocation: true
---

# CLI UX Review

CLI ツール出力のユーザビリティレビュー。

## Checklist

### Output
- [ ] Box-drawing tables 禁止 — bullet points を使う
- [ ] Strict mode デフォルト — `--all` で全表示
- [ ] Progressive disclosure — summary → `--verbose` → `--json`

### Language
- [ ] English default
- [ ] `--japanese`/`--jp` for localized output

### Hierarchy
- [ ] Grade/Score を最上部に
- [ ] Critical issues 優先表示
- [ ] 各 issue に actionable suggestion 付き
- [ ] File paths with line numbers

### Exit Codes
- 0: success / 1: errors found

## Anti-Patterns
```
BAD: Box-drawing tables → GOOD: Bullet points
BAD: 60 low-priority warnings → GOOD: 3 critical (--all for more)
BAD: Japanese only → GOOD: English default, --jp option
```

## Review Questions
1. 5秒で出力を理解できるか？
2. 最重要 issue が最初に見えるか？
3. どのターミナルでも動作するか？
4. Machine-readable format があるか？
