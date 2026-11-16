#!/usr/bin/env bash
# On the measuring Mac, after scripts/run-remote.sh has started the linux-all
# container: run every remaining set one after another, so no two runs share
# the CPU. The image committed from the linux-all container keeps the tools,
# toolchains and work copies for the later Linux sets.
#
#   nohup bash ~/relax-bench/src/relax-jit/scripts/remote-sequence.sh > ~/relax-bench/sequence.log 2>&1 &
set -o pipefail
export PATH=$HOME/homebrew/bin:/opt/homebrew/bin:$PATH
b=$HOME/relax-bench
s=$b/src/relax-jit/scripts
log() { echo "[$(date +%T)] $*"; }
mounts=(-v "$b/src:/src:ro" -v "$b/out:/out" -v "$b/ca:/ca:ro")

log "waiting for relax-bench (linux-all.sh)"
docker wait relax-bench
docker commit relax-bench relax-bench:after-all > /dev/null && log "committed relax-bench:after-all"

log "linux-best.sh"
docker run --rm --name relax-best "${mounts[@]}" relax-bench:after-all bash /src/relax-jit/scripts/linux-best.sh

log "macos-all.sh"
bash "$s/macos-all.sh"

log "linux-round2.sh"
docker run --rm --name relax-round2 --tmpfs /ram:rw,exec,size=16g "${mounts[@]}" \
  relax-bench:after-all bash /src/relax-jit/scripts/linux-round2.sh

log "macos-firstrun.sh"
bash "$s/macos-firstrun.sh"

log "linux-tests.sh"
docker run --rm --name relax-tests "${mounts[@]}" relax-bench:after-all bash /src/relax-jit/scripts/linux-tests.sh

log "macos-checkfirst.sh"
bash "$s/macos-checkfirst.sh"
log "all done"
