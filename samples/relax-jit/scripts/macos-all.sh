#!/usr/bin/env bash
# Relax compared with cargo on macOS arm64 (a platform Relax supports), on
# the same machine as scripts/linux-all.sh, after that run has finished.
# Toolchains and tools live under $BASE and do not touch ~/.rustup or ~/.cargo.
# Expects: ~/relax-bench/src (synced sources), $BASE/bin/{hyperfine,sccache,cargo-nextest},
# and optionally ~/relax-bench/ca/corp.pem (root CA of a TLS-inspecting network).
set -o pipefail

BASE=${BASE:-$HOME/relax-bench/mac}
SRC=$HOME/relax-bench/src
out=$BASE/out
mkdir -p "$out" "$BASE/bin"
exec > >(tee -a "$out/run.log") 2>&1
step() { echo "== [$(date +%T)] $*"; }

export RUSTUP_HOME=$BASE/rustup CARGO_HOME=$BASE/cargo
export PATH=$BASE/cargo/bin:$BASE/bin:/usr/bin:/bin:/usr/sbin:/sbin
export SCCACHE_DIR=$BASE/sccache-cache
if [ -f "$HOME/relax-bench/ca/corp.pem" ]; then
  cat /etc/ssl/cert.pem "$HOME/relax-bench/ca/corp.pem" > "$BASE/ca-bundle.pem"
  export SSL_CERT_FILE=$BASE/ca-bundle.pem CURL_CA_BUNDLE=$BASE/ca-bundle.pem CARGO_HTTP_CAINFO=$BASE/ca-bundle.pem
fi

# ---------------------------------------------------------------- toolchains
step toolchains
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  curl -sSf -o "$BASE/rustup-init" https://static.rust-lang.org/rustup/dist/aarch64-apple-darwin/rustup-init
  chmod +x "$BASE/rustup-init"
  "$BASE/rustup-init" -y -q --no-modify-path --profile minimal --default-toolchain 1.100.0
fi
NIGHTLY=nightly-2026-11-14
rustup toolchain install "$NIGHTLY" --profile minimal --component rustc-codegen-cranelift-preview > /dev/null
{
  sw_vers; sysctl -n machdep.cpu.brand_string hw.ncpu hw.memsize
  rustc +1.100.0 -V; rustc +"$NIGHTLY" -V; ld -v 2>&1 | head -1
  hyperfine --version; sccache --version; cargo-nextest nextest --version | head -1
} > "$out/versions.txt" 2>&1

# ---------------------------------------------------------------- relax
step relax build
[ -d "$BASE/relax" ] || git clone -q https://github.com/makepad/relax "$BASE/relax"
git -C "$BASE/relax" checkout -q c1301db6264986df964cc79ff1f6a9afda3b7fe6
SECONDS=0
cargo +1.100.0 build --release --manifest-path "$BASE/relax/Cargo.toml" -p relax-runner --bin relax > "$out/relax-build.log" 2>&1
echo "relax build exit=$? seconds=$SECONDS" | tee -a "$out/relax-build.log"
R=$BASE/relax/target/release/relax

# ---------------------------------------------------------------- workloads
step workloads
W=$BASE/work T=$BASE/t
rm -rf "$W" "$T"
mkdir -p "$W" "$T"
rsync -a --exclude target "$SRC/grokking/" "$W/grokking/"
printf '\n[workspace]\n' >> "$W/grokking/Cargo.toml"
sed -i '' -E 's/let \(mut low, mut high\) = \(0, values.len\(\)\);/let (mut low, mut high) = (0 * 1, values.len());/' "$W/grokking/src/search.rs"
for p in cli-deps closure-capture borrowck-bg perf-rounds live-counter; do
  rsync -a --exclude target "$SRC/relax-jit/$p/" "$W/$p/"
done
grok_edit="sed -i '' -E 's/\\(0 \\* [0-9]+, values.len\\(\\)\\)/(0 * '\$RANDOM', values.len())/' src/search.rs"
cli_edit="sed -i '' -E 's/lines.len\\(\\) \\+ 0 \\* [0-9]+,/lines.len() + 0 * '\$RANDOM',/' src/main.rs"

DBG="--config 'profile.dev.debug=\"line-tables-only\"' --config 'profile.dev.package.\"*\".debug=false'"
CL="-Zcodegen-backend --config 'profile.dev.codegen-backend=\"cranelift\"'"
FU="-Zfeature-unification --config 'resolver.feature-unification=\"workspace\"'"
OPT1="--config profile.dev.opt-level=1"
D3="--config 'profile.dev.package.\"*\".opt-level=3'"
BO3="--config profile.dev.build-override.opt-level=3"
SC="RUSTC_WRAPPER=$BASE/bin/sccache"
# macOS ships bash 3.2 (no associative arrays), so configurations are a case.
conf() {
  case $1 in
    stable-default) echo "cargo +1.100.0" ;;
    debuginfo) echo "cargo +1.100.0 $DBG" ;;
    opt1) echo "cargo +1.100.0 $OPT1" ;;
    deps-opt3) echo "cargo +1.100.0 $D3" ;;
    build-override) echo "cargo +1.100.0 $BO3" ;;
    run-all) echo "cargo +1.100.0 $DBG $OPT1 $D3 $BO3" ;;
    edit-best-stable) echo "cargo +1.100.0 $DBG $OPT1" ;;
    edit-best) echo "RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG $OPT1" ;;
    edit-best-deps3) echo "RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG $OPT1 $D3" ;;
    threads8) echo "RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG" ;;
    build-all) echo "RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG $CL $FU" ;;
    clean-best) echo "$SC CARGO_INCREMENTAL=0 RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY $DBG $CL" ;;
    sccache) echo "$SC CARGO_INCREMENTAL=0 cargo +1.100.0" ;;
    release) echo "cargo +1.100.0" ;;
    release-best) echo "$SC RUSTFLAGS=-Zthreads=8 cargo +$NIGHTLY" ;;
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
  local targs edit bin rpkg args=() extra=()
  case $wl in
    grokking) targs="--example all_chapters"; edit=$grok_edit; bin=examples/all_chapters; rpkg="rust-grokking-ai-algorithms --example all_chapters" ;;
    cli-deps) targs=""; edit=$cli_edit; bin=cli-deps; rpkg="cli-deps" ;;
  esac
  [ "$mode" = runonly ] && extra=(--warmup 3)
  cd "$W/$wl" || return
  for name in "$@"; do
    if [ "$name" = relax ]; then
      local r="$R run --quiet $rpkg"
      case $mode in
        edit) eval "$r" > "$out/$wl-relax.out" 2> /dev/null; args+=(-n relax --prepare "$edit" "$r") ;;
        runonly) eval "$r" > /dev/null 2>&1; args+=(-n relax-warm "$r") ;;
        cold) args+=(-n relax --prepare "$R clean" "$r") ;;
        check) args+=(-n relax --prepare "$edit" "$R check rust-grokking-ai-algorithms") ;;
      esac
      continue
    fi
    local td="$T/$wl/$name" rel="" prof=debug
    case $name in release*) rel="--release"; prof=release ;; esac
    local p="CARGO_TARGET_DIR=$td $(conf "$name")"
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
step grokking edit;    run_set grokking edit 10 stable-default debuginfo opt1 edit-best-stable threads8 edit-best build-all sccache release relax
step grokking build;   run_set grokking build 10 stable-default debuginfo opt1 edit-best build-all sccache release
step grokking runonly; run_set grokking runonly 10 stable-default opt1 edit-best build-all release relax
step grokking cold;    run_set grokking cold 3 stable-default debuginfo opt1 threads8 build-all clean-best sccache release relax
step grokking check;   run_set grokking check 10 stable-default threads8 relax
step grokking relcold; run_set grokking relcold 3 release release-best
step cli-deps edit;    run_set cli-deps edit 10 stable-default opt1 deps-opt3 run-all edit-best edit-best-deps3 build-all release relax
step cli-deps cold;    run_set cli-deps cold 3 stable-default opt1 deps-opt3 build-override run-all build-all clean-best sccache release relax
step cli-deps relcold; run_set cli-deps relcold 3 release release-best

step grokking test
cd "$W/grokking"
nx="nextest run --hide-progress-bar --status-level none --final-status-level none"
tb_env="RUSTFLAGS=-Zthreads=8 CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only"
eval "CARGO_TARGET_DIR=$T/test-s0 cargo +1.100.0 test -q" > /dev/null 2>&1
eval "CARGO_TARGET_DIR=$T/nextest-s0 cargo +1.100.0 $nx" > /dev/null 2>&1
eval "CARGO_TARGET_DIR=$T/test-best $tb_env cargo +$NIGHTLY $nx" > /dev/null 2>&1
"$R" test rust-grokking-ai-algorithms > "$out/relax-test.out" 2>&1
bench grokking-test 10 \
  -n stable-default-cargo-test --prepare "$grok_edit" "CARGO_TARGET_DIR=$T/test-s0 cargo +1.100.0 test -q" \
  -n stable-default-nextest --prepare "$grok_edit" "CARGO_TARGET_DIR=$T/nextest-s0 cargo +1.100.0 $nx" \
  -n test-best --prepare "$grok_edit" "CARGO_TARGET_DIR=$T/test-best $tb_env cargo +$NIGHTLY $nx" \
  -n relax --prepare "$grok_edit" "$R test rust-grokking-ai-algorithms"

# ---------------------------------------------------------------- behaviour
step perf-rounds
cd "$W/perf-rounds"
eval "CARGO_TARGET_DIR=$T/perf-s0 $(conf stable-default) build -q"
eval "CARGO_TARGET_DIR=$T/perf-opt1 $(conf opt1) build -q"
eval "CARGO_TARGET_DIR=$T/perf-cl $(conf build-all) build -q" 2> /dev/null
eval "CARGO_TARGET_DIR=$T/perf-rel $(conf release) build -q --release"
"$R" run --quiet perf-rounds > /dev/null 2>&1
for i in 1 2 3; do
  "$T/perf-s0/debug/perf-rounds" > "$out/perf-stable-default-$i.txt"
  "$T/perf-opt1/debug/perf-rounds" > "$out/perf-opt1-$i.txt"
  "$T/perf-cl/debug/perf-rounds" > "$out/perf-build-all-$i.txt"
  "$T/perf-rel/release/perf-rounds" > "$out/perf-release-$i.txt"
  "$R" run --quiet perf-rounds > "$out/perf-relax-$i.txt" 2> /dev/null
done

step closure-capture
cd "$W/closure-capture"
cargo +1.100.0 run -q > "$out/closure-cargo.txt" 2>&1
for _ in 1 2 3; do "$R" run --quiet closure-capture 2> /dev/null | grep -v '^relax' >> "$out/closure-relax.txt"; done

step borrowck
cd "$W/borrowck-bg"
cargo +1.100.0 run -q > "$out/borrowck-cargo.txt" 2>&1
CALL_BAD=1 WAIT_MS=10000 "$R" run borrowck-bg > "$out/borrowck-relax.out" 2> "$out/borrowck-relax.err"
CALL_BAD=1 "$R" run --check-first borrowck-bg > "$out/borrowck-checkfirst.out" 2> "$out/borrowck-checkfirst.err" &
cf=$!
sleep 20
kill "$cf" 2> /dev/null && echo "check-first still waiting after 20 s (killed)" >> "$out/borrowck-checkfirst.out"

step live patch
cd "$W/live-counter"
src=src/main.rs
"$R" run live-counter > "$out/live-stdout.log" 2> "$out/live-stderr.log" &
lp=$!
for _ in $(seq 1 100); do grep -q 'iter=' "$out/live-stdout.log" 2> /dev/null && break; sleep 0.1; done
sleep 3
: > "$out/live-saves.tsv"
for _ in 1 2 3 4 5; do
  if grep -q 'state.counter += 1;' $src; then from=1; to=100; else from=100; to=1; fi
  sed -i '' "s/state.counter += $from;/state.counter += $to;/" $src
  printf '%s\tstep+=%s\n' "$(echo "$(stat -f %Fm $src) * 1000 / 1" | bc)" "$to" >> "$out/live-saves.tsv"
  sleep 6
done
kill $lp 2> /dev/null

step done
