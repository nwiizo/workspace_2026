#!/bin/sh
set -eu

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo verus build -p verus-discount-verification

if cargo verus focus -p verus-discount-verification --features failing; then
    echo "expected failing Verus target to be rejected" >&2
    exit 1
fi
