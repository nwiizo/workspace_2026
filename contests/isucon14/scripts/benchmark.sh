#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"
duration=${BENCHMARK_DURATION:-${1:-60}}

case "$duration" in
  ""|*[!0-9]*)
    echo "BENCHMARK_DURATION は 0 以上の整数で指定してください: $duration" >&2
    exit 2
    ;;
esac

# 前回起動したISUCON stackは、再ビルドには不要。matcherのpollingと
# MySQLのmemory保持が、固定されたローカル資源をRust buildと奪い合わない
# よう正常停止し、up.shの中で新しいwebappと一緒に再開する。
"$compose" stop matcher nginx webapp db >/dev/null 2>&1 || true

"$script_dir/up.sh"
"$compose" --profile benchmark build benchmark

mysql_status_sampler_pid=
stop_mysql_status_sampler() {
  if [ -n "$mysql_status_sampler_pid" ]; then
    kill "$mysql_status_sampler_pid" >/dev/null 2>&1 || true
    wait "$mysql_status_sampler_pid" 2>/dev/null || true
    mysql_status_sampler_pid=
  fi
}
trap 'stop_mysql_status_sampler; exit 129' HUP
trap 'stop_mysql_status_sampler; exit 130' INT
trap 'stop_mysql_status_sampler; exit 143' TERM

if [ -n "${MYSQL_STATUS_OUTPUT_FILE:-}" ]; then
  "$script_dir/sample-mysql-status.sh" "$MYSQL_STATUS_OUTPUT_FILE" &
  mysql_status_sampler_pid=$!
fi

benchmark_name="isucon14-benchmark-$$"
set -- run \
  --target http://nginx \
  --payment-url "http://$benchmark_name:12345" \
  --load-timeout "$duration" \
  --fail-on-error

if [ "${SKIP_STATIC_SANITY_CHECK:-0}" = "1" ]; then
  set -- "$@" --skip-static-sanity-check
fi

if [ -n "${BENCHMARK_OUTPUT_FILE:-}" ]; then
  set +e
  "$compose" --profile benchmark run \
    --rm \
    --name "$benchmark_name" \
    benchmark "$@" >"$BENCHMARK_OUTPUT_FILE" 2>&1
  benchmark_status=$?
  set -e
  stop_mysql_status_sampler
  cat "$BENCHMARK_OUTPUT_FILE"
  exit "$benchmark_status"
fi

if [ -n "$mysql_status_sampler_pid" ]; then
  set +e
  "$compose" --profile benchmark run \
    --rm \
    --name "$benchmark_name" \
    benchmark "$@"
  benchmark_status=$?
  set -e
  stop_mysql_status_sampler
  exit "$benchmark_status"
fi

exec "$compose" --profile benchmark run \
  --rm \
  --name "$benchmark_name" \
  benchmark "$@"
