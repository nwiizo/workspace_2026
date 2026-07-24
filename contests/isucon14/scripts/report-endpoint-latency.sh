#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
diagnostic_since=${1:-${DIAGNOSTIC_SINCE:-}}

if ! command -v alp >/dev/null 2>&1; then
  echo "alp が見つかりません。README.local.md の診断ツール手順を確認してください。" >&2
  exit 1
fi

if [ -z "$diagnostic_since" ]; then
  echo "診断run開始時刻を第1引数または DIAGNOSTIC_SINCE で指定してください。" >&2
  echo "例: ./scripts/report-endpoint-latency.sh 2026-07-25T00:00:00Z" >&2
  exit 2
fi

matching_groups='/api/app/rides/[^/]+/evaluation,/api/chair/rides/[^/]+/status'
raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-nginx-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-nginx-json.XXXXXX")

cleanup() {
  rm -f "$raw_log" "$json_log"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

ISUCON_DIAGNOSTIC=1 "$compose" logs \
  --no-color \
  --since "$diagnostic_since" \
  nginx >"$raw_log"
sed -n 's/^[^{]*//p' "$raw_log" >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のnginx診断JSON logがありません: $diagnostic_since" >&2
  exit 1
fi

alp json \
  --matching-groups "$matching_groups" \
  --percentiles 50,95,99 \
  --sort sum \
  --reverse \
  --output count,method,uri,min,avg,p50,p95,p99,max,sum,2xx,4xx,5xx \
  --format markdown <"$json_log"

printf '\nHTTP 499（clientがresponse完了前に切断）\n\n'
alp json \
  --file "$json_log" \
  --filters "Status == 499" \
  --matching-groups "$matching_groups" \
  --sort count \
  --reverse \
  --output count,method,uri \
  --format markdown
