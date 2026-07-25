#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
diagnostic_since=${1:-${DIAGNOSTIC_SINCE:-}}
mysql_status_file=${2:-${MYSQL_STATUS_OUTPUT_FILE:-}}

if ! command -v jq >/dev/null 2>&1; then
  echo "jq が見つかりません。" >&2
  exit 1
fi
if [ -z "$diagnostic_since" ]; then
  echo "診断run開始時刻を第1引数または DIAGNOSTIC_SINCE で指定してください。" >&2
  exit 2
fi
if [ -z "$mysql_status_file" ] || [ ! -f "$mysql_status_file" ]; then
  echo "MySQL status TSVを第2引数または MYSQL_STATUS_OUTPUT_FILE で指定してください。" >&2
  exit 2
fi

"$script_dir/flush-diagnostics.sh"

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-db-admission-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-db-admission-json.XXXXXX")

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
sed -n 's/^.*DB_ADMISSION_DIAGNOSTIC //p' "$raw_log" |
  jq --compact-output . >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のDB admission診断sampleがありません: $diagnostic_since" >&2
  exit 1
fi

printf 'database admission coverage\n\n'
jq --slurp --raw-output '
  (map(.sequence) | max) as $max_sequence |
  (map(select(.periodic_sample)) | length) as $periodic |
  (map(select(.wait_us >= 30000)) | length) as $over_30ms |
  "observed_sequence_upper_bound=\($max_sequence + 1) " +
  "periodic_samples=\($periodic) " +
  "forced_or_periodic_over_30ms=\($over_30ms)"
' "$json_log"

printf '\nperiodic admission wait distribution\n\n'
printf '| samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.periodic_sample) | .wait_us ] | sort as $values |
  ($values | length) as $length |
  if $length == 0 then
    error("周期admission sampleがありません")
  else
    [
      $length,
      (($values | add) / $length | floor),
      $values[(($length - 1) * 0.50 | floor)],
      $values[(($length - 1) * 0.95 | floor)],
      $values[(($length - 1) * 0.99 | floor)],
      $values[$length - 1]
    ] |
    "| \(.[0]) | \(.[1]) | \(.[2]) | \(.[3]) | \(.[4]) | \(.[5]) |"
  end
' "$json_log"

printf '\nperiodic admission wait by DB phase\n\n'
printf '| phase | samples | avg_us | p95_us | max_us |\n'
printf '|---|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.periodic_sample) ] |
  group_by(.label) |
  map(
    . as $samples |
    ([ $samples[].wait_us ] | sort) as $waits |
    {
      label: $samples[0].label,
      samples: ($samples | length),
      avg: (($waits | add) / ($waits | length) | floor),
      p95: $waits[((($waits | length) - 1) * 0.95 | floor)],
      max: ($waits | max)
    }
  ) |
  sort_by(-.p95, .label)[] |
  "| \(.label) | \(.samples) | \(.avg) | \(.p95) | \(.max) |"
' "$json_log"

printf '\nshared pool state before periodic admission\n\n'
printf '| pool size | idle | in use | samples |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [ .[] | select(.periodic_sample) ] |
  group_by([.pool_size_before, .pool_idle_before, .pool_in_use_before])[] |
  "| \(.[0].pool_size_before) | \(.[0].pool_idle_before) | \(.[0].pool_in_use_before) | \(length) |"
' "$json_log"

mysql_samples=$(awk 'END { print NR - 1 }' "$mysql_status_file")
case "$mysql_samples" in
  "" | *[!0-9]* | 0)
    echo "MySQL status sampleがありません: $mysql_status_file" >&2
    exit 1
    ;;
esac

printf '\nMySQL one-second status samples\n\n'
printf '| samples | connected max | running avg | running max | row lock waits delta | row lock time delta ms | questions delta |\n'
printf '|---:|---:|---:|---:|---:|---:|---:|\n'
awk -F '\t' '
  NR == 2 {
    first_waits = $4
    first_lock_time = $5
    first_questions = $6
  }
  NR > 1 {
    samples += 1
    if ($2 > connected_max) connected_max = $2
    running_sum += $3
    if ($3 > running_max) running_max = $3
    last_waits = $4
    last_lock_time = $5
    last_questions = $6
  }
  END {
    printf "| %d | %d | %.2f | %d | %d | %d | %d |\n",
      samples,
      connected_max,
      running_sum / samples,
      running_max,
      last_waits - first_waits,
      last_lock_time - first_lock_time,
      last_questions - first_questions
  }
' "$mysql_status_file"
