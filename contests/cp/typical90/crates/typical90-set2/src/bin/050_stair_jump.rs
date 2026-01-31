// 050 - Stair Jump (★3)
// https://atcoder.jp/contests/typical90/tasks/typical90_ax
//
// ============================================================
// 問題の理解
// ============================================================
//
// N段の階段を登る方法の数を求める
// - 1歩で1段または L段上がれる
// - 0段目からスタート
//
// ============================================================
// 解法: 動的計画法 (DP)
// ============================================================
//
// dp[i] = i段目に到達する方法の数
//
// 漸化式:
// - dp[0] = 1 (スタート地点)
// - dp[i] = dp[i-1] + dp[i-L]  (i >= L の場合)
// - dp[i] = dp[i-1]             (i < L の場合)
//
// 計算量: O(N)
//
// ============================================================

use proconio::input;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        n: usize,
        l: usize,
    }
    println!("{}", solve(n, l));
}

fn solve(n: usize, l: usize) -> i64 {
    let mut dp = vec![0i64; n + 1];
    dp[0] = 1;

    for i in 1..=n {
        // 1段上がってくる場合
        dp[i] = dp[i - 1];

        // L段上がってくる場合 (i >= L のとき)
        if i >= l {
            dp[i] = (dp[i] + dp[i - l]) % MOD;
        }
    }

    dp[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        // 1+1+1, 1+2, 2+1 の3通り
        assert_eq!(solve(3, 2), 3);
    }

    #[test]
    fn test_example2() {
        // 1+1+1+1, 4 の2通り
        assert_eq!(solve(4, 4), 2);
    }

    #[test]
    fn test_example3() {
        assert_eq!(solve(5, 2), 8);
    }

    #[test]
    fn test_example4() {
        assert_eq!(solve(6783, 125), 674508908);
    }
}
