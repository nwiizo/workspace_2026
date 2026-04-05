# RustLean

MIR-based optimization assistance tool for Rust.

## Commands

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Architecture

```
src/
├── lib.rs              # Library root (#![feature(rustc_private)])
├── error.rs            # thiserror error types
├── config.rs           # TOML config (rustlean.toml)
├── cost.rs             # Cost model and scoring
├── report.rs           # Text/JSON report generation
├── driver.rs           # rustc_driver Callbacks
├── analysis/
│   ├── mod.rs          # Diagnostic, AnalysisPass trait
│   ├── clone.rs        # Clone/Copy reduction
│   ├── alloc.rs        # Heap allocation detection
│   ├── layout.rs       # Struct layout / padding
│   └── loops.rs        # Loop detection (CFG back-edges)
└── bin/
    ├── rustlean_driver.rs  # rustc wrapper binary
    └── cargo_rustlean.rs   # cargo subcommand
```

## Key Constraints

- Requires nightly Rust with `rustc-dev` component
- `rustc_*` crates via `extern crate`, not Cargo.toml deps
- Skip const/static items in `optimized_mir()` (panics otherwise)
- macOS: `DYLD_LIBRARY_PATH` must include sysroot/lib at runtime
