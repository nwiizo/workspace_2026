// 008 - AtCounter (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_h
//
// 問題: 文字列Sの部分列で "atcoder" となるものの個数を求めよ。
//
// 解法: DP
// - dp[i] = "atcoder" の最初の i 文字を作る方法の数
// - S を左から見ていき、対応する文字が来たら遷移

use proconio::input;
use proconio::marker::Chars;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        _n: usize,
        s: Chars,
    }
    println!("{}", solve(&s));
}

fn solve(s: &[char]) -> i64 {
    let target: Vec<char> = "atcoder".chars().collect();

    // dp[i] = target の最初の i 文字を作る方法の数
    // dp[0] = 1 (空文字列を作る方法は1通り)
    let mut dp = [0i64; 8];
    dp[0] = 1;

    for &c in s {
        // target の各位置について、c が一致するか確認
        // 後ろから更新しないと同じ文字を複数回使ってしまう
        for i in (0..7).rev() {
            if c == target[i] {
                dp[i + 1] = (dp[i + 1] + dp[i]) % MOD;
            }
        }
    }

    dp[7]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // "atcoder" → 1通り
        let s: Vec<char> = "atcoder".chars().collect();
        assert_eq!(solve(&s), 1);
    }

    #[test]
    fn test_example2() {
        // "aattccooddeerr" → 各文字2つずつ → 2^7 = 128通り
        let s: Vec<char> = "aattccooddeerr".chars().collect();
        assert_eq!(solve(&s), 128);
    }

    #[test]
    fn test_no_match() {
        // "xyz" → 0通り
        let s: Vec<char> = "xyz".chars().collect();
        assert_eq!(solve(&s), 0);
    }
}
