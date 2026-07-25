#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
diagnostic_since=${1:-${DIAGNOSTIC_SINCE:-}}
diagnostic_until=${2:-${DIAGNOSTIC_UNTIL:-}}

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi

if [ -z "$diagnostic_since" ]; then
  echo "診断run開始時刻を第1引数または DIAGNOSTIC_SINCE で指定してください。" >&2
  exit 2
fi
if [ -z "$diagnostic_until" ]; then
  echo "診断run終了時刻を第2引数または DIAGNOSTIC_UNTIL で指定してください。" >&2
  exit 2
fi

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-matcher-raw.XXXXXX")
nginx_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-matcher-nginx.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-matcher-json.XXXXXX")

cleanup() {
  rm -f "$raw_log" "$nginx_log" "$json_log"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

ISUCON_DIAGNOSTIC=1 "$compose" logs \
  --no-color \
  --timestamps \
  --since "$diagnostic_since" \
  --until "$diagnostic_until" \
  webapp >"$raw_log"
ISUCON_DIAGNOSTIC=1 "$compose" logs \
  --no-color \
  --timestamps \
  --since "$diagnostic_since" \
  --until "$diagnostic_until" \
  nginx >"$nginx_log"

initialize_boundary=$(
  awk '
    index($0, "\"method\":\"POST\"") &&
    index($0, "\"uri\":\"/api/initialize\"") &&
    index($0, "\"status\":200") {
      boundary = $3
    }
    END {
      print boundary
    }
  ' "$nginx_log"
)
if [ -z "$initialize_boundary" ]; then
  echo "指定時刻以降に成功したPOST /api/initializeがありません: $diagnostic_since" >&2
  exit 1
fi

awk -v initialize_boundary="$initialize_boundary" '
  $3 >= initialize_boundary && index($0, "MATCHER_DIAGNOSTIC ") {
    sub(/^.*MATCHER_DIAGNOSTIC /, "")
    print
  }
' "$raw_log" >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "initialize完了後のmatcher診断sampleがありません: $initialize_boundary" >&2
  exit 1
fi

printf 'initialize boundary: %s\n\n' "$initialize_boundary"
printf 'diagnostic end boundary: %s\n\n' "$diagnostic_until"
printf 'matcher outcome and terminal phase\n\n'
printf '| outcome | terminal phase | samples |\n'
printf '|---|---|---:|\n'
jq --slurp --raw-output '
  group_by([.outcome, .terminal_phase])[] |
  "| \(.[0].outcome) | \(.[0].terminal_phase) | \(length) |"
' "$json_log"

printf '\nmatcher phase latency\n\n'
printf 'Percentiles use the zero-based lower order statistic floor((n - 1) * p).\n\n'
printf '| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
for phase in \
  pool_begin_us \
  pending_query_us \
  available_query_us \
  matching_update_us \
  commit_us \
  cache_invalidation_us \
  total_us
do
  jq --slurp --raw-output --arg phase "$phase" '
    [
      .[] |
      select(.outcome == "success" and .[$phase] != null) |
      .[$phase]
    ] | sort as $values |
    ($values | length) as $length |
    if $length == 0 then
      "| \($phase) | 0 | n/a | n/a | n/a | n/a | n/a |"
    else
      [
        $phase,
        $length,
        (($values | add) / $length | floor),
        $values[(($length - 1) * 0.50 | floor)],
        $values[(($length - 1) * 0.95 | floor)],
        $values[(($length - 1) * 0.99 | floor)],
        $values[$length - 1]
      ] |
      "| \(.[0]) | \(.[1]) | \(.[2]) | \(.[3]) | \(.[4]) | \(.[5]) | \(.[6]) |"
    end
  ' "$json_log"
done

printf '\nmatcher capacity paths\n\n'
jq --slurp --raw-output '
  [.[] | select(.outcome == "success")] as $samples |
  "calls=\($samples | length) " +
  "pending_empty=\($samples | map(select(.pending_selected == 0)) | length) " +
  "pending_batch_full=\($samples | map(select(.pending_batch_full)) | length) " +
  "available_zero_with_pending=\($samples | map(select(
    .pending_selected > 0 and .available_selected == 0
  )) | length) " +
  "update_conflicts=\($samples | map(
    .matching_attempted - .matched
  ) | add // 0) " +
  "matched_zero_with_pending=\($samples | map(select(
    .pending_selected > 0 and .matched == 0
  )) | length) " +
  "pending_exceeds_available=\($samples | map(select(
    .pending_selected > .available_selected
  )) | length)"
' "$json_log"

printf '\nmatcher region candidates\n\n'
printf '| region index | pending selected | available selected |\n'
printf '|---:|---:|---:|\n'
jq --slurp --raw-output '
  [0, 1][] as $region |
  "| \($region) | " +
  "\(map(.pending_selected_by_region[$region]) | add // 0) | " +
  "\(map(.available_selected_by_region[$region]) | add // 0) |"
' "$json_log"

printf '\nmatched pickup distance\n\n'
printf 'The benchmark regions are separated enough that a Manhattan distance over 200 cannot be intra-region.\n\n'
jq --slurp --raw-output '
  [.[] | select(.outcome == "success" and .matched > 0)] as $samples |
  ($samples | map(.matched) | add // 0) as $matched |
  ($samples | map(.matched_distance_sum) | add // 0) as $distance_sum |
  "matched=\($matched) " +
  "avg_distance=\(if $matched == 0 then "n/a" else ($distance_sum / $matched | floor) end) " +
  "max_distance=\($samples | map(.matched_distance_max) | max // "n/a") " +
  "distance_gt_200=\($samples | map(.matched_distance_gt_200) | add // 0)"
' "$json_log"

printf '\n20 samples with the most cross-region-distance matches\n\n'
printf '| sequence | matched | distance sum | max distance | distance > 200 | oldest age ms |\n'
printf '|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.outcome == "success" and .matched_distance_gt_200 > 0)
  ] |
  sort_by([.matched_distance_gt_200, .matched_distance_max]) |
  reverse |
  .[:20][] |
  "| \(.sequence) | \(.matched) | \(.matched_distance_sum) | " +
  "\(.matched_distance_max) | \(.matched_distance_gt_200) | " +
  "\(.oldest_pending_age_ms // "n/a") |"
' "$json_log"

printf '\npool state before matcher transaction\n\n'
printf '| pool size | idle | in use | samples |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  group_by([.pool_size_before, .pool_idle_before, .pool_in_use_before])[] |
  "| \(.[0].pool_size_before) | \(.[0].pool_idle_before) | " +
  "\(.[0].pool_in_use_before) | \(length) |"
' "$json_log"

printf '\noldest pending observations\n\n'
jq --slurp --raw-output '
  [.[] | select(.oldest_pending_age_ms != null)] as $samples |
  if ($samples | length) == 0 then
    "samples=0 max_age_ms=n/a age_ge_30000ms=0"
  else
    "samples=\($samples | length) " +
    "max_age_ms=\($samples | map(.oldest_pending_age_ms) | max) " +
    "age_ge_30000ms=\($samples | map(select(.oldest_pending_age_ms >= 30000)) | length)"
  end
' "$json_log"

printf '\n20 samples with the oldest pending ride\n\n'
printf '| sequence | pending | available | matched | unmatched batch | oldest age ms | oldest ride |\n'
printf '|---:|---:|---:|---:|---:|---:|---|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.oldest_pending_age_ms != null)
  ] |
  sort_by(.oldest_pending_age_ms) |
  reverse |
  .[:20][] |
  "| \(.sequence) | \(.pending_selected) | \(.available_selected) | " +
  "\(.matched) | \(.unmatched_in_batch) | \(.oldest_pending_age_ms) | " +
  "\(.oldest_pending_id) |"
' "$json_log"
