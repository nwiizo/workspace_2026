#!/bin/bash
# gen_template.sh - Rust競プロテンプレート生成
#
# Usage: ./gen_template.sh <problem_number> <problem_name>
# Example: ./gen_template.sh 021 sum_of_something

set -e

NUM="${1:?Usage: $0 <number> <name>}"
NAME="${2:?Usage: $0 <number> <name>}"

FILE="src/${NUM}_${NAME}.rs"

if [[ -f "$FILE" ]]; then
    echo "Error: $FILE already exists"
    exit 1
fi

cat > "$FILE" << 'EOF'
// ${NUM} - ${NAME}
// https://atcoder.jp/contests/typical90/tasks/typical90_
//
// 問題:
//
// 解法:

use proconio::input;

fn main() {
    input! {
        n: usize,
    }
    println!("{}", solve(n));
}

fn solve(n: usize) -> i64 {
    // TODO: implement
    n as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // TODO: add test
        assert_eq!(solve(1), 1);
    }
}
EOF

# Replace placeholders
sed -i '' "s/\${NUM}/${NUM}/g" "$FILE"
sed -i '' "s/\${NAME}/${NAME}/g" "$FILE"

# Add to Cargo.toml if not exists
if ! grep -q "name = \"${NUM}\"" Cargo.toml; then
    cat >> Cargo.toml << EOF

[[bin]]
name = "${NUM}"
path = "src/${NUM}_${NAME}.rs"
EOF
    echo "Added to Cargo.toml"
fi

echo "Created: $FILE"
