#!/bin/sh
# 品質ゲート、変異テスト、Kani の証明をまとめて実行する。
set -eu

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test

# 全行の例表では、コンパイルできる変異をすべて捕まえる
cargo mutants

# ON 点の行を外すと `>` を `>=` に変えた変異を取り逃す。ここは失敗が期待値
if cargo mutants --features without-border -o "${TMPDIR:-/tmp}"; then
    echo "expected a missed mutant without the border row" >&2
    exit 1
fi

cargo kani
