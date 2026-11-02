#!/bin/sh
set -eu

expect_contains() {
    label=$1
    actual=$2
    expected=$3

    case $actual in
        *"$expected"*) ;;
        *)
            echo "$label version mismatch: expected $expected, got $actual" >&2
            exit 1
            ;;
    esac
}

flux_version=$(cargo flux --version)
rust_toolchain=$(rustup show active-toolchain)
fixpoint_version=$(fixpoint --version)
z3_version=$(z3 --version)

expect_contains Flux "$flux_version" "bd5de27"
expect_contains Rust "$rust_toolchain" "nightly-2026-10-22"
expect_contains Liquid-Fixpoint "$fixpoint_version" "0.9.6.3.8"
expect_contains Z3 "$z3_version" "4.16"
