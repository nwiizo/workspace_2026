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

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-evaluation-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-evaluation-json.XXXXXX")

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
sed -n 's/^.*EVALUATION_DIAGNOSTIC //p' "$raw_log" >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のevaluation診断sampleがありません: $diagnostic_since" >&2
  exit 1
fi

first_sample_timestamp=$(
  sed -n '/EVALUATION_DIAGNOSTIC/ {
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
  echo "最初のevaluation sample時刻を解釈できません: $first_sample_timestamp" >&2
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
  ""|*[!0-9]*)
    echo "MySQLの起動時刻を取得できませんでした。" >&2
    exit 1
    ;;
esac

if [ "$server_started_epoch" -lt "$run_started_epoch" ] ||
  [ "$server_started_epoch" -gt "$first_sample_epoch" ]; then
  echo "MySQLが診断run用に再起動されたことを確認できません。" >&2
  echo "run開始=$diagnostic_since MySQL起動=$server_started_at 最初のsample=$first_sample_timestamp" >&2
  echo "./scripts/benchmark.sh はDBを停止・再起動します。同じrun直後にreportを実行してください。" >&2
  exit 1
fi

printf 'database metric scope\n\n'
printf '| boundary | UTC |\n'
printf '|---|---|\n'
printf '| requested run start | %s |\n' "$diagnostic_since"
printf '| MySQL process start | %s |\n' "$server_started_at"
printf '| first evaluation sample | %s |\n' "$first_sample_timestamp"
printf '\n'

printf 'successful evaluation phase latency\n\n'
printf 'Percentiles use the zero-based lower order statistic floor((n - 1) * p).\n\n'
printf '| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
for phase in \
  validation_us \
  pool_acquire_us \
  transaction_begin_us \
  ride_lock_status_us \
  tracker_begin_us \
  preparation_us \
  payment_us \
  payment_request_us \
  payment_retry_sleep_us \
  completion_write_us \
  commit_us \
  cache_response_us \
  connection_owned_us \
  total_us
do
  jq --slurp --raw-output --arg phase "$phase" '
    [
      .[] |
      select(.outcome == "success") |
      select(.[$phase] != null) |
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

printf '\noutcome and terminal phase\n\n'
printf '| outcome | terminal phase | samples |\n'
printf '|---|---|---:|\n'
jq --slurp --raw-output '
  sort_by([.outcome, .terminal_phase]) |
  group_by([.outcome, .terminal_phase])[] |
  "| \(.[0].outcome) | \(.[0].terminal_phase) | \(length) |"
' "$json_log"

printf '\npool state before sampled acquire\n\n'
printf 'summary: '
jq --slurp --raw-output '
  [.[] | select(.pool_size_before != null)] as $samples |
  if ($samples | length) == 0 then
    "no pool samples"
  else
    ($samples | map(.pool_size_before) | max) as $max_size |
    "samples=\($samples | length) " +
    "no_idle=\($samples | map(select(.pool_idle_before == 0)) | length) " +
    "observed_max_size=\($max_size) " +
    "observed_max_size_no_idle=\($samples | map(select(
      .pool_size_before == $max_size and .pool_idle_before == 0
    )) | length)"
  end
' "$json_log"
printf '\n'
printf '| pool size | idle | in use | samples |\n'
printf '|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.pool_size_before != null) |
    {
      size: .pool_size_before,
      idle: .pool_idle_before,
      in_use: .pool_in_use_before
    }
  ] |
  sort_by([.size, .idle, .in_use]) |
  group_by([.size, .idle, .in_use])[] |
  "| \(.[0].size) | \(.[0].idle) | \(.[0].in_use) | \(length) |"
' "$json_log"

printf '\npool acquire latency by state before acquire\n\n'
printf '| state | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.outcome == "success") |
    select(.pool_size_before != null)
  ] as $samples |
  ($samples | map(.pool_size_before) | max) as $max_size |
  def row($label; $values):
    ($values | sort) as $sorted |
    ($sorted | length) as $length |
    if $length == 0 then
      "| \($label) | 0 | n/a | n/a | n/a | n/a | n/a |"
    else
      "| \($label) | \($length) | " +
      "\(($sorted | add) / $length | floor) | " +
      "\($sorted[(($length - 1) * 0.50 | floor)]) | " +
      "\($sorted[(($length - 1) * 0.95 | floor)]) | " +
      "\($sorted[(($length - 1) * 0.99 | floor)]) | " +
      "\($sorted[$length - 1]) |"
    end;
  row(
    "observed_max_size_no_idle";
    [
      $samples[] |
      select(.pool_size_before == $max_size and .pool_idle_before == 0) |
      .pool_acquire_us
    ]
  ),
  row(
    "idle_positive";
    [
      $samples[] |
      select(.pool_idle_before > 0) |
      .pool_acquire_us
    ]
  ),
  row(
    "below_observed_max_no_idle";
    [
      $samples[] |
      select(.pool_size_before < $max_size and .pool_idle_before == 0) |
      .pool_acquire_us
    ]
  )
' "$json_log"

printf '\npayment attempts and terminal status\n\n'
printf '| attempts | terminal status | samples | avg payment_us | avg request_us | avg retry_sleep_us |\n'
printf '|---:|---|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.payment_attempts > 0) |
    {
      attempts: .payment_attempts,
      terminal_status: (.payment_terminal_status // "network_or_cancelled"),
      payment_us: .payment_us,
      request_us: .payment_request_us,
      retry_sleep_us: .payment_retry_sleep_us
    }
  ] |
  sort_by([.attempts, .terminal_status]) |
  group_by([.attempts, .terminal_status])[] |
  (length) as $length |
  "| \(.[0].attempts) | \(.[0].terminal_status) | \($length) | " +
  "\((map(.payment_us) | add) / $length | floor) | " +
  "\((map(.request_us) | add) / $length | floor) | " +
  "\((map(.retry_sleep_us) | add) / $length | floor) |"
' "$json_log"

printf '\npayment error counters across sampled evaluations\n\n'
jq --slurp --raw-output '
  {
    samples: length,
    attempts: (map(.payment_attempts) | add),
    network_errors: (map(.payment_network_errors) | add),
    conflict_errors: (map(.payment_conflict_errors) | add),
    server_errors: (map(.payment_server_errors) | add),
    other_status_errors: (map(.payment_other_status_errors) | add)
  } |
  "samples=\(.samples) attempts=\(.attempts) " +
  "network_errors=\(.network_errors) conflict_errors=\(.conflict_errors) " +
  "server_errors=\(.server_errors) other_status_errors=\(.other_status_errors)"
' "$json_log"

printf '\nactive evaluation concurrency after tracker begin\n\n'
printf 'summary: '
jq --slurp --raw-output '
  [.[] | select(.active_evaluations != null)] as $samples |
  if ($samples | length) == 0 then
    "no tracker samples"
  else
    "samples=\($samples | length) " +
    "max_active=\($samples | map(.active_evaluations) | max) " +
    "same_ride_concurrent=\($samples | map(select(.same_ride_evaluations > 1)) | length)"
  end
' "$json_log"
printf '\n'
printf '| active evaluations | same ride evaluations | samples |\n'
printf '|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.active_evaluations != null) |
    {
      active: .active_evaluations,
      same_ride: .same_ride_evaluations
    }
  ] |
  sort_by([.active, .same_ride]) |
  group_by([.active, .same_ride])[] |
  "| \(.[0].active) | \(.[0].same_ride) | \(length) |"
' "$json_log"
