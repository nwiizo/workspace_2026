#!/usr/bin/env bash
# Combinations of the speedups that worked in scripts/linux-all.sh, one per
# kind of wait, measured against a fresh stable baseline and Relax.
# Runs in an image committed from the linux-all container, so the tools,
# toolchains, Relax and the work copies are already there.
set -uo pipefail

out=/out/best
mkdir -p "$out"
exec > >(tee -a "$out/run.log") 2>&1
step() { echo "== [$(date +%T)] $*"; }

export PATH=/opt/bin:$PATH SCCACHE_DIR=/tmp/sccache
if ls /ca/*.pem > /dev/null 2>&1; then
  export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt
fi
NIGHTLY=$(rustup toolchain list | grep -o '^nightly[^ ]*' | head -1)
R=/opt/relax/target/release/relax
grok_edit="sed -i -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
cli_edit="sed -i -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"

DBG="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
CL="-Zcodegen-backend --config 'profile.dev.codegen-backend=\"cranelift\"'"
OPT1="--config profile.dev.opt-level=1"
D3="--config 'profile.dev.package.\"*\".opt-level=3'"
SPLIT="--config 'profile.dev.split-debuginfo=\"unpacked\"'"
FAST="-Zthreads=8 -Clink-arg=-fuse-ld=mold"
declare -A C=(
  [stable-default]="cargo +stable"
  [edit-best-stable]="RUSTFLAGS=-Clink-arg=-fuse-ld=mold cargo +stable $DBG $OPT1"
  [edit-best]="RUSTFLAGS='$FAST' cargo +$NIGHTLY $DBG $OPT1"
  [edit-best-split]="RUSTFLAGS='$FAST' cargo +$NIGHTLY $DBG $OPT1 $SPLIT"
  [edit-best-deps3]="RUSTFLAGS='$FAST' cargo +$NIGHTLY $DBG $OPT1 $D3"
  [clean-best]="RUSTC_WRAPPER=/opt/bin/sccache CARGO_INCREMENTAL=0 RUSTFLAGS='$FAST' cargo +$NIGHTLY $DBG $CL"
  [release]="cargo +stable"
  [release-best]="RUSTC_WRAPPER=/opt/bin/sccache RUSTFLAGS='$FAST' cargo +$NIGHTLY"
  [release-best-stable]="RUSTC_WRAPPER=/opt/bin/sccache RUSTFLAGS=-Clink-arg=-fuse-ld=mold cargo +stable"
)

bench() {
  local name=$1 runs=$2
  shift 2
  hyperfine -i --shell=bash --style basic --runs "$runs" --export-json "$out/$name.json" "$@" > "$out/$name.log" 2>&1
  echo "$name exit=$?"
}

run_set() { # workload mode runs names...
  local wl=$1 mode=$2 runs=$3
  shift 3
  local targs edit bin rpkg args=() extra=()
  case $wl in
    grokking) targs="--example all_chapters"; edit=$grok_edit; bin=examples/all_chapters; rpkg="rust-grokking-ai-algorithms --example all_chapters" ;;
    cli-deps) targs=""; edit=$cli_edit; bin=cli-deps; rpkg="cli-deps" ;;
  esac
  [ "$mode" = runonly ] && extra=(--warmup 3)
  cd "/work/$wl" || return
  for name in "$@"; do
    if [ "$name" = relax ]; then
      local r="$R run --quiet $rpkg"
      case $mode in
        edit) eval "$r" > /dev/null 2>&1; args+=(-n relax --prepare "$edit" "$r") ;;
        runonly) eval "$r" > /dev/null 2>&1; args+=(-n relax-warm "$r") ;;
        cold) args+=(-n relax --prepare "$R clean" "$r") ;;
      esac
      continue
    fi
    local td="/tb/$wl/$name" rel="" prof=debug
    case $name in release*) rel="--release"; prof=release ;; esac
    local p="CARGO_TARGET_DIR=$td ${C[$name]}"
    case $mode in
      edit)
        eval "$p run -q $rel $targs" > "$out/$wl-$name.out" 2> "$out/$wl-$name.err"
        args+=(-n "$name" --prepare "$edit" "$p run -q $rel $targs") ;;
      runonly)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" "$td/$prof/$bin") ;;
      cold)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p run -q $rel $targs") ;;
      relcold)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p build -q $rel $targs") ;;
    esac
  done
  bench "$wl-$mode" "$runs" "${extra[@]}" "${args[@]}"
}

step grokking edit;    run_set grokking edit 10 stable-default edit-best-stable edit-best edit-best-split
step grokking runonly; run_set grokking runonly 10 stable-default edit-best-stable edit-best release
step grokking cold;    run_set grokking cold 3 stable-default edit-best-stable clean-best
step grokking relcold; run_set grokking relcold 3 release release-best-stable release-best
step cli-deps edit;    run_set cli-deps edit 10 stable-default edit-best-stable edit-best edit-best-deps3
step cli-deps cold;    run_set cli-deps cold 3 stable-default edit-best-stable clean-best
step cli-deps relcold; run_set cli-deps relcold 3 release release-best-stable release-best

step grokking test
cd /work/grokking
nx="nextest run --hide-progress-bar --status-level none --final-status-level none"
tb_env="RUSTFLAGS='$FAST' CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only"
eval "CARGO_TARGET_DIR=/tb/test-s0 cargo +stable test -q" > /dev/null 2>&1
eval "CARGO_TARGET_DIR=/tb/test-best $tb_env cargo +$NIGHTLY $nx" > /dev/null 2>&1
bench grokking-test 10 \
  -n stable-default-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=/tb/test-s0 cargo +stable test -q" \
  -n test-best --prepare "$grok_edit" "CARGO_TARGET_DIR=/tb/test-best $tb_env cargo +$NIGHTLY $nx"
step done
