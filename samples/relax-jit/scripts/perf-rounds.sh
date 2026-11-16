#!/usr/bin/env bash
# Run perf-rounds (8 rounds of sort + sieve) 3 times per toolchain setting and
# keep each round's time: stable dev, the Cargo book's settings all together
# (dev + Cranelift), --release, and Relax (default mode, tiers up while running).
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
relax="$here/vendor/relax/target/release/relax"
pkg="$here/perf-rounds"
targets="$here/results/targets-perf"
out="$here/results/perf"
lld="$(rustc +nightly --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"
mkdir -p "$out" "$targets"
cd "$pkg"

CARGO_TARGET_DIR="$targets/s0" cargo +stable build -q
CARGO_TARGET_DIR="$targets/rel" cargo +stable build -q --release
CARGO_TARGET_DIR="$targets/all" RUSTFLAGS="-Zthreads=8 -Clink-arg=--ld-path=$lld" \
  cargo +nightly -Zcodegen-backend -Zfeature-unification \
  --config 'profile.dev.debug="line-tables-only"' --config 'profile.dev.package."*".debug=false' \
  --config 'profile.dev.codegen-backend="cranelift"' --config 'resolver.feature-unification="workspace"' \
  build -q 2> /dev/null
"$relax" run --quiet perf-rounds > /dev/null 2>&1

for i in 1 2 3; do
  "$targets/s0/debug/perf-rounds" > "$out/stable-default-$i.txt"
  "$targets/all/debug/perf-rounds" > "$out/official-all-$i.txt"
  "$targets/rel/release/perf-rounds" > "$out/stable-release-$i.txt"
  /usr/bin/time -l "$relax" run --quiet perf-rounds > "$out/relax-$i.txt" 2> "$out/relax-$i.time"
done

for f in "$out"/*-1.txt; do
  name=$(basename "$f" -1.txt)
  printf '%-16s' "$name"
  for i in 1 2 3; do
    printf ' | %s' "$(grep -o '[0-9]* ms' "$out/$name-$i.txt" | tr -d ' ms' | paste -sd, -)"
  done
  printf ' | %s\n' "$(grep checksum "$out/$name-1.txt")"
done
