# cargo-feature-doctor

`cargo-feature-doctor` statically lists Cargo feature flag risks without building the crate.

It is a design diagnostic tool, not a replacement for `cargo-hack`. Use it to find suspicious
feature combinations and public API exposure first, then run targeted `cargo-hack` commands for the
cases that matter.

## Quick Start

```bash
cargo run -- .
cargo run -- --summary .
cargo run -- --json .
cargo run -- --check --fail-on high .
cargo run -- --matrix .
cargo run -- --suggest-hack .
```

As a cargo subcommand after installation:

```bash
cargo feature-doctor --check
```

## Issue Types

- `default-leak`: a dependency is used with default features enabled, and that dependency's default feature set appears to enable heavy or optional functionality.
- `exclusive-undeclared`: feature pairs such as `rt-tokio` / `rt-async-std` or `tls-native` / `tls-rustls` exist without a `compile_error!` guard for simultaneous enablement.
- `untested-cfg-path`: a `#[cfg(feature = "...")]` branch is false under both default features and all features.
- `optional-dep-exposure`: an optional dependency type appears in a public API signature without a matching feature gate.
- `non-additive-feature`: a public API item is removed by `#[cfg(not(feature = "..."))]`.

## Scoring Model

Severity is derived from:

- estimated affected feature combinations
- whether the issue reaches public API
- approximate usage breadth

`usage` is issue-type specific. For public API diagnostics it counts matched public items. For
`default-leak` it counts risky default entries expanded from the dependency feature graph. The
planned downstream-dependent-crate axis is not implemented yet, so `default-leak` findings are
usually `Low` or `Medium` unless other score inputs change.

The shared `design-gate-core` severity scale is used: `Low`, `Medium`, `High`, `Critical`.
The project grade uses the same shared A-F grade model as the sister design-gate tools.

Large feature sets also emit a powerset note, for example `2^12 = 4096` combinations, so the report
shows when static narrowing should be followed by targeted `cargo-hack` runs.

## Configuration

```bash
cargo feature-doctor --baseline main --check --fail-on high
```

With `--baseline`, the gate is applied only to new issues. Without `--baseline`, it is applied to all
current issues. `--check` always prints a `check: PASS` or `check: FAIL` line and includes a JSON
gate object when `--json` is used.

`feature-doctor.toml` can suppress manifest-originated issue types:

```toml
allow = ["default-leak"]
```

## Blind Spots

```bash
cargo feature-doctor --blind-spots
```

Important limits are intentional:

- The tool does not build the crate.
- `untested-cfg-path` only evaluates two synthetic points: default features and all features.
- Non-feature cfg predicates such as `unix` are treated as unknown, not as feature constraints.
- Parent-file `#[cfg(feature)] mod x;` attributes are not propagated into `x.rs`.
- Mutually exclusive feature pairs can overlap `untested-cfg-path` because the two-point model does
  not solve full feature constraints.
- CI workflows, target triples, build scripts, and downstream workspace feature unification are not fully modeled.
- Macro expansion and full Rust type resolution are not performed.

These limits are reported in the blind spot manifest so CI output states what the static report can
and cannot prove.

## Suppressions

Rust source issues can be suppressed near the affected item:

```rust
// feature-doctor-allow: optional-dep-exposure
pub fn leak(value: serde::Serialize) {}
```

Use `all` to suppress all issue types for an item:

```rust
// feature-doctor-allow: all
```

Manifest-originated issues, such as `default-leak`, are suppressed in `feature-doctor.toml`:

```toml
allow = ["default-leak"]
```

Suppression keys are stable and line-independent:

```text
(issue_type, source, target)
```

For example:

```text
(optional-dep-exposure, src/lib.rs:leak, serde)
```
