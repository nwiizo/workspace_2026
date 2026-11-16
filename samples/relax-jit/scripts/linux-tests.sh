#!/usr/bin/env bash
# Edit -> test with --no-fail-fast, so every test runs even when one fails
# (one test of the sample fails under rustc on aarch64 Linux). The same build
# settings are compared between `cargo test` and cargo-nextest. Settings go
# through the environment, which nextest's inner cargo inherits.
set -uo pipefail

out=/out/tests
mkdir -p "$out"
exec > >(tee -a "$out/run.log") 2>&1
export PATH=/opt/bin:$PATH
cd /work/grokking || exit 1
edit="sed -i -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
WILD="-Clinker=clang -Clink-arg=--ld-path=/opt/bin/wild"
nx="nextest run --no-fail-fast --hide-progress-bar --status-level none --final-status-level none"

args=()
for v in "default|" "opt1|CARGO_PROFILE_DEV_OPT_LEVEL=1" "wild-opt1|RUSTFLAGS='$WILD' CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only"; do
  name=${v%%|*}; env=${v#*|}
  t="CARGO_TARGET_DIR=/t3/test-$name $env cargo +stable test -q --no-fail-fast"
  n="CARGO_TARGET_DIR=/t3/nextest-$name $env cargo +stable $nx"
  eval "$t" > "$out/cargo-test-$name.txt" 2>&1
  eval "$n" > "$out/nextest-$name.txt" 2>&1
  args+=(-n "$name-cargo-test" --prepare "$edit" "$t" -n "$name-nextest" --prepare "$edit" "$n")
done
hyperfine -i --shell=bash --style basic --runs 10 --export-json "$out/grokking-test.json" "${args[@]}" > "$out/grokking-test.log" 2>&1
echo "tests exit=$?"
for f in "$out"/cargo-test-*.txt; do echo "$(basename "$f"): $(grep 'test result' "$f" | awk '{p+=$4; f+=$6} END {print p" passed, "f" failed"}')"; done
for f in "$out"/nextest-*.txt; do echo "$(basename "$f"): $(grep -E 'tests? run' "$f" | tail -1)"; done
