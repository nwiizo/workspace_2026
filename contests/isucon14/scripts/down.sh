#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

if [ "${RESET:-0}" = "1" ]; then
  "$script_dir/compose.sh" down --volumes --remove-orphans
else
  "$script_dir/compose.sh" down --remove-orphans
fi
