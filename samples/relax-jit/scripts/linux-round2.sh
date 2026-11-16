#!/usr/bin/env bash
# Round 2: start from the best combination of each wait in linux-best.sh and
# add one more change at a time: wild instead of mold, debug = 0, no debug
# assertions / overflow checks, codegen-units = 16, and the target dir on a
# tmpfs (/ram). Runs in the image committed from the linux-all container.
set -uo pipefail

out=/out/round2
mkdir -p "$out"
exec > >(tee -a "$out/run.log") 2>&1
step() { echo "== [$(date +%T)] $*"; }

export PATH=/opt/bin:$PATH SCCACHE_DIR=/tmp/sccache
if ls /ca/*.pem > /dev/null 2>&1; then
  export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt
fi
NIGHTLY=$(rustup toolchain list | grep -o '^nightly[^ ]*' | head -1)
grok_edit="sed -i -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
cli_edit="sed -i -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"

DBG="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
D0="--config profile.dev.debug=0"
CL="-Zcodegen-backend --config 'profile.dev.codegen-backend=\"cranelift\"'"
OPT1="--config profile.dev.opt-level=1"
NOCHK="--config profile.dev.debug-assertions=false --config profile.dev.overflow-checks=false"
CGU="--config profile.dev.codegen-units=16"
MOLD="-Clink-arg=-fuse-ld=mold"
WILD="-Clinker=clang -Clink-arg=--ld-path=/opt/bin/wild"
SC="RUSTC_WRAPPER=/opt/bin/sccache"
D0R="--config profile.release.debug=0"

# name -> "target-root|command prefix"
conf() {
  case $1 in
    stable-default) echo "/t2|cargo +stable" ;;
    e-best) echo "/t2|RUSTFLAGS='$MOLD' cargo +stable $DBG $OPT1" ;;
    e-wild) echo "/t2|RUSTFLAGS='$WILD' cargo +stable $DBG $OPT1" ;;
    e-mold-d0) echo "/t2|RUSTFLAGS='$MOLD' cargo +stable $D0 $OPT1" ;;
    e-wild-d0) echo "/t2|RUSTFLAGS='$WILD' cargo +stable $D0 $OPT1" ;;
    e-wild-d0-nochk) echo "/t2|RUSTFLAGS='$WILD' cargo +stable $D0 $OPT1 $NOCHK" ;;
    e-wild-d0-cgu16) echo "/t2|RUSTFLAGS='$WILD' cargo +stable $D0 $OPT1 $CGU" ;;
    e-wild-d0-ram) echo "/ram|RUSTFLAGS='$WILD' cargo +stable $D0 $OPT1" ;;
    e-wild-d0-threads8) echo "/t2|RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY $D0 $OPT1" ;;
    e-wild-d0-threads8-ram) echo "/ram|RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY $D0 $OPT1" ;;
    c-best) echo "/t2|$SC CARGO_INCREMENTAL=0 RUSTFLAGS='-Zthreads=8 $MOLD' cargo +$NIGHTLY $DBG $CL" ;;
    c-wild) echo "/t2|$SC CARGO_INCREMENTAL=0 RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY $DBG $CL" ;;
    c-wild-d0) echo "/t2|$SC CARGO_INCREMENTAL=0 RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY $D0 $CL" ;;
    c-wild-d0-ram) echo "/ram|$SC CARGO_INCREMENTAL=0 RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY $D0 $CL" ;;
    release) echo "/t2|cargo +stable" ;;
    r-best) echo "/t2|$SC RUSTFLAGS='$MOLD' cargo +stable" ;;
    r-wild) echo "/t2|$SC RUSTFLAGS='$WILD' cargo +stable" ;;
    r-wild-threads8) echo "/t2|$SC RUSTFLAGS='-Zthreads=8 $WILD' cargo +$NIGHTLY" ;;
    r-wild-ram) echo "/ram|$SC RUSTFLAGS='$WILD' cargo +stable" ;;
  esac
}

bench() {
  local name=$1 runs=$2
  shift 2
  hyperfine -i --shell=bash --style basic --runs "$runs" --export-json "$out/$name.json" "$@" > "$out/$name.log" 2>&1
  echo "$name exit=$?"
}

run_set() { # workload mode runs names...
  local wl=$1 mode=$2 runs=$3
  shift 3
  local targs edit args=()
  case $wl in
    grokking) targs="--example all_chapters"; edit=$grok_edit ;;
    cli-deps) targs=""; edit=$cli_edit ;;
  esac
  cd "/work/$wl" || return
  for name in "$@"; do
    local c root p td rel=""
    c=$(conf "$name"); root=${c%%|*}; p=${c#*|}
    td="$root/$wl/$name"
    case $name in release|r-*) rel="--release" ;; esac
    p="CARGO_TARGET_DIR=$td $p"
    case $mode in
      edit)
        eval "$p run -q $targs" > "$out/$wl-$name.out" 2> "$out/$wl-$name.err"
        args+=(-n "$name" --prepare "$edit" "$p run -q $targs") ;;
      cold)
        eval "$p build -q $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p run -q $targs") ;;
      relcold)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p build -q $rel $targs") ;;
    esac
  done
  bench "$wl-$mode" "$runs" "${args[@]}"
}

EDIT=(stable-default e-best e-wild e-mold-d0 e-wild-d0 e-wild-d0-nochk e-wild-d0-cgu16 e-wild-d0-ram e-wild-d0-threads8 e-wild-d0-threads8-ram)
step grokking edit;    run_set grokking edit 10 "${EDIT[@]}"
step cli-deps edit;    run_set cli-deps edit 10 "${EDIT[@]}"
CLEAN=(stable-default c-best c-wild c-wild-d0 c-wild-d0-ram)
step grokking cold;    run_set grokking cold 3 "${CLEAN[@]}"
step cli-deps cold;    run_set cli-deps cold 3 "${CLEAN[@]}"
REL=(release r-best r-wild r-wild-threads8 r-wild-ram)
step grokking relcold; run_set grokking relcold 3 "${REL[@]}"
step cli-deps relcold; run_set cli-deps relcold 3 "${REL[@]}"

step grokking test
cd /work/grokking
nx="nextest run --hide-progress-bar --status-level none --final-status-level none"
t_best="RUSTFLAGS='-Zthreads=8 $MOLD' CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo +$NIGHTLY $nx"
t_wild="RUSTFLAGS='$WILD' CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=0 cargo +stable $nx"
for v in "best|/t2|$t_best" "wild-d0|/t2|$t_wild" "wild-d0-ram|/ram|$t_wild"; do
  IFS='|' read -r name root cmd <<< "$v"
  eval "CARGO_TARGET_DIR=$root/test-$name $cmd" > /dev/null 2>&1
done
bench grokking-test 10 \
  -n stable-default-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=/t2/test-s0 cargo +stable test -q" \
  -n test-best --prepare "$grok_edit" "CARGO_TARGET_DIR=/t2/test-best $t_best" \
  -n test-wild-d0 --prepare "$grok_edit" "CARGO_TARGET_DIR=/t2/test-wild-d0 $t_wild" \
  -n test-wild-d0-ram --prepare "$grok_edit" "CARGO_TARGET_DIR=/ram/test-wild-d0-ram $t_wild"
step done
