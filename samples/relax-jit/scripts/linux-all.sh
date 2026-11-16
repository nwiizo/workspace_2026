#!/usr/bin/env bash
# Every comparison of this sample in one aarch64 Linux container
# (rust:1.100-bookworm), so all numbers come from one quiet machine.
# Started by scripts/run-remote.sh with /src (read-only) and /out mounted.
#
# Toolchains and tools are installed inside the container: nightly + Cranelift,
# mold, LLD, wild 0.11.0, sccache 0.19.0, cargo-nextest 0.9.152, hyperfine,
# and Relax at c1301db built from source.
set -uo pipefail

out=/out
mkdir -p "$out"
exec > >(tee -a "$out/run.log") 2>&1
step() { echo "== [$(date +%T)] $*"; }

# ---------------------------------------------------------------- tools
step tools
# A network that re-signs TLS (SSL inspection) needs its root CA. If one is
# mounted at /ca, trust it inside this container only.
if ls /ca/*.pem > /dev/null 2>&1; then
  for f in /ca/*.pem; do cp "$f" "/usr/local/share/ca-certificates/$(basename "$f" .pem).crt"; done
  update-ca-certificates > /dev/null 2>&1
  export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt RUSTUP_USE_CURL=1
fi
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq > /dev/null
apt-get install -y -qq mold lld clang hyperfine git jq > /dev/null
mkdir -p /opt/bin /tmp/dl
export PATH=/opt/bin:$PATH
NIGHTLY=nightly-2026-11-14
rustup toolchain install "$NIGHTLY" --profile minimal --component rustc-codegen-cranelift-preview > /dev/null 2>&1 \
  || { NIGHTLY=nightly; rustup toolchain install nightly --profile minimal --component rustc-codegen-cranelift-preview > /dev/null; }
dl() { curl -sSL --retry 3 -o "$2" "$1"; }
gh=https://github.com
dl $gh/davidlattimore/wild/releases/download/0.11.0/wild-linker-0.11.0-aarch64-unknown-linux-gnu.tar.gz /tmp/dl/wild.tgz
tar -xzf /tmp/dl/wild.tgz -C /opt/bin --strip-components=1
s=sccache-v0.19.0-aarch64-unknown-linux-musl
dl $gh/mozilla/sccache/releases/download/v0.19.0/$s.tar.gz /tmp/dl/$s.tar.gz
dl $gh/mozilla/sccache/releases/download/v0.19.0/$s.tar.gz.sha256 /tmp/dl/$s.tar.gz.sha256
(cd /tmp/dl && echo "$(cut -d' ' -f1 < $s.tar.gz.sha256)  $s.tar.gz" | sha256sum -c -) && tar -xzf /tmp/dl/$s.tar.gz -C /opt/bin --strip-components=1 $s/sccache
n=cargo-nextest-0.9.152-aarch64-unknown-linux-gnu
dl $gh/nextest-rs/nextest/releases/download/cargo-nextest-0.9.152/$n.tar.gz /tmp/dl/$n.tar.gz
dl $gh/nextest-rs/nextest/releases/download/cargo-nextest-0.9.152/$n.sha256 /tmp/dl/$n.sha256
(cd /tmp/dl && echo "$(cut -d' ' -f1 < $n.sha256)  $n.tar.gz" | sha256sum -c -) && tar -xzf /tmp/dl/$n.tar.gz -C /opt/bin
export SCCACHE_DIR=/tmp/sccache
{
  uname -srm; nproc; free -g | head -2
  rustc +stable -V; cargo +stable -V; rustc +"$NIGHTLY" -V
  ld.bfd --version | head -1; ld.lld --version; mold --version; wild --version
  sccache --version; cargo nextest --version | head -1; hyperfine --version; clang --version | head -1
} > "$out/versions.txt" 2>&1

# ---------------------------------------------------------------- relax
step relax build
git clone -q https://github.com/makepad/relax /opt/relax && git -C /opt/relax checkout -q c1301db6264986df964cc79ff1f6a9afda3b7fe6
SECONDS=0
cargo +stable build --release --manifest-path /opt/relax/Cargo.toml -p relax-runner --bin relax > "$out/relax-build.log" 2>&1
echo "relax build exit=$? seconds=$SECONDS" | tee -a "$out/relax-build.log"
R=/opt/relax/target/release/relax

# ---------------------------------------------------------------- workloads
step workloads
mkdir -p /work /t
cp -a /src/grokking /work/grokking && rm -rf /work/grokking/target
printf '\n[workspace]\n' >> /work/grokking/Cargo.toml
sed -i -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' /work/grokking/src/search.rs
for p in cli-deps closure-capture borrowck-bg perf-rounds live-counter; do
  cp -a "/src/relax-jit/$p" "/work/$p" && rm -rf "/work/$p/target"
done
grok_edit="sed -i -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
cli_edit="sed -i -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"

DBG="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
CL="-Zcodegen-backend --config 'profile.dev.codegen-backend=\"cranelift\"'"
FU="-Zfeature-unification --config 'resolver.feature-unification=\"workspace\"'"
OPT1="--config profile.dev.opt-level=1"
D3="--config 'profile.dev.package.\"*\".opt-level=3'"
BO3="--config profile.dev.build-override.opt-level=3"
declare -A C=(
  [stable-default]="cargo +stable"
  [debuginfo]="cargo +stable $DBG"
  [nightly-debuginfo]="cargo +$NIGHTLY $DBG"
  [threads8]="RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG"
  [threads8-cranelift]="RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG $CL"
  [build-all]="RUSTFLAGS='-Zthreads=8 -Clink-arg=-fuse-ld=mold' cargo +$NIGHTLY $DBG $CL $FU"
  [lld]="RUSTFLAGS=-Clink-arg=-fuse-ld=lld cargo +stable"
  [mold]="RUSTFLAGS=-Clink-arg=-fuse-ld=mold cargo +stable"
  [wild]="RUSTFLAGS='-Clinker=clang -Clink-arg=--ld-path=/opt/bin/wild' cargo +stable"
  [opt1]="cargo +stable $OPT1"
  [deps-opt3]="cargo +stable $D3"
  [build-override]="cargo +stable $BO3"
  [run-all]="RUSTFLAGS=-Clink-arg=-fuse-ld=mold cargo +stable $DBG $OPT1 $D3 $BO3"
  [sccache]="RUSTC_WRAPPER=/opt/bin/sccache CARGO_INCREMENTAL=0 cargo +stable"
  [release]="cargo +stable"
  [release-mold]="RUSTFLAGS=-Clink-arg=-fuse-ld=mold cargo +stable"
  [release-threads8]="RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY"
  [release-sccache]="RUSTC_WRAPPER=/opt/bin/sccache cargo +stable"
)

bench() { # json-name runs hyperfine-args...
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
        runonly) eval "$r" > "$out/$wl-relax.out" 2> /dev/null; args+=(-n relax-warm "$r") ;;
        cold) args+=(-n relax --prepare "$R clean" "$r") ;;
        test) "$R" test rust-grokking-ai-algorithms > "$out/relax-test.out" 2>&1; args+=(-n relax --prepare "$edit" "$R test rust-grokking-ai-algorithms") ;;
        check) args+=(-n relax --prepare "$edit" "$R check rust-grokking-ai-algorithms") ;;
      esac
      continue
    fi
    local td="/t/$wl/$name" rel="" prof=debug
    case $name in release*) rel="--release"; prof=release ;; esac
    local p="CARGO_TARGET_DIR=$td ${C[$name]}"
    case $mode in
      edit)
        eval "$p run -q $rel $targs" > "$out/$wl-$name.out" 2> "$out/$wl-$name.err"
        args+=(-n "$name" --prepare "$edit" "$p run -q $rel $targs") ;;
      build)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "$edit" "$p build -q $rel $targs") ;;
      runonly)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" "$td/$prof/$bin") ;;
      cold)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p run -q $rel $targs") ;;
      relcold)
        eval "$p build -q $rel $targs" > /dev/null 2>&1
        args+=(-n "$name" --prepare "rm -rf $td" "$p build -q $rel $targs") ;;
      check)
        eval "$p check -q --all-targets" > /dev/null 2>&1
        args+=(-n "$name" --prepare "$edit" "$p check -q --all-targets") ;;
    esac
  done
  bench "$wl-$mode" "$runs" "${extra[@]}" "${args[@]}"
}

# ---------------------------------------------------------------- measurements
LADDER=(stable-default debuginfo nightly-debuginfo threads8 threads8-cranelift build-all lld mold wild opt1 run-all sccache)
step grokking edit;    run_set grokking edit 10 "${LADDER[@]}" release relax
step grokking build;   run_set grokking build 10 "${LADDER[@]}" release
step grokking runonly; run_set grokking runonly 10 stable-default debuginfo threads8-cranelift build-all opt1 run-all sccache release relax
step grokking cold;    run_set grokking cold 3 "${LADDER[@]}" release relax
step grokking check;   run_set grokking check 10 stable-default threads8 relax
step grokking relcold; run_set grokking relcold 3 release release-mold release-threads8 release-sccache

DEPS=(stable-default debuginfo threads8-cranelift build-all lld mold wild opt1 deps-opt3 build-override run-all sccache)
step cli-deps edit;    run_set cli-deps edit 10 "${DEPS[@]}" release relax
step cli-deps runonly; run_set cli-deps runonly 10 stable-default opt1 deps-opt3 run-all release relax
step cli-deps cold;    run_set cli-deps cold 3 "${DEPS[@]}" release relax
step cli-deps relcold; run_set cli-deps relcold 3 release release-mold release-threads8 release-sccache

step grokking test
cd /work/grokking
nx="nextest run --hide-progress-bar --status-level none --final-status-level none"
for v in "s0|" "opt1|CARGO_PROFILE_DEV_OPT_LEVEL=1"; do
  name=${v%%|*}; env=${v#*|}
  eval "CARGO_TARGET_DIR=/t/test-$name $env cargo +stable test -q" > /dev/null 2>&1
  eval "CARGO_TARGET_DIR=/t/nextest-$name $env cargo +stable $nx" > /dev/null 2>&1
done
eval "CARGO_TARGET_DIR=/t/test-build-all ${C[build-all]} test -q" > /dev/null 2>&1
"$R" test rust-grokking-ai-algorithms > "$out/relax-test.out" 2>&1
bench grokking-test 10 \
  -n stable-default-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=/t/test-s0 cargo +stable test -q" \
  -n stable-default-nextest --prepare "$grok_edit" "CARGO_TARGET_DIR=/t/nextest-s0 cargo +stable $nx" \
  -n opt1-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=/t/test-opt1 CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable test -q" \
  -n opt1-nextest --prepare "$grok_edit" "CARGO_TARGET_DIR=/t/nextest-opt1 CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable $nx" \
  -n build-all-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=/t/test-build-all ${C[build-all]} test -q" \
  -n relax --prepare "$grok_edit" "$R test rust-grokking-ai-algorithms"

step nextest config forwarding
CARGO_TARGET_DIR=/t/fwd cargo +stable --config profile.dev.opt-level=1 nextest list --cargo-verbose 2>&1 \
  | grep -o 'crate-name grokking_ai_algorithms.*' | grep -o 'opt-level=[0-9]' | sort | uniq -c > "$out/nextest-config.txt"
echo "-- env" >> "$out/nextest-config.txt"
CARGO_TARGET_DIR=/t/fwd-env CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo +stable nextest list --cargo-verbose 2>&1 \
  | grep -o 'crate-name grokking_ai_algorithms.*' | grep -o 'opt-level=[0-9]' | sort | uniq -c >> "$out/nextest-config.txt"

# ---------------------------------------------------------------- behaviour
step perf-rounds
cd /work/perf-rounds
CARGO_TARGET_DIR=/t/perf-s0 cargo +stable build -q
CARGO_TARGET_DIR=/t/perf-cl eval "${C[threads8-cranelift]} build -q" 2> /dev/null
CARGO_TARGET_DIR=/t/perf-opt1 eval "${C[opt1]} build -q"
CARGO_TARGET_DIR=/t/perf-rel cargo +stable build -q --release
"$R" run --quiet perf-rounds > /dev/null 2>&1
for i in 1 2 3; do
  /t/perf-s0/debug/perf-rounds > "$out/perf-stable-default-$i.txt"
  /t/perf-cl/debug/perf-rounds > "$out/perf-threads8-cranelift-$i.txt"
  /t/perf-opt1/debug/perf-rounds > "$out/perf-opt1-$i.txt"
  /t/perf-rel/release/perf-rounds > "$out/perf-release-$i.txt"
  "$R" run --quiet perf-rounds > "$out/perf-relax-$i.txt" 2> /dev/null
done

step closure-capture
cd /work/closure-capture
cargo +stable run -q > "$out/closure-cargo.txt" 2>&1
for i in 1 2 3; do "$R" run --quiet closure-capture 2> /dev/null | grep -v '^relax' >> "$out/closure-relax.txt"; done

step borrowck
cd /work/borrowck-bg
cargo +stable run -q > "$out/borrowck-cargo.txt" 2>&1
CALL_BAD=1 WAIT_MS=10000 "$R" run borrowck-bg > "$out/borrowck-relax.out" 2> "$out/borrowck-relax.err"
CALL_BAD=1 timeout 20 "$R" run --check-first borrowck-bg > "$out/borrowck-checkfirst.out" 2> "$out/borrowck-checkfirst.err"
echo "check-first exit=$?" >> "$out/borrowck-checkfirst.out"

step live patch
cd /work/live-counter
src=src/main.rs
"$R" run live-counter > "$out/live-stdout.log" 2> "$out/live-stderr.log" &
lp=$!
for _ in $(seq 1 100); do grep -q 'iter=' "$out/live-stdout.log" 2> /dev/null && break; sleep 0.1; done
sleep 3
: > "$out/live-saves.tsv"
for _ in 1 2 3 4 5; do
  if grep -q 'state.counter += 1;' $src; then from=1; to=100; else from=100; to=1; fi
  sed -i "s/state.counter += $from;/state.counter += $to;/" $src
  printf '%s\tstep+=%s\n' "$(date +%s%3N)" "$to" >> "$out/live-saves.tsv"
  sleep 6
done
kill $lp 2> /dev/null

step done
