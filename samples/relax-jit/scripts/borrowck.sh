#!/usr/bin/env bash
# borrowck-bg: rustc rejects `bad` (E0502). What does a Relax run do before and
# after its background borrow checker reports it?
set -uo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
relax="$here/vendor/relax/target/release/relax"
out="$here/results/borrowck"
mkdir -p "$out"
cd "$here/borrowck-bg"

run() { # label, command...
  local label=$1
  shift
  "$@" > "$out/$label.out" 2> "$out/$label.err"
  echo "== $label (exit $?)"
  grep -v '^relax-remote' "$out/$label.out"
  grep -E 'error|E0[0-9]{3}|borrow' "$out/$label.err" | head -5
}

run cargo-run cargo +stable run -q
run relax-no-call "$relax" run --quiet borrowck-bg
run relax-call-now env CALL_BAD=1 "$relax" run --quiet borrowck-bg
run relax-call-after-3s env CALL_BAD=1 WAIT_MS=3000 "$relax" run --quiet borrowck-bg
run relax-unattended-call-after-3s env CALL_BAD=1 WAIT_MS=3000 "$relax" run --quiet --unattended borrowck-bg
run relax-check-first env CALL_BAD=1 "$relax" run --quiet --check-first borrowck-bg
run relax-check "$relax" check borrowck-bg
