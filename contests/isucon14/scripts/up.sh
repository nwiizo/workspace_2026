#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

"$script_dir/compose.sh" up \
  --detach \
  --build \
  --wait \
  --wait-timeout 300 \
  db webapp nginx matcher

echo "ISURIDE: http://localhost:${APP_PORT:-8080}"
