# RustLean

MIR-based optimization assistance tool for Rust. Detects unnecessary clones, heap allocations, struct padding waste, and more — at compile time, without running your code.

## Features

- **Clone/Copy reduction**: Detect `.clone()` calls where move or borrow suffices
- **Allocation cost analysis**: Find heap allocations, especially inside loops
- **Struct layout optimization**: Detect padding waste and oversized stack values
- **Efficiency scoring**: Per-function and per-project optimization scores

## Requirements

- Nightly Rust with `rustc-dev` component:

```bash
rustup toolchain install nightly --component rustc-dev
```

## Usage

```bash
# Build
cargo build --release

# Run on a target crate (via cargo subcommand)
cd /path/to/your/crate
cargo rustlean

# Or run the driver directly on a single file
rustlean-driver your_file.rs --edition 2021

# JSON output
cargo rustlean --format json
```

## Configuration

Create `rustlean.toml` in your project root. See `rustlean.toml.example` for all options.

## How It Works

RustLean hooks into the Rust compiler via `rustc_driver` Callbacks. After semantic analysis, it walks the optimized MIR (Mid-level Intermediate Representation) of every function to detect performance anti-patterns:

1. **Clone detection**: Finds `TerminatorKind::Call` to `<T as Clone>::clone`, checks if the source is used afterward
2. **Allocation detection**: Pattern-matches calls to `Box::new`, `Vec::new`, `String::from`, `format!`, etc.
3. **Loop detection**: Identifies back-edges in the MIR CFG to flag operations inside loops
4. **Layout analysis**: Uses `tcx.layout_of()` to compute struct sizes and detect padding waste

## License

Apache-2.0
