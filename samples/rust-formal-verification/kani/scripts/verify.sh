#!/bin/sh
set -eu

expect_failure() {
    label=$1
    shift

    if "$@"; then
        echo "expected failure but succeeded: $label" >&2
        exit 1
    fi
}

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
expect_failure generated-overflow \
    cargo test --features failing generated_inputs_find_intermediate_overflow -- --ignored
cargo kani \
    --harness valid_rate_round_trips \
    --harness invalid_rate_is_rejected \
    --harness discounted_price_never_increases

expect_failure off-by-one \
    cargo kani --harness detects_off_by_one_constructor
expect_failure intermediate-overflow \
    cargo kani --harness detects_intermediate_overflow
