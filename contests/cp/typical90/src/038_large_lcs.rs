// 038 - Large LCS (★5)
// https://atcoder.jp/contests/typical90/tasks/typical90_al
//
// 問題: 文字列S, Tの最長共通部分列(LCS)を求めよ。
//
// 解法: DP + 復元
//       dp[i][j] = S[0..i] と T[0..j] のLCS長
//       O(|S| × |T|) だが、復元も必要

use proconio::input;

fn main() {
    input! {
        s: String,
        t: String,
    }
    println!("{}", solve(&s, &t));
}

fn solve(s: &str, t: &str) -> String {
    let s: Vec<char> = s.chars().collect();
    let t: Vec<char> = t.chars().collect();
    let n = s.len();
    let m = t.len();

    // DP
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            if s[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // 復元
    let mut result = Vec::new();
    let (mut i, mut j) = (n, m);

    while i > 0 && j > 0 {
        if s[i - 1] == t[j - 1] {
            result.push(s[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        // S="abcde", T="ace" → LCS="ace"
        let result = solve("abcde", "ace");
        assert_eq!(result, "ace");
    }

    #[test]
    fn example2() {
        // S="abc", T="def" → LCS=""
        let result = solve("abc", "def");
        assert_eq!(result, "");
    }

    #[test]
    fn example3() {
        // S="axyb", T="abyxb" → LCS="axb" or "ayb" or "ab" (length 3)
        let result = solve("axyb", "abyxb");
        assert!(result.len() == 3);
    }

    #[test]
    fn same_string() {
        let result = solve("hello", "hello");
        assert_eq!(result, "hello");
    }
}
