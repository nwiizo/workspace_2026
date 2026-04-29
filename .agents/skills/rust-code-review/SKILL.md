---
name: rust-code-review
description: Rustコードレビュー。Balanced Coupling分析、所有権・ライフタイム・エラー処理のイディオムチェック。Rustコードの変更後やPRレビュー時に使用。
disable-model-invocation: true
---

# Rust Code Review

Khononov's Balanced Coupling による Rust コードレビュー。

## Coupling Analysis (3 Dimensions)

- **Strength**: Contract(trait bounds) > Model(shared types) > Functional(fn calls) > Intrusive(field access)
- **Distance**: Same module=strong OK / Different module=weak preferred / External crate=Contract required
- **Volatility**: `git log --oneline -20 <file>` — high change freq + strong coupling = pain

## Idiom Checklist
- [ ] Newtype for domain types: `struct UserId(u64)`
- [ ] `pub(crate)` over `pub`
- [ ] No public fields across module boundaries
- [ ] `?` operator, no `.unwrap()` in production
- [ ] Serde types separate from domain models

## Red Flags
- Circular dependencies between modules
- God modules (>30 functions or >15 types)
- Primitive obsession (3+ primitive params)

## Output
```
## Coupling Review: {module}

### Issues
- [{severity}] {issue} — Suggestion: {fix}

### Metrics
- Balance Score: X.XX
- Classification: HighCohesion / LooseCoupling / Acceptable / Pain
```
