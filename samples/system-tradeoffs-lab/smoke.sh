#!/usr/bin/env bash
set -euo pipefail

output="$(cargo run --quiet)"

check() {
  local needle="$1"
  if ! grep -Fq "$needle" <<<"$output"; then
    echo "missing expected output: $needle" >&2
    exit 1
  fi
}

check "scenario 1: retry without idempotency"
check "charges: 2 / total_amount: 10000"
check "scenario 2: retry with idempotency"
check "charges: 1 / total_amount: 5000"
check "scenario 3: consistency vs latency"
check "primary stock right after purchase: 2"
check "replica stock before replication: 3"
check "replica stock after replication: 2"

echo "smoke check passed"
