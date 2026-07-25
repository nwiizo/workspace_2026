#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
diagnostic_since=${1:-${DIAGNOSTIC_SINCE:-}}

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi
if [ -z "$diagnostic_since" ]; then
  echo "診断run開始時刻を第1引数または DIAGNOSTIC_SINCE で指定してください。" >&2
  exit 2
fi

"$script_dir/flush-diagnostics.sh"

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-app-rides-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-app-rides-json.XXXXXX")

cleanup() {
  rm -f "$raw_log" "$json_log"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

ISUCON_DIAGNOSTIC=1 "$compose" logs \
  --no-color \
  --timestamps \
  --since "$diagnostic_since" \
  webapp >"$raw_log"
sed -n 's/^.*APP_RIDES_DIAGNOSTIC //p' "$raw_log" |
  jq --compact-output . >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のapp rides診断sampleがありません: $diagnostic_since" >&2
  exit 1
fi

first_sample_timestamp=$(
  sed -n '/APP_RIDES_DIAGNOSTIC/ {
    s/^[^|]*| \([^ ]*\) .*/\1/p
    q
  }' "$raw_log"
)
first_sample_seconds=$(printf '%s\n' "$first_sample_timestamp" | sed 's/\.[0-9][0-9]*Z$/Z/')
if ! run_started_epoch=$(printf '%s\n' "$diagnostic_since" | jq -Rer 'fromdateiso8601'); then
  echo "診断run開始時刻はUTCのISO 8601秒形式で指定してください: $diagnostic_since" >&2
  exit 2
fi
if ! first_sample_epoch=$(printf '%s\n' "$first_sample_seconds" | jq -Rer 'fromdateiso8601'); then
  echo "最初のapp rides sample時刻を解釈できません: $first_sample_timestamp" >&2
  exit 1
fi

database_scope=$(
  ISUCON_DIAGNOSTIC=1 "$compose" exec -T \
    -e MYSQL_PWD=isucon \
    db \
    mysql \
    --batch \
    --skip-column-names \
    -uroot \
    performance_schema \
    -e "
SELECT
  UNIX_TIMESTAMP(NOW()) - CAST(VARIABLE_VALUE AS UNSIGNED),
  DATE_FORMAT(
    FROM_UNIXTIME(UNIX_TIMESTAMP(NOW()) - CAST(VARIABLE_VALUE AS UNSIGNED)),
    '%Y-%m-%dT%H:%i:%sZ'
  )
FROM global_status
WHERE VARIABLE_NAME = 'Uptime'
"
)
server_started_epoch=$(printf '%s\n' "$database_scope" | awk 'NR == 1 { print $1 }')
server_started_at=$(printf '%s\n' "$database_scope" | awk 'NR == 1 { print $2 }')

case "$server_started_epoch" in
  "" | *[!0-9]*)
    echo "MySQLの起動時刻を取得できませんでした。" >&2
    exit 1
    ;;
esac

if [ "$server_started_epoch" -lt "$run_started_epoch" ] ||
  [ "$server_started_epoch" -gt "$first_sample_epoch" ]; then
  echo "MySQLが診断run用に再起動されたことを確認できません。" >&2
  echo "run開始=$diagnostic_since MySQL起動=$server_started_at 最初のsample=$first_sample_timestamp" >&2
  exit 1
fi

printf 'diagnostic scope\n\n'
printf '| boundary | UTC |\n'
printf '|---|---|\n'
printf '| requested run start | %s |\n' "$diagnostic_since"
printf '| MySQL process start | %s |\n' "$server_started_at"
printf '| first app rides sample | %s |\n' "$first_sample_timestamp"

printf '\nsample coverage\n\n'
jq --slurp --raw-output '
  (map(.sequence) | max) as $max_sequence |
  "samples=\(length) last_sampled_sequence=\($max_sequence) " +
  "inferred_request_count_range=\($max_sequence + 1)-\($max_sequence + 64)"
' "$json_log"

printf '\noutcome and terminal phase\n\n'
printf '| path | outcome | terminal phase | samples |\n'
printf '|---|---|---|---:|\n'
jq --slurp --raw-output '
  sort_by([.path, .outcome, .terminal_phase]) |
  group_by([.path, .outcome, .terminal_phase])[] |
  "| \(.[0].path) | \(.[0].outcome) | \(.[0].terminal_phase) | \(length) |"
' "$json_log"

printf '\ncache-hit share among successful samples\n\n'
printf '| successful samples | cache hits | DB misses | cache-hit share |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.outcome == "success") ] as $samples |
  ($samples | map(select(.path == "cache_hit")) | length) as $hits |
  ($samples | map(select(.path == "db_miss")) | length) as $misses |
  "| \($samples | length) | \($hits) | \($misses) | " +
  "\((1000 * $hits / ($samples | length) | floor) / 10)% |"
' "$json_log"

printf '\nsuccessful handler phase latency\n\n'
printf 'Percentiles use the zero-based lower order statistic floor((n - 1) * p).\n'
printf 'sql_decode_us includes MySQL execution, network transfer, and SQLx row decoding.\n'
printf 'connection_owned_us overlaps sql_decode_us and is not part of the residual sum.\n\n'
printf '| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
for phase in \
  cache_lookup_us \
  admission_us \
  pool_acquire_us \
  sql_decode_us \
  connection_owned_us \
  mapping_us \
  response_us \
  cache_insert_us \
  residual_us \
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
      "| \($phase) | \($length) | " +
      "\(($values | add) / $length | floor) | " +
      "\($values[(($length - 1) * 0.50 | floor)]) | " +
      "\($values[(($length - 1) * 0.95 | floor)]) | " +
      "\($values[(($length - 1) * 0.99 | floor)]) | " +
      "\($values[$length - 1]) |"
    end
  ' "$json_log"
done

printf '\nsummed component share among successful samples\n\n'
printf '| component | summed_us | share of summed total |\n'
printf '|---|---:|---:|\n'
for phase in \
  cache_lookup_us \
  admission_us \
  pool_acquire_us \
  sql_decode_us \
  mapping_us \
  response_us \
  cache_insert_us \
  residual_us
do
  jq --slurp --raw-output --arg phase "$phase" '
    [ .[] | select(.outcome == "success") ] as $samples |
    ($samples | map(.total_us) | add) as $total |
    ($samples | map(.[$phase] // 0) | add) as $component |
    "| \($phase) | \($component) | " +
    "\((1000 * $component / $total | floor) / 10)% |"
  ' "$json_log"
done

printf '\nlatency by returned ride count\n\n'
printf '| rows | samples | sql avg_us | mapping avg_us | response avg_us | total avg_us |\n'
printf '|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.outcome == "success" and .row_count != null) ] |
  sort_by(.row_count) |
  group_by(.row_count)[] |
  "| \(.[0].row_count) | \(length) | " +
  "\((map(.sql_decode_us) | add) / length | floor) | " +
  "\((map(.mapping_us) | add) / length | floor) | " +
  "\((map(.response_us) | add) / length | floor) | " +
  "\((map(.total_us) | add) / length | floor) |"
' "$json_log"

printf '\npool state before sampled acquire\n\n'
printf '| samples | no idle | observed max size | max size and no idle |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.pool_size_before != null) ] as $samples |
  ($samples | map(.pool_size_before) | max) as $max_size |
  "| \($samples | length) | " +
  "\($samples | map(select(.pool_idle_before == 0)) | length) | " +
  "\($max_size) | " +
  "\($samples | map(select(.pool_size_before == $max_size and .pool_idle_before == 0)) | length) |"
' "$json_log"
