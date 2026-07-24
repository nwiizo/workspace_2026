#!/bin/sh

set -eu

barrier_dir=${PAYMENT_BARRIER_DIR:-/tmp/payment-barrier}
count_file="$barrier_dir/request-count"
lock_dir="$barrier_dir/count-lock"
release_file="$barrier_dir/release"

mkdir -p "$barrier_dir"

# Consume the request headers before recording arrival. The test releases the
# barrier only after two handler processes reach this point, proving that both
# evaluation requests passed their preparation transaction.
carriage_return=$(printf '\r')
while IFS= read -r line; do
  case "$line" in
    ""|"$carriage_return") break ;;
  esac
done

while ! mkdir "$lock_dir" 2>/dev/null; do
  sleep 0.01
done

request_count=0
if [ -f "$count_file" ]; then
  IFS= read -r request_count <"$count_file"
fi
request_count=$((request_count + 1))
printf '%s\n' "$request_count" >"$count_file"
: >"$barrier_dir/arrived-$request_count"
rmdir "$lock_dir"

while [ ! -f "$release_file" ]; do
  sleep 0.01
done

printf 'HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n'
