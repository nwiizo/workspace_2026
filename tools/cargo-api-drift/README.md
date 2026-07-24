# cargo-api-drift

`cargo-api-drift` classifies public Rust API changes between the current working tree and a git ref as `breaking`, `risky`, or `safe`.

This is not a strict semver auditor. For comprehensive semver compatibility checks, use [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks). `cargo-api-drift` is positioned as a fast CI and release-notes helper that needs only git worktrees plus `ra_ap_syntax` CST parsing, not rustdoc JSON.

## Quick Start

```bash
cargo run -- --against HEAD~1
cargo run -- --summary --against main
cargo run -- --check --fail-on high --against origin/main
cargo run -- --ai --against HEAD~5
cargo run -- --changelog --against HEAD~1
```

As a cargo subcommand after installation:

```bash
cargo api-drift --against HEAD~1 --check
```

If `--against` is omitted, the tool tries `main`, then `master`, then `refs/remotes/origin/HEAD`, and finally `HEAD~1`. Fallback choices are printed to stderr. In a single-commit repository with no default branch ref, pass `--against` explicitly.

## Classification Model

| Class | Default severity | Examples | Default output |
| --- | --- | --- | --- |
| `breaking` | High to Critical | public item removal, public function/header signature changes, bound additions, public const/static type changes, exhaustive struct field additions, public field removal/type changes, exhaustive enum variant addition/removal, required trait method addition | shown |
| `risky` | Medium to High | `#[cfg]` / `#[cfg_attr]` changes, bound removals, `#[non_exhaustive]` struct field or enum variant addition, derive removal, default trait method addition, `repr` or public order changes, error enum variant addition, public re-export removal | shown |
| `safe` | Low | new public item, derive addition, additive public API | hidden unless `--all` |

Stable keys are shaped as:

```text
(classification, "rel_path:Type::item", change-kind)
```

Suppress a known finding with a nearby line or item comment:

```rust
// api-drift-allow: risky
#[non_exhaustive]
pub enum Event {
    Existing,
    New,
}
```

## Why Not cargo-semver-checks?

Use `cargo-semver-checks` when you need a strict semver audit. It uses rustdoc JSON and understands more of Rust's public API surface.

Use `cargo-api-drift` when you want:

- fast git-diff based feedback without rustdoc JSON,
- a separate `risky` layer for changes that may be semver-minor but still surprise downstream users,
- CI-friendly `check: PASS/FAIL` output aligned with the `design-gate-core` tool family,
- `--ai` review output and `--changelog` fragments.

## Blind Spot Policy

The blind spot manifest is part of the tool contract and is available with:

```bash
cargo run -- --blind-spots
```

Known blind spots include:

- strict semver auditing belongs to `cargo-semver-checks`,
- `macro_rules!` exports and proc-macro public signatures are not tracked,
- re-export and module visibility tracking is approximate and does not perform full Rust name resolution,
- cfg and feature matrices are parsed as source text, not as a complete cargo feature build matrix,
- `pub const` / `pub static` existence and type changes are tracked, but initializer expression changes are not classified,
- public exposure through type aliases, such as `pub type Alias = pub(crate) Inner`, is not resolved transitively.

This tool intentionally has no configuration-file support; use CLI flags and local `api-drift-allow:` suppressions instead.

## Changelog Fragment

`--changelog` emits a Keep a Changelog style fragment. It always includes all classifications, regardless of `--all`, so safe additions can appear in release notes while remaining hidden in default human output.

```markdown
## [Unreleased]

### Added
- **risky** `src/lib.rs:Event::New`: public enum variant `New` was added to `Event`

### Changed
- **risky** `src/lib.rs:Config::Clone`: derive `Clone` was removed from public type `Config`

### Removed
- **breaking** `src/lib.rs:old_fn`: public fn `old_fn` was removed
```

## CI

```bash
cargo api-drift --against origin/main --check --fail-on high
```

The `check:` line is printed directly after `Breakdown:` in human output. Exit code is non-zero only when `--check` is set and the configured threshold fails.

Unlike sibling design-gate tools that expose a separate `--baseline` ratchet, `cargo-api-drift` intentionally uses `--against` for that role: the compared git ref is both the API baseline and the CI ratchet point. `--check --fail-on` controls whether the current comparison fails the gate.
