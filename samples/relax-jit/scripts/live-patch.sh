#!/usr/bin/env bash
# Save edits into live-counter/src/main.rs while `relax run live-counter` runs,
# and record the save times. Phases:
#   1. ROUNDS body edits of step(): `+= 1` <-> `+= 100`
#   2. a save that does not type-check, then its fix
#   3. a field added to State (a layout change), then `session relaunch`
# The source is restored on exit. Analyse with scripts/live-patch-report.sh.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
relax="$here/vendor/relax/target/release/relax"
pkg="$here/live-counter"
src="$pkg/src/main.rs"
out="$here/results/live"
rounds=${ROUNDS:-5}
mkdir -p "$out"
cp "$src" "$out/main.rs.orig"
: > "$out/saves.tsv"

pid=""
cleanup() {
  cp "$out/main.rs.orig" "$src"
  [ -n "$pid" ] && kill "$pid" 2> /dev/null || true
}
trap cleanup EXIT

mark() { # phase label -> saves.tsv: <mtime ms> <label>
  local m
  m=$(stat -f %Fm "$src")
  printf '%s\t%s\n' "$(echo "$m * 1000 / 1" | bc)" "$1" >> "$out/saves.tsv"
}

cd "$pkg"
"$relax" run live-counter > "$out/stdout.log" 2> "$out/stderr.log" &
pid=$!
until grep -q 'iter=' "$out/stdout.log" 2> /dev/null; do sleep 0.1; done
sleep 3

for _ in $(seq 1 "$rounds"); do
  if grep -q 'state.counter += 1;' "$src"; then
    sed -i '' 's/state.counter += 1;/state.counter += 100;/' "$src"
    mark "step+=100"
  else
    sed -i '' 's/state.counter += 100;/state.counter += 1;/' "$src"
    mark "step+=1"
  fi
  sleep 6
done

sed -i '' 's/state.counter += \([0-9]*\);/state.counter += "x";/' "$src"
mark "type-error"
sleep 8
sed -i '' 's/state.counter += "x";/state.counter += 1000;/' "$src"
mark "type-error-fixed:+=1000"
sleep 8

sed -i '' 's/    pub message: String,/    pub message: String,\
    pub extra: f64,/' "$src"
sed -i '' 's/            message: String::from("start"),/            message: String::from("start"),\
            extra: 0.0,/' "$src"
sed -i '' 's/    state.message = format!("counter = {}", state.counter);/    state.extra += 0.5;\
    state.message = format!("counter = {} extra = {}", state.counter, state.extra);/' "$src"
mark "layout-change"
sleep 10
"$relax" inspect "$pid" session relaunch > "$out/relaunch.log" 2>&1 || true
printf '%s\t%s\n' "$(echo "$(date +%s) * 1000" | bc)" "relaunch-requested" >> "$out/saves.tsv"
sleep 12
