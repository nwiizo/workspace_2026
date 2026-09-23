# cargo-anti-slop

`cargo-anti-slop` is a cargo subcommand that diagnoses low-evidence "slop" patterns in Rust code. It is a Rust reinterpretation of [dmmulroy/anti-slop](https://github.com/dmmulroy/anti-slop) (opinionated Oxlint rules against low-evidence TypeScript): code that fabricates or discards type evidence through erased types, runtime type probing, lossy cast chains, and silently fabricated defaults — the patterns AI assistants reach for when they lack evidence about the data they are handling.

It also carries the function-boundary side of "型は壁 — バグを直すな、表現できなくせよ" (関数型まつり 2026, see `blogs/contents/rust-types-as-walls/` and `samples/rust-types-as-walls/`): validation that leaves no proof, raw identifiers that can be swapped, and derives that tunnel through smart constructors. Struct-field walls (bool-option-pair, raw-id-field, status-string-field) belong to `rbp-lint`; this tool owns function signatures and flows.

The job it solves: catch low-evidence Rust before commit/review, and stop it from getting worse via a baseline ratchet — with severity grading, an AI repair plan, and Japanese output, consistent with the design-gate tool family.

## Quick Start

```bash
cargo run -- .
cargo run -- --summary
cargo run -- --print          # include the offending source line
cargo run -- --json
cargo run -- --ai
cargo run -- --check --fail-on high
cargo run -- --baseline HEAD~1 --check
cargo run -- --blind-spots
cargo run -- --jp
```

When installed as a cargo subcommand:

```bash
cargo anti-slop --check
```

## Detected Issue Types

- `chained-cast`: `x as u8 as u64` style chains. Visible narrowing / float-to-int steps and narrow-then-widen masks score high.
- `erased-signature`: fn params/returns or struct fields typed `serde_json::Value`, `toml::Value`, `serde_yaml::Value`, `Box<dyn Any>`, `&dyn Any` (behind `Box`/`Rc`/`Arc`/`Option`/`Vec`/`Cow` too). Trait impl methods are skipped (the trait dictates the signature); the trait declaration itself is checked.
- `erased-alias`: `type Config = serde_json::Value;` — an alias that conceals erasure without restoring evidence.
- `erased-dictionary`: `HashMap`/`BTreeMap`/`IndexMap`/`DashMap` whose value type is erased. A field marked `#[serde(flatten)]` is a declared forward-compatibility boundary and scores Low.
- `runtime-downcast`: `downcast_ref/mut/downcast`, `is::<T>()`, `TypeId::of` probing. Two or more in one function (poor man's enum) score high; error-looking receivers (`err`/`error`/`cause`/`source`) score low, mirroring anti-slop's `cause` exception.
- `widen-then-assert`: a locally known value is widened (`let x: Box<dyn Any> = Box::new(v)`, `let v = json!({...})`, `serde_json::to_value(v)`) and re-asserted in the same function (`downcast_*`, `v["k"]`, `as_str()`...). Values born erased by boundary parsing (`from_str`/`from_slice`/`from_reader`/`.parse`) are not reported.
- `default-swallow`: `.ok();` in statement position, and `unwrap_or_default()` after a fallible chain (`.parse`, `from_str`, `try_into`, `try_from`, `.ok()`) that fabricates a value on failure.
- `vague-symbol-name`: struct/enum/trait/alias names that are exact or CamelCase-suffix matches against a configurable low-signal word list (`Data`, `Info`, `Helper`, `Util(s)`, `Manager`, `Wrapper`, `Misc`, `Temp`, `Stuff`, `Thing`).
- `bool-validation`: a function named like validation (`*valid*`, `verify*`) that takes input and returns bare `bool`, discarding the proof. Parse, don't validate: return `Result<TypedValue, E>` from a smart constructor. State queries like `fn is_valid(&self)` and single-byte/char classifier predicates are not flagged.
- `raw-id-params`: two or more identifier parameters (`id`, `*_id`) sharing the same raw type (`u64`, `String`, ...) in one signature — call sites can swap them silently; wrap each in a newtype.
- `constructor-bypass`: a private-field struct with a smart constructor (`new`/`parse`/`try_*`/`from_str` returning `Result`, not taking `self` — `try_clone(&self)` is a transformation, not a constructor) that also derives `Default` or `Deserialize` (without `#[serde(try_from)]`/`#[serde(from)]`), or has an infallible `impl From<...>` — each is a tunnel around the validation.
- `orphan-file`: a `.rs` file under `src/` that no `mod` declaration reaches from `lib.rs`/`main.rs`/`src/bin` roots — typically a refactor leftover (human or agent). The orphan is reported once and its content findings are masked, since the code is never compiled. Runs only for single-crate `src/` layouts; `#[path]` attributes and inline module nesting are resolved, `include!` is not.

Test code (`#[test]`, `#[tokio::test]`, `#[cfg(test)]` modules) is excluded from all rules.

## anti-slop Rule Mapping

| anti-slop (TS/Oxlint) | cargo-anti-slop (Rust) |
|---|---|
| no-chained-type-assertions | `chained-cast` |
| no-unknown-parameters / no-unknown-returns / no-object-parameters | `erased-signature` |
| no-unknown-type-aliases | `erased-alias` |
| no-unsafe-dictionary-type | `erased-dictionary` |
| no-runtime-typeof / no-reflect-get | `runtime-downcast` |
| no-widen-then-assert / no-known-value-widening | `widen-then-assert` |
| no-conditional-empty-object-spread | `default-swallow` (silently produced empty/default values) |
| no-shape-in-symbol-names | `vague-symbol-name` |
| require-safety-comment-for-type-assertion | partially `chained-cast`; SAFETY comments are rbp-lint territory |
| no-module-mocking / no-reflect-apply | not mapped (no Rust equivalent) |

## 型は壁 (rust-types-as-walls) Rule Mapping

Function-boundary rules derived from the talk's four patterns. Struct-field counterparts live in `rbp-lint`.

| Talk pattern | cargo-anti-slop (fn boundary) | rbp-lint (struct field) |
|---|---|---|
| Smart constructor / parse-don't-validate | `bool-validation`, `constructor-bypass` | `pub-field-newtype` |
| Newtype for identifiers | `raw-id-params` | `raw-id-field` |
| Correct combinations via enums | — | `bool-option-pair`, `status-string-field` |
| Boundary parsing | exempts `widen-then-assert`; remediation for `erased-signature` | — |

## Score Model

Severity uses the shared `design-gate-core` `Severity` and `Grade` types.

```text
severity = risk impact + condition ease + git volatility
```

Impact is ordered data-loss (wrong values / swallowed errors at runtime) > evidence (compiler guarantees defeated) > signal (intent legibility). Condition comes from visibility (pub > pub(crate) > private), chain lossiness, downcast density, or fallible-marker presence. Volatility counts file changes in recent git history; on a shallow clone (CI checkouts default to depth 1) the axis is disabled instead of inflating every finding by one notch. The evidence axis intentionally never reaches Critical.

Grade tracks the default view: Medium+ findings drive the bands (B/C/D/F by weighted count), while Low findings — hidden without `--all` — only demote A to B. A Value-centric design with hundreds of Lows reads as "B with a documented tradeoff", not F.

## Baseline And CI Gate

`--baseline <GIT_REF>` uses `design-gate-core` to create a temporary detached worktree with a Drop guard. Issue keys are stable and repo-relative:

```text
(issue_type, "rel_path:Type::method", target)
```

Line numbers are not part of the key. With `--check`, the output includes a `check: PASS` or `check: FAIL` line and the exit code is non-zero on failure. JSON output includes a `gate` object.

## Suppressions

Use a line or item-level comment. Always attach the reason after `--` (or in parentheses) so the exception carries its justification, as the rbp-lint rollout playbook recommends:

```rust
// anti-slop-allow: erased-signature -- protocol dispatcher, Value is the contract
pub fn dispatch(payload: Value) -> Value {
    route(payload)
}
```

`all` is also accepted by the shared suppression resolver.

## Phased Rollout

For an existing codebase, follow note → warning → error in three steps: start with `cargo anti-slop --all` to see the landscape, gate CI with `--baseline origin/main --check` so only new findings fail, then tighten `--fail-on` (high → medium) as the backlog shrinks.

## Configuration

Create `anti-slop.toml` near the analyzed path. All lists extend the defaults; `allow` disables issue types globally.

```toml
erased_types = ["simd_json::OwnedValue"]
map_types = ["FxHashMap"]
vague_words = ["Blob"]
fallible_markers = [".fetch"]
allow = [
  # "vague-symbol-name",
]
```

## Positioning (related tools)

- **clippy**: overlaps only at the edges — `cast_*` pedantic lints judge single casts and `unused_result_ok` (restriction, opt-in) flags bare `.ok();`. Clippy has no lints for `Value`/`Any` in signatures, downcast probing, widen-then-assert, or fallible `unwrap_or_default`. This tool adds chain/context detection, severity grading, a baseline ratchet, and an AI repair plan instead of replacing clippy.
- **rbp-lint** (this repo): owns unwrap/expect/panic/SAFETY-comment slop and the struct-field walls (bool-option-pair, raw-id-field, status-string-field, pub-field-newtype). This tool takes the function-boundary side; neither re-implements the other.
- **antislop (crates.io, skew202)**: multi-language comment/placeholder slop (TODO, hedging, stubs) — a different axis.
- **mizchi/similarity**: AST duplicate-code detection — the duplication axis of slop. Its `--print` UX (show the offending code inline) is adopted here.
- **dylint**: the type-resolved alternative implementation path; rejected here to stay consistent with the CST-based design-gate family and avoid rustc toolchain coupling.

## Blind Spots

The tool uses `ra_ap_syntax` CST walking and does not invoke rustc type resolution. Simple `use` aliases and grouped imports are resolved for erased types, maps, `json!`-style macros, and `TypeId::of`; glob imports, re-exports, and newtype wrappers concealing `Value`/`Any` are missed. A user-defined `Any` trait is indistinguishable from `std::any::Any`. `widen-then-assert` follows single bindings inside one function only. `default-swallow` matches textual fallible markers and does not trace helper functions. Chain lossiness is judged only between written cast types; the source expression's type is unknown. `bool-validation` is name-based, so crypto-style `verify` APIs that legitimately return `bool` need suppression. `raw-id-params` and `constructor-bypass` are single-file checks: aliased raw types and impls in other files are not resolved. Use `--blind-spots` for the full manifest.
