# RustProbe

MIR-based performance profiler for Rust. Analyzes ownership operations (clone/move/drop) at the MIR level.

## Quick Start

```sh
cargo install --path crates/rustprobe
cd your-project
cargo rustprobe probe
cargo rustprobe report
```

## Architecture

- `rustprobe` — cargo subcommand (`cargo rustprobe`)
- `rustprobe-driver` — rustc_driver wrapper for MIR analysis
- `rustprobe-runtime` — lightweight runtime event collector
- `rustprobe-analysis` — offline analysis and reporting

## Requirements

- Rust nightly (pinned in `rust-toolchain.toml`)
- `rustc-dev` component

## License

MIT
