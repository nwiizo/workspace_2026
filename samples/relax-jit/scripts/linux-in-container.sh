#!/usr/bin/env bash
# Runs inside rust:1.100-bookworm (aarch64) started by scripts/linux-linkers.sh.
# aarch64 Linux links with the system linker (cc -> GNU ld.bfd) by default; the
# Cargo book names this case as the one where an alternative linker helps.
# Sources are copied into the container so builds do not run on the bind mount.
set -euo pipefail

out=/out
mkdir -p "$out"
apt-get update -qq > /dev/null
apt-get install -y -qq mold lld clang hyperfine > /dev/null
mkdir -p /opt/wild
tar -xzf /dl/wild-linker-0.11.0-aarch64-unknown-linux-gnu.tar.gz -C /opt/wild --strip-components=1
{
  rustc -V; cargo -V; ld.bfd --version | head -1; ld.lld --version; mold --version
  /opt/wild/wild --version; hyperfine --version; nproc; uname -srm
} > "$out/versions.txt" 2>&1

debuginfo="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
common="$debuginfo --config profile.dev.opt-level=1 --config 'profile.dev.package.\"*\".opt-level=3' --config profile.dev.build-override.opt-level=3"
configs=(
  "default||"
  "lld|-Clink-arg=-fuse-ld=lld|"
  "mold|-Clink-arg=-fuse-ld=mold|"
  "wild|-Clinker=clang -Clink-arg=--ld-path=/opt/wild/wild|"
  "mold-common|-Clink-arg=-fuse-ld=mold|$common"
)

run_workload() { # name, source dir, cargo target args, sed edit expression, file to edit
  local wl=$1 src=$2 targs=$3 expr=$4 file=$5
  local work=/tmp/work-$wl
  rm -rf "$work"
  cp -a "$src" "$work"
  rm -rf "$work/target"
  grep -q '^\[workspace\]' "$work/Cargo.toml" || printf '\n[workspace]\n' >> "$work/Cargo.toml"
  cd "$work"
  if [ "$wl" = grokking ]; then
    sed -i -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' src/search.rs
  fi
  local edit="sed -i -E '$expr' $file"
  for mode in edit cold release; do
    local args=() runs=10
    [ "$mode" != edit ] && runs=3
    for entry in "${configs[@]}"; do
      IFS='|' read -r name flags extra <<< "$entry"
      if [ "$mode" = release ]; then
        # --release clean build (what CI and deploys run); the dev-only extras do not apply.
        [ "$name" = mold-common ] && continue
        local tdir="/tmp/r-$wl-$name"
        local cmd="CARGO_TARGET_DIR=$tdir RUSTFLAGS='$flags' cargo build -q --release $targs"
        args+=(--command-name "$name" --prepare "rm -rf $tdir" "$cmd")
        continue
      fi
      local cmd="CARGO_TARGET_DIR=/tmp/t-$wl-$name RUSTFLAGS='$flags' cargo $extra run -q $targs"
      eval "$cmd" > "$out/$wl-$name.out" 2> "$out/$wl-$name.err"
      if [ "$mode" = edit ]; then
        args+=(--command-name "$name" --prepare "$edit" "$cmd")
      else
        args+=(--command-name "$name" --prepare "rm -rf /tmp/t-$wl-$name" "$cmd")
      fi
    done
    hyperfine --shell=bash --style basic --runs "$runs" --export-json "$out/$wl-$mode.json" "${args[@]}" > "$out/$wl-$mode.log" 2>&1
  done
}

run_workload grokking /src/grokking "--example all_chapters" \
  's/\(0 \* [0-9]+, values.len\(\)\)/(0 * '"'"'$RANDOM'"'"', values.len())/' src/search.rs
run_workload cli-deps /src/relax-jit/cli-deps "" \
  's/lines.len\(\) \+ 0 \* [0-9]+,/lines.len() + 0 * '"'"'$RANDOM'"'"',/' src/main.rs
