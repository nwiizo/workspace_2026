#!/usr/bin/env bash
# `cargo build --release` from a clean target dir (what CI and deploys run) on
# macOS: stable default, sccache with a warm cache, nightly -Zthreads=8, and
# nightly rust-lld. RUNS=3 each, for both workloads.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
sccache="$here/results/tools/sccache"
export SCCACHE_DIR="$here/results/sccache-cache"
lld="$(rustc +nightly --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"
runs=${RUNS:-3}

for wl in grokking cli-deps; do
  work="$here/results/work/release-$wl"
  targets="$here/results/targets-release-$wl"
  rm -rf "$work"
  mkdir -p "$work" "$targets"
  if [ "$wl" = grokking ]; then
    rsync -a --exclude target "$here/../rust-grokking-ai-algorithms/" "$work/"
    printf '\n[workspace]\n' >> "$work/Cargo.toml"
    targs="--example all_chapters"
  else
    rsync -a --exclude target "$here/cli-deps/" "$work/"
    targs=""
  fi
  cd "$work"
  configs=(
    "stable-default|cargo +stable"
    "sccache|RUSTC_WRAPPER=$sccache cargo +stable"
    "nightly-threads8|RUSTFLAGS=-Zthreads=8 cargo +nightly"
    "nightly-lld|RUSTFLAGS=-Clink-arg=--ld-path=$lld cargo +nightly"
  )
  args=()
  for entry in "${configs[@]}"; do
    name=${entry%%|*}
    tdir="$targets/$name"
    cmd="CARGO_TARGET_DIR=$tdir ${entry#*|} build -q --release $targs"
    eval "$cmd" > /dev/null 2>&1 # fills sccache's cache
    args+=(--command-name "$name" --prepare "rm -rf $tdir" "$cmd")
  done
  hyperfine --shell=default --style basic --runs "$runs" \
    --export-json "$here/results/release-cold-$wl.json" "${args[@]}"
done
