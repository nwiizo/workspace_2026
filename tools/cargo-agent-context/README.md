# cargo-agent-context

`cargo-agent-context` is a Rust repo reporter for AI coding agents. It produces one deterministic markdown file that summarizes crate shape, module/API surface, usable local commands, conventions, and sibling design-tool JSON results.

This tool is intentionally not a detector. It has no score model, no severity threshold, no grade calculation, no `--check`, and no baseline gate.

## Quick Start

```bash
cargo run -- . --format markdown
cargo run -- ../kuroko --run --output kuroko-agent-context.md
cargo run -- ../kuroko --from ./reports --format agents-md
```

As an installed cargo subcommand:

```bash
cargo agent-context PATH --format markdown --output agent-context.md
```

## Output Sections

1. `Overview`: crate name, edition, crate type, workspace members, and direct dependency summary from `cargo_metadata`.
2. `Module graph`: top-level modules and first-level child modules, with public item counts and major public type names from `ra_ap_syntax` CST parsing.
3. `Key types & public API`: public `struct`, `enum`, `trait`, and free `fn` items, grouped by module and ranked by simple fan-in, capped by `--top`.
4. `Build & test commands`: commands backed by visible repository files, such as `Cargo.toml`, `rustfmt.toml`, `clippy.toml`, or Cargo lints.
5. `Known risks`: integrated sibling tool JSON summaries.
6. `Blind spots & caveats`: this reporter's syntax-only limits plus sibling blind spot notes.
7. `Conventions`: presence and first lines of `rustfmt.toml`, `clippy.toml`, `deny.toml`, `CLAUDE.md`, and `AGENTS.md`.

## Sibling Tool Integration

Use `--from <dir>` when CI or a previous run has already generated JSON reports:

```bash
cargo agent-context . --from ./reports
```

The directory may contain any subset of:

- `boundary.json`
- `error-map.json`
- `async-smell.json`
- `trait-surface.json`
- `feature-doctor.json`
- `test-gap.json`
- `api-drift.json`

Missing files are reported as `not provided`. JSON files with unknown fields are accepted. JSON files missing expected top-level fields such as `grade` or `issues` are reported as `schema mismatch` for that tool only.

Use `--run` when sibling binaries are available locally:

```bash
cargo agent-context . --run
```

The reporter looks for each binary on `PATH`, then at `../<tool>/target/release/<binary>` and `../<tool>/target/debug/<binary>`. Missing tools are marked `not run`. If no sibling binaries are available, the Known risks section says `no sibling tools available` and exits successfully.

## Blind Spot Policy

`cargo-agent-context` parses Rust source with `ra_ap_syntax` and does not expand macros, run rustc name resolution, or evaluate most `cfg` branches. It excludes items under `#[cfg(test)]`, but target-specific APIs can still be summarized together. Workspace roots list members and conventions; pass a member crate path for per-crate API context.

Sibling tool blind spot manifests are copied as short notes when they are present in JSON output. The reporter does not reinterpret sibling severities or grades.

## CLI

```text
cargo agent-context [PATH]
  --format markdown|agents-md|claude-md
  --output <file>
  --from <dir>
  --run
  --top <N>
  --japanese
```

`--from` and `--run` are mutually exclusive. Usage errors exit with code 2. Runtime errors exit with code 1.
