# cargo-test-gap

`cargo-test-gap` ranks Rust functions by where tests are likely to pay off first.

It complements coverage-only tools such as `cargo-llvm-cov` and heavier tools such
as `cargo-mutants` by producing a risk-sorted list:

```text
risk = churn * complexity * exposure / (coverage + 1)
```

The output is designed for ratcheting with `--baseline`: existing gaps can remain
visible while `--check --baseline <ref>` gates only newly introduced risky gaps.

## Quick Start

```bash
cargo run -- .
cargo run -- . --top 20 --all
cargo run -- . --check --fail-on high
cargo run -- . --baseline main --check
cargo run -- . --llvm-cov target/llvm-cov/export.json
```

Installed as a cargo subcommand, the equivalent form is:

```bash
cargo test-gap --summary
cargo test-gap --json --top 50
cargo test-gap --ai --baseline main
```

## Score Model

Each reported `test-gap` candidate carries both the final `risk` and the raw axis
scores used to compute it.

- `churn`: repo-wide `git log --name-only` file history. The first commit is
  treated as churn `0`; later changes add count and recent-change weight. If git
  history cannot be read, the tool falls back to a conservative approximation and
  records a blind spot. Churn is intentionally file-grained: every function in
  the same file receives the same churn score.
- `complexity`: CST-based approximate cyclomatic complexity. It counts
  `if`/`loop`/`while`/`for`, `match` arms, and `&&` / `||`. Closure bodies are
  counted inside the enclosing function.
- `exposure`: public API and error-path exposure. Only bare `pub` discovered via
  `visibility_inner().is_none()` counts as public; `Result` return types add
  error-path weight.
- `coverage`: from `--llvm-cov <path>` when provided. Without llvm-cov JSON, the
  fallback is an approximation: direct calls from same-crate `#[test]` functions
  and functions inside `#[cfg(test)] mod` are treated as covered. The fallback
  scans Rust files under `src/`, `tests/`, `benches/`, and `examples/`.

Severity is derived from risk using four bands: Low, Medium, High, Critical.
Grade is normalized for this tool's "every production function is a candidate"
model: Low findings are excluded from grade calculation, and the grade is based
on the ratio of High/Critical candidates to total candidates.

Issue identity is stable by:

```text
(test-gap, rel_path:Type::fn, stable exposure labels)
```

The key intentionally avoids raw line numbers, raw churn values, and churn /
complexity / coverage bucket labels. Crossing a bucket threshold should not
create noisy `new` / `resolved` baseline churn when the function identity and
exposure are unchanged. Raw scores remain visible in human, JSON, and AI output.

## cargo-llvm-cov

Generate JSON with `cargo-llvm-cov` and pass it to the tool:

```bash
cargo llvm-cov --json --output-path target/llvm-cov/export.json
cargo test-gap --llvm-cov target/llvm-cov/export.json
```

The parser accepts the common llvm-cov shape with `functions`, `name`,
`filenames`, `regions`, and count/percent fields. Function matching prefers the
crate-root-relative file key plus qualified names such as `Type::method`. Bare
name matching is only a limited fallback for unambiguous free functions. Region
coverage uses llvm-cov's execution-count field and falls back to function-level
`count` when region data is empty or all-zero.

If a valid-looking JSON file matches zero production functions, the command emits
a stderr warning and records a blind-spot note so accidental 0% reports are not
silent. A missing path is a hard error:

```text
error: llvm-cov file does not exist: ...
```

## Baseline And Gates

```bash
cargo test-gap --baseline main
cargo test-gap --baseline main --check --fail-on medium
```

With `--baseline`, the report shows `new`, `resolved`, and `unchanged` issue
keys. With `--check --baseline`, only new issues at or above `--fail-on` fail the
gate. Without `--baseline`, all current issues are considered by the gate.

The `check: PASS/FAIL` line appears after suppression and baseline summary lines.
Human output applies `--top` only to the displayed ranked list. JSON always emits
all candidates and includes `total_candidates`; AI output is not truncated by
`--top`.

## Suppression

Use a local suppression comment immediately above the function:

```rust
// test-gap-allow: test-gap
pub fn intentionally_untested() {}
```

`all` is also accepted by the shared suppression matcher, but local
`test-gap-allow: test-gap` comments are preferred.

## Blind Spots

This tool is intentionally syntactic. It uses `ra_ap_syntax` CST walking and does
not expand macros, perform type resolution, execute tests, or resolve trait and
dynamic dispatch. The blind spot manifest is always included in JSON and AI
output and can be shown directly:

```bash
cargo test-gap --blind-spots
```

Important blind spots:

- macro-generated functions, branches, or tests are not expanded
- churn is file-grained, not function-grained
- fallback coverage is approximate and only sees direct syntactic test calls from
  `src/`, `tests/`, `benches/`, and `examples/`
- llvm-cov JSON path/name mismatches are reported, but unmatched functions still
  fall back to 0% coverage
- qualified names are preferred for coverage; bare-name fallback is approximate
- path-based git history can undercount complex rename/copy/split histories
- inactive cfg branches and generated files are only analyzed if present as Rust
  source files
