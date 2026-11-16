#!/usr/bin/env bash
# On macOS, does the first launch of a freshly linked binary take longer than
# later launches of the same file? Each timed run of "first" follows an edit
# and a rebuild; "repeat" runs one unchanged binary after a warmup.
set -o pipefail

BASE=${BASE:-$HOME/relax-bench/mac}
export RUSTUP_HOME=$BASE/rustup CARGO_HOME=$BASE/cargo
export PATH=$BASE/cargo/bin:$BASE/bin:/usr/bin:/bin:/usr/sbin:/sbin
out=$BASE/out/firstrun
mkdir -p "$out"
cd "$BASE/work/grokking" || exit 1
T=$BASE/t/firstrun
bin=$T/debug/examples/all_chapters
build="CARGO_TARGET_DIR=$T cargo +1.100.0 build -q --example all_chapters"
edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
eval "$build"

hyperfine --shell=bash --style basic --runs 10 --export-json "$out/first.json" \
  -n first-launch-after-relink --prepare "$edit && $build" "$bin" > "$out/first.log" 2>&1
hyperfine --shell=bash --style basic --warmup 3 --runs 10 --export-json "$out/repeat.json" \
  -n repeat-launch "$bin" > "$out/repeat.log" 2>&1
# The same check for a binary copied to a new path (same bytes, new file).
cp "$bin" "$T/copy-check"
hyperfine --shell=bash --style basic --runs 10 --export-json "$out/copy.json" \
  -n first-launch-of-copy --prepare "rm -f $T/copy-check && cp $bin $T/copy-check" "$T/copy-check" > "$out/copy.log" 2>&1
echo done
