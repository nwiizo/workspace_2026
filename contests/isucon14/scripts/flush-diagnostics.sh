#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi

response=$(
  ISUCON_DIAGNOSTIC=1 "$compose" exec -T nginx \
    curl \
    --fail \
    --silent \
    --show-error \
    --request POST \
    http://webapp:8080/api/internal/diagnostics/flush
)
dropped_lines=$(printf '%s\n' "$response" | jq --exit-status '.dropped_lines')

if [ "$dropped_lines" -ne 0 ]; then
  echo "診断queueで$dropped_lines行を欠落したため、集計を中止します。" >&2
  exit 1
fi

printf 'diagnostic writer flush: dropped_lines=0\n' >&2
