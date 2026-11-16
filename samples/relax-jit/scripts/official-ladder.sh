#!/usr/bin/env bash
# Edit one library function of ../rust-grokking-ai-algorithms, then run its
# all_chapters example: under the build-performance settings of the Cargo book
# (https://doc.rust-lang.org/cargo/guide/build-performance.html), one step at a
# time, and under Relax.
#
# The original sample is not touched: a work copy goes to results/work/grokking.
# Each configuration has its own CARGO_TARGET_DIR.
#
#   scripts/official-ladder.sh             # edit -> run, RUNS=10 each
#   MODE=build scripts/official-ladder.sh   # edit -> build only
#   MODE=runonly scripts/official-ladder.sh # run the built example only (Relax: warm run)
#   MODE=cold scripts/official-ladder.sh    # clean -> run, RUNS=3 each
#   MODE=test scripts/official-ladder.sh    # edit -> test
#   MODE=check scripts/official-ladder.sh   # edit -> check
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
relax="$here/vendor/relax/target/release/relax"
work="$here/results/work/grokking"
targets="$here/results/targets"
mode=${MODE:-edit}
if [ "$mode" = cold ]; then runs=${RUNS:-3}; else runs=${RUNS:-10}; fi
lld="$(rustc +nightly --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"

rm -rf "$work"
mkdir -p "$work" "$targets"
rsync -a --exclude target "$here/../rust-grokking-ai-algorithms/" "$work/"
printf '\n[workspace]\n' >> "$work/Cargo.toml"
# A body change that keeps the result: (0, ..) becomes (0 * <n>, ..).
sed -i '' -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' "$work/src/search.rs"
grep -q '(0 \* 1, values.len())' "$work/src/search.rs"
edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"

debuginfo="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
cranelift="-Zcodegen-backend --config 'profile.dev.codegen-backend=\"cranelift\"'"
unification="-Zfeature-unification --config 'resolver.feature-unification=\"workspace\"'"

# name|command prefix (env + cargo invocation up to the subcommand's flags)
configs=(
  "stable-default|CARGO_TARGET_DIR=$targets/s0 cargo +stable"
  "stable-debuginfo|CARGO_TARGET_DIR=$targets/s1 cargo +stable $debuginfo"
  "nightly-default|CARGO_TARGET_DIR=$targets/n0 cargo +nightly"
  "nightly-debuginfo|CARGO_TARGET_DIR=$targets/n1 cargo +nightly $debuginfo"
  "nightly-debuginfo-threads8|CARGO_TARGET_DIR=$targets/n2 RUSTFLAGS=-Zthreads=8 cargo +nightly $debuginfo"
  "nightly-debuginfo-cranelift|CARGO_TARGET_DIR=$targets/n3 cargo +nightly $debuginfo $cranelift"
  "nightly-debuginfo-threads8-cranelift|CARGO_TARGET_DIR=$targets/n4 RUSTFLAGS=-Zthreads=8 cargo +nightly $debuginfo $cranelift"
  "official-all|CARGO_TARGET_DIR=$targets/n5 RUSTFLAGS='-Zthreads=8 -Clink-arg=--ld-path=$lld' cargo +nightly $debuginfo $cranelift $unification"
  "stable-release|CARGO_TARGET_DIR=$targets/rel cargo +stable"
)

cd "$work"
args=()
for entry in "${configs[@]}"; do
  name=${entry%%|*}
  prefix=${entry#*|}
  case "$mode" in
    edit)
      if [ "$name" = stable-release ]; then cmd="$prefix run -q --release --example all_chapters"; else cmd="$prefix run -q --example all_chapters"; fi
      eval "$cmd" > /dev/null
      args+=(--command-name "$name" --prepare "$edit" "$cmd")
      ;;
    cold)
      if [ "$name" = stable-release ]; then cmd="$prefix run -q --release --example all_chapters"; else cmd="$prefix run -q --example all_chapters"; fi
      tdir=$(printf '%s' "$prefix" | sed -E 's/^CARGO_TARGET_DIR=([^ ]+).*/\1/')
      args+=(--command-name "$name" --prepare "rm -rf $tdir" "$cmd")
      ;;
    build)
      if [ "$name" = stable-release ]; then cmd="$prefix build -q --release --example all_chapters"; else cmd="$prefix build -q --example all_chapters"; fi
      eval "$cmd" > /dev/null 2>&1
      args+=(--command-name "$name" --prepare "$edit" "$cmd")
      ;;
    runonly)
      tdir=$(printf '%s' "$prefix" | sed -E 's/^CARGO_TARGET_DIR=([^ ]+).*/\1/')
      if [ "$name" = stable-release ]; then
        eval "$prefix build -q --release --example all_chapters" > /dev/null 2>&1
        bin="$tdir/release/examples/all_chapters"
      else
        eval "$prefix build -q --example all_chapters" > /dev/null 2>&1
        bin="$tdir/debug/examples/all_chapters"
      fi
      "$bin" > "$here/results/runonly-$name.out"
      args+=(--command-name "$name" "$bin")
      ;;
    test)
      [ "$name" = stable-release ] && continue
      cmd="$prefix test -q"
      eval "$cmd" > /dev/null 2>&1
      args+=(--command-name "$name" --prepare "$edit" "$cmd")
      ;;
    check)
      case "$name" in stable-default|nightly-debuginfo-threads8) ;; *) continue ;; esac
      cmd="$prefix check -q --all-targets"
      eval "$cmd" > /dev/null 2>&1
      args+=(--command-name "$name-check" --prepare "$edit" "$cmd")
      ;;
  esac
done

case "$mode" in
  edit)
    "$relax" run --quiet rust-grokking-ai-algorithms --example all_chapters > /dev/null 2>&1
    args+=(--command-name relax --prepare "$edit" "$relax run --quiet rust-grokking-ai-algorithms --example all_chapters")
    ;;
  cold)
    args+=(--command-name relax --prepare "$relax clean" "$relax run --quiet rust-grokking-ai-algorithms --example all_chapters")
    ;;
  build) ;;
  runonly)
    # No edit: the run starts from the warm cache. This is Relax's floor, not a build-free run.
    "$relax" run --quiet rust-grokking-ai-algorithms --example all_chapters 2> /dev/null | grep -v '^relax' > "$here/results/runonly-relax.out"
    args+=(--command-name relax-warm "$relax run --quiet rust-grokking-ai-algorithms --example all_chapters")
    ;;
  test)
    "$relax" test rust-grokking-ai-algorithms > /dev/null 2>&1 || true
    args+=(--command-name relax --prepare "$edit" "$relax test rust-grokking-ai-algorithms")
    ;;
  check)
    args+=(--command-name relax-check --prepare "$edit" "$relax check rust-grokking-ai-algorithms")
    ;;
esac

# relax test exits 1 here (2 of 57 tests fail under Relax), so failures are ignored.
hyperfine -i --shell=default --style basic --runs "$runs" --export-json "$here/results/ladder-$mode.json" \
  --export-markdown "$here/results/ladder-$mode.md" "${args[@]}"
