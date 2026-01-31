// 074 - ABC String 2 (★6)
// https://atcoder.jp/contests/typical90/tasks/typical90_bv
//
// 操作の本質を理解する:
// - 'b' を 'a' に変え、左側を +1 (mod 3) する
// - 'c' を 'b' に変え、左側を +1 (mod 3) する
//
// これは3進数として見ると:
// 文字列 S を 3進数として解釈した値が答え
// a=0, b=1, c=2 として、S を逆順に読んで 3進数として解釈

use proconio::input;

fn main() {
    input! {
        _n: usize,
        s: String,
    }
    println!("{}", solve(&s));
}

fn solve(s: &str) -> u64 {
    // 文字列を3進数として解釈
    // 最上位桁が左端
    let mut result = 0u64;
    for c in s.chars() {
        let digit = match c {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            _ => unreachable!(),
        };
        result = result * 3 + digit;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve("aba"), 2);
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve("aaaaaaaaaa"), 0);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve("baaca"), 17);
    }
}
