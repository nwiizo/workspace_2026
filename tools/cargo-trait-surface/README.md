# cargo-trait-surface

`cargo-trait-surface` diagnoses Rust trait abstraction boundaries. It complements coupling-distance tools by looking for over-abstracted and under-abstracted trait surfaces: traits that are too large, abstractions with only one production implementation, dyn usage that conflicts with object safety, broad blanket impls, and public APIs that expose concrete I/O dependencies without a trait boundary.

This is a local design-gate tool and is not published to crates.io.

## Quick Start

```bash
cargo run -- .
cargo run -- --summary
cargo run -- --all
cargo run -- --json
cargo run -- --ai
cargo run -- --check --fail-on high
cargo run -- --baseline HEAD~1 --check
cargo run -- --blind-spots
cargo run -- --trait Repository
cargo run -- --jp
```

As a cargo subcommand after installation:

```bash
cargo trait-surface --check
```

Suppress a specific finding on an item with:

```rust
// trait-surface-allow: single-impl-abstraction
pub trait PaymentPort {
    fn charge(&self);
}
```

## Issue Types

- `oversized-trait`: method count or associated type count exceeds configured thresholds. Defaults are 10 methods and 4 associated types.
- `single-impl-abstraction`: a trait has zero or exactly one non-test implementation. Severity starts and stays `Low` because future extension intent is plausible; zero-impl findings say `no non-test implementations`.
- `object-safety-risk`: `dyn TraitName` is used while the trait contains methods that can break object safety, such as async methods, generic methods, `Self` parameters, or `Self` returns without `where Self: Sized`. Traits annotated with `#[async_trait]` are not flagged for async-method object-safety risk.
- `broad-blanket-impl`: `impl<T> Trait for T` or `impl<T: Bound> Trait for T` where the target is an impl generic parameter and the bounds are absent or broad, such as `Clone`, `Debug`, `Send`, or `Sync`. Bounds from `where` clauses are included and sorted in stable keys.
- `unmockable-boundary`: a public API signature exposes concrete file, network, process, or time types such as `std::fs::File` or `std::process::Command` without an intervening trait boundary. Function bodies are not scanned for this issue.

## Score Model

Findings are mapped to `design-gate-core` severities: `Low`, `Medium`, `High`, and `Critical`. The local formula combines:

- abstraction direction and degree, for example oversized trait magnitude or object-safety method count
- fan-in, approximated from production impls and `dyn` uses
- public exposure, where bare `pub` is public and `pub(crate)` is not

The grade is computed by `design-gate-core` from the severity distribution. `single-impl-abstraction` is intentionally capped at `Low` and excluded from grade weighting, including zero-impl findings, to avoid turning deliberate extension points into noisy low-severity grade churn. `--check` still evaluates these findings when the threshold includes `Low`.

## Configuration

Place `trait-surface.toml` at the crate root or an ancestor:

```toml
[thresholds]
methods = 12
associated_types = 5

[intent]
intentional_abstractions = [
  "PaymentPort",
  "Clock",
]
```

Traits listed under `[intent].intentional_abstractions` are excluded from `single-impl-abstraction`. Use this for architectural ports, test seams, plugin boundaries, or semver-stable extension points.

## Baseline and CI Gate

`--baseline <GIT_REF>` analyzes a detached worktree through `design-gate-core` and diffs stable issue keys:

```text
(issue_type, "rel_path:TraitName" or "rel_path:Type::item", target)
```

Line numbers are not part of the key, so formatting churn does not create false new issues. Public impl methods are qualified with the enclosing type, for example `src/lib.rs:Repository::load`, so same-named methods on different types do not collapse during deduplication or baseline diffing. With `--check`, the gate evaluates current issues or only new baseline issues when `--baseline` is present. The default threshold is `high`.

## Trait Detail Mode

`--trait <NAME>` prints one trait detail view. If no trait matches, it writes `error: trait '<NAME>' not found` to stderr and exits 1; case-insensitive or near substring matches are listed as suggestions. If multiple traits have the same name, it lists every `file:line` match on stderr, exits 1, and does not merge unrelated impls. `--trait` conflicts with report modes plus `--check` and `--baseline`.

## Blind Spots

The parser uses `ra_ap_syntax` CST walking and does not inspect comments as source code. It still uses approximation where Rust type resolution would be required:

- trait and impl matching is name based
- type aliases and re-exports are not resolved
- `unmockable-boundary` only recognizes a curated list of concrete std/tokio I/O, process, and time types
- blanket impl broadness is token based and conservative
- traits annotated with `#[async_trait]` are treated as macro-rewritten for async-method object-safety checks, but other proc-macro rewrites are not expanded
- cargo metadata or edition fallback notes are included in the blind spot manifest when package metadata cannot be read exactly

Run `--blind-spots` or inspect the JSON report for the manifest before treating results as a hard architectural verdict.
