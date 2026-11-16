#!/usr/bin/env bash
# 1. Does `cargo --config ... nextest run` pass --config to the cargo that
#    nextest runs? Read the opt-level from the build's fingerprint.
# 2. Rebuild dev opt-level=1 binaries and time "run only" for stable default,
#    opt-level=1, and --release in one hyperfine run with warmup.
# 3. Time edit -> test with nextest for opt-level=1, with the setting given
#    through the environment (CARGO_PROFILE_DEV_OPT_LEVEL), which nextest's
#    inner cargo inherits.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
work="$here/results/work/recheck"
t="$here/results/targets-recheck"
rm -rf "$work" "$t"
mkdir -p "$work"
rsync -a --exclude target "$here/../rust-grokking-ai-algorithms/" "$work/"
printf '\n[workspace]\n' >> "$work/Cargo.toml"
sed -i '' -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' "$work/src/search.rs"
cd "$work"

echo "== 1. opt-level passed to rustc by nextest's build"
echo "-- cargo --config profile.dev.opt-level=1 nextest list"
CARGO_TARGET_DIR="$t/fwd" cargo +stable --config profile.dev.opt-level=1 nextest list --cargo-verbose 2>&1 \
  | grep -o 'crate-name grokking_ai_algorithms[^`]*' | grep -o 'opt-level=[0-9]' | sort | uniq -c || echo "no opt-level flag (rustc default: 0)"
echo "-- CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo nextest list"
CARGO_TARGET_DIR="$t/env" CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable nextest list --cargo-verbose 2>&1 \
  | grep -o 'crate-name grokking_ai_algorithms[^`]*' | grep -o 'opt-level=[0-9]' | sort | uniq -c || echo "no opt-level flag (rustc default: 0)"

echo "== 2. run only"
CARGO_TARGET_DIR="$t/s0" cargo +stable build -q --example all_chapters
CARGO_TARGET_DIR="$t/opt1" cargo +stable --config profile.dev.opt-level=1 build -q --example all_chapters
CARGO_TARGET_DIR="$t/rel" cargo +stable build -q --release --example all_chapters
hyperfine -N --warmup 3 --runs 10 --style basic --export-json "$here/results/recheck-runonly.json" \
  -n stable-default "$t/s0/debug/examples/all_chapters" \
  -n dev-opt1 "$t/opt1/debug/examples/all_chapters" \
  -n stable-release "$t/rel/release/examples/all_chapters"

echo "== 3. edit -> test"
edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
for c in "s0|" "opt1|CARGO_PROFILE_DEV_OPT_LEVEL=1"; do
  name=${c%%|*}; env=${c#*|}
  eval "CARGO_TARGET_DIR=$t/test-$name $env cargo +stable test -q" > /dev/null 2>&1
  eval "CARGO_TARGET_DIR=$t/nextest-$name $env cargo +stable nextest run" > /dev/null 2>&1
done
hyperfine --shell=default --runs 10 --style basic --export-json "$here/results/recheck-test.json" \
  -n stable-default-cargo-test --prepare "$edit" "CARGO_TARGET_DIR=$t/test-s0 cargo +stable test -q" \
  -n stable-default-nextest --prepare "$edit" "CARGO_TARGET_DIR=$t/nextest-s0 cargo +stable nextest run --hide-progress-bar --status-level none --final-status-level none" \
  -n dev-opt1-cargo-test --prepare "$edit" "CARGO_TARGET_DIR=$t/test-opt1 CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable test -q" \
  -n dev-opt1-nextest --prepare "$edit" "CARGO_TARGET_DIR=$t/nextest-opt1 CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable nextest run --hide-progress-bar --status-level none --final-status-level none"
