#!/usr/bin/env bash
# Linux linkers (GNU ld.bfd, LLD, mold, wild) in an aarch64 Linux container on
# this Mac's Docker VM (Colima). Results go to results/linux/. Compare only
# within Linux: the VM has fewer cores than the host.
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
dl="$here/results/tools/dl"
mkdir -p "$dl" "$here/results/linux"
wild=wild-linker-0.11.0-aarch64-unknown-linux-gnu.tar.gz
if [ ! -f "$dl/$wild" ]; then
  curl -sSL -o "$dl/$wild" "https://github.com/davidlattimore/wild/releases/download/0.11.0/$wild"
fi
shasum -a 256 "$dl/$wild" > "$here/results/linux/wild.sha256"

docker run --rm \
  -v "$here:/src/relax-jit:ro" \
  -v "$here/../rust-grokking-ai-algorithms:/src/grokking:ro" \
  -v "$dl:/dl:ro" \
  -v "$here/results/linux:/out" \
  rust:1.100-bookworm bash /src/relax-jit/scripts/linux-in-container.sh
