#!/bin/sh
set -eu

./scripts/check-toolchain.sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo flux --Fcheck-overflow strict

if cargo flux --features failing --Fcheck-overflow strict; then
    echo "expected failing Flux target to be rejected" >&2
    exit 1
fi
