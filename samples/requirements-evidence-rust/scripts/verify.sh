#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
local_tools="$project_dir/target/tools/bin"
local_kani="$project_dir/target/kani"

if [ -x "$local_tools/cargo-kani" ]; then
    PATH="$local_tools:$PATH"
    export PATH
fi
if [ -d "$local_kani" ]; then
    KANI_HOME="$local_kani"
    export KANI_HOME
fi

cd "$project_dir"

cargo fmt --all -- --check
cargo +nightly fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
cargo test --doc
cargo test --release --test performance -- --ignored --nocapture
cargo bench --bench transfer
cargo kani \
    --harness successful_transition_preserves_total \
    --harness transition_result_matches_preconditions
RUSTUP_TOOLCHAIN=nightly cargo fuzz run parse_transfer -- \
    -max_total_time=15 -max_len=512
