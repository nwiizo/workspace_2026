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

raw_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-notification-raw.XXXXXX")
json_log=$(mktemp "${TMPDIR:-/tmp}/isucon14-notification-json.XXXXXX")

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
sed -n 's/^.*NOTIFICATION_DIAGNOSTIC //p' "$raw_log" |
  jq --compact-output \
    'select(if has("periodic_sample") then .periodic_sample == true else true end)' \
    >"$json_log"

if [ ! -s "$json_log" ]; then
  echo "指定時刻以降のnotification診断sampleがありません: $diagnostic_since" >&2
  exit 1
fi

first_sample_timestamp=$(
  sed -n '/NOTIFICATION_DIAGNOSTIC/ {
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
  echo "最初のnotification sample時刻を解釈できません: $first_sample_timestamp" >&2
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
  echo "./scripts/benchmark.sh はDBを停止・再起動します。同じrun直後にreportを実行してください。" >&2
  exit 1
fi

printf 'diagnostic scope\n\n'
printf '| boundary | UTC |\n'
printf '|---|---|\n'
printf '| requested run start | %s |\n' "$diagnostic_since"
printf '| MySQL process start | %s |\n' "$server_started_at"
printf '| first notification sample | %s |\n' "$first_sample_timestamp"

printf '\nsample paths\n\n'
printf '| endpoint | path | outcome | samples | cache insert attempted |\n'
printf '|---|---|---|---:|---:|\n'
jq --slurp --raw-output '
  sort_by([.endpoint, .path, .outcome]) |
  group_by([.endpoint, .path, .outcome])[] |
  "| \(.[0].endpoint) | \(.[0].path) | \(.[0].outcome) | \(length) | " +
  "\(map(select(.cache_insert_attempted // .cache_inserted // false)) | length) |"
' "$json_log"

printf '\ncache-hit share among successful samples\n\n'
printf '| endpoint | successful samples | cache hits | cache-hit share |\n'
printf '|---|---:|---:|---:|\n'
jq --slurp --raw-output '
  [.[] | select(.outcome == "success")] |
  group_by(.endpoint)[] |
  (length) as $total |
  (map(select(.path == "cache_hit")) | length) as $hits |
  "| \(.[0].endpoint) | \($total) | \($hits) | " +
  "\((1000 * $hits / $total | floor) / 10)% |"
' "$json_log"

printf '\ntotal latency by successful path\n\n'
printf '| endpoint | path | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [.[] | select(.outcome == "success")] |
  sort_by([.endpoint, .path]) |
  group_by([.endpoint, .path])[] |
  ([.[].total_us] | sort) as $values |
  ($values | length) as $length |
  "| \(.[0].endpoint) | \(.[0].path) | \($length) | " +
  "\(($values | add) / $length | floor) | " +
  "\($values[(($length - 1) * 0.50 | floor)]) | " +
  "\($values[(($length - 1) * 0.95 | floor)]) | " +
  "\($values[(($length - 1) * 0.99 | floor)]) | " +
  "\($values[$length - 1]) |"
' "$json_log"

printf '\nconnection ownership by successful DB path\n\n'
printf '| endpoint | DB path | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
printf '|---|---|---:|---:|---:|---:|---:|---:|\n'
jq --slurp --raw-output '
  [
    .[] |
    select(.outcome == "success" and .connection_owned_us != null) |
    {
      endpoint,
      db_path: (if .path == "no_ride" then "no_ride" else "ride_present" end),
      connection_owned_us
    }
  ] |
  sort_by([.endpoint, .db_path]) |
  group_by([.endpoint, .db_path])[] |
  ([.[].connection_owned_us] | sort) as $values |
  ($values | length) as $length |
  "| \(.[0].endpoint) | \(.[0].db_path) | \($length) | " +
  "\(($values | add) / $length | floor) | " +
  "\($values[(($length - 1) * 0.50 | floor)]) | " +
  "\($values[(($length - 1) * 0.95 | floor)]) | " +
  "\($values[(($length - 1) * 0.99 | floor)]) | " +
  "\($values[$length - 1]) |"
' "$json_log"

for endpoint in app chair; do
  printf '\nsuccessful %s notification phase latency\n\n' "$endpoint"
  printf 'Percentiles use the zero-based lower order statistic floor((n - 1) * p).\n\n'
  printf '| phase | samples | avg_us | p50_us | p95_us | p99_us | max_us |\n'
  printf '|---|---:|---:|---:|---:|---:|---:|\n'
  for phase in \
    cache_lookup_us \
    initial_pool_acquire_us \
    latest_ride_query_us \
    initial_connection_owned_us \
    dependency_revision_us \
    transaction_pool_acquire_us \
    transaction_begin_us \
    ride_query_us \
    pending_status_query_us \
    latest_status_query_us \
    fare_query_us \
    chair_query_us \
    chair_stats_query_us \
    user_query_us \
    sent_update_us \
    commit_us \
    transaction_connection_owned_us \
    connection_owned_us \
    response_us \
    total_us
  do
    jq --slurp --raw-output --arg endpoint "$endpoint" --arg phase "$phase" '
      [
        .[] |
        select(.endpoint == $endpoint and .outcome == "success") |
        select(.[$phase] != null) |
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
done

for stage in initial transaction; do
  printf '\n%s pool state before sampled acquire\n\n' "$stage"
  printf '| endpoint | samples | no idle | observed max size | max size and no idle |\n'
  printf '|---|---:|---:|---:|---:|\n'
  jq --slurp --raw-output --arg stage "$stage" '
    ($stage + "_pool_size_before") as $size_key |
    ($stage + "_pool_idle_before") as $idle_key |
    [
      .[] |
      select(.[$size_key] != null)
    ] |
    group_by(.endpoint)[] |
    (map(.[$size_key]) | max) as $max_size |
    "| \(.[0].endpoint) | \(length) | " +
    "\(map(select(.[$idle_key] == 0)) | length) | \($max_size) | " +
    "\(map(select(.[$size_key] == $max_size and .[$idle_key] == 0)) | length) |"
  ' "$json_log"
done

printf '\noutcome and terminal phase\n\n'
printf '| endpoint | outcome | terminal phase | samples |\n'
printf '|---|---|---|---:|\n'
jq --slurp --raw-output '
  sort_by([.endpoint, .outcome, .terminal_phase]) |
  group_by([.endpoint, .outcome, .terminal_phase])[] |
  "| \(.[0].endpoint) | \(.[0].outcome) | \(.[0].terminal_phase) | \(length) |"
' "$json_log"
