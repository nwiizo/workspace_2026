#!/bin/sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
compose="$script_dir/compose.sh"

"$compose" exec -T nginx curl --fail --silent --show-error http://127.0.0.1/ >/dev/null

response=$(
  "$compose" exec -T nginx curl \
    --fail \
    --silent \
    --show-error \
    --request POST \
    --header "Content-Type: application/json" \
    --data '{"payment_server":"http://benchmark:12345"}' \
    http://127.0.0.1/api/initialize
)

case "$response" in
  *'"language":"rust"'*)
    echo "OK: GET / -> 200"
    echo "OK: POST /api/initialize -> $response"
    ;;
  *)
    echo "POST /api/initialize の応答が想定外です: $response" >&2
    exit 1
    ;;
esac
