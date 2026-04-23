#!/bin/sh
set -u

mode="${1:-baseline}"
target="${ZAP_TARGET:-http://vulnerable-app:5000}"
spider_minutes="${ZAP_SPIDER_MINUTES:-1}"
zap_options="${ZAP_OPTIONS:-}"
config_file=""

case "$target" in
  http://vulnerable-app:*|https://vulnerable-app:*|http://fixed-app:*|https://fixed-app:*|http://localhost:*|https://localhost:*|http://127.0.0.1:*|https://127.0.0.1:*)
    ;;
  *)
    if [ "${ALLOW_NON_LOCAL_TARGET:-0}" != "1" ]; then
      echo "Refusing non-local target: $target" >&2
      echo "Set ALLOW_NON_LOCAL_TARGET=1 only for explicitly authorized targets." >&2
      exit 64
    fi
    ;;
esac

mkdir -p reports

case "$mode" in
  baseline)
    zap_script="zap-baseline.py"
    prefix="${REPORT_PREFIX:-zap-baseline}"
    scan_args="-m $spider_minutes"
    ;;
  full)
    zap_script="zap-full-scan.py"
    prefix="${REPORT_PREFIX:-zap-full}"
    scan_args="-m $spider_minutes"
    config_file="/zap/conf/full-scan.conf"
    max_rule_minutes="${ZAP_MAX_RULE_MINUTES:-1}"
    max_scan_minutes="${ZAP_MAX_SCAN_MINUTES:-5}"
    zap_options="-config scanner.maxRuleDurationInMins=$max_rule_minutes -config scanner.maxScanDurationInMins=$max_scan_minutes $zap_options"
    ;;
  *)
    echo "usage: run-zap.sh baseline|full" >&2
    exit 64
    ;;
esac

echo "Running $zap_script against $target"

run_scan() {
  if [ -n "$config_file" ] && [ -n "$zap_options" ]; then
    docker compose run --rm zap \
      "$zap_script" \
      -t "$target" \
      $scan_args \
      -c "$config_file" \
      -r "$prefix.html" \
      -J "$prefix.json" \
      -w "$prefix.md" \
      -z "$zap_options" \
      -I
  elif [ -n "$config_file" ]; then
    docker compose run --rm zap \
      "$zap_script" \
      -t "$target" \
      $scan_args \
      -c "$config_file" \
      -r "$prefix.html" \
      -J "$prefix.json" \
      -w "$prefix.md" \
      -I
  elif [ -n "$zap_options" ]; then
    docker compose run --rm zap \
      "$zap_script" \
      -t "$target" \
      $scan_args \
      -r "$prefix.html" \
      -J "$prefix.json" \
      -w "$prefix.md" \
      -z "$zap_options" \
      -I
  else
    docker compose run --rm zap \
      "$zap_script" \
      -t "$target" \
      $scan_args \
      -r "$prefix.html" \
      -J "$prefix.json" \
      -w "$prefix.md" \
      -I
  fi
}

set +e
run_scan
status=$?
set -e

case "$status" in
  0|1|2)
    echo "ZAP scan completed with exit code $status"
    echo "Reports:"
    echo "  reports/$prefix.html"
    echo "  reports/$prefix.md"
    echo "  reports/$prefix.json"
    exit 0
    ;;
  *)
    echo "ZAP scan failed with exit code $status" >&2
    exit "$status"
    ;;
esac
