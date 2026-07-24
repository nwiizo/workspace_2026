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

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-coordinate-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-coordinate-json.XXXXXX")

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
sed -n 's/^.*COORDINATE_DIAGNOSTIC //p' "$raw_log" >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のcoordinate診断sampleがありません: $diagnostic_since" >&2
  exit 1
fi

first_sample_timestamp=$(
  sed -n '/COORDINATE_DIAGNOSTIC/ {
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
  echo "最初のcoordinate sample時刻を解釈できません: $first_sample_timestamp" >&2
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
printf '| first coordinate sample | %s |\n' "$first_sample_timestamp"
printf '\nInnoDB metrics below are cumulative since this fresh MySQL process started.\n'
printf 'Prepared-statement metrics are a lossy live snapshot at report time.\n\n'

printf 'successful coordinate phase latency\n\n'
printf 'Percentiles use the zero-based lower order statistic floor((n - 1) * p).\n\n'
printf '| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
for phase in \
  cache_lookup_us \
  pool_acquire_us \
  transaction_begin_us \
  pool_begin_us \
  history_insert_us \
  current_write_us \
  ride_lookup_us \
  transition_us \
  commit_us \
  cache_update_us \
  total_us
do
  jq --slurp --raw-output --arg phase "$phase" '
    [
      .[] |
      select((.outcome // "success") == "success") |
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

printf '\npool state before sampled acquire\n\n'
printf 'summary: '
jq --slurp --raw-output '
  [
    .[] |
    select(.pool_size_before != null)
  ] as $samples |
  if ($samples | length) == 0 then
    "no split pool samples"
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
  group_by([.size, .idle, .in_use])[] |
  "| \(.[0].size) | \(.[0].idle) | \(.[0].in_use) | \(length) |"
' "$json_log"

printf '\npool acquire latency by state before acquire\n\n'
printf '| state | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select((.outcome // "success") == "success") |
    select(.pool_size_before != null and .pool_acquire_us != null)
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

printf '\ncurrent-state write path\n\n'
printf '| path | samples |\n'
printf '|---|---:|\n'
jq --slurp --raw-output '
  group_by(.current_write_path)[] |
  "| \(.[0].current_write_path) | \(length) |"
' "$json_log"

printf '\ntransition sample\n\n'
jq --slurp --raw-output '
  "candidate=\(map(select(.transition_candidate)) | length) " +
  "inserted=\(map(select(.transition_inserted)) | length) " +
  "total=\(length)"
' "$json_log"

printf '\noutcome and terminal phase\n\n'
printf '| outcome | terminal phase | samples |\n'
printf '|---|---|---:|\n'
jq --slurp --raw-output '
  group_by([
    (.outcome // "success_legacy"),
    (.terminal_phase // "complete_legacy")
  ])[] |
  "| \(.[0].outcome // "success_legacy") | " +
  "\(.[0].terminal_phase // "complete_legacy") | \(length) |"
' "$json_log"

printf '\nerror / cancellation total latency by terminal phase\n\n'
printf '| terminal phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.outcome == "error_or_cancelled")
  ] |
  group_by(.terminal_phase)[] |
  ([.[].total_us] | sort) as $values |
  ($values | length) as $length |
  [
    .[0].terminal_phase,
    $length,
    (($values | add) / $length | floor),
    $values[(($length - 1) * 0.50 | floor)],
    $values[(($length - 1) * 0.95 | floor)],
    $values[(($length - 1) * 0.99 | floor)],
    $values[$length - 1]
  ] |
  "| \(.[0]) | \(.[1]) | \(.[2]) | \(.[3]) | \(.[4]) | \(.[5]) | \(.[6]) |"
' "$json_log"

printf '\nInnoDB row-lock metrics (fresh MySQL process lifetime cumulative)\n\n'
ISUCON_DIAGNOSTIC=1 "$compose" exec -T \
  -e MYSQL_PWD=isucon \
  db \
  mysql \
  --batch \
  --table \
  -uroot \
  information_schema \
  -e "
SELECT NAME, COUNT, TIME_ENABLED, TIME_ELAPSED, MAX_COUNT, AVG_COUNT
FROM INNODB_METRICS
WHERE NAME LIKE 'lock_row_lock%'
ORDER BY NAME
"

printf '\nprepared statement: chair_current_locations write (lossy live snapshot)\n\n'
ISUCON_DIAGNOSTIC=1 "$compose" exec -T \
  -e MYSQL_PWD=isucon \
  db \
  mysql \
  --batch \
  --table \
  -uroot \
  performance_schema \
  -e "
SELECT
  SUM(COUNT_EXECUTE) AS executions,
  ROUND(SUM(SUM_TIMER_EXECUTE) / 1e12, 3) AS total_seconds,
  ROUND(
    SUM(SUM_TIMER_EXECUTE) / NULLIF(SUM(COUNT_EXECUTE), 0) / 1e9,
    3
  ) AS avg_ms,
  ROUND(MAX(MAX_TIMER_EXECUTE) / 1e9, 3) AS max_ms,
  SUM(SUM_ROWS_AFFECTED) AS rows_affected
FROM prepared_statements_instances
WHERE SQL_TEXT LIKE '%chair_current_locations%'
  AND (
    SQL_TEXT LIKE '%UPDATE chair_current_locations%'
    OR SQL_TEXT LIKE '%INSERT INTO chair_current_locations%'
  )
"
