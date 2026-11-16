#!/usr/bin/env bash
# Relax edit -> run with and without --check-first (the start-up this sample
# recommends), in one hyperfine run, on the macOS set-up of macos-all.sh.
set -o pipefail
BASE=${BASE:-$HOME/relax-bench/mac}
export RUSTUP_HOME=$BASE/rustup CARGO_HOME=$BASE/cargo
export PATH=$BASE/cargo/bin:$BASE/bin:/usr/bin:/bin:/usr/sbin:/sbin
out=$BASE/out/checkfirst
mkdir -p "$out"
cd "$BASE/work/grokking" || exit 1
R=$BASE/relax/target/release/relax
edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
"$R" run --quiet rust-grokking-ai-algorithms --example all_chapters > /dev/null 2>&1
"$R" run --quiet --check-first rust-grokking-ai-algorithms --example all_chapters > "$out/checkfirst.out" 2>&1
hyperfine --shell=bash --style basic --runs 10 --export-json "$out/edit.json" \
  -n relax --prepare "$edit" "$R run --quiet rust-grokking-ai-algorithms --example all_chapters" \
  -n relax-check-first --prepare "$edit" "$R run --quiet --check-first rust-grokking-ai-algorithms --example all_chapters" \
  > "$out/edit.log" 2>&1
echo "checkfirst exit=$?"
