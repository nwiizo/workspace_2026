// 042 - Multiple of 9 (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_ap
//
// 9の倍数の性質: 各桁の和が9の倍数 ⇔ 元の数が9の倍数
//
// よって:
// 1. Kが9の倍数でなければ答えは0
// 2. Kが9の倍数なら、桁の和がKになる場合の数を求める
//
// DP: dp[s] = 桁の合計がsになる正の整数の個数
// dp[0] = 1 (空)
// dp[s] = Σ dp[s-d] for d = 1 to 9

use proconio::input;

const MOD: i64 = 1_000_000_007;

fn main() {
    input! {
        k: usize,
    }
    println!("{}", solve(k));
}

fn solve(k: usize) -> i64 {
    // 9の倍数でなければ0
    if k % 9 != 0 {
        return 0;
    }

    let mut dp = vec![0i64; k + 1];
    dp[0] = 1;

    for s in 1..=k {
        for d in 1..=9 {
            if s >= d {
                dp[s] = (dp[s] + dp[s - d]) % MOD;
            }
        }
    }

    dp[k]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(solve(1), 0);
    }

    #[test]
    fn test_example2() {
        assert_eq!(solve(234), 757186539);
    }

    #[test]
    fn test_k9() {
        // K=9: 9, 18, 27, 36, 45, 54, 63, 72, 81, 111, 112, ...
        // 1桁: 9 (1通り)
        // 2桁: 18, 27, 36, 45, 54, 63, 72, 81 (8通り)
        // 合計を計算
        assert_eq!(solve(9), 256);
    }
}
