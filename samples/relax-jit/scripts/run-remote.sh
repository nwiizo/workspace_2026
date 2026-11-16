#!/usr/bin/env bash
# Copy this sample and ../rust-grokking-ai-algorithms to a Mac reachable over
# SSH and run scripts/linux-all.sh in a detached container on its Docker
# (Colima). Results land in ~/relax-bench/out on that machine; fetch them with
#   rsync -a -e "ssh -p $PORT" "$HOST:relax-bench/out/" results/linux-all/
# Then run scripts/remote-sequence.sh there for the remaining sets.
#
#   HOST=user@host [PORT=22] [CA_NAME=<root CA name>] scripts/run-remote.sh
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
host=${HOST:?set HOST=user@host}
port=${PORT:-22}
remote=relax-bench
ssh_cmd="ssh -o BatchMode=yes -p $port"

$ssh_cmd "$host" "mkdir -p $remote/src/relax-jit $remote/src/grokking $remote/out $remote/ca"
# The network re-signs TLS for static.rust-lang.org and crates.io. Pass the
# MDM-installed root CA (CA_NAME) to the container; the host is not changed.
if [ -n "${CA_NAME:-}" ]; then
  $ssh_cmd "$host" "security find-certificate -c '$CA_NAME' -p /Library/Keychains/System.keychain > $remote/ca/corp.pem"
fi
rsync -a --delete -e "$ssh_cmd" --exclude vendor --exclude results --exclude target \
  "$here/" "$host:$remote/src/relax-jit/"
rsync -a --delete -e "$ssh_cmd" --exclude target \
  "$here/../rust-grokking-ai-algorithms/" "$host:$remote/src/grokking/"
# Homebrew may live in ~/homebrew or /opt/homebrew on the remote Mac.
$ssh_cmd "$host" "export PATH=\$HOME/homebrew/bin:/opt/homebrew/bin:\$PATH
  docker run -d --name relax-bench \
    -v \$HOME/$remote/src:/src:ro -v \$HOME/$remote/out:/out -v \$HOME/$remote/ca:/ca:ro \
    rust:1.100-bookworm bash /src/relax-jit/scripts/linux-all.sh"
