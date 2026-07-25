#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
output_file=${1:-}
interval_seconds=${2:-1}

if [ -z "$output_file" ]; then
  echo "出力先を第1引数に指定してください。" >&2
  exit 2
fi
case "$interval_seconds" in
  "" | *[!0-9]*)
    echo "採取間隔は1以上の整数秒で指定してください: $interval_seconds" >&2
    exit 2
    ;;
esac
if [ "$interval_seconds" -eq 0 ]; then
  echo "採取間隔は1以上の整数秒で指定してください。" >&2
  exit 2
fi

printf 'sampled_at\tthreads_connected\tthreads_running\trow_lock_waits\trow_lock_time_ms\tquestions\n' >"$output_file"

while :; do
  sampled_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  values=$(
    ISUCON_DIAGNOSTIC=1 "$compose" exec -T \
      -e MYSQL_PWD=isucon \
      db \
      mysql \
      --batch \
      --raw \
      --skip-column-names \
      -uroot \
      performance_schema \
      -e "
SELECT
  MAX(CASE WHEN VARIABLE_NAME = 'Threads_connected' THEN VARIABLE_VALUE END),
  MAX(CASE WHEN VARIABLE_NAME = 'Threads_running' THEN VARIABLE_VALUE END),
  MAX(CASE WHEN VARIABLE_NAME = 'Innodb_row_lock_waits' THEN VARIABLE_VALUE END),
  MAX(CASE WHEN VARIABLE_NAME = 'Innodb_row_lock_time' THEN VARIABLE_VALUE END),
  MAX(CASE WHEN VARIABLE_NAME = 'Questions' THEN VARIABLE_VALUE END)
FROM global_status
WHERE VARIABLE_NAME IN (
  'Threads_connected',
  'Threads_running',
  'Innodb_row_lock_waits',
  'Innodb_row_lock_time',
  'Questions'
)
"
  )
  printf '%s\t%s\n' "$sampled_at" "$values" >>"$output_file"
  sleep "$interval_seconds"
done
