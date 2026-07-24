# cargo-error-map

`cargo-error-map` diagnoses Rust error design drift. It builds an approximate error propagation graph from syntax, scores design risk, and emits human, JSON, CI, and AI-agent oriented reports.

## Quick Start

```bash
cargo run -- error-map .
cargo run -- error-map --summary .
cargo run -- error-map --all .
cargo run -- error-map --json .
cargo run -- error-map --ai .
cargo run -- error-map --check .
```

Installed as a cargo subcommand, it supports:

```bash
cargo error-map [PATH]
cargo error-map --check --baseline main --fail-on=high
```

The binary also works standalone:

```bash
cargo-error-map [PATH]
```

## Issue Types

- `anyhow-leak`: `anyhow::Result` or `anyhow::Error` appears in library public API signatures, including public trait methods and public struct/enum field or variant payload types.
- `error-enum-bloat`: a `thiserror` enum has more variants than the configured threshold. It does not yet detect unrelated domain concepts mixed into one enum.
- `missing-context`: a chain of `?` propagation reaches the configured depth without `.context()` or `.with_context()`.
- `boundary-panic`: `unwrap`, `expect`, or `panic!` appears outside boundary code.
- `dyn-error-exposure`: `Box<dyn Error>` appears in library public API signatures.

Boundary code defaults to `main.rs`, `src/bin/`, `tests/`, `benches/`, `examples/`, `src/handlers/`, `src/routes/`, and `src/api/`. Inline `#[cfg(test)]` modules and test functions are excluded from boundary-panic findings.

## Score Model

Each issue receives `Low`, `Medium`, `High`, or `Critical`.

The severity model combines:

- reach: public API exposure weighs more than crate-internal findings
- layer: library/internal code weighs more than boundary code
- fan-in: same-crate callers approximate call frequency
- issue type: API leaks and panics start with higher weight

Project grade is A-F from weighted issue volume. The grade is intentionally conservative: a few critical public API findings can pull a project down even when total issue count is low.

## Propagation Graph

```bash
cargo error-map --graph
cargo error-map --graph=dot
```

Text output uses:

```text
caller -> callee [?]
caller -> callee [context]
caller -> callee [call]
```

`?` means the caller contains question-mark propagation. `context` means the caller contains `.context()` or `.with_context()`. DOT output can be rendered by Graphviz.

## Baseline and CI Gate

```bash
cargo error-map --baseline main
cargo error-map --check
cargo error-map --check --baseline main --fail-on=medium
```

Issue identity is stable by `(issue_type, source, target)`. `source` is `rel_path:item_name` with a same-file disambiguation suffix only when needed, and line numbers are display-only. A baseline run creates a temporary git worktree for the requested ref, analyzes the same repository-relative path, and reports new, resolved, and unchanged issue keys. With `--check --baseline`, only new issues at or above `--fail-on` fail the gate.

`--check` prints an explicit gate line in human-oriented output:

```text
check: FAIL (fail-on=high, 2 issue(s) at/above threshold)
```

JSON includes:

```json
"gate": { "passed": false, "fail_on": "high", "failing": 2 }
```

## Blind Spot Policy

This tool uses syntax only. It does not perform type resolution, macro expansion, trait dispatch resolution, or cfg evaluation. Cargo edition is read from `cargo metadata` and passed to the parser; if it cannot be read, sources are parsed as Edition2024 and the blind spot manifest records that fallback. The blind spot manifest is always included in `--json` and `--ai`, and can be shown in text mode:

```bash
cargo error-map --blind-spots
```

A clean report means no configured syntactic risk was found. It does not prove the absence of error design problems hidden behind macros, dynamic dispatch, generated code, type aliases, approximate public trait/impl reachability, or unresolved call paths. If call graph resolution falls back from same-file and same-module matching to bare function names, the blind spot manifest records that approximation.

## Suppressions

Use a line or immediately preceding item comment:

```rust
// error-map-allow: boundary-panic
pub fn invariant_checked_elsewhere() {
    panic!("documented invariant");
}
```

Multiple issue types are comma-separated:

```rust
// error-map-allow: boundary-panic, missing-context
```

`all` suppresses every cargo-error-map issue at that location.

When suppressions apply, every output mode reports the number of suppressed issues.

## Configuration

Create `error-map.toml` at the project root or an ancestor:

```toml
[thresholds]
enum_variants = 16
context_depth = 4

boundary_layers = [
  "/xtask/",
  "/cli/",
]

allow = [
  "missing-context",
]
```

`allow` is a project-wide escape hatch. Prefer local `error-map-allow` comments when a suppression has a narrow reason.

## Japanese Output

```bash
cargo error-map --japanese
cargo error-map --jp
```
