#!/bin/sh
set -eu

url="${1:?usage: wait-for-http.sh URL}"
attempts="${WAIT_ATTEMPTS:-60}"
sleep_seconds="${WAIT_SLEEP_SECONDS:-1}"

i=1
while [ "$i" -le "$attempts" ]; do
  if curl -fsS "$url" >/dev/null 2>&1; then
    echo "ready: $url"
    exit 0
  fi

  sleep "$sleep_seconds"
  i=$((i + 1))
done

echo "timeout waiting for $url" >&2
exit 1

