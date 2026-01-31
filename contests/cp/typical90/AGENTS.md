# Repository Guidelines

## Project Structure & Module Organization
This crate lives under `contests/cp/typical90` inside `workspace_2026`; scan the nearest `CLAUDE.md` before refactors so your change matches the directory’s charter. The root package now exposes only the shared library (`src/lib.rs` plus helpers such as `dp.rs`, `graph.rs`, `convolution.rs`, `modint.rs`, `segtree.rs`, `string_algo.rs`) and its regression tests under `tests/comprehensive_tests.rs`. Problem binaries are grouped into the sibling crates `crates/typical90-set1|2|3`, each shipping `src/bin/NNN_problem_name.rs` and a matching `[[bin]] name = "NNN"` entry so we can build smaller slices at a time. Generated artifacts still stay inside `target/`.

## Build, Test, and Development Commands
Run hygiene from this workspace root in the order `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets`. When iterating on a specific slice, feel free to target a member package (e.g., `cargo clippy -p typical90-set2 --bins`). To replay AtCoder inputs quickly, execute the bin inside the matching crate: `cargo run -p typical90-set2 --bin 040 < samples/040.txt`. Scripts should stay in `scripts/` if a reusable runner is required. When introducing new dependencies, validate lockfiles with `cargo check` before pushing.

## Coding Style & Naming Conventions
Code follows `rustfmt` defaults (4 spaces, 100-column mindset). Problem modules stay snake_case with the `NNN_problem_name.rs` pattern, matching the numeric identifier already present in `Cargo.toml`; helper types remain `PascalCase`, constants SCREAMING_SNAKE_CASE, and generic functions snake_case. Prefer explicit generics over macros, keep modules focused (DP helpers in `dp.rs`, data structures in `segtree.rs`, etc.), and document any tricky math with a brief `///` comment.

## Testing Guidelines
Leverage Rust’s built-in `#[test]` blocks for narrow unit verification and expand `tests/comprehensive_tests.rs` for randomized or multi-problem sweeps (see existing `yokan_party` and `longest_circular_road` modules). Each new algorithm should ship AtCoder sample assertions plus at least one boundary case mirroring the official constraints. Before submitting, rerun `cargo test --all --all-targets` and capture any reproduction commands you expect reviewers to use.

## Commit & Pull Request Guidelines
Commits follow Conventional Commits scoped to this crate, e.g., `feat(typical90): add segment tree helpers` or `fix(typical90): handle zero-length cuts`. Squash noisy WIP history locally. PRs must describe the scenario solved, list the exact commands executed (`cargo fmt`, `cargo clippy`, `cargo test`), attach sample IO pairs when behavior changes, and reference any upstream contest discussion or blog note that motivated the tweak. Include new env vars or scripts inline so other agents can replay the workflow unassisted.
