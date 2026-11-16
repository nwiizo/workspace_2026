#!/usr/bin/env bash
# cli-deps (clap derive + regex + serde, 33 packages in Cargo.lock):
# stable default, the Cargo book's settings all together, --release, and Relax.
#
#   MODE=cold scripts/deps-compare.sh  # clean -> run, RUNS=3
#   MODE=edit scripts/deps-compare.sh  # edit summarize() -> run, RUNS=10
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
relax="$here/vendor/relax/target/release/relax"
work="$here/results/work/cli-deps"
targets="$here/results/targets-deps"
mode=${MODE:-edit}
if [ "$mode" = cold ]; then runs=${RUNS:-3}; else runs=${RUNS:-10}; fi
lld="$(rustc +nightly --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"

rm -rf "$work"
mkdir -p "$work" "$targets"
rsync -a --exclude target "$here/cli-deps/" "$work/"
grep -q 'lines.len() + 0 \* 1,' "$work/src/main.rs"
edit="sed -i '' -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"

official="RUSTFLAGS='-Zthreads=8 -Clink-arg=--ld-path=$lld' cargo +nightly -Zcodegen-backend -Zfeature-unification"
official+=" --config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
official+=" --config 'profile.dev.codegen-backend=\"cranelift\"' --config 'resolver.feature-unification=\"workspace\"'"

configs=(
  "stable-default|$targets/s0|cargo +stable run -q"
  "official-all|$targets/all|$official run -q"
  "stable-release|$targets/rel|cargo +stable run -q --release"
)

cd "$work"
args=()
for entry in "${configs[@]}"; do
  IFS='|' read -r name tdir cmd <<< "$entry"
  cmd="CARGO_TARGET_DIR=$tdir $cmd"
  if [ "$mode" = cold ]; then
    args+=(--command-name "$name" --prepare "rm -rf $tdir" "$cmd")
  else
    eval "$cmd" > "$here/results/deps-$name.out" 2> /dev/null
    args+=(--command-name "$name" --prepare "$edit" "$cmd")
  fi
done
if [ "$mode" = cold ]; then
  args+=(--command-name relax --prepare "$relax clean" "$relax run --quiet cli-deps")
else
  "$relax" run --quiet cli-deps 2> /dev/null | grep -v '^relax' > "$here/results/deps-relax.out"
  args+=(--command-name relax --prepare "$edit" "$relax run --quiet cli-deps")
fi

hyperfine --shell=default --style basic --runs "$runs" --export-json "$here/results/deps-$mode.json" "${args[@]}"
