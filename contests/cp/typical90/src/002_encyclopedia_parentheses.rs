// 002 - Encyclopedia of Parentheses (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_b
//
// 問題: 長さNの正しいカッコ列を辞書順に全て出力せよ。
//
// 解法: bit全探索 または 再帰
// - N<=20なので全探索可能 (2^20 ≈ 10^6)
// - 各ビット位置で '(' か ')' を選ぶ
// - 正しいカッコ列の条件:
//   1. '(' と ')' の数が同じ (= N/2個ずつ)
//   2. どの位置でも、それまでの '(' の数 >= ')' の数

use proconio::input;

fn main() {
    input! {
        n: usize,
    }
    solve(n);
}

fn solve(n: usize) {
    // Nが奇数なら正しいカッコ列は存在しない
    if n % 2 == 1 {
        return;
    }

    // bit全探索: 0='(', 1=')' とすると辞書順になる
    for mask in 0..(1u64 << n) {
        if is_valid_parentheses(mask, n) {
            let s: String = (0..n)
                .map(|i| if (mask >> i) & 1 == 0 { '(' } else { ')' })
                .collect();
            println!("{}", s);
        }
    }
}

fn is_valid_parentheses(mask: u64, n: usize) -> bool {
    let mut open_count = 0i32;
    let mut close_count = 0i32;

    for i in 0..n {
        if (mask >> i) & 1 == 0 {
            open_count += 1;
        } else {
            close_count += 1;
        }

        // どの時点でも ')' が '(' を超えてはいけない
        if close_count > open_count {
            return false;
        }
    }

    // 最終的に '(' と ')' の数が同じ
    open_count == close_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_n2() {
        // N=2 → "()" のみ
        assert!(is_valid_parentheses(0b10, 2)); // "()"
        assert!(!is_valid_parentheses(0b00, 2)); // "(("
        assert!(!is_valid_parentheses(0b01, 2)); // ")("
        assert!(!is_valid_parentheses(0b11, 2)); // "))"
    }

    #[test]
    fn test_n4() {
        // N=4 → "(())", "()()"
        // bit i: 0='(', 1=')'
        // "(())" → bits: 0,0,1,1 → 0b1100 = 12
        // "()()" → bits: 0,1,0,1 → 0b1010 = 10
        assert!(is_valid_parentheses(0b1100, 4)); // "(())"
        assert!(is_valid_parentheses(0b1010, 4)); // "()()"
    }
}
