#!/usr/bin/env bash
# Commonly recommended Rust build/dev-loop speedups outside the Cargo book's
# build-performance guide, on stable, with a fresh baseline in the same run.
#
#   WORKLOAD=grokking|cli-deps MODE=edit|build|runonly|cold|test scripts/common-tricks.sh
#
# dev-opt1        [profile.dev] opt-level = 1
# deps-opt3       [profile.dev.package."*"] opt-level = 3
# build-override  [profile.dev.build-override] opt-level = 3 (proc macros, build scripts)
# common-all      the three above + the Cargo book's debuginfo reduction
# sccache         RUSTC_WRAPPER=sccache, CARGO_INCREMENTAL=0 (sccache does not cache
#                 incremental builds); binary from results/tools (see README)
# nextest         `cargo nextest run` instead of `cargo test` (MODE=test only)
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
workload=${WORKLOAD:-grokking}
mode=${MODE:-edit}
if [ "$mode" = cold ]; then runs=${RUNS:-3}; else runs=${RUNS:-10}; fi
sccache="$here/results/tools/sccache"
export SCCACHE_DIR="$here/results/sccache-cache"
targets="$here/results/targets-common-$workload"
work="$here/results/work/common-$workload"

rm -rf "$work"
mkdir -p "$work" "$targets"
case "$workload" in
  grokking)
    rsync -a --exclude target "$here/../rust-grokking-ai-algorithms/" "$work/"
    printf '\n[workspace]\n' >> "$work/Cargo.toml"
    sed -i '' -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' "$work/src/search.rs"
    edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
    target_args="--example all_chapters"
    bin_rel="examples/all_chapters"
    ;;
  cli-deps)
    rsync -a --exclude target "$here/cli-deps/" "$work/"
    edit="sed -i '' -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"
    target_args=""
    bin_rel="cli-deps"
    ;;
esac

debuginfo="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
opt1="--config profile.dev.opt-level=1"
deps3="--config 'profile.dev.package.\"*\".opt-level=3'"
bo3="--config profile.dev.build-override.opt-level=3"

configs=(
  "stable-default|cargo +stable"
  "dev-opt1|cargo +stable $opt1"
)
if [ "$workload" = cli-deps ]; then
  configs+=("deps-opt3|cargo +stable $deps3" "build-override|cargo +stable $bo3")
fi
configs+=(
  "common-all|cargo +stable $debuginfo $opt1 $deps3 $bo3"
  "sccache|RUSTC_WRAPPER=$sccache CARGO_INCREMENTAL=0 cargo +stable"
)

cd "$work"
args=()
for entry in "${configs[@]}"; do
  name=${entry%%|*}
  tdir="$targets/$name"
  prefix="CARGO_TARGET_DIR=$tdir ${entry#*|}"
  case "$mode" in
    edit)
      cmd="$prefix run -q $target_args"
      eval "$cmd" > "$here/results/common-$workload-$name.out" 2> /dev/null
      args+=(--command-name "$name" --prepare "$edit" "$cmd")
      ;;
    build)
      cmd="$prefix build -q $target_args"
      eval "$cmd" > /dev/null 2>&1
      args+=(--command-name "$name" --prepare "$edit" "$cmd")
      ;;
    runonly)
      eval "$prefix build -q $target_args" > /dev/null 2>&1
      args+=(--command-name "$name" "$tdir/debug/$bin_rel")
      ;;
    cold)
      cmd="$prefix run -q $target_args"
      # Fill sccache's cache (and check the build) before the timed runs.
      eval "$cmd" > /dev/null 2>&1
      args+=(--command-name "$name" --prepare "rm -rf $tdir" "$cmd")
      ;;
    test)
      case "$name" in stable-default|dev-opt1|common-all) ;; *) continue ;; esac
      eval "$prefix test -q" > /dev/null 2>&1
      eval "$prefix nextest run" > /dev/null 2>&1
      args+=(--command-name "$name-cargo-test" --prepare "$edit" "$prefix test -q")
      args+=(--command-name "$name-nextest" --prepare "$edit" "$prefix nextest run --hide-progress-bar --status-level none --final-status-level none")
      ;;
  esac
done

"$sccache" --zero-stats > /dev/null 2>&1 || true
hyperfine --shell=default --style basic --runs "$runs" \
  --export-json "$here/results/common-$workload-$mode.json" "${args[@]}"
"$sccache" --show-stats 2> /dev/null | grep -E 'Compile requests|Cache hits|Cache misses|Non-cacheable' || true
